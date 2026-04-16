use crate::player::backend::{create_backend, PlaybackBackend};
use crate::player::events::{emit_progress, emit_state};
use crate::player::media::MediaSession;
use crate::player::state::PlayerState;
use crate::player::stream::{AudioFormat, SampleBuffer};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use souvlaki::MediaControlEvent;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

/// Queue entry shared between the frontend and backend playback context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackQueueEntry {
    /// Song identifier passed back into `play_song`.
    pub cid: String,
    /// Display name shown in the queue UI.
    pub name: String,
    /// Artist names rendered in the player and queue flyout.
    pub artists: Vec<String>,
    /// Optional artwork URL used for media session metadata.
    pub cover_url: Option<String>,
}

/// Playback order sent by the frontend when it starts playback from an album or
/// a shuffled queue.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackContext {
    /// Ordered queue entries available for previous/next navigation.
    pub entries: Vec<PlaybackQueueEntry>,
    /// Frontend-selected index at the time playback starts.
    pub current_index: usize,
}

#[derive(Default)]
struct PlaybackQueueState {
    entries: Vec<PlaybackQueueEntry>,
    current_index: Option<usize>,
}

impl PlaybackQueueState {
    fn set_context(&mut self, context: PlaybackContext, current_cid: &str) {
        self.entries = context.entries;
        if self.entries.is_empty() {
            self.current_index = None;
            return;
        }

        let bounded_index = context.current_index.min(self.entries.len() - 1);
        let resolved_index = self
            .entries
            .iter()
            .position(|entry| entry.cid == current_cid)
            .unwrap_or(bounded_index);
        self.current_index = Some(resolved_index);
    }

    fn sync_or_replace_current(&mut self, entry: PlaybackQueueEntry) {
        if let Some(index) = self.entries.iter().position(|item| item.cid == entry.cid) {
            self.entries[index] = entry;
            self.current_index = Some(index);
            return;
        }

        self.entries = vec![entry];
        self.current_index = Some(0);
    }

    fn select_next(&mut self) -> Option<PlaybackQueueEntry> {
        let next_index = self
            .current_index
            .and_then(|index| (index + 1 < self.entries.len()).then_some(index + 1))?;
        self.current_index = Some(next_index);
        self.entries.get(next_index).cloned()
    }

    fn select_previous(&mut self) -> Option<PlaybackQueueEntry> {
        let previous_index = self.current_index.and_then(|index| index.checked_sub(1))?;
        self.current_index = Some(previous_index);
        self.entries.get(previous_index).cloned()
    }

    fn has_previous(&self) -> bool {
        self.current_index.is_some_and(|index| index > 0)
    }

    fn has_next(&self) -> bool {
        self.current_index
            .is_some_and(|index| index + 1 < self.entries.len())
    }
}

/// Backend playback controller used by Tauri commands and media-session events.
pub struct AudioPlayer {
    app: AppHandle,
    state: Arc<Mutex<PlayerState>>,
    backend: Mutex<Box<dyn PlaybackBackend>>,
    media_session: Arc<Mutex<Option<MediaSession>>>,
    queue: Arc<Mutex<PlaybackQueueState>>,
    volume: Arc<Mutex<f64>>,
    active_session_id: Arc<AtomicU64>,
    stop_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
}

impl AudioPlayer {
    pub fn new(app: AppHandle) -> Result<Self> {
        let backend = create_backend()?;
        Ok(Self {
            app,
            state: Arc::new(Mutex::new(PlayerState::default())),
            backend: Mutex::new(backend),
            media_session: Arc::new(Mutex::new(None)),
            queue: Arc::new(Mutex::new(PlaybackQueueState::default())),
            volume: Arc::new(Mutex::new(1.0)),
            active_session_id: Arc::new(AtomicU64::new(0)),
            stop_flag: Arc::new(AtomicBool::new(false)),
            pause_flag: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn app_handle(&self) -> AppHandle {
        self.app.clone()
    }

    pub fn bind_media_controls<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(MediaControlEvent) + Send + 'static,
    {
        let session = MediaSession::new(&self.app)?;
        session.attach(handler)?;
        *self.media_session.lock().unwrap() = Some(session);
        self.sync_media_controls(false);
        Ok(())
    }

    pub fn prepare_playback_context(
        &self,
        context: Option<PlaybackContext>,
        current_entry: PlaybackQueueEntry,
    ) {
        {
            let mut queue = self.queue.lock().unwrap();
            if let Some(context) = context {
                queue.set_context(context, &current_entry.cid);
                if let Some(index) = queue.current_index {
                    queue.entries[index] = current_entry;
                }
            } else {
                queue.sync_or_replace_current(current_entry);
            }
        }
        self.sync_navigation_flags();
    }

    pub fn select_next_entry(&self) -> Option<PlaybackQueueEntry> {
        let next = {
            let mut queue = self.queue.lock().unwrap();
            queue.select_next()
        };
        self.sync_navigation_flags();
        next
    }

    pub fn select_previous_entry(&self) -> Option<PlaybackQueueEntry> {
        let previous = {
            let mut queue = self.queue.lock().unwrap();
            queue.select_previous()
        };
        self.sync_navigation_flags();
        previous
    }

    pub fn begin_loading_session(
        &self,
        song_cid: String,
        song_name: String,
        artists: Vec<String>,
        cover_url: Option<String>,
        initial_progress: f64,
        initial_duration: Option<f64>,
    ) -> Result<u64> {
        self.stop()?;

        let session_id = self.active_session_id.fetch_add(1, Ordering::SeqCst) + 1;
        self.stop_flag.store(false, Ordering::SeqCst);
        self.pause_flag.store(false, Ordering::SeqCst);
        let initial_progress = initial_progress.max(0.0);
        let initial_duration = initial_duration.unwrap_or(0.0).max(initial_progress);

        {
            let mut state = self.state.lock().unwrap();
            let volume = *self.volume.lock().unwrap();
            state.song_cid = Some(song_cid);
            state.song_name = Some(song_name);
            state.artists = artists;
            state.cover_url = cover_url;
            state.is_loading = true;
            state.is_playing = false;
            state.is_paused = false;
            state.progress = initial_progress;
            state.duration = initial_duration;
            state.volume = volume;
            apply_queue_flags(&self.queue.lock().unwrap(), &mut state);
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);

        Ok(session_id)
    }

    pub fn negotiate_output_format(&self, source_format: AudioFormat) -> Result<AudioFormat> {
        self.backend
            .lock()
            .unwrap()
            .negotiate_output_format(source_format)
    }

    pub fn start_stream_playback(
        &self,
        session_id: u64,
        format: AudioFormat,
        samples: SampleBuffer,
        initial_progress: f64,
    ) -> Result<f64> {
        if !self.is_session_active(session_id) {
            anyhow::bail!("Playback session expired");
        }

        self.pause_flag.store(false, Ordering::SeqCst);
        let initial_progress = if format.duration_secs > 0.0 {
            initial_progress.clamp(0.0, format.duration_secs)
        } else {
            initial_progress.max(0.0)
        };

        {
            let mut state = self.state.lock().unwrap();
            state.progress = initial_progress;
            state.duration = format.duration_secs.max(initial_progress);
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);

        let state_for_progress = Arc::clone(&self.state);
        let state_for_finish = Arc::clone(&self.state);
        let app_for_progress = self.app.clone();
        let app_for_finish = self.app.clone();
        let media_for_progress = Arc::clone(&self.media_session);
        let media_for_finish = Arc::clone(&self.media_session);
        let active_session = Arc::clone(&self.active_session_id);
        let stop_flag = Arc::clone(&self.stop_flag);

        let progress_callback: Arc<dyn Fn(f64, f64) + Send + Sync> =
            Arc::new(move |progress, duration| {
                if active_session.load(Ordering::SeqCst) != session_id
                    || stop_flag.load(Ordering::SeqCst)
                {
                    return;
                }
                {
                    let mut state = state_for_progress.lock().unwrap();
                    let absolute_progress = if duration > 0.0 {
                        (initial_progress + progress).min(duration)
                    } else {
                        initial_progress + progress
                    };
                    state.progress = absolute_progress;
                    if duration > 0.0 {
                        state.duration = duration.max(initial_progress);
                    }
                }
                emit_progress_and_sync(&app_for_progress, &state_for_progress, &media_for_progress);
            });

        let active_session = Arc::clone(&self.active_session_id);
        let stop_flag = Arc::clone(&self.stop_flag);
        let pause_flag = Arc::clone(&self.pause_flag);
        let finish_callback: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            if active_session.load(Ordering::SeqCst) != session_id
                || stop_flag.load(Ordering::SeqCst)
            {
                return;
            }
            pause_flag.store(false, Ordering::SeqCst);
            {
                let mut state = state_for_finish.lock().unwrap();
                state.is_playing = false;
                state.is_paused = false;
                state.is_loading = false;
                if state.duration <= 0.0 {
                    state.duration = state.progress;
                }
                state.progress = state.duration;
            }
            emit_state_and_sync(&app_for_finish, &state_for_finish, &media_for_finish);
        });

        self.backend
            .lock()
            .unwrap()
            .play_stream(
                format,
                samples,
                Arc::clone(&self.stop_flag),
                Arc::clone(&self.volume),
                progress_callback,
                finish_callback,
            )
            .context("Failed to start audio backend")?;

        {
            let mut state = self.state.lock().unwrap();
            state.is_loading = false;
            state.is_playing = true;
            state.is_paused = false;
            state.progress = initial_progress;
            if format.duration_secs > 0.0 {
                state.duration = format.duration_secs;
            }
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);

        Ok(self.state.lock().unwrap().duration)
    }

    pub fn pause(&self) -> Result<()> {
        let should_pause = {
            let state = self.state.lock().unwrap();
            state.is_playing && !state.is_loading
        };
        if !should_pause {
            return Ok(());
        }

        self.backend
            .lock()
            .unwrap()
            .pause()
            .context("Failed to pause audio backend")?;
        self.pause_flag.store(true, Ordering::SeqCst);

        {
            let mut state = self.state.lock().unwrap();
            state.is_playing = false;
            state.is_paused = true;
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);
        Ok(())
    }

    pub fn resume(&self) -> Result<()> {
        let should_resume = {
            let state = self.state.lock().unwrap();
            state.is_paused && state.song_cid.is_some()
        };
        if !should_resume {
            return Ok(());
        }

        self.backend
            .lock()
            .unwrap()
            .resume()
            .context("Failed to resume audio backend")?;
        self.pause_flag.store(false, Ordering::SeqCst);

        {
            let mut state = self.state.lock().unwrap();
            state.is_playing = true;
            state.is_paused = false;
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);
        Ok(())
    }

    pub fn toggle_playback(&self) -> Result<()> {
        let state = self.get_state();
        if state.is_paused {
            self.resume()
        } else if state.is_playing {
            self.pause()
        } else {
            Ok(())
        }
    }

    pub fn fail_session(&self, session_id: u64) {
        if self.active_session_id.load(Ordering::SeqCst) != session_id {
            return;
        }

        self.stop_flag.store(true, Ordering::SeqCst);
        self.pause_flag.store(false, Ordering::SeqCst);
        self.active_session_id.fetch_add(1, Ordering::SeqCst);
        let _ = self.backend.lock().unwrap().stop();

        {
            let mut state = self.state.lock().unwrap();
            *state = PlayerState::default();
            state.volume = *self.volume.lock().unwrap();
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);
    }

    pub fn stop_signal(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stop_flag)
    }

    pub fn pause_signal(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.pause_flag)
    }

    pub fn is_session_active(&self, session_id: u64) -> bool {
        self.active_session_id.load(Ordering::SeqCst) == session_id
            && !self.stop_flag.load(Ordering::SeqCst)
    }

    pub fn stop(&self) -> Result<()> {
        self.stop_flag.store(true, Ordering::SeqCst);
        self.pause_flag.store(false, Ordering::SeqCst);
        self.active_session_id.fetch_add(1, Ordering::SeqCst);
        self.backend.lock().unwrap().stop()?;

        {
            let mut state = self.state.lock().unwrap();
            *state = PlayerState::default();
            state.volume = *self.volume.lock().unwrap();
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);
        Ok(())
    }

    pub fn get_state(&self) -> PlayerState {
        self.state.lock().unwrap().clone()
    }

    pub fn set_volume(&self, volume: f64) -> f64 {
        let safe_volume = volume.clamp(0.0, 1.0);
        *self.volume.lock().unwrap() = safe_volume;

        {
            let mut state = self.state.lock().unwrap();
            state.volume = safe_volume;
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);

        safe_volume
    }

    fn sync_media_controls(&self, progress_only: bool) {
        sync_media_session(&self.media_session, &self.state, progress_only);
    }

    fn sync_navigation_flags(&self) {
        {
            let queue = self.queue.lock().unwrap();
            let mut state = self.state.lock().unwrap();
            apply_queue_flags(&queue, &mut state);
        }
        self.sync_media_controls(false);
    }
}

fn apply_queue_flags(queue: &PlaybackQueueState, state: &mut PlayerState) {
    state.has_previous = queue.has_previous();
    state.has_next = queue.has_next();
}

fn emit_state_and_sync(
    app: &AppHandle,
    state: &Arc<Mutex<PlayerState>>,
    media_session: &Arc<Mutex<Option<MediaSession>>>,
) {
    let snapshot = state.lock().unwrap().clone();
    emit_state(app, state);
    sync_media_session(media_session, state, false);
    crate::notification::notify_playback_changed(app, &snapshot);
}

fn emit_progress_and_sync(
    app: &AppHandle,
    state: &Arc<Mutex<PlayerState>>,
    media_session: &Arc<Mutex<Option<MediaSession>>>,
) {
    emit_progress(app, state);
    sync_media_session(media_session, state, true);
}

fn sync_media_session(
    media_session: &Arc<Mutex<Option<MediaSession>>>,
    state: &Arc<Mutex<PlayerState>>,
    progress_only: bool,
) {
    let snapshot = state.lock().unwrap().clone();
    if let Some(session) = media_session.lock().unwrap().as_ref() {
        session.sync_state(&snapshot, progress_only);
    }
}
