use crate::player::state::PlayerState;
use anyhow::{Context, Result};
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};
use std::ffi::c_void;
use std::sync::Mutex;
use std::time::Duration;
use tauri::AppHandle;
#[cfg(target_os = "windows")]
use tauri::Manager;

/// 系统媒体会话封装。
pub struct MediaSession {
    controls: Mutex<MediaControls>,
    last_progress_second: Mutex<Option<u64>>,
}

impl MediaSession {
    /// 创建新的系统媒体会话实例。
    pub fn new(app: &AppHandle) -> Result<Self> {
        let controls = MediaControls::new(platform_config(app)?)
            .context("Failed to create system media controls")?;
        Ok(Self {
            controls: Mutex::new(controls),
            last_progress_second: Mutex::new(None),
        })
    }

    /// 绑定系统媒体控制事件处理器。
    pub fn attach<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(MediaControlEvent) + Send + 'static,
    {
        self.controls
            .lock()
            .unwrap()
            .attach(handler)
            .context("Failed to attach system media controls handler")
    }

    /// 把当前播放器状态同步到系统媒体会话。
    pub fn sync_state(&self, state: &PlayerState, progress_only: bool) {
        let mut controls = self.controls.lock().unwrap();

        if state.song_cid.is_none() {
            *self.last_progress_second.lock().unwrap() = None;
            let _ = controls.set_playback(MediaPlayback::Stopped);
            let _ = controls.set_metadata(MediaMetadata::default());
            return;
        }

        let artist = (!state.artists.is_empty()).then(|| state.artists.join(", "));

        if !progress_only {
            let _ = controls.set_metadata(MediaMetadata {
                title: state.song_name.as_deref(),
                album: None,
                artist: artist.as_deref(),
                cover_url: state.cover_url.as_deref(),
                duration: duration_from_secs(state.duration),
            });
        }

        let progress = duration_from_secs(state.progress).map(MediaPosition);
        let progress_second = progress.map(|value| value.0.as_secs());

        {
            let mut last_progress_second = self.last_progress_second.lock().unwrap();
            if progress_only && state.is_playing && *last_progress_second == progress_second {
                return;
            }
            *last_progress_second = progress_second;
        }

        let playback = if state.is_playing {
            MediaPlayback::Playing { progress }
        } else if state.is_paused {
            MediaPlayback::Paused { progress }
        } else {
            MediaPlayback::Stopped
        };

        let _ = controls.set_playback(playback);
    }
}

fn duration_from_secs(value: f64) -> Option<Duration> {
    if value.is_finite() && value > 0.0 {
        Some(Duration::from_secs_f64(value.max(0.0)))
    } else {
        None
    }
}

fn platform_config(app: &AppHandle) -> Result<PlatformConfig<'static>> {
    Ok(PlatformConfig {
        display_name: "Siren Music Download",
        dbus_name: "com.siren.musicdownload",
        hwnd: platform_hwnd(app)?,
    })
}

#[cfg(target_os = "windows")]
fn platform_hwnd(app: &AppHandle) -> Result<Option<*mut c_void>> {
    let window = app
        .get_webview_window("main")
        .context("Failed to locate main window for media controls")?;
    Ok(Some(window.hwnd()?.0 as *mut c_void))
}

#[cfg(not(target_os = "windows"))]
fn platform_hwnd(_app: &AppHandle) -> Result<Option<*mut c_void>> {
    Ok(None)
}
