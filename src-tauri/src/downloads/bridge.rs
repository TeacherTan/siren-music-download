//! Bridge between `siren-core` download service and Tauri events.
//!
//! This module wires the download execution loop to Tauri's event system so
//! that the frontend can subscribe to download progress via events.

use crate::app_state::AppState;
use crate::downloads::events::{
    emit_download_job_updated, emit_download_manager_state_changed, DOWNLOAD_TASK_PROGRESS,
};
use siren_core::download::model::{DownloadJobKind, DownloadTaskStatus};
use siren_core::download::worker::TaskExecutionResult;
use siren_core::{album_cover_exists, album_output_dir, download_album_cover};
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

/// The main execution loop.
///
/// Runs forever, polling for queued jobs and processing them serially.
/// Emits Tauri events at each significant state transition.
async fn execution_loop(
    app: &AppHandle,
    service: Arc<tokio::sync::Mutex<siren_core::DownloadService>>,
    api: Arc<siren_core::ApiClient>,
) {
    loop {
        // Small delay between cycles to avoid busy-spinning
        tokio::time::sleep(Duration::from_millis(500)).await;

        let job_snapshot = {
            let mut svc = service.lock().await;
            svc.start_next_queued_job()
        };

        let Some(job_snapshot) = job_snapshot else {
            continue;
        };

        // A job has been claimed — emit its initial running state
        let job_id = job_snapshot.id.clone();
        emit_download_job_updated(app, &job_snapshot);
        let manager_snapshot = service.lock().await.manager_snapshot();
        emit_download_manager_state_changed(app, &manager_snapshot);

        // Process all tasks in the job serially
        loop {
            let task = {
                let mut svc = service.lock().await;
                svc.pop_next_task(&job_id)
            };

            let Some((task, preparing_snapshot)) = task else {
                // No more queued tasks — finish the job
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
                (job_kind, std::path::PathBuf::from(output_dir))
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

            // Clone Arcs before moving into the closure
            let service_for_progress = Arc::clone(&service);
            let app_for_progress = app.clone();
            let api_for_exec = Arc::clone(&api);

            // Spawn the actual download
            let result = task
                .execute(api_for_exec.as_ref(), &out_dir, cancellation_flag, {
                    move |progress| {
                        // Also update task state in service. Emit only if the update was accepted;
                        // cancelled jobs/tasks reject stale progress so the UI does not regress.
                        let service = Arc::clone(&service_for_progress);
                        let app = app_for_progress.clone();
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

            // Update task with final state
            let (final_status, output_path_str, error) = match result {
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
            };

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

        // All tasks done — finalize the job
        let snapshot = {
            let mut svc = service.lock().await;
            svc.finish_job(&job_id)
        };

        if let Some(s) = snapshot {
            emit_download_job_updated(app, &s);
            let manager_snapshot = service.lock().await.manager_snapshot();
            emit_download_manager_state_changed(app, &manager_snapshot);
        }
    }
}
