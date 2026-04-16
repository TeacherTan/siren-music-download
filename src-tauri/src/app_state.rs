use crate::audio_cache;
use crate::player::stream::{GrowingFileHandle, PlaybackInput, SampleBuffer};
use crate::player::{AudioPlayer, PlaybackContext, PlaybackQueueEntry};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use siren_core::DownloadService;
use siren_core::OutputFormat;
use souvlaki::{MediaControlEvent, SeekDirection};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NotificationPreferences {
    pub(crate) notify_on_download_complete: bool,
    pub(crate) notify_on_playback_change: bool,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            notify_on_download_complete: true,
            notify_on_playback_change: true,
        }
    }
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) player: Arc<AudioPlayer>,
    pub(crate) api: Arc<siren_core::ApiClient>,
    pub(crate) download_service: Arc<Mutex<DownloadService>>,
    pub(crate) notification_preferences: Arc<StdMutex<NotificationPreferences>>,
}

impl AppState {
    pub(crate) fn new(app: tauri::AppHandle) -> Result<Self, String> {
        let player = AudioPlayer::new(app).map_err(|e| e.to_string())?;
        let api = siren_core::ApiClient::new().map_err(|e| e.to_string())?;
        let download_service = Arc::new(Mutex::new(DownloadService::new()));
        let notification_preferences = Arc::new(StdMutex::new(NotificationPreferences::default()));
        Ok(Self {
            player: Arc::new(player),
            api: Arc::new(api),
            download_service,
            notification_preferences,
        })
    }

    pub(crate) fn notification_preferences(&self) -> NotificationPreferences {
        self.notification_preferences.lock().unwrap().clone()
    }

    pub(crate) fn set_notification_preferences(&self, preferences: NotificationPreferences) {
        *self.notification_preferences.lock().unwrap() = preferences;
    }

    pub(crate) async fn play_song_internal(
        &self,
        song_cid: String,
        cover_url: Option<String>,
        playback_context: Option<PlaybackContext>,
    ) -> Result<f64, String> {
        let song_detail = self
            .api
            .get_song_detail(&song_cid)
            .await
            .map_err(|e| e.to_string())?;

        self.player.prepare_playback_context(
            playback_context,
            PlaybackQueueEntry {
                cid: song_cid.clone(),
                name: song_detail.name.clone(),
                artists: song_detail.artists.clone(),
                cover_url: cover_url.clone(),
            },
        );

        let session_id = self
            .player
            .begin_loading_session(
                song_cid.clone(),
                song_detail.name.clone(),
                song_detail.artists.clone(),
                cover_url,
                0.0,
                None,
            )
            .map_err(|e| e.to_string())?;

        let result: Result<f64> = async {
            self.start_playback_session(session_id, &song_cid, &song_detail.source_url, 0.0)
                .await
        }
        .await;

        match result {
            Ok(duration) => Ok(duration),
            Err(error) => {
                self.player.fail_session(session_id);
                Err(error.to_string())
            }
        }
    }

    pub(crate) async fn seek_current_internal(&self, position_secs: f64) -> Result<f64, String> {
        let current_state = self.player.get_state();
        let song_cid = current_state
            .song_cid
            .clone()
            .ok_or_else(|| "No active track".to_string())?;

        if current_state.is_loading {
            return Err("Playback is still loading".to_string());
        }

        let target_position = normalize_seek_position(position_secs, current_state.duration);
        if (current_state.progress - target_position).abs() < 0.05 {
            return Ok(current_state.duration);
        }

        let song_detail = self
            .api
            .get_song_detail(&song_cid)
            .await
            .map_err(|e| e.to_string())?;

        let session_id = self
            .player
            .begin_loading_session(
                song_cid.clone(),
                song_detail.name.clone(),
                song_detail.artists.clone(),
                current_state.cover_url.clone(),
                target_position,
                (current_state.duration > 0.0).then_some(current_state.duration),
            )
            .map_err(|e| e.to_string())?;

        let should_pause_after_seek = current_state.is_paused;
        let result: Result<f64> = async {
            let duration = self
                .start_playback_session(
                    session_id,
                    &song_cid,
                    &song_detail.source_url,
                    target_position,
                )
                .await?;

            if should_pause_after_seek {
                self.player.pause()?;
            }

            Ok(duration)
        }
        .await;

        match result {
            Ok(duration) => Ok(duration),
            Err(error) => {
                self.player.fail_session(session_id);
                Err(error.to_string())
            }
        }
    }

    async fn start_playback_session(
        &self,
        session_id: u64,
        song_cid: &str,
        source_url: &str,
        start_position_secs: f64,
    ) -> Result<f64> {
        let stop_flag = self.player.stop_signal();
        let pause_flag = self.player.pause_signal();
        let cache_path = audio_cache::cached_song_path(song_cid, source_url)?;
        let pending_marker = audio_cache::pending_marker_path(&cache_path);

        let input = if audio_cache::is_song_cached(&cache_path) {
            PlaybackInput::cached_file(cache_path)
        } else {
            let _ = std::fs::remove_file(&cache_path);
            let _ = std::fs::remove_file(&pending_marker);
            std::fs::write(&pending_marker, b"pending").with_context(|| {
                format!("Failed to create cache marker {}", pending_marker.display())
            })?;

            let (handle, mut writer) = GrowingFileHandle::new(cache_path.clone())?;
            let api = Arc::clone(&self.api);
            let stop_for_download = Arc::clone(&stop_flag);
            let handle_for_download = handle.clone();
            let source_url = source_url.to_string();
            let cache_path_for_cleanup = cache_path.clone();
            let pending_for_cleanup = pending_marker.clone();

            tokio::spawn(async move {
                let download_result = api
                    .download_stream(&source_url, |chunk, _, _| {
                        if stop_for_download.load(Ordering::SeqCst) {
                            return Ok(false);
                        }
                        handle_for_download.append_chunk(&mut writer, chunk)?;
                        Ok(true)
                    })
                    .await;

                match download_result {
                    Ok(()) if !stop_for_download.load(Ordering::SeqCst) => {
                        handle_for_download.mark_complete();
                        let _ = std::fs::remove_file(&pending_for_cleanup);
                    }
                    Ok(()) => {
                        handle_for_download.mark_error("Playback stopped");
                        let _ = std::fs::remove_file(&pending_for_cleanup);
                        let _ = std::fs::remove_file(&cache_path_for_cleanup);
                    }
                    Err(error) => {
                        eprintln!("[player] download error: {error:#}");
                        handle_for_download.mark_error(error.to_string());
                        let _ = std::fs::remove_file(&pending_for_cleanup);
                        let _ = std::fs::remove_file(&cache_path_for_cleanup);
                    }
                }
            });

            PlaybackInput::growing_file(handle)
        };

        let inspect_input = input.clone();
        let source_format = tokio::task::spawn_blocking(move || inspect_input.inspect_format())
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))??;

        anyhow::ensure!(
            self.player.is_session_active(session_id),
            "Playback stopped"
        );

        let output_format = self.player.negotiate_output_format(source_format)?;
        let start_position_secs =
            normalize_seek_position(start_position_secs, source_format.duration_secs);
        let sample_buffer = SampleBuffer::new();
        let _decode_worker = input.spawn_decode_worker(
            source_format,
            output_format,
            sample_buffer.clone(),
            Arc::clone(&stop_flag),
            Arc::clone(&pause_flag),
            start_position_secs,
        )?;

        let minimum_samples =
            ((output_format.sample_rate as usize * output_format.channels as usize) / 3)
                .max(output_format.channels as usize * 4096)
                .min(output_format.channels as usize * 32_768);

        let wait_buffer = sample_buffer.clone();
        let wait_stop = Arc::clone(&stop_flag);
        tokio::task::spawn_blocking(move || {
            wait_buffer.wait_for_samples(minimum_samples, &wait_stop)
        })
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))??;

        anyhow::ensure!(
            self.player.is_session_active(session_id),
            "Playback stopped"
        );

        self.player.start_stream_playback(
            session_id,
            output_format,
            sample_buffer,
            start_position_secs,
        )
    }

    pub(crate) async fn play_next_internal(&self) -> Result<f64, String> {
        let target = self
            .player
            .select_next_entry()
            .ok_or_else(|| "No next track available".to_string())?;
        self.play_song_internal(target.cid, target.cover_url, None)
            .await
    }

    pub(crate) async fn play_previous_internal(&self) -> Result<f64, String> {
        let target = self
            .player
            .select_previous_entry()
            .ok_or_else(|| "No previous track available".to_string())?;
        self.play_song_internal(target.cid, target.cover_url, None)
            .await
    }

    pub(crate) fn handle_media_control(&self, event: MediaControlEvent) {
        match event {
            MediaControlEvent::Play => {
                if let Err(error) = self.player.resume() {
                    eprintln!("[media-session] failed to resume playback: {error:#}");
                }
            }
            MediaControlEvent::Pause => {
                if let Err(error) = self.player.pause() {
                    eprintln!("[media-session] failed to pause playback: {error:#}");
                }
            }
            MediaControlEvent::Toggle => {
                if let Err(error) = self.player.toggle_playback() {
                    eprintln!("[media-session] failed to toggle playback: {error:#}");
                }
            }
            MediaControlEvent::Stop | MediaControlEvent::Quit => {
                if let Err(error) = self.player.stop() {
                    eprintln!("[media-session] failed to stop playback: {error:#}");
                }
            }
            MediaControlEvent::Next => {
                let state = self.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = state.play_next_internal().await {
                        eprintln!("[media-session] failed to play next track: {error}");
                    }
                });
            }
            MediaControlEvent::Previous => {
                let state = self.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = state.play_previous_internal().await {
                        eprintln!("[media-session] failed to play previous track: {error}");
                    }
                });
            }
            MediaControlEvent::SetPosition(position) => {
                let state = self.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = state.seek_current_internal(position.0.as_secs_f64()).await
                    {
                        eprintln!("[media-session] failed to seek playback: {error}");
                    }
                });
            }
            MediaControlEvent::SeekBy(direction, delta) => {
                let state = self.clone();
                tauri::async_runtime::spawn(async move {
                    let current = state.player.get_state();
                    let delta_secs = delta.as_secs_f64();
                    let target = match direction {
                        SeekDirection::Forward => current.progress + delta_secs,
                        SeekDirection::Backward => current.progress - delta_secs,
                    };
                    if let Err(error) = state.seek_current_internal(target).await {
                        eprintln!("[media-session] failed to seek playback by delta: {error}");
                    }
                });
            }
            MediaControlEvent::Seek(direction) => {
                let state = self.clone();
                tauri::async_runtime::spawn(async move {
                    let current = state.player.get_state();
                    let target = match direction {
                        SeekDirection::Forward => current.progress + 10.0,
                        SeekDirection::Backward => current.progress - 10.0,
                    };
                    if let Err(error) = state.seek_current_internal(target).await {
                        eprintln!("[media-session] failed to seek playback: {error}");
                    }
                });
            }
            _ => {}
        }
    }
}

fn normalize_seek_position(position_secs: f64, duration_secs: f64) -> f64 {
    let position_secs = position_secs.max(0.0);
    if duration_secs > 0.0 {
        position_secs.min((duration_secs - 0.05).max(0.0))
    } else {
        position_secs
    }
}

pub(crate) fn parse_output_format(format: &str) -> Result<OutputFormat, String> {
    match format {
        "wav" => Ok(OutputFormat::Wav),
        "flac" => Ok(OutputFormat::Flac),
        "mp3" => Ok(OutputFormat::Mp3),
        _ => Err(format!("Unsupported output format: {format}")),
    }
}
