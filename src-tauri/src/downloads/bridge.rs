//! Bridge between `siren-core` download service and Tauri events.
//!
//! This module wires the download execution loop to Tauri's event system so
//! that the frontend can subscribe to download progress via events.
//!
//! ## Pipeline architecture
//!
//! Tasks within a job are executed in a two-phase pipeline:
//!
//! ```text
//! Song N:    [download (network)] ──► channel ──► [write (disk I/O)]
//! Song N+1:                        [download]  ──► channel ──► [write]
//! ```
//!
//! The download phase (HTTP fetching) and write phase (disk I/O, FLAC tagging,
//! lyric sidecar) overlap so that song N+1's download can start while song N
//! is still being written.  A bounded `tokio::sync::mpsc` channel provides
//! back-pressure: at most one completed [`WritePayload`] waits in the buffer,
//! keeping peak memory to roughly two songs' worth of audio data.

use crate::app_state::AppState;
use crate::downloads::events::{
    emit_download_job_updated, emit_download_manager_state_changed, DOWNLOAD_TASK_PROGRESS,
};
use siren_core::download::model::{DownloadJobKind, DownloadTaskStatus, InternalDownloadTask};
use siren_core::download::worker::TaskExecutionResult;
use siren_core::WritePayload;
use siren_core::{album_cover_exists, album_output_dir, download_album_cover};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Called once during `main.rs` setup.  Starts the download execution loop
/// that polls for queued jobs and drives tasks to completion.
pub(crate) fn initialize(app: &AppHandle, state: &AppState) {
    let app = app.clone();
    let service = Arc::clone(&state.download_service);
    let api = Arc::clone(&state.api);

    tauri::async_runtime::spawn(async move {
        execution_loop(&app, service, api).await;
    });
}

// ─── Write worker types ──────────────────────────────────────────────────────

/// A job sent through the write channel to the write worker.
struct WriteJob {
    /// The task metadata (needed for `execute_write_phase`).
    task: InternalDownloadTask,
    /// The payload produced by the download phase.
    payload: WritePayload,
    /// Channel to send the write result back to the pipeline driver.
    result_tx: tokio::sync::oneshot::Sender<WriteResult>,
    /// Progress callback dependencies.
    progress_ctx: WriteProgressCtx,
}

/// Everything the write worker needs to emit progress events.
#[derive(Clone)]
struct WriteProgressCtx {
    service: Arc<tokio::sync::Mutex<siren_core::DownloadService>>,
    app: AppHandle,
}

/// Result of a write operation.
struct WriteResult {
    task_id: String,
    job_id: String,
    outcome: TaskExecutionResult,
}

// ─── Main execution loop ─────────────────────────────────────────────────────

/// The main execution loop.
///
/// Runs forever, polling for queued jobs and processing them with a pipelined
/// download/write strategy.  Emits Tauri events at each significant state
/// transition.
async fn execution_loop(
    app: &AppHandle,
    service: Arc<tokio::sync::Mutex<siren_core::DownloadService>>,
    api: Arc<siren_core::ApiClient>,
) {
    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;

        let job_snapshot = {
            let mut svc = service.lock().await;
            svc.start_next_queued_job()
        };

        let Some(job_snapshot) = job_snapshot else {
            continue;
        };

        let job_id = job_snapshot.id.clone();
        emit_download_job_updated(app, &job_snapshot);
        let manager_snapshot = service.lock().await.manager_snapshot();
        emit_download_manager_state_changed(app, &manager_snapshot);

        // Spawn a write worker for this job.  Channel capacity = 1 so at most
        // one WritePayload sits in the buffer (back-pressure).
        let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<WriteJob>(1);

        let write_worker_handle = tokio::spawn(async move {
            while let Some(job) = write_rx.recv().await {
                let task_id = job.task.id.clone();
                let job_id = job.task.job_id.clone();

                let progress_ctx = job.progress_ctx.clone();
                let task_for_write = job.task;

                // Execute write on a blocking thread pool — disk I/O + FLAC
                // encoding can be CPU-intensive.
                let outcome = tokio::task::spawn_blocking(move || {
                    task_for_write.execute_write_phase(&job.payload, {
                        let service = progress_ctx.service;
                        let app = progress_ctx.app;
                        move |progress| {
                            let service = Arc::clone(&service);
                            let app = app.clone();
                            // Fire-and-forget progress update (same pattern as before)
                            tauri::async_runtime::spawn(async move {
                                let snapshot = {
                                    let mut svc = service.lock().await;
                                    svc.update_task_state(
                                        &progress.job_id,
                                        &progress.task_id,
                                        progress.status,
                                        Some(progress.bytes_done),
                                        progress.bytes_total,
                                        None,
                                        None,
                                    )
                                };
                                if let Some(s) = snapshot {
                                    let _ = app.emit(DOWNLOAD_TASK_PROGRESS, &progress);
                                    emit_download_job_updated(&app, &s);
                                }
                            });
                        }
                    })
                })
                .await
                .unwrap_or_else(|e| {
                    TaskExecutionResult::Failed(siren_core::download::model::DownloadErrorInfo {
                        code: siren_core::download::model::DownloadErrorCode::Internal,
                        message: "Write worker panicked".to_string(),
                        retryable: false,
                        details: Some(e.to_string()),
                    })
                });

                let _ = job.result_tx.send(WriteResult {
                    task_id,
                    job_id,
                    outcome,
                });
            }
        });

        // Track the pending write from the previous task.
        let mut pending_write: Option<tokio::sync::oneshot::Receiver<WriteResult>> = None;

        loop {
            let task = {
                let mut svc = service.lock().await;
                svc.pop_next_task(&job_id)
            };

            let Some((task, preparing_snapshot)) = task else {
                break;
            };

            emit_download_job_updated(app, &preparing_snapshot);
            let manager_snapshot = service.lock().await.manager_snapshot();
            emit_download_manager_state_changed(app, &manager_snapshot);

            let task_id = task.id.clone();
            let job_id_clone = job_id.clone();

            let cancellation_flag = {
                let svc = service.lock().await;
                svc.active_task_cancel_flag(&job_id, &task_id)
            };

            let (job_kind, base_output_dir) = {
                let svc = service.lock().await;
                let job_snapshot = svc.get_job(&job_id);
                let job_kind = job_snapshot
                    .as_ref()
                    .map(|job| job.kind)
                    .unwrap_or(DownloadJobKind::Song);
                let output_dir = svc
                    .job_output_dir(&job_id)
                    .unwrap_or_else(|| ".".to_string());
                (job_kind, PathBuf::from(output_dir))
            };

            let out_dir = match job_kind {
                DownloadJobKind::Song => base_output_dir.clone(),
                DownloadJobKind::Album | DownloadJobKind::Selection => {
                    album_output_dir(&base_output_dir, &task.album_name)
                }
            };

            if matches!(job_kind, DownloadJobKind::Album) && !album_cover_exists(&out_dir) {
                let _ = tokio::fs::create_dir_all(&out_dir).await;
                if let Ok(album) = api.get_album_detail(&task.album_cid).await {
                    let _ = download_album_cover(
                        api.as_ref(),
                        &album,
                        &out_dir,
                        cancellation_flag.as_ref(),
                    )
                    .await;
                }
            }

            let service_for_progress = Arc::clone(&service);
            let app_for_progress = app.clone();

            // Phase 1: download (network I/O)
            let download_result = task
                .execute_download_phase(api.as_ref(), &out_dir, cancellation_flag, {
                    let service = Arc::clone(&service_for_progress);
                    let app = app_for_progress.clone();
                    move |progress| {
                        let service = Arc::clone(&service);
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let snapshot = {
                                let mut svc = service.lock().await;
                                svc.update_task_state(
                                    &progress.job_id,
                                    &progress.task_id,
                                    progress.status,
                                    Some(progress.bytes_done),
                                    progress.bytes_total,
                                    None,
                                    None,
                                )
                            };
                            if let Some(s) = snapshot {
                                let _ = app.emit(DOWNLOAD_TASK_PROGRESS, &progress);
                                emit_download_job_updated(&app, &s);
                            }
                        });
                    }
                })
                .await;

            match download_result {
                Ok(payload) => {
                    // Send this task's write to the worker.  If the channel is
                    // full (capacity=1), this `.send()` awaits until the worker
                    // finishes the previous write — natural back-pressure.
                    let (result_tx, result_rx) = tokio::sync::oneshot::channel();

                    // Before sending, collect any completed previous write.
                    if let Some(prev_rx) = pending_write.take() {
                        collect_write_result(prev_rx, &service, app).await;
                    }

                    let _ = write_tx
                        .send(WriteJob {
                            task: task.clone(),
                            payload,
                            result_tx,
                            progress_ctx: WriteProgressCtx {
                                service: Arc::clone(&service),
                                app: app.clone(),
                            },
                        })
                        .await;

                    pending_write = Some(result_rx);
                }
                Err(failed_result) => {
                    // Download phase failed — update task state immediately.
                    let (final_status, output_path_str, error) = unpack_task_result(failed_result);

                    let snapshot = {
                        let mut svc = service.lock().await;
                        svc.update_task_state(
                            &job_id_clone,
                            &task_id,
                            final_status,
                            None,
                            None,
                            output_path_str.as_deref(),
                            error,
                        )
                    };

                    if let Some(s) = snapshot {
                        emit_download_job_updated(app, &s);
                        let manager_snapshot = service.lock().await.manager_snapshot();
                        emit_download_manager_state_changed(app, &manager_snapshot);
                    }
                }
            }
        }

        // Collect the last pending write, if any.
        if let Some(prev_rx) = pending_write.take() {
            collect_write_result(prev_rx, &service, app).await;
        }

        // Drop the sender so the write worker shuts down.
        drop(write_tx);
        let _ = write_worker_handle.await;

        // All tasks done — finalize the job.
        let snapshot = {
            let mut svc = service.lock().await;
            svc.finish_job(&job_id)
        };

        if let Some(s) = snapshot {
            emit_download_job_updated(app, &s);
            let manager_snapshot = service.lock().await.manager_snapshot();
            emit_download_manager_state_changed(app, &manager_snapshot);

            crate::notification::notify_download_completed(app, &s);
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Wait for a write result and apply the final task state.
async fn collect_write_result(
    rx: tokio::sync::oneshot::Receiver<WriteResult>,
    service: &Arc<tokio::sync::Mutex<siren_core::DownloadService>>,
    app: &AppHandle,
) {
    let write_result = match rx.await {
        Ok(r) => r,
        Err(_) => {
            // Channel closed — write worker crashed or was dropped.
            return;
        }
    };

    let (final_status, output_path_str, error) = unpack_task_result(write_result.outcome);

    let snapshot = {
        let mut svc = service.lock().await;
        svc.update_task_state(
            &write_result.job_id,
            &write_result.task_id,
            final_status,
            None,
            None,
            output_path_str.as_deref(),
            error,
        )
    };

    if let Some(s) = snapshot {
        emit_download_job_updated(app, &s);
        let manager_snapshot = service.lock().await.manager_snapshot();
        emit_download_manager_state_changed(app, &manager_snapshot);
    }
}

/// Decompose a [`TaskExecutionResult`] into the triple needed by
/// `update_task_state`.
fn unpack_task_result(
    result: TaskExecutionResult,
) -> (
    DownloadTaskStatus,
    Option<String>,
    Option<siren_core::download::model::DownloadErrorInfo>,
) {
    match result {
        TaskExecutionResult::Completed { output_path } => {
            (DownloadTaskStatus::Completed, Some(output_path), None)
        }
        TaskExecutionResult::Cancelled => (
            DownloadTaskStatus::Cancelled,
            None,
            Some(siren_core::download::model::DownloadErrorInfo {
                code: siren_core::download::model::DownloadErrorCode::Cancelled,
                message: "Cancelled by user".to_string(),
                retryable: false,
                details: None,
            }),
        ),
        TaskExecutionResult::Failed(info) => (DownloadTaskStatus::Failed, None, Some(info)),
    }
}
