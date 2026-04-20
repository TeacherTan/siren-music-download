use crate::app_state::AppState;
use siren_core::{
    aggregate_album_badge, badge_for_detected_file, empty_album_badge, has_detected_track,
    missing_track_badge, Album, AlbumDetail, LocalInventoryScanProgressEvent,
    LocalInventorySnapshot, LocalInventoryStatus, SongDetail, TrackDownloadBadge, VerificationMode,
};
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;
use tokio::sync::Mutex;

pub(crate) const LOCAL_INVENTORY_STATE_CHANGED: &str = "local-inventory-state-changed";
pub(crate) const LOCAL_INVENTORY_SCAN_PROGRESS: &str = "local-inventory-scan-progress";

#[derive(Debug, Clone)]
pub(crate) struct LocalInventoryState {
    pub(crate) snapshot: LocalInventorySnapshot,
    pub(crate) verification_mode: VerificationMode,
    pub(crate) relative_audio_paths: HashSet<String>,
}

impl Default for LocalInventoryState {
    fn default() -> Self {
        Self {
            snapshot: LocalInventorySnapshot::default(),
            verification_mode: VerificationMode::WhenAvailable,
            relative_audio_paths: HashSet::new(),
        }
    }
}

enum ScanCollectionOutcome {
    Completed(HashSet<String>),
    Cancelled,
}

#[derive(Clone)]
pub(crate) struct LocalInventoryService {
    state: Arc<Mutex<LocalInventoryState>>,
    cancel_flag: Arc<AtomicBool>,
}

impl LocalInventoryService {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(LocalInventoryState::default())),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) async fn snapshot(&self) -> LocalInventorySnapshot {
        self.state.lock().await.snapshot.clone()
    }

    pub(crate) async fn verification_mode(&self) -> VerificationMode {
        self.state.lock().await.verification_mode
    }

    pub(crate) async fn begin_scan(
        &self,
        root_output_dir: String,
        verification_mode: VerificationMode,
    ) -> LocalInventorySnapshot {
        self.cancel_flag.store(false, Ordering::SeqCst);
        let inventory_version = next_inventory_version();
        let mut state = self.state.lock().await;
        state.verification_mode = verification_mode;
        state.snapshot = LocalInventorySnapshot {
            root_output_dir,
            status: LocalInventoryStatus::Scanning,
            inventory_version,
            started_at: Some(iso_timestamp_now()),
            finished_at: None,
            scanned_file_count: 0,
            matched_track_count: 0,
            verified_track_count: 0,
            last_error: None,
        };
        state.snapshot.clone()
    }

    pub(crate) async fn complete_scan(
        &self,
        inventory_version: &str,
        relative_audio_paths: HashSet<String>,
    ) -> LocalInventorySnapshot {
        let mut state = self.state.lock().await;
        if state.snapshot.inventory_version != inventory_version
            || state.snapshot.status != LocalInventoryStatus::Scanning
        {
            return state.snapshot.clone();
        }
        let matched_track_count = relative_audio_paths.len();
        state.relative_audio_paths = relative_audio_paths;
        state.snapshot.status = LocalInventoryStatus::Completed;
        state.snapshot.scanned_file_count = matched_track_count;
        state.snapshot.matched_track_count = matched_track_count;
        state.snapshot.verified_track_count = 0;
        state.snapshot.finished_at = Some(iso_timestamp_now());
        state.snapshot.last_error = None;
        state.snapshot.clone()
    }

    pub(crate) async fn fail_scan(
        &self,
        inventory_version: &str,
        error: String,
    ) -> LocalInventorySnapshot {
        let mut state = self.state.lock().await;
        if state.snapshot.inventory_version != inventory_version
            || state.snapshot.status != LocalInventoryStatus::Scanning
        {
            return state.snapshot.clone();
        }
        state.relative_audio_paths.clear();
        state.snapshot.status = LocalInventoryStatus::Failed;
        state.snapshot.scanned_file_count = 0;
        state.snapshot.matched_track_count = 0;
        state.snapshot.verified_track_count = 0;
        state.snapshot.finished_at = Some(iso_timestamp_now());
        state.snapshot.last_error = Some(error);
        state.snapshot.clone()
    }

    pub(crate) async fn cancel_scan(&self) -> LocalInventorySnapshot {
        self.cancel_flag.store(true, Ordering::SeqCst);
        let mut state = self.state.lock().await;
        state.snapshot.status = LocalInventoryStatus::Idle;
        state.snapshot.finished_at = Some(iso_timestamp_now());
        state.snapshot.clone()
    }

    pub(crate) async fn enrich_album_list_with_details(
        &self,
        albums: Vec<Album>,
        album_details: &[AlbumDetail],
    ) -> Vec<Album> {
        let state = self.state.lock().await;
        albums
            .into_iter()
            .map(|mut album| {
                album.download = album_details
                    .iter()
                    .find(|detail| detail.cid == album.cid)
                    .map(|detail| {
                        detail
                            .songs
                            .iter()
                            .map(|song| {
                                track_badge_for_song(
                                    &state.relative_audio_paths,
                                    &detail.name,
                                    &song.name,
                                    state.verification_mode,
                                    &state.snapshot.inventory_version,
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .map(|track_badges| {
                        aggregate_album_badge(
                            &track_badges,
                            state.snapshot.inventory_version.clone(),
                        )
                    })
                    .unwrap_or_else(|| {
                        if has_detected_album(&state.relative_audio_paths, &album.name) {
                            aggregate_album_badge(
                                &[badge_for_detected_file(
                                    state.verification_mode,
                                    state.snapshot.inventory_version.clone(),
                                )],
                                state.snapshot.inventory_version.clone(),
                            )
                        } else {
                            empty_album_badge(state.snapshot.inventory_version.clone())
                        }
                    });
                album
            })
            .collect()
    }

    pub(crate) async fn enrich_album_detail(&self, mut album: AlbumDetail) -> AlbumDetail {
        let state = self.state.lock().await;
        let inventory_version = state.snapshot.inventory_version.clone();
        let track_badges = album
            .songs
            .iter_mut()
            .map(|song| {
                let badge = track_badge_for_song(
                    &state.relative_audio_paths,
                    &album.name,
                    &song.name,
                    state.verification_mode,
                    &inventory_version,
                );
                song.download = badge.clone();
                badge
            })
            .collect::<Vec<_>>();
        album.download = aggregate_album_badge(&track_badges, inventory_version);
        album
    }

    pub(crate) async fn enrich_song_detail(
        &self,
        mut song: SongDetail,
        album_name: &str,
    ) -> SongDetail {
        let state = self.state.lock().await;
        song.download = track_badge_for_song(
            &state.relative_audio_paths,
            album_name,
            &song.name,
            state.verification_mode,
            &state.snapshot.inventory_version,
        );
        song
    }
}

pub(crate) fn emit_local_inventory_state_changed(
    app: &AppHandle,
    snapshot: &LocalInventorySnapshot,
) {
    let _ = app.emit(LOCAL_INVENTORY_STATE_CHANGED, snapshot);
}

pub(crate) fn emit_local_inventory_scan_progress(
    app: &AppHandle,
    event: &LocalInventoryScanProgressEvent,
) {
    let _ = app.emit(LOCAL_INVENTORY_SCAN_PROGRESS, event);
}

pub(crate) fn spawn_inventory_scan(
    app: AppHandle,
    state: AppState,
    root_output_dir: String,
    verification_mode: Option<VerificationMode>,
) {
    tauri::async_runtime::spawn(async move {
        let mode = match verification_mode {
            Some(mode) => mode,
            None => state.local_inventory_service.verification_mode().await,
        };
        let started = state
            .local_inventory_service
            .begin_scan(root_output_dir.clone(), mode)
            .await;
        emit_local_inventory_state_changed(&app, &started);

        let inventory_version = started.inventory_version.clone();
        let scan_result = collect_relative_audio_paths(
            Path::new(&root_output_dir),
            &root_output_dir,
            &inventory_version,
            &state.local_inventory_service.cancel_flag,
            |event| emit_local_inventory_scan_progress(&app, &event),
        );

        let finished = match scan_result {
            Ok(ScanCollectionOutcome::Completed(relative_audio_paths)) => {
                state
                    .local_inventory_service
                    .complete_scan(&inventory_version, relative_audio_paths)
                    .await
            }
            Ok(ScanCollectionOutcome::Cancelled) => state.local_inventory_service.snapshot().await,
            Err(error) => {
                state
                    .local_inventory_service
                    .fail_scan(&inventory_version, error)
                    .await
            }
        };
        emit_local_inventory_state_changed(&app, &finished);
    });
}

fn track_badge_for_song(
    relative_audio_paths: &HashSet<String>,
    album_name: &str,
    song_name: &str,
    verification_mode: VerificationMode,
    inventory_version: &str,
) -> TrackDownloadBadge {
    if has_detected_track(relative_audio_paths, album_name, song_name) {
        badge_for_detected_file(verification_mode, inventory_version.to_string())
    } else {
        missing_track_badge(inventory_version.to_string())
    }
}

fn has_detected_album(relative_audio_paths: &HashSet<String>, album_name: &str) -> bool {
    let album_prefix = format!("{}/", siren_core::audio::sanitize_filename(album_name));
    relative_audio_paths
        .iter()
        .any(|relative_path| relative_path.starts_with(&album_prefix))
}

fn collect_relative_audio_paths(
    root_output_dir: &Path,
    root_output_dir_text: &str,
    inventory_version: &str,
    cancel_flag: &AtomicBool,
    mut on_progress: impl FnMut(LocalInventoryScanProgressEvent),
) -> Result<ScanCollectionOutcome, String> {
    if root_output_dir.as_os_str().is_empty() || !root_output_dir.exists() {
        return Ok(ScanCollectionOutcome::Completed(HashSet::new()));
    }

    if !root_output_dir.is_dir() {
        return Err("outputDir 不是目录".to_string());
    }

    let mut relative_audio_paths = HashSet::new();
    let mut files_scanned = 0_usize;
    let visit_result = visit_directory(
        root_output_dir,
        root_output_dir,
        cancel_flag,
        &mut |current_path| {
            files_scanned += 1;
            let relative_path = current_path
                .strip_prefix(root_output_dir)
                .ok()
                .map(to_normalized_relative_path);
            if is_audio_file(current_path) {
                if let Some(relative_path) = relative_path.clone() {
                    relative_audio_paths.insert(relative_path);
                }
            }
            on_progress(LocalInventoryScanProgressEvent {
                root_output_dir: root_output_dir_text.to_string(),
                inventory_version: inventory_version.to_string(),
                files_scanned,
                matched_track_count: relative_audio_paths.len(),
                verified_track_count: 0,
                current_path: relative_path,
            });
        },
    )?;

    if visit_result {
        Ok(ScanCollectionOutcome::Completed(relative_audio_paths))
    } else {
        Ok(ScanCollectionOutcome::Cancelled)
    }
}

fn visit_directory(
    root_output_dir: &Path,
    current_path: &Path,
    cancel_flag: &AtomicBool,
    on_file: &mut impl FnMut(&Path),
) -> Result<bool, String> {
    let mut entries = std::fs::read_dir(current_path)
        .map_err(|_| "读取目录失败".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "枚举目录失败".to_string())?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        if cancel_flag.load(Ordering::SeqCst) {
            return Ok(false);
        }

        let path = entry.path();
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|_| "读取文件元信息失败".to_string())?;

        if metadata.file_type().is_symlink() {
            continue;
        }

        if metadata.is_dir() {
            if !visit_directory(root_output_dir, &path, cancel_flag, on_file)? {
                return Ok(false);
            }
        } else if metadata.is_file() {
            let _ = root_output_dir;
            on_file(&path);
        }
    }

    Ok(true)
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "flac" | "wav" | "mp3"
            )
        })
        .unwrap_or(false)
}

fn to_normalized_relative_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn iso_timestamp_now() -> String {
    OffsetDateTime::now_utc()
        .format(&Iso8601::DEFAULT)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn next_inventory_version() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("inventory-{millis}")
}

#[cfg(test)]
mod tests {
    use super::{
        collect_relative_audio_paths, track_badge_for_song, LocalInventoryService,
        ScanCollectionOutcome,
    };
    use siren_core::{Album, AlbumDetail, LocalInventoryStatus, SongEntry, VerificationMode};
    use std::path::Path;
    use std::sync::atomic::AtomicBool;
    use tempfile::tempdir;

    #[tokio::test]
    async fn enriches_album_and_song_download_badges_from_output_dir() {
        let temp_dir = tempdir().expect("temp dir");
        std::fs::create_dir_all(temp_dir.path().join("Album")).expect("album dir");
        std::fs::write(temp_dir.path().join("Album/Track.flac"), b"audio").expect("audio file");

        let service = LocalInventoryService::new();
        let started = service
            .begin_scan(
                temp_dir.path().to_string_lossy().to_string(),
                VerificationMode::WhenAvailable,
            )
            .await;
        let files = collect_relative_audio_paths(
            temp_dir.path(),
            &started.root_output_dir,
            &started.inventory_version,
            &AtomicBool::new(false),
            |_| {},
        )
        .expect("scan files");
        let ScanCollectionOutcome::Completed(files) = files else {
            panic!("scan should complete");
        };
        let snapshot = service
            .complete_scan(&started.inventory_version, files)
            .await;

        assert_eq!(snapshot.status, LocalInventoryStatus::Completed);

        let album = Album {
            cid: "album-1".to_string(),
            name: "Album".to_string(),
            cover_url: "cover".to_string(),
            artists: vec!["Artist".to_string()],
            download: Default::default(),
        };
        let album_detail = AlbumDetail {
            cid: "album-1".to_string(),
            name: "Album".to_string(),
            intro: None,
            belong: "test".to_string(),
            cover_url: "cover".to_string(),
            cover_de_url: None,
            artists: Some(vec!["Artist".to_string()]),
            songs: vec![SongEntry {
                cid: "song-1".to_string(),
                name: "Track".to_string(),
                artists: vec!["Artist".to_string()],
                download: Default::default(),
            }],
            download: Default::default(),
        };

        let enriched_detail = service.enrich_album_detail(album_detail).await;
        let enriched_albums = service
            .enrich_album_list_with_details(vec![album], &[enriched_detail.clone()])
            .await;

        assert!(enriched_albums[0].download.has_downloaded_tracks);
        assert!(enriched_detail.download.has_downloaded_tracks);
        assert!(enriched_detail.songs[0].download.is_downloaded);
    }

    #[tokio::test]
    async fn missing_output_dir_yields_empty_snapshot() {
        let service = LocalInventoryService::new();
        let started = service
            .begin_scan(
                "/path/that/does/not/exist".to_string(),
                VerificationMode::WhenAvailable,
            )
            .await;
        let files = collect_relative_audio_paths(
            Path::new("/path/that/does/not/exist"),
            &started.root_output_dir,
            &started.inventory_version,
            &AtomicBool::new(false),
            |_| {},
        )
        .expect("empty scan");
        let ScanCollectionOutcome::Completed(files) = files else {
            panic!("missing path scan should complete");
        };
        let snapshot = service
            .complete_scan(&started.inventory_version, files)
            .await;

        assert_eq!(snapshot.status, LocalInventoryStatus::Completed);
        assert_eq!(snapshot.scanned_file_count, 0);
    }

    #[test]
    fn root_level_single_track_layout_marks_song_as_downloaded() {
        let mut files = std::collections::HashSet::new();
        files.insert("Track.flac".to_string());
        assert!(
            track_badge_for_song(
                &files,
                "Album",
                "Track",
                VerificationMode::WhenAvailable,
                "v1",
            )
            .is_downloaded
        );
    }
}
