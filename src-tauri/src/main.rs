//! Tauri UI entry point
//!
//! Provides web-based UI using Tauri framework for cross-platform support.

mod player;

use player::{decode_audio, AudioPlayer, PlayerState};
use std::sync::Arc;
use tauri::{Manager, State};

struct AppState {
    player: Arc<AudioPlayer>,
    api: Arc<siren_core::ApiClient>,
}

impl AppState {
    fn new(app: tauri::AppHandle) -> Result<Self, String> {
        let player = AudioPlayer::new(app).map_err(|e| e.to_string())?;
        let api = siren_core::ApiClient::new().map_err(|e| e.to_string())?;
        Ok(Self {
            player: Arc::new(player),
            api: Arc::new(api),
        })
    }
}

#[tauri::command]
async fn get_albums(state: State<'_, AppState>) -> Result<Vec<siren_core::api::Album>, String> {
    state.api.get_albums().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_album_detail(
    state: State<'_, AppState>,
    album_cid: String,
) -> Result<siren_core::api::AlbumDetail, String> {
    state.api.get_album_detail(&album_cid).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_song_detail(
    state: State<'_, AppState>,
    cid: String,
) -> Result<siren_core::api::SongDetail, String> {
    state.api.get_song_detail(&cid).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn get_default_output_dir() -> String {
    dirs::download_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("SirenMusic")
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
async fn play_song(
    state: State<'_, AppState>,
    song_cid: String,
    cover_url: Option<String>,
) -> Result<f64, String> {
    // Fetch song metadata (async, non-blocking)
    let song_detail = state
        .api
        .get_song_detail(&song_cid)
        .await
        .map_err(|e| e.to_string())?;

    // Download audio bytes (async, non-blocking)
    let audio_bytes = state
        .api
        .download_bytes(&song_detail.source_url, |_, _| {})
        .await
        .map_err(|e| e.to_string())?;

    // Decode audio (CPU-intensive, run in blocking pool)
    let decoded = tokio::task::spawn_blocking(move || decode_audio(&audio_bytes))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    // Play (spawns its own background thread)
    state
        .player
        .play(song_cid, song_detail.name, song_detail.artists, cover_url, decoded)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_playback(state: State<'_, AppState>) -> Result<(), String> {
    state.player.stop().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_player_state(state: State<'_, AppState>) -> Result<PlayerState, String> {
    Ok(state.player.get_state())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState::new(app.handle().clone())
                .expect("Failed to initialize app state");
            app.manage(state);

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_albums,
            get_album_detail,
            get_song_detail,
            get_default_output_dir,
            play_song,
            stop_playback,
            get_player_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}