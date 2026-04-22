use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use siren_core::ApiClient;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

const SNAPSHOT_FILE_NAME: &str = "library_search_snapshot.json";
const INDEX_ROOT_DIR_NAME: &str = "indexes";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LibrarySearchSnapshot {
    pub root_output_dir: String,
    pub inventory_version: String,
    pub built_at: String,
    pub albums: Vec<LibrarySearchAlbumRecord>,
    pub songs: Vec<LibrarySearchSongRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LibrarySearchAlbumRecord {
    pub album_cid: String,
    pub album_title: String,
    pub artist_line: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LibrarySearchSongRecord {
    pub album_cid: String,
    pub song_cid: String,
    pub album_title: String,
    pub song_title: String,
    pub artist_line: Option<String>,
}

pub(crate) async fn build_library_search_snapshot(
    api: Arc<ApiClient>,
    root_output_dir: String,
    inventory_version: String,
) -> Result<LibrarySearchSnapshot> {
    let albums = api.get_albums().await?;
    let mut album_records = Vec::with_capacity(albums.len());
    let mut song_records = Vec::new();

    for album in albums {
        let album_artist_line = join_artists(&album.artists);
        album_records.push(LibrarySearchAlbumRecord {
            album_cid: album.cid.clone(),
            album_title: album.name.clone(),
            artist_line: album_artist_line.clone(),
        });

        let detail = api
            .get_album_detail(&album.cid)
            .await
            .with_context(|| format!("failed to fetch album detail {}", album.cid))?;

        let fallback_artist_line = detail
            .artists
            .as_ref()
            .and_then(|artists| join_artists(artists))
            .or(album_artist_line.clone());

        song_records.extend(
            detail
                .songs
                .into_iter()
                .map(|song| LibrarySearchSongRecord {
                    album_cid: album.cid.clone(),
                    song_cid: song.cid,
                    album_title: album.name.clone(),
                    song_title: song.name,
                    artist_line: join_artists(&song.artists)
                        .or_else(|| fallback_artist_line.clone()),
                }),
        );
    }

    Ok(LibrarySearchSnapshot {
        root_output_dir,
        inventory_version,
        built_at: OffsetDateTime::now_utc()
            .format(&Iso8601::DEFAULT)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string()),
        albums: album_records,
        songs: song_records,
    })
}

pub(crate) fn load_library_search_snapshot(
    base_dir: &Path,
) -> Result<Option<LibrarySearchSnapshot>> {
    let path = snapshot_file_path(base_dir);
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    if content.trim().is_empty() {
        return Ok(None);
    }

    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))
        .map(Some)
}

pub(crate) fn save_library_search_snapshot(
    base_dir: &Path,
    snapshot: &LibrarySearchSnapshot,
) -> Result<()> {
    std::fs::create_dir_all(base_dir)
        .with_context(|| format!("failed to create {}", base_dir.display()))?;
    let path = snapshot_file_path(base_dir);
    let content = serde_json::to_string_pretty(snapshot)?;
    std::fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))
}

pub(crate) fn snapshot_file_path(base_dir: &Path) -> PathBuf {
    base_dir.join(SNAPSHOT_FILE_NAME)
}

pub(crate) fn indexes_root_dir(base_dir: &Path) -> PathBuf {
    base_dir.join(INDEX_ROOT_DIR_NAME)
}

pub(crate) fn inventory_index_dir(base_dir: &Path, inventory_version: &str) -> PathBuf {
    indexes_root_dir(base_dir).join(index_directory_name(inventory_version))
}

fn join_artists(artists: &[String]) -> Option<String> {
    let line = artists
        .iter()
        .map(|artist| artist.trim())
        .filter(|artist| !artist.is_empty())
        .collect::<Vec<_>>()
        .join(", ");

    if line.is_empty() {
        None
    } else {
        Some(line)
    }
}

fn index_directory_name(inventory_version: &str) -> String {
    inventory_version
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => character,
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::index_directory_name;

    #[test]
    fn sanitizes_inventory_version_for_index_directory() {
        assert_eq!(index_directory_name("alpha/beta:01"), "alpha_beta_01");
    }
}
