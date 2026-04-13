use crate::api::{ApiClient, AlbumDetail, SongDetail};
use crate::audio::{save_audio, tag_flac, AudioFormat, OutputFormat};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Optional metadata overrides applied when tagging downloaded FLAC files.
/// Empty strings/vecs mean "use the value from the API".
pub struct MetaOverride {
    pub album_name: String,
    pub artists: Vec<String>,
}

/// Progress event emitted during a download
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub song_name: String,
    /// Bytes received so far for current file
    pub bytes_done: u64,
    /// Total bytes for current file (None if unknown)
    pub bytes_total: Option<u64>,
    /// Index of song in the current batch (0-based)
    pub song_index: usize,
    /// Total songs in batch
    pub song_count: usize,
}

/// Download a single song with metadata tagging.
pub async fn download_song(
    client: &ApiClient,
    song: &SongDetail,
    album: &AlbumDetail,
    out_dir: &Path,
    format: OutputFormat,
    meta: &MetaOverride,
    on_progress: impl Fn(DownloadProgress) + Send + 'static,
) -> Result<PathBuf> {
    // Fetch cover art (best-effort)
    let cover_bytes: Option<Vec<u8>> = client
        .download_bytes(&album.cover_url, |_, _| {})
        .await
        .ok();

    let name_for_progress = song.name.clone();
    let progress_fn = Arc::new(on_progress);
    let pfn = Arc::clone(&progress_fn);

    let audio_bytes = client
        .download_bytes(&song.source_url, move |done, total| {
            pfn(DownloadProgress {
                song_name: name_for_progress.clone(),
                bytes_done: done,
                bytes_total: total,
                song_index: 0,
                song_count: 1,
            });
        })
        .await?;

    let out_path = save_audio(&audio_bytes, out_dir, &song.name, format)?;

    // Tag FLAC files with metadata (apply overrides if provided)
    if AudioFormat::detect(&audio_bytes) == AudioFormat::Flac
        || (AudioFormat::detect(&audio_bytes) == AudioFormat::Wav && format == OutputFormat::Flac)
    {
        if out_path.extension().and_then(|e| e.to_str()) == Some("flac") {
            let eff_album = if meta.album_name.is_empty() { &album.name } else { &meta.album_name };
            let eff_artists = if meta.artists.is_empty() { &song.artists } else { &meta.artists };
            let _ = tag_flac(
                &out_path,
                &song.name,
                eff_artists,
                eff_album,
                cover_bytes.as_deref(),
            );
        }
    }

    Ok(out_path)
}

/// Download all songs in an album into `out_dir/<album_name>/`.
/// Calls `on_progress` for each chunk of each file.
pub async fn download_album(
    client: &ApiClient,
    album_cid: &str,
    base_out_dir: &Path,
    format: OutputFormat,
    on_progress: impl Fn(DownloadProgress) + Send + Clone + 'static,
) -> Result<Vec<PathBuf>> {
    let album = client.get_album_detail(album_cid).await?;
    let song_count = album.songs.len();

    let album_dir = base_out_dir.join(crate::audio::sanitize_filename(&album.name));
    std::fs::create_dir_all(&album_dir)?;

    let cover_bytes: Option<Vec<u8>> = client
        .download_bytes(&album.cover_url, |_, _| {})
        .await
        .ok();

    let mut paths = Vec::new();

    for (idx, song_entry) in album.songs.iter().enumerate() {
        let song_detail = client.get_song_detail(&song_entry.cid).await?;
        let prog = on_progress.clone();
        let song_name = song_detail.name.clone();

        let audio_bytes = client
            .download_bytes(&song_detail.source_url, move |done, total| {
                prog(DownloadProgress {
                    song_name: song_name.clone(),
                    bytes_done: done,
                    bytes_total: total,
                    song_index: idx,
                    song_count,
                });
            })
            .await?;

        let out_path = save_audio(&audio_bytes, &album_dir, &song_detail.name, format)?;

        if out_path.extension().and_then(|e| e.to_str()) == Some("flac") {
            let _ = tag_flac(
                &out_path,
                &song_detail.name,
                &song_detail.artists,
                &album.name,
                cover_bytes.as_deref(),
            );
        }

        paths.push(out_path);
    }

    Ok(paths)
}
