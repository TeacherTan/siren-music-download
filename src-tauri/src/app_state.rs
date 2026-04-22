use crate::audio_cache;
use crate::download_session::DownloadSessionStore;
use crate::local_inventory::LocalInventoryService;
use crate::local_inventory_provenance::LocalInventoryProvenanceStore;
use crate::logging::{LogCenter, LogLevel, LogPayload};
use crate::player::stream::{GrowingFileHandle, PlaybackInput, SampleBuffer};
use crate::player::{AudioPlayer, PlaybackContext, PlaybackQueueEntry};
use crate::preferences::{AppPreferences, PreferencesStore};
use anyhow::{Context, Result};
use siren_core::{DownloadManagerSnapshot, DownloadService};
use souvlaki::{MediaControlEvent, SeekDirection};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex as StdMutex};
use tauri::Manager;
use tokio::sync::Mutex;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) player: Arc<AudioPlayer>,
    pub(crate) api: Arc<siren_core::ApiClient>,
    pub(crate) download_service: Arc<Mutex<DownloadService>>,
    pub(crate) local_inventory_service: LocalInventoryService,
    pub(crate) local_inventory_provenance_store: Arc<LocalInventoryProvenanceStore>,
    pub(crate) download_session_store: Arc<DownloadSessionStore>,
    pub(crate) preferences_store: Arc<PreferencesStore>,
    pub(crate) preferences: Arc<StdMutex<AppPreferences>>,
    pub(crate) log_center: Arc<LogCenter>,
}

struct PreparedPlaybackInput {
    input: PlaybackInput,
}

impl AppState {
    pub(crate) fn new(app: tauri::AppHandle) -> Result<Self, String> {
        let log_center = Arc::new(LogCenter::new(app.clone())?);
        let player = AudioPlayer::new(app.clone()).map_err(|e| e.to_string())?;
        let api = siren_core::ApiClient::new().map_err(|e| e.to_string())?;
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("failed to get app data dir: {e}"))?;
        let download_session_store = Arc::new(DownloadSessionStore::new(app_data_dir.clone()));
        let loaded_download_session = download_session_store.load(Some(log_center.as_ref()));
        let download_service = Arc::new(Mutex::new(DownloadService::from_manager_snapshot(
            loaded_download_session.snapshot.clone(),
        )));
        let local_inventory_provenance_store = Arc::new(
            LocalInventoryProvenanceStore::new(app_data_dir.clone()).map_err(|e| e.to_string())?,
        );
        let local_inventory_service =
            LocalInventoryService::new(local_inventory_provenance_store.clone());
        let store = PreferencesStore::new(app_data_dir);
        let preferences = store.load(Some(log_center.as_ref()));
        let state = Self {
            player: Arc::new(player),
            api: Arc::new(api),
            download_service,
            local_inventory_service,
            local_inventory_provenance_store,
            download_session_store,
            preferences_store: Arc::new(store),
            preferences: Arc::new(StdMutex::new(preferences)),
            log_center,
        };
        if loaded_download_session.should_persist {
            state.persist_download_snapshot(&loaded_download_session.snapshot);
        }
        Ok(state)
    }

    pub(crate) fn preferences(&self) -> AppPreferences {
        self.preferences.lock().unwrap().clone()
    }

    pub(crate) fn set_preferences(&self, prefs: AppPreferences) {
        *self.preferences.lock().unwrap() = prefs;
    }

    pub(crate) fn preferences_store(&self) -> Arc<PreferencesStore> {
        self.preferences_store.clone()
    }

    pub(crate) fn persist_download_snapshot(&self, snapshot: &DownloadManagerSnapshot) {
        if let Err(error) = self.download_session_store.save(snapshot) {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Error,
                    "download-session",
                    "download_session.write_failed",
                    "Failed to persist download session",
                )
                .user_message("下载历史保存失败")
                .details(error),
            );
        }
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
        let prepared_input = self.prepare_playback_input(song_cid, source_url, &stop_flag)?;
        let input = prepared_input.input.clone();

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

        let log_center = Arc::clone(&self.log_center);
        let error_handler: crate::player::stream::PlaybackErrorHandler = Arc::new(move |message| {
            log_center.record(
                crate::logging::LogPayload::new(
                    crate::logging::LogLevel::Error,
                    "player",
                    "player.decode_worker_failed",
                    "Audio decode worker failed",
                )
                .details(message),
            );
        });

        let _decode_worker = input.spawn_decode_worker(
            source_format,
            output_format,
            sample_buffer.clone(),
            Arc::clone(&stop_flag),
            Arc::clone(&pause_flag),
            start_position_secs,
            error_handler,
        )?;

        self.wait_for_initial_buffer(&sample_buffer, output_format, &stop_flag)
            .await?;

        self.start_prepared_playback(
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
            MediaControlEvent::Play => self.handle_media_play(),
            MediaControlEvent::Pause => self.handle_media_pause(),
            MediaControlEvent::Toggle => self.handle_media_toggle(),
            MediaControlEvent::Stop | MediaControlEvent::Quit => self.handle_media_stop_or_quit(),
            MediaControlEvent::Next => self.handle_media_next(),
            MediaControlEvent::Previous => self.handle_media_previous(),
            MediaControlEvent::SetPosition(position) => {
                self.handle_media_set_position(position.0.as_secs_f64())
            }
            MediaControlEvent::SeekBy(direction, delta) => {
                self.handle_media_seek_by(direction, delta.as_secs_f64())
            }
            MediaControlEvent::Seek(direction) => self.handle_media_seek(direction),
            _ => {}
        }
    }

    fn handle_media_play(&self) {
        if let Err(error) = self.player.resume() {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Warn,
                    "media-session",
                    "media_session.resume_failed",
                    "Failed to resume playback",
                )
                .details(format!("{error:#}")),
            );
        }
    }

    fn handle_media_pause(&self) {
        if let Err(error) = self.player.pause() {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Warn,
                    "media-session",
                    "media_session.pause_failed",
                    "Failed to pause playback",
                )
                .details(format!("{error:#}")),
            );
        }
    }

    fn handle_media_toggle(&self) {
        if let Err(error) = self.player.toggle_playback() {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Warn,
                    "media-session",
                    "media_session.toggle_failed",
                    "Failed to toggle playback",
                )
                .details(format!("{error:#}")),
            );
        }
    }

    fn handle_media_stop_or_quit(&self) {
        if let Err(error) = self.player.stop() {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Warn,
                    "media-session",
                    "media_session.stop_failed",
                    "Failed to stop playback",
                )
                .details(format!("{error:#}")),
            );
        }
    }

    fn handle_media_next(&self) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = state.play_next_internal().await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.next_track_failed",
                        "Failed to play next track",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }

    fn handle_media_previous(&self) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = state.play_previous_internal().await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.previous_track_failed",
                        "Failed to play previous track",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }

    fn handle_media_set_position(&self, position_secs: f64) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = state.seek_current_internal(position_secs).await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.seek_failed",
                        "Failed to seek playback",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }

    fn handle_media_seek_by(&self, direction: SeekDirection, delta_secs: f64) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            let current = state.player.get_state();
            let target = match direction {
                SeekDirection::Forward => current.progress + delta_secs,
                SeekDirection::Backward => current.progress - delta_secs,
            };
            if let Err(error) = state.seek_current_internal(target).await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.seek_by_delta_failed",
                        "Failed to seek by delta",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }

    fn handle_media_seek(&self, direction: SeekDirection) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            let current = state.player.get_state();
            let target = match direction {
                SeekDirection::Forward => current.progress + 10.0,
                SeekDirection::Backward => current.progress - 10.0,
            };
            if let Err(error) = state.seek_current_internal(target).await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.seek_forward_failed",
                        "Failed to seek forward/backward",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }

    fn prepare_playback_input(
        &self,
        song_cid: &str,
        source_url: &str,
        stop_flag: &Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<PreparedPlaybackInput> {
        let cache_path = audio_cache::cached_song_path(song_cid, source_url)?;
        let pending_marker = audio_cache::pending_marker_path(&cache_path);

        let input = if audio_cache::is_song_cached(&cache_path) {
            PlaybackInput::cached_file(cache_path.clone())
        } else {
            let _ = std::fs::remove_file(&cache_path);
            let _ = std::fs::remove_file(&pending_marker);
            std::fs::write(&pending_marker, b"pending").with_context(|| {
                format!("Failed to create cache marker {}", pending_marker.display())
            })?;

            let (handle, writer) = GrowingFileHandle::new(cache_path.clone())?;
            self.spawn_stream_download(
                source_url.to_string(),
                Arc::clone(stop_flag),
                handle.clone(),
                writer,
                cache_path.clone(),
                pending_marker.clone(),
            );
            PlaybackInput::growing_file(handle)
        };

        Ok(PreparedPlaybackInput { input })
    }

    fn spawn_stream_download(
        &self,
        source_url: String,
        stop_flag: Arc<std::sync::atomic::AtomicBool>,
        handle: GrowingFileHandle,
        mut writer: std::fs::File,
        cache_path: PathBuf,
        pending_marker: PathBuf,
    ) {
        let api = Arc::clone(&self.api);
        let log_center = Arc::clone(&self.log_center);

        tokio::spawn(async move {
            let download_result = api
                .download_stream(&source_url, |chunk, _, _| {
                    if stop_flag.load(Ordering::SeqCst) {
                        return Ok(false);
                    }
                    handle.append_chunk(&mut writer, chunk)?;
                    Ok(true)
                })
                .await;

            match download_result {
                Ok(()) if !stop_flag.load(Ordering::SeqCst) => {
                    handle.mark_complete();
                    let _ = std::fs::remove_file(&pending_marker);
                    audio_cache::spawn_cleanup_if_needed();
                }
                Ok(()) => {
                    handle.mark_error("Playback stopped");
                    let _ = std::fs::remove_file(&pending_marker);
                    let _ = std::fs::remove_file(&cache_path);
                }
                Err(error) => {
                    log_center.record(
                        LogPayload::new(
                            LogLevel::Error,
                            "player",
                            "player.stream_download_failed",
                            "Streaming download failed during playback",
                        )
                        .details(format!("{error:#}")),
                    );
                    handle.mark_error(error.to_string());
                    let _ = std::fs::remove_file(&pending_marker);
                    let _ = std::fs::remove_file(&cache_path);
                }
            }
        });
    }

    async fn wait_for_initial_buffer(
        &self,
        sample_buffer: &SampleBuffer,
        output_format: crate::player::stream::AudioFormat,
        stop_flag: &Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<()> {
        let minimum_samples =
            ((output_format.sample_rate as usize * output_format.channels as usize) / 3)
                .max(output_format.channels as usize * 4096)
                .min(output_format.channels as usize * 32_768);

        let wait_buffer = sample_buffer.clone();
        let wait_stop = Arc::clone(stop_flag);
        tokio::task::spawn_blocking(move || {
            wait_buffer.wait_for_samples(minimum_samples, &wait_stop)
        })
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))??;
        Ok(())
    }

    fn start_prepared_playback(
        &self,
        session_id: u64,
        output_format: crate::player::stream::AudioFormat,
        sample_buffer: SampleBuffer,
        start_position_secs: f64,
    ) -> Result<f64> {
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
}

fn normalize_seek_position(position_secs: f64, duration_secs: f64) -> f64 {
    let position_secs = position_secs.max(0.0);
    if duration_secs > 0.0 {
        position_secs.min((duration_secs - 0.05).max(0.0))
    } else {
        position_secs
    }
}
