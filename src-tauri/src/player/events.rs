use crate::player::state::PlayerState;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

pub const PLAYER_STATE_CHANGED: &str = "player-state-changed";
pub const PLAYER_PROGRESS: &str = "player-progress";

pub fn emit_state(app: &AppHandle, state: &Arc<Mutex<PlayerState>>) {
    let _ = app.emit(PLAYER_STATE_CHANGED, state.lock().unwrap().clone());
}

pub fn emit_progress(app: &AppHandle, state: &Arc<Mutex<PlayerState>>) {
    let _ = app.emit(PLAYER_PROGRESS, state.lock().unwrap().clone());
}