//! System notification integration for download completion and playback events.

mod cover;
#[cfg(not(target_os = "macos"))]
mod desktop;
#[cfg(target_os = "macos")]
mod macos;

use crate::app_state::AppState;
use crate::player::state::PlayerState;
use siren_core::download::model::{DownloadJobKind, DownloadJobSnapshot, DownloadJobStatus};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

/// Tracks the last notified song CID to prevent duplicate playback notifications.
static LAST_NOTIFIED_SONG: Mutex<Option<String>> = Mutex::new(None);

/// Trigger a system notification when a download job reaches a terminal state.
///
/// Notification content varies by job kind and status:
/// - Single song: "Song Name" / "下载完成"
/// - Album (completed): "Album Name" / "专辑下载完成（N 首歌曲）"
/// - Album (partially failed): "Album Name" / "专辑下载完成（N 首成功，M 首失败）"
pub fn notify_download_completed(app: &AppHandle, job: &DownloadJobSnapshot) {
    let state = app.state::<AppState>();
    let prefs = state.preferences();
    if !prefs.notify_on_download_complete {
        return;
    }

    if !matches!(
        job.status,
        DownloadJobStatus::Completed | DownloadJobStatus::PartiallyFailed
    ) {
        return;
    }

    let (title, body) = match job.kind {
        DownloadJobKind::Song => (job.title.clone(), "下载完成".to_string()),
        DownloadJobKind::Album | DownloadJobKind::Selection => {
            let title = job.title.clone();
            let body = if job.status == DownloadJobStatus::PartiallyFailed {
                format!(
                    "专辑下载完成（{} 首成功，{} 首失败）",
                    job.completed_task_count, job.failed_task_count
                )
            } else {
                format!("专辑下载完成（{} 首歌曲）", job.completed_task_count)
            };
            (title, body)
        }
    };

    eprintln!(
        "[notification] download: title=\"{}\" body=\"{}\" kind={:?} status={:?}",
        title, body, job.kind, job.status
    );

    #[cfg(target_os = "macos")]
    if let Err(error) = macos::show_download(app, &title, &body) {
        eprintln!("[notification] Failed to show download notification: {error}");
    }

    #[cfg(not(target_os = "macos"))]
    if let Err(error) = desktop::show_download(app, title, body) {
        eprintln!("[notification] Failed to show download notification: {error}");
    }
}

/// Trigger a system notification when playback switches to a new song.
///
/// Notification shows: song name (title) and artists (body).
/// Deduplicates by song CID to avoid repeated notifications for the same track.
pub fn notify_playback_changed(app: &AppHandle, player_state: &PlayerState) {
    let app_state = app.state::<AppState>();
    let prefs = app_state.preferences();
    if !prefs.notify_on_playback_change {
        return;
    }

    if !player_state.is_playing {
        return;
    }

    let Some(ref song_cid) = player_state.song_cid else {
        return;
    };

    let last_notified = LAST_NOTIFIED_SONG.lock().unwrap();
    if last_notified.as_ref() == Some(song_cid) {
        return;
    }
    drop(last_notified);

    let title = player_state.song_name.clone().unwrap_or_default();
    let body = if player_state.artists.is_empty() {
        String::new()
    } else {
        player_state.artists.join(", ")
    };
    let cover_url = player_state.cover_url.clone();

    eprintln!(
        "[notification] playback: title=\"{}\" body=\"{}\" cover_url={:?}",
        title, body, cover_url
    );

    let app_for_task = app.clone();
    let song_cid_for_task = song_cid.clone();

    tauri::async_runtime::spawn(async move {
        let cover_path = if let Some(ref url) = cover_url {
            cover::download_to_temp(&app_for_task, url).await
        } else {
            None
        };

        #[cfg(target_os = "macos")]
        if let Err(error) = macos::show_playback(&app_for_task, &title, &body, cover_path.as_ref())
        {
            eprintln!("[notification] Failed to show playback notification: {error}");
        }

        #[cfg(not(target_os = "macos"))]
        if let Err(error) = desktop::show_playback(&app_for_task, title, body, cover_path) {
            eprintln!("[notification] Failed to show playback notification: {error}");
        }

        if let Ok(mut last) = LAST_NOTIFIED_SONG.lock() {
            *last = Some(song_cid_for_task);
        }
    });
}

/// Send a test notification to verify the notification pipeline is working.
pub fn notify_test(app: AppHandle) -> Result<(), String> {
    eprintln!("[notification] send_test_notification called");

    #[cfg(target_os = "macos")]
    {
        macos::show_test(&app)
    }

    #[cfg(not(target_os = "macos"))]
    {
        desktop::show_test(&app)
    }
}
