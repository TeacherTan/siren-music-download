use crate::audio::sanitize_filename;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const AUDIO_EXTENSIONS: [&str; 3] = ["flac", "wav", "mp3"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalTrackDownloadStatus {
    Missing,
    Detected,
    Verified,
    Mismatch,
    Partial,
    Unverifiable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalInventoryStatus {
    Idle,
    Scanning,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VerificationMode {
    None,
    WhenAvailable,
    Strict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackDownloadBadge {
    pub is_downloaded: bool,
    pub download_status: LocalTrackDownloadStatus,
    pub inventory_version: String,
}

impl Default for TrackDownloadBadge {
    fn default() -> Self {
        Self {
            is_downloaded: false,
            download_status: LocalTrackDownloadStatus::Missing,
            inventory_version: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDownloadBadge {
    pub has_downloaded_tracks: bool,
    pub downloaded_track_count: usize,
    pub verified_track_count: usize,
    pub mismatch_track_count: usize,
    pub inventory_version: String,
}

impl Default for AlbumDownloadBadge {
    fn default() -> Self {
        Self {
            has_downloaded_tracks: false,
            downloaded_track_count: 0,
            verified_track_count: 0,
            mismatch_track_count: 0,
            inventory_version: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalInventorySnapshot {
    pub root_output_dir: String,
    pub status: LocalInventoryStatus,
    pub inventory_version: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub scanned_file_count: usize,
    pub matched_track_count: usize,
    pub verified_track_count: usize,
    pub last_error: Option<String>,
}

impl Default for LocalInventorySnapshot {
    fn default() -> Self {
        Self {
            root_output_dir: String::new(),
            status: LocalInventoryStatus::Idle,
            inventory_version: String::new(),
            started_at: None,
            finished_at: None,
            scanned_file_count: 0,
            matched_track_count: 0,
            verified_track_count: 0,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalInventoryScanProgressEvent {
    pub root_output_dir: String,
    pub inventory_version: String,
    pub files_scanned: usize,
    pub matched_track_count: usize,
    pub verified_track_count: usize,
    pub current_path: Option<String>,
}

pub fn is_downloaded_status(status: LocalTrackDownloadStatus) -> bool {
    matches!(
        status,
        LocalTrackDownloadStatus::Detected
            | LocalTrackDownloadStatus::Verified
            | LocalTrackDownloadStatus::Partial
            | LocalTrackDownloadStatus::Unverifiable
    )
}

pub fn missing_track_badge(inventory_version: impl Into<String>) -> TrackDownloadBadge {
    badge_for_status(LocalTrackDownloadStatus::Missing, inventory_version)
}

pub fn empty_album_badge(inventory_version: impl Into<String>) -> AlbumDownloadBadge {
    AlbumDownloadBadge {
        inventory_version: inventory_version.into(),
        ..AlbumDownloadBadge::default()
    }
}

pub fn badge_for_detected_file(
    verification_mode: VerificationMode,
    inventory_version: impl Into<String>,
) -> TrackDownloadBadge {
    let status = match verification_mode {
        VerificationMode::Strict => LocalTrackDownloadStatus::Unverifiable,
        VerificationMode::None | VerificationMode::WhenAvailable => {
            LocalTrackDownloadStatus::Detected
        }
    };
    badge_for_status(status, inventory_version)
}

pub fn badge_for_status(
    status: LocalTrackDownloadStatus,
    inventory_version: impl Into<String>,
) -> TrackDownloadBadge {
    TrackDownloadBadge {
        is_downloaded: is_downloaded_status(status),
        download_status: status,
        inventory_version: inventory_version.into(),
    }
}

pub fn aggregate_album_badge(
    track_badges: &[TrackDownloadBadge],
    inventory_version: impl Into<String>,
) -> AlbumDownloadBadge {
    let downloaded_track_count = track_badges
        .iter()
        .filter(|badge| badge.is_downloaded)
        .count();
    let verified_track_count = track_badges
        .iter()
        .filter(|badge| badge.download_status == LocalTrackDownloadStatus::Verified)
        .count();
    let mismatch_track_count = track_badges
        .iter()
        .filter(|badge| badge.download_status == LocalTrackDownloadStatus::Mismatch)
        .count();

    AlbumDownloadBadge {
        has_downloaded_tracks: downloaded_track_count > 0,
        downloaded_track_count,
        verified_track_count,
        mismatch_track_count,
        inventory_version: inventory_version.into(),
    }
}

pub fn candidate_relative_paths(album_name: &str, song_name: &str) -> Vec<String> {
    let safe_song_name = sanitize_filename(song_name);
    let safe_album_name = sanitize_filename(album_name);
    let mut candidates = Vec::with_capacity(AUDIO_EXTENSIONS.len() * 2);

    for extension in AUDIO_EXTENSIONS {
        candidates.push(format!("{safe_song_name}.{extension}"));
        candidates.push(format!("{safe_album_name}/{safe_song_name}.{extension}"));
    }

    candidates
}

pub fn has_detected_track(
    relative_audio_paths: &HashSet<String>,
    album_name: &str,
    song_name: &str,
) -> bool {
    candidate_relative_paths(album_name, song_name)
        .into_iter()
        .any(|candidate| relative_audio_paths.contains(&candidate))
}

#[cfg(test)]
mod tests {
    use super::{
        aggregate_album_badge, badge_for_detected_file, candidate_relative_paths,
        has_detected_track, is_downloaded_status, LocalTrackDownloadStatus, TrackDownloadBadge,
        VerificationMode,
    };
    use std::collections::HashSet;

    #[test]
    fn builds_root_and_album_candidates_for_all_audio_extensions() {
        let candidates = candidate_relative_paths("A/B:C?D", "Track/01");

        assert!(candidates.contains(&"Track_01.flac".to_string()));
        assert!(candidates.contains(&"Track_01.wav".to_string()));
        assert!(candidates.contains(&"Track_01.mp3".to_string()));
        assert!(candidates.contains(&"A_B_C_D/Track_01.flac".to_string()));
    }

    #[test]
    fn detects_track_from_single_song_or_album_layout() {
        let mut files = HashSet::new();
        files.insert("Album/Track.flac".to_string());
        files.insert("Other.wav".to_string());

        assert!(has_detected_track(&files, "Album", "Track"));
        assert!(has_detected_track(&files, "Anything", "Other"));
        assert!(!has_detected_track(&files, "Album", "Missing"));
    }

    #[test]
    fn maps_detected_files_to_unverifiable_in_strict_mode() {
        let strict_badge = badge_for_detected_file(VerificationMode::Strict, "v1");
        let relaxed_badge = badge_for_detected_file(VerificationMode::WhenAvailable, "v1");

        assert_eq!(
            strict_badge.download_status,
            LocalTrackDownloadStatus::Unverifiable
        );
        assert_eq!(
            relaxed_badge.download_status,
            LocalTrackDownloadStatus::Detected
        );
        assert!(strict_badge.is_downloaded);
    }

    #[test]
    fn aggregates_album_counts_from_track_badges() {
        let badges = vec![
            TrackDownloadBadge {
                is_downloaded: true,
                download_status: LocalTrackDownloadStatus::Detected,
                inventory_version: "v1".to_string(),
            },
            TrackDownloadBadge {
                is_downloaded: false,
                download_status: LocalTrackDownloadStatus::Mismatch,
                inventory_version: "v1".to_string(),
            },
            TrackDownloadBadge {
                is_downloaded: true,
                download_status: LocalTrackDownloadStatus::Verified,
                inventory_version: "v1".to_string(),
            },
        ];

        let album_badge = aggregate_album_badge(&badges, "v1");

        assert!(album_badge.has_downloaded_tracks);
        assert_eq!(album_badge.downloaded_track_count, 2);
        assert_eq!(album_badge.verified_track_count, 1);
        assert_eq!(album_badge.mismatch_track_count, 1);
    }

    #[test]
    fn downloaded_status_mapping_matches_contract() {
        assert!(is_downloaded_status(LocalTrackDownloadStatus::Detected));
        assert!(is_downloaded_status(LocalTrackDownloadStatus::Verified));
        assert!(is_downloaded_status(LocalTrackDownloadStatus::Partial));
        assert!(is_downloaded_status(LocalTrackDownloadStatus::Unverifiable));
        assert!(!is_downloaded_status(LocalTrackDownloadStatus::Missing));
        assert!(!is_downloaded_status(LocalTrackDownloadStatus::Mismatch));
        assert!(!is_downloaded_status(LocalTrackDownloadStatus::Unknown));
    }
}
