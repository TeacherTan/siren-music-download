#![allow(dead_code)]

use siren_core::download::model::{DownloadJobSnapshot, DownloadManagerSnapshot};
use tauri::{AppHandle, Emitter};

/// Emitted whenever the full download manager snapshot changes.
/// Payload: `DownloadManagerSnapshot`.
pub(crate) const DOWNLOAD_MANAGER_STATE_CHANGED: &str = "download-manager-state-changed";

/// Emitted after any job is created, updated, or transitioned state.
/// Payload: `DownloadJobSnapshot`.
pub(crate) const DOWNLOAD_JOB_UPDATED: &str = "download-job-updated";

/// Emitted during download progress of a task.
/// Payload: `DownloadTaskProgressEvent`.
pub(crate) const DOWNLOAD_TASK_PROGRESS: &str = "download-task-progress";

pub(crate) fn emit_download_manager_state_changed(
    app: &AppHandle,
    snapshot: &DownloadManagerSnapshot,
) {
    let _ = app.emit(DOWNLOAD_MANAGER_STATE_CHANGED, snapshot);
}

pub(crate) fn emit_download_job_updated(app: &AppHandle, snapshot: &DownloadJobSnapshot) {
    let _ = app.emit(DOWNLOAD_JOB_UPDATED, snapshot);
}
