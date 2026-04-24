//! 下载事件名与事件发射辅助函数。
//!
//! 该模块定义下载管理器、批次更新与任务进度相关的事件常量，并提供统一的 Tauri
//! 事件发送入口，供下载桥接层在状态变化后向前端广播快照。

#![allow(dead_code)]

use siren_core::download::model::{DownloadJobSnapshot, DownloadManagerSnapshot};
use tauri::{AppHandle, Emitter};

/// 完整下载管理器快照发生变化时发出的事件名。
/// 事件载荷为 `DownloadManagerSnapshot`。
pub(crate) const DOWNLOAD_MANAGER_STATE_CHANGED: &str = "download-manager-state-changed";

/// 任意下载批次被创建、更新或发生状态迁移后发出的事件名。
/// 事件载荷为 `DownloadJobSnapshot`。
pub(crate) const DOWNLOAD_JOB_UPDATED: &str = "download-job-updated";

/// 下载子任务推进过程中的进度事件名。
/// 事件载荷为 `DownloadTaskProgressEvent`。
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
