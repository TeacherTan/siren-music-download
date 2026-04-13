use crate::player::backend::{create_backend, PlaybackBackend};
use crate::player::decode::DecodedAudio;
use crate::player::events::{emit_progress, emit_state};
use crate::player::state::PlayerState;
use anyhow::Result;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

pub struct AudioPlayer {
    app: AppHandle,
    state: Arc<Mutex<PlayerState>>,
    backend: Mutex<Box<dyn PlaybackBackend>>,
    active_session_id: Arc<AtomicU64>,
    stop_flag: Arc<AtomicBool>,
}

impl AudioPlayer {
    pub fn new(app: AppHandle) -> Result<Self> {
        let backend = create_backend()?;
        Ok(Self {
            app,
            state: Arc::new(Mutex::new(PlayerState::default())),
            backend: Mutex::new(backend),
            active_session_id: Arc::new(AtomicU64::new(0)),
            stop_flag: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn play(
        &self,
        song_cid: String,
        song_name: String,
        artists: Vec<String>,
        cover_url: Option<String>,
        audio: DecodedAudio,
    ) -> Result<f64> {
        self.stop()?;

        let session_id = self.active_session_id.fetch_add(1, Ordering::SeqCst) + 1;
        self.stop_flag.store(false, Ordering::SeqCst);

        {
            let mut state = self.state.lock().unwrap();
            state.song_cid = Some(song_cid);
            state.song_name = Some(song_name);
            state.artists = artists;
            state.cover_url = cover_url;
            state.is_loading = true;
            state.is_playing = false;
            state.progress = 0.0;
            state.duration = audio.duration_secs;
        }
        emit_state(&self.app, &self.state);

        let state_for_progress = Arc::clone(&self.state);
        let state_for_finish = Arc::clone(&self.state);
        let app_for_progress = self.app.clone();
        let app_for_finish = self.app.clone();
        let active_session = Arc::clone(&self.active_session_id);
        let stop_flag = Arc::clone(&self.stop_flag);

        let progress_callback: Arc<dyn Fn(f64, f64) + Send + Sync> = Arc::new(move |progress, duration| {
            if active_session.load(Ordering::SeqCst) != session_id || stop_flag.load(Ordering::SeqCst) {
                return;
            }
            {
                let mut state = state_for_progress.lock().unwrap();
                state.progress = progress;
                state.duration = duration;
            }
            emit_progress(&app_for_progress, &state_for_progress);
        });

        let active_session = Arc::clone(&self.active_session_id);
        let stop_flag = Arc::clone(&self.stop_flag);
        let finish_callback: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            if active_session.load(Ordering::SeqCst) != session_id || stop_flag.load(Ordering::SeqCst) {
                return;
            }
            {
                let mut state = state_for_finish.lock().unwrap();
                state.is_playing = false;
                state.progress = state.duration;
            }
            emit_state(&app_for_finish, &state_for_finish);
        });

        self.backend
            .lock()
            .unwrap()
            .play(audio, Arc::clone(&self.stop_flag), progress_callback, finish_callback)?;

        {
            let mut state = self.state.lock().unwrap();
            state.is_loading = false;
            state.is_playing = true;
        }
        emit_state(&self.app, &self.state);

        Ok(self.state.lock().unwrap().duration)
    }

    pub fn stop(&self) -> Result<()> {
        self.stop_flag.store(true, Ordering::SeqCst);
        self.active_session_id.fetch_add(1, Ordering::SeqCst);
        self.backend.lock().unwrap().stop()?;

        {
            let mut state = self.state.lock().unwrap();
            *state = PlayerState::default();
        }
        emit_state(&self.app, &self.state);
        Ok(())
    }

    pub fn get_state(&self) -> PlayerState {
        self.state.lock().unwrap().clone()
    }
}