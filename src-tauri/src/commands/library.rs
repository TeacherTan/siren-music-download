use crate::app_state::AppState;
use crate::theme;
use base64::Engine;
use tauri::State;

#[tauri::command]
pub async fn get_albums(state: State<'_, AppState>) -> Result<Vec<siren_core::api::Album>, String> {
    state.api.get_albums().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_album_detail(
    state: State<'_, AppState>,
    album_cid: String,
) -> Result<siren_core::api::AlbumDetail, String> {
    let album = state
        .api
        .get_album_detail(&album_cid)
        .await
        .map_err(|e| e.to_string())?;
    Ok(state
        .local_inventory_service
        .enrich_album_detail(album)
        .await)
}

#[tauri::command]
pub async fn get_song_detail(
    state: State<'_, AppState>,
    cid: String,
) -> Result<siren_core::api::SongDetail, String> {
    let song = state
        .api
        .get_song_detail(&cid)
        .await
        .map_err(|e| e.to_string())?;
    let album = state
        .api
        .get_album_detail(&song.album_cid)
        .await
        .map_err(|e| e.to_string())?;
    Ok(state
        .local_inventory_service
        .enrich_song_detail(song, &album.name)
        .await)
}

#[tauri::command]
pub async fn get_song_lyrics(
    state: State<'_, AppState>,
    cid: String,
) -> Result<Option<String>, String> {
    let song_detail = state
        .api
        .get_song_detail(&cid)
        .await
        .map_err(|e| e.to_string())?;

    let Some(lyric_url) = song_detail.lyric_url else {
        return Ok(None);
    };

    state
        .api
        .download_text(&lyric_url)
        .await
        .map(Some)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn extract_image_theme(
    state: State<'_, AppState>,
    image_url: String,
) -> Result<theme::ThemePalette, String> {
    let bytes = state
        .api
        .download_bytes(&image_url, |_, _| {})
        .await
        .map_err(|e| e.to_string())?;

    tokio::task::spawn_blocking(move || theme::extract_theme_palette(&bytes))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

fn encode_image_data_url(mime: &str, bytes: &[u8]) -> String {
    format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

#[tauri::command]
pub async fn get_image_data_url(
    state: State<'_, AppState>,
    image_url: String,
) -> Result<String, String> {
    let bytes = state
        .api
        .download_bytes(&image_url, |_, _| {})
        .await
        .map_err(|e| e.to_string())?;

    let mime = siren_core::audio::detect_image_mime(&bytes).unwrap_or("application/octet-stream");
    Ok(encode_image_data_url(mime, &bytes))
}

#[tauri::command]
pub fn get_default_output_dir() -> String {
    dirs::download_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/")))
        .join("SirenMusic")
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::encode_image_data_url;

    #[test]
    fn encodes_image_data_url() {
        let url = encode_image_data_url("image/png", b"abc");
        assert_eq!(url, "data:image/png;base64,YWJj");
    }
}
