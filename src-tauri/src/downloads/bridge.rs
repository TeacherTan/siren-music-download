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
use crate::local_inventory::spawn_inventory_scan;
use siren_core::download::model::{DownloadJobKind, DownloadTaskStatus, InternalDownloadTask};
use siren_core::download::worker::{CompletedTaskArtifacts, TaskExecutionResult};
use siren_core::WritePayload;
use siren_core::{album_cover_exists, album_output_dir, download_album_cover};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Called once during `main.rs` setup.  Starts the download execution loop
/// that polls for queued jobs and drives tasks to completion.
pub(crate) fn initialize(app: &AppHandle, state: &AppState) {
    let app = app.clone();
    let state = state.clone();

    tauri::async_runtime::spawn(async move {
        execution_loop(&app, state).await;
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
    task: InternalDownloadTask,
    outcome: TaskExecutionResult,
}

struct StartedJob {
    job_id: String,
    write_tx: tokio::sync::mpsc::Sender<WriteJob>,
    write_worker_handle: tokio::task::JoinHandle<()>,
}

// ─── Main execution loop ─────────────────────────────────────────────────────

/// The main execution loop.
///
/// Runs forever, polling for queued jobs and processing them with a pipelined
/// download/write strategy.  Emits Tauri events at each significant state
/// transition.
async fn execution_loop(app: &AppHandle, state: AppState) {
    let service = Arc::clone(&state.download_service);
    let api = Arc::clone(&state.api);

    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;

        let Some(started_job) = start_job(app, &service).await else {
            continue;
        };

        let mut pending_write: Option<tokio::sync::oneshot::Receiver<WriteResult>> = None;

        loop {
            let task = {
                let mut svc = service.lock().await;
                svc.pop_next_task(&started_job.job_id)
            };

            let Some((task, preparing_snapshot)) = task else {
                break;
            };

            state.persist_download_snapshot(&service.lock().await.manager_snapshot());
            emit_download_job_updated(app, &preparing_snapshot);
            let manager_snapshot = service.lock().await.manager_snapshot();
            emit_download_manager_state_changed(app, &manager_snapshot);

            let task_id = task.id.clone();
            let job_id_clone = started_job.job_id.clone();
            let cancellation_flag = {
                let svc = service.lock().await;
                svc.active_task_cancel_flag(&started_job.job_id, &task_id)
            };
            let out_dir = prepare_task_output_dir(
                &service,
                &api,
                &started_job.job_id,
                &task,
                cancellation_flag.as_ref(),
            )
            .await;
            let download_result =
                run_download_phase(app, &service, &api, &task, &out_dir, cancellation_flag).await;

            match download_result {
                Ok(payload) => {
                    let (result_tx, result_rx) = tokio::sync::oneshot::channel();

                    flush_pending_write(&mut pending_write, &state, app).await;

                    let _ = started_job
                        .write_tx
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
                    let (final_status, output_path_str, error, _) =
                        unpack_task_result(failed_result);

                    let update = {
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

                    if let Some(update) = update {
                        let manager_snapshot = service.lock().await.manager_snapshot();
                        state.persist_download_snapshot(&manager_snapshot);
                        emit_download_job_updated(app, &update.snapshot);
                        emit_download_manager_state_changed(app, &manager_snapshot);
                    }
                }
            }
        }

        flush_pending_write(&mut pending_write, &state, app).await;
        finalize_job(app, &service, started_job).await;
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn start_job(
    app: &AppHandle,
    service: &Arc<tokio::sync::Mutex<siren_core::DownloadService>>,
) -> Option<StartedJob> {
    let job_snapshot = {
        let mut svc = service.lock().await;
        svc.start_next_queued_job()
    }?;

    let manager_snapshot = service.lock().await.manager_snapshot();
    if let Some(state) = app.try_state::<AppState>() {
        state.persist_download_snapshot(&manager_snapshot);
    }
    emit_download_job_updated(app, &job_snapshot);
    emit_download_manager_state_changed(app, &manager_snapshot);

    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<WriteJob>(1);
    let write_worker_handle = tokio::spawn(async move {
        while let Some(job) = write_rx.recv().await {
            let progress_ctx = job.progress_ctx.clone();
            let task_for_write = job.task.clone();
            let task_for_result = job.task;

            let outcome = tokio::task::spawn_blocking(move || {
                task_for_write.execute_write_phase(&job.payload, {
                    let service = progress_ctx.service;
                    let app = progress_ctx.app;
                    move |progress| {
                        let service = Arc::clone(&service);
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let update = {
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
                            if let Some(update) = update {
                                let _ = app.emit(DOWNLOAD_TASK_PROGRESS, &progress);
                                if update.should_persist {
                                    let manager_snapshot = service.lock().await.manager_snapshot();
                                    if let Some(state) = app.try_state::<AppState>() {
                                        state.persist_download_snapshot(&manager_snapshot);
                                    }
                                }
                                emit_download_job_updated(&app, &update.snapshot);
                            }
                        });
                    }
                })
            })
            .await
            .unwrap_or_else(|_| {
                TaskExecutionResult::Failed(siren_core::download::model::DownloadErrorInfo {
                    code: siren_core::download::model::DownloadErrorCode::Internal,
                    message: "Write worker panicked".to_string(),
                    retryable: false,
                    details: None,
                })
            });

            let _ = job.result_tx.send(WriteResult {
                task: task_for_result,
                outcome,
            });
        }
    });

    Some(StartedJob {
        job_id: job_snapshot.id.clone(),
        write_tx,
        write_worker_handle,
    })
}

async fn prepare_task_output_dir(
    service: &Arc<tokio::sync::Mutex<siren_core::DownloadService>>,
    api: &Arc<siren_core::ApiClient>,
    job_id: &str,
    task: &InternalDownloadTask,
    cancellation_flag: Option<&Arc<std::sync::atomic::AtomicBool>>,
) -> PathBuf {
    let (job_kind, base_output_dir) = {
        let svc = service.lock().await;
        let job_snapshot = svc.get_job(job_id);
        let job_kind = job_snapshot
            .as_ref()
            .map(|job| job.kind)
            .unwrap_or(DownloadJobKind::Song);
        let output_dir = svc
            .job_output_dir(job_id)
            .unwrap_or_else(|| ".".to_string());
        (job_kind, PathBuf::from(output_dir))
    };

    let out_dir = match job_kind {
        DownloadJobKind::Song => base_output_dir,
        DownloadJobKind::Album | DownloadJobKind::Selection => {
            album_output_dir(&base_output_dir, &task.album_name)
        }
    };

    if matches!(job_kind, DownloadJobKind::Album) && !album_cover_exists(&out_dir) {
        let _ = tokio::fs::create_dir_all(&out_dir).await;
        if let Ok(album) = api.get_album_detail(&task.album_cid).await {
            let _ = download_album_cover(api.as_ref(), &album, &out_dir, cancellation_flag).await;
        }
    }

    out_dir
}

async fn run_download_phase(
    app: &AppHandle,
    service: &Arc<tokio::sync::Mutex<siren_core::DownloadService>>,
    api: &Arc<siren_core::ApiClient>,
    task: &InternalDownloadTask,
    out_dir: &PathBuf,
    cancellation_flag: Option<Arc<std::sync::atomic::AtomicBool>>,
) -> Result<WritePayload, TaskExecutionResult> {
    let service_for_progress = Arc::clone(service);
    let app_for_progress = app.clone();

    task.execute_download_phase(api.as_ref(), out_dir, cancellation_flag, {
        let service = Arc::clone(&service_for_progress);
        let app = app_for_progress.clone();
        move |progress| {
            let service = Arc::clone(&service);
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let update = {
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
                if let Some(update) = update {
                    let _ = app.emit(DOWNLOAD_TASK_PROGRESS, &progress);
                    if update.should_persist {
                        let manager_snapshot = service.lock().await.manager_snapshot();
                        if let Some(app_state) = app.try_state::<AppState>() {
                            app_state.persist_download_snapshot(&manager_snapshot);
                        }
                    }
                    emit_download_job_updated(&app, &update.snapshot);
                }
            });
        }
    })
    .await
}

async fn flush_pending_write(
    pending_write: &mut Option<tokio::sync::oneshot::Receiver<WriteResult>>,
    state: &AppState,
    app: &AppHandle,
) {
    let Some(rx) = pending_write.take() else {
        return;
    };
    collect_write_result(rx, state, app).await;
}

async fn finalize_job(
    app: &AppHandle,
    service: &Arc<tokio::sync::Mutex<siren_core::DownloadService>>,
    started_job: StartedJob,
) {
    drop(started_job.write_tx);
    let _ = started_job.write_worker_handle.await;

    let snapshot = {
        let mut svc = service.lock().await;
        svc.finish_job(&started_job.job_id)
    };

    if let Some(s) = snapshot {
        let manager_snapshot = service.lock().await.manager_snapshot();
        if let Some(state) = app.try_state::<AppState>() {
            state.persist_download_snapshot(&manager_snapshot);
        }
        emit_download_job_updated(app, &s);
        emit_download_manager_state_changed(app, &manager_snapshot);

        crate::notification::notify_download_completed(app, &s);
    }
}

async fn collect_write_result(
    rx: tokio::sync::oneshot::Receiver<WriteResult>,
    state: &AppState,
    app: &AppHandle,
) {
    let write_result = match rx.await {
        Ok(r) => r,
        Err(_) => return,
    };

    let (final_status, output_path_str, error, completed_artifacts) =
        unpack_task_result(write_result.outcome);

    let update = {
        let mut svc = state.download_service.lock().await;
        svc.update_task_state(
            &write_result.task.job_id,
            &write_result.task.id,
            final_status,
            None,
            None,
            output_path_str.as_deref(),
            error,
        )
    };

    if let Some(artifacts) = completed_artifacts {
        let root_output_dir = state.preferences().output_dir;
        let _ = state
            .local_inventory_provenance_store
            .record_completed_download(
                PathBuf::from(&root_output_dir).as_path(),
                &write_result.task,
                &artifacts,
            )
            .await;
        spawn_inventory_scan(app.clone(), state.clone(), root_output_dir, None);
    }

    if let Some(update) = update {
        let manager_snapshot = state.download_service.lock().await.manager_snapshot();
        state.persist_download_snapshot(&manager_snapshot);
        emit_download_job_updated(app, &update.snapshot);
        emit_download_manager_state_changed(app, &manager_snapshot);
    }
}

fn unpack_task_result(
    result: TaskExecutionResult,
) -> (
    DownloadTaskStatus,
    Option<String>,
    Option<siren_core::download::model::DownloadErrorInfo>,
    Option<CompletedTaskArtifacts>,
) {
    match result {
        TaskExecutionResult::Completed(artifacts) => (
            DownloadTaskStatus::Completed,
            Some(artifacts.output_path.clone()),
            None,
            Some(artifacts),
        ),
        TaskExecutionResult::Cancelled => (
            DownloadTaskStatus::Cancelled,
            None,
            Some(siren_core::download::model::DownloadErrorInfo {
                code: siren_core::download::model::DownloadErrorCode::Cancelled,
                message: "Cancelled by user".to_string(),
                retryable: false,
                details: None,
            }),
            None,
        ),
        TaskExecutionResult::Failed(info) => (DownloadTaskStatus::Failed, None, Some(info), None),
    }
}
