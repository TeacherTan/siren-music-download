//! 塞壬音乐下载器的 Tauri 桌面后端。
//!
//! 这个二进制 crate 通过 Tauri 命令和播放器事件向前端暴露后端能力。
//!
//! # 命令面
//!
//! Svelte 前端通过 `@tauri-apps/api/core::invoke` 调用下面这些命令：
//!
//! - 目录数据：[`commands::get_albums`]、[`commands::get_album_detail`]、
//!   [`commands::get_song_detail`]、[`commands::get_song_lyrics`]
//! - 播放控制：[`commands::play_song`]、[`commands::pause_playback`]、
//!   [`commands::resume_playback`]、[`commands::seek_current_playback`]、
//!   [`commands::play_next`]、[`commands::play_previous`]、
//!   [`commands::stop_playback`]、[`commands::get_player_state`]、
//!   [`commands::set_playback_volume`]
//! - 下载和工具：[`commands::get_default_output_dir`]、
//!   [`commands::clear_audio_cache`]、[`commands::extract_image_theme`]
//!
//! # 事件
//!
//! - [`player::events::PLAYER_STATE_CHANGED`] 会在播放状态、队列能力或音量
//!   变化时发出完整的 [`player::PlayerState`] 快照。
//! - [`player::events::PLAYER_PROGRESS`] 会在播放推进过程中持续发出完整的
//!   [`player::PlayerState`] 快照。
//!
//! # 生成 rustdoc
//!
//! 因为 Tauri 命令定义在二进制目标里，请使用：
//!
//! ```bash
//! cargo doc -p siren-music-download --bin siren-music-download --no-deps --document-private-items
//! ```

mod app_state;
mod audio_cache;
mod commands;
mod downloads;
mod local_inventory;
mod local_inventory_provenance;
mod logging;
mod notification;
mod player;
mod preferences;
mod theme;

use anyhow::Context;
use app_state::AppState;
use logging::{LogLevel, LogPayload};
use tauri::{LogicalSize, Manager, RunEvent, WebviewWindow};

const PLAYER_BAR_SAFE_WINDOW_WIDTH: f64 = 1120.0;
const MIN_LAYOUT_WINDOW_WIDTH: f64 = 1120.0;
const DEFAULT_WINDOW_HEIGHT: f64 = 800.0;
const MIN_WINDOW_HEIGHT: f64 = 600.0;
const WINDOW_MARGIN_X: f64 = 48.0;
const WINDOW_MARGIN_Y: f64 = 72.0;

fn fit_main_window_to_monitor<R: tauri::Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let monitor = window.current_monitor()?.or(window.primary_monitor()?);
    let Some(monitor) = monitor else {
        return Ok(());
    };

    let work_area = monitor.work_area();
    let scale_factor = monitor.scale_factor().max(1.0);
    let available_width = work_area.size.width as f64 / scale_factor;
    let available_height = work_area.size.height as f64 / scale_factor;
    if available_width <= 0.0 || available_height <= 0.0 {
        return Ok(());
    }

    let max_width = if available_width > WINDOW_MARGIN_X {
        available_width - WINDOW_MARGIN_X
    } else {
        available_width
    };
    let max_height = if available_height > WINDOW_MARGIN_Y {
        available_height - WINDOW_MARGIN_Y
    } else {
        available_height
    };

    let width = PLAYER_BAR_SAFE_WINDOW_WIDTH.min(max_width).round();
    let height = DEFAULT_WINDOW_HEIGHT.min(max_height).round();
    let min_width = MIN_LAYOUT_WINDOW_WIDTH.min(width).round();
    let min_height = MIN_WINDOW_HEIGHT.min(height).round();

    window.set_min_size(Some(LogicalSize::new(min_width, min_height)))?;
    window.set_size(LogicalSize::new(width, height))?;
    window.center()?;

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let _ =
                tauri_plugin_notification::NotificationExt::notification(app).request_permission();
            let window = app
                .get_webview_window("main")
                .context("Failed to locate main window")?;
            let state =
                AppState::new(app.handle().clone()).expect("Failed to initialize app state");
            if let Err(error) = fit_main_window_to_monitor(&window) {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "window",
                        "window.fit_monitor_failed",
                        "Failed to fit main window to monitor",
                    )
                    .details(error.to_string()),
                );
            }
            let media_state = state.clone();
            if let Err(error) = state
                .player
                .bind_media_controls(move |event| media_state.handle_media_control(event))
            {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.bind_failed",
                        "Failed to bind media controls",
                    )
                    .details(format!("{error:#}")),
                );
            }
            downloads::bridge::initialize(app.handle(), &state);
            local_inventory::spawn_inventory_scan(
                app.handle().clone(),
                state.clone(),
                state.preferences().output_dir,
                None,
            );
            app.manage(state);

            #[cfg(debug_assertions)]
            {
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::library::get_albums,
            commands::library::get_album_detail,
            commands::library::get_song_detail,
            commands::library::get_song_lyrics,
            commands::library::extract_image_theme,
            commands::library::get_image_data_url,
            commands::library::get_default_output_dir,
            commands::playback::play_song,
            commands::playback::stop_playback,
            commands::playback::pause_playback,
            commands::playback::resume_playback,
            commands::playback::seek_current_playback,
            commands::playback::play_next,
            commands::playback::play_previous,
            commands::playback::get_player_state,
            commands::playback::set_playback_volume,
            commands::preferences::get_preferences,
            commands::preferences::set_preferences,
            commands::preferences::export_preferences,
            commands::preferences::import_preferences,
            commands::local_inventory::get_local_inventory_snapshot,
            commands::local_inventory::rescan_local_inventory,
            commands::local_inventory::cancel_local_inventory_scan,
            commands::preferences::get_notification_permission_state,
            commands::preferences::send_test_notification,
            commands::logging::list_log_records,
            commands::logging::get_log_file_status,
            commands::downloads::clear_audio_cache,
            commands::downloads::clear_response_cache,
            commands::downloads::create_download_job,
            commands::downloads::list_download_jobs,
            commands::downloads::get_download_job,
            commands::downloads::cancel_download_job,
            commands::downloads::cancel_download_task,
            commands::downloads::retry_download_job,
            commands::downloads::retry_download_task,
            commands::downloads::clear_download_history,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            RunEvent::ExitRequested { .. } | RunEvent::Exit => {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    let threshold =
                        LogLevel::parse(&state.preferences().log_level).unwrap_or(LogLevel::Error);
                    if let Err(error) = state.log_center.flush_session_to_persistent(threshold) {
                        eprintln!("[logging] failed to flush session logs: {error:#}");
                    }
                }
            }
            _ => {}
        });
}
