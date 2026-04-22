use crate::audio::OutputFormat;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadOptions {
    pub output_dir: String,
    pub format: OutputFormat,
    pub download_lyrics: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DownloadJobKind {
    Song,
    Album,
    Selection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadJobStatus {
    Queued,
    Running,
    Completed,
    PartiallyFailed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadTaskStatus {
    Queued,
    Preparing,
    Downloading,
    Writing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadErrorCode {
    Network,
    Api,
    Io,
    Decode,
    Tagging,
    Lyrics,
    Cancelled,
    InvalidRequest,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadErrorInfo {
    pub code: DownloadErrorCode,
    pub message: String,
    pub retryable: bool,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTaskSnapshot {
    pub id: String,
    pub job_id: String,
    pub song_cid: String,
    pub song_name: String,
    pub artists: Vec<String>,
    pub album_cid: String,
    pub album_name: String,
    pub status: DownloadTaskStatus,
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
    pub output_path: Option<String>,
    pub error: Option<DownloadErrorInfo>,
    pub attempt: u32,
    pub song_index: usize,
    pub song_count: usize,
}

/// Internal task representation used by DownloadService.
#[derive(Debug, Clone)]
pub struct InternalDownloadTask {
    pub id: String,
    pub job_id: String,
    pub song_cid: String,
    pub song_name: String,
    pub artists: Vec<String>,
    pub album_cid: String,
    pub album_name: String,
    pub status: DownloadTaskStatus,
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
    pub output_path: Option<String>,
    pub error: Option<DownloadErrorInfo>,
    pub attempt: u32,
    pub song_index: usize,
    pub song_count: usize,
    /// Target output format for this task (from job options).
    pub format: crate::audio::OutputFormat,
    /// Whether to download lyrics sidecar (from job options).
    pub download_lyrics: bool,
}

impl InternalDownloadTask {
    /// Convert to the frontend-facing snapshot.
    pub fn to_snapshot(&self, root_output_dir: &str) -> DownloadTaskSnapshot {
        DownloadTaskSnapshot {
            id: self.id.clone(),
            job_id: self.job_id.clone(),
            song_cid: self.song_cid.clone(),
            song_name: self.song_name.clone(),
            artists: self.artists.clone(),
            album_cid: self.album_cid.clone(),
            album_name: self.album_name.clone(),
            status: self.status,
            bytes_done: self.bytes_done,
            bytes_total: self.bytes_total,
            output_path: self
                .output_path
                .as_deref()
                .and_then(|path| normalize_snapshot_path(path, root_output_dir)),
            error: self.error.clone(),
            attempt: self.attempt,
            song_index: self.song_index,
            song_count: self.song_count,
        }
    }
}

/// Internal job representation used by DownloadService.
#[derive(Debug, Clone)]
pub struct DownloadJob {
    pub id: String,
    pub kind: DownloadJobKind,
    pub status: DownloadJobStatus,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub options: DownloadOptions,
    pub title: String,
    pub tasks: Vec<InternalDownloadTask>,
    pub error: Option<DownloadErrorInfo>,
}

impl DownloadJob {
    /// Convert to the frontend-facing snapshot.
    pub fn to_snapshot(&self) -> DownloadJobSnapshot {
        let completed_task_count = self
            .tasks
            .iter()
            .filter(|t| t.status == DownloadTaskStatus::Completed)
            .count();
        let failed_task_count = self
            .tasks
            .iter()
            .filter(|t| t.status == DownloadTaskStatus::Failed)
            .count();
        let cancelled_task_count = self
            .tasks
            .iter()
            .filter(|t| t.status == DownloadTaskStatus::Cancelled)
            .count();

        DownloadJobSnapshot {
            id: self.id.clone(),
            kind: self.kind,
            status: self.status,
            created_at: self.created_at.clone(),
            started_at: self.started_at.clone(),
            finished_at: self.finished_at.clone(),
            options: self.options.clone(),
            title: self.title.clone(),
            task_count: self.tasks.len(),
            completed_task_count,
            failed_task_count,
            cancelled_task_count,
            tasks: self
                .tasks
                .iter()
                .map(|t| t.to_snapshot(&self.options.output_dir))
                .collect(),
            error: self.error.clone(),
        }
    }

    pub fn job_status(&self) -> DownloadJobStatus {
        if self.tasks.is_empty() {
            return self.status;
        }

        let has_non_terminal = self.tasks.iter().any(|task| {
            matches!(
                task.status,
                DownloadTaskStatus::Queued
                    | DownloadTaskStatus::Preparing
                    | DownloadTaskStatus::Downloading
                    | DownloadTaskStatus::Writing
            )
        });
        if has_non_terminal {
            return DownloadJobStatus::Running;
        }

        let completed_count = self
            .tasks
            .iter()
            .filter(|task| task.status == DownloadTaskStatus::Completed)
            .count();
        let failed_count = self
            .tasks
            .iter()
            .filter(|task| task.status == DownloadTaskStatus::Failed)
            .count();
        let cancelled_count = self
            .tasks
            .iter()
            .filter(|task| task.status == DownloadTaskStatus::Cancelled)
            .count();

        if completed_count == self.tasks.len() {
            return DownloadJobStatus::Completed;
        }

        if completed_count > 0 && (failed_count > 0 || cancelled_count > 0) {
            return DownloadJobStatus::PartiallyFailed;
        }

        if completed_count == 0 && failed_count > 0 {
            return DownloadJobStatus::Failed;
        }

        if completed_count == 0 && cancelled_count > 0 {
            return DownloadJobStatus::Cancelled;
        }

        self.status
    }
}

fn normalize_snapshot_path(path: &str, root_output_dir: &str) -> Option<String> {
    let path = Path::new(path);
    let root = Path::new(root_output_dir);

    path.strip_prefix(root)
        .map(|relative| {
            relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        })
        .ok()
        .filter(|relative| !relative.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        DownloadJob, DownloadJobKind, DownloadJobStatus, DownloadOptions, DownloadTaskStatus,
        InternalDownloadTask,
    };
    use crate::audio::OutputFormat;

    fn make_task(status: DownloadTaskStatus) -> InternalDownloadTask {
        InternalDownloadTask {
            id: "task-1".to_string(),
            job_id: "job-1".to_string(),
            song_cid: "song-1".to_string(),
            song_name: "Song".to_string(),
            artists: vec!["Artist".to_string()],
            album_cid: "album-1".to_string(),
            album_name: "Album".to_string(),
            status,
            bytes_done: 0,
            bytes_total: None,
            output_path: None,
            error: None,
            attempt: 0,
            song_index: 0,
            song_count: 2,
            format: OutputFormat::Flac,
            download_lyrics: true,
        }
    }

    fn make_job(tasks: Vec<InternalDownloadTask>) -> DownloadJob {
        DownloadJob {
            id: "job-1".to_string(),
            kind: DownloadJobKind::Album,
            status: DownloadJobStatus::Running,
            created_at: "2026-04-15T00:00:00Z".to_string(),
            started_at: None,
            finished_at: None,
            options: DownloadOptions {
                output_dir: "/tmp".to_string(),
                format: OutputFormat::Flac,
                download_lyrics: true,
            },
            title: "Album".to_string(),
            tasks,
            error: None,
        }
    }

    #[test]
    fn returns_completed_when_all_tasks_completed() {
        let job = make_job(vec![
            make_task(DownloadTaskStatus::Completed),
            make_task(DownloadTaskStatus::Completed),
        ]);

        assert_eq!(job.job_status(), DownloadJobStatus::Completed);
    }

    #[test]
    fn returns_partially_failed_when_completed_and_failed_are_mixed() {
        let job = make_job(vec![
            make_task(DownloadTaskStatus::Completed),
            make_task(DownloadTaskStatus::Failed),
        ]);

        assert_eq!(job.job_status(), DownloadJobStatus::PartiallyFailed);
    }

    #[test]
    fn returns_failed_when_no_task_completed_and_some_failed() {
        let job = make_job(vec![
            make_task(DownloadTaskStatus::Failed),
            make_task(DownloadTaskStatus::Cancelled),
        ]);

        assert_eq!(job.job_status(), DownloadJobStatus::Failed);
    }

    #[test]
    fn returns_cancelled_when_no_task_completed_and_only_cancelled() {
        let job = make_job(vec![
            make_task(DownloadTaskStatus::Cancelled),
            make_task(DownloadTaskStatus::Cancelled),
        ]);

        assert_eq!(job.job_status(), DownloadJobStatus::Cancelled);
    }

    #[test]
    fn normalizes_output_path_to_relative_snapshot_form() {
        let task = InternalDownloadTask {
            id: "task-1".to_string(),
            job_id: "job-1".to_string(),
            song_cid: "song-1".to_string(),
            song_name: "Song".to_string(),
            artists: vec!["Artist".to_string()],
            album_cid: "album-1".to_string(),
            album_name: "Album".to_string(),
            status: DownloadTaskStatus::Completed,
            bytes_done: 0,
            bytes_total: None,
            output_path: Some("/Users/me/Downloads/SirenMusic/Album/Disc 1/Track.flac".to_string()),
            error: None,
            attempt: 0,
            song_index: 0,
            song_count: 1,
            format: OutputFormat::Flac,
            download_lyrics: true,
        };

        let snapshot = task.to_snapshot("/Users/me/Downloads/SirenMusic");

        assert_eq!(
            snapshot.output_path.as_deref(),
            Some("Album/Disc 1/Track.flac")
        );
    }

    #[test]
    fn omits_output_path_when_it_cannot_be_normalized_from_root() {
        let task = InternalDownloadTask {
            id: "task-1".to_string(),
            job_id: "job-1".to_string(),
            song_cid: "song-1".to_string(),
            song_name: "Song".to_string(),
            artists: vec!["Artist".to_string()],
            album_cid: "album-1".to_string(),
            album_name: "Album".to_string(),
            status: DownloadTaskStatus::Completed,
            bytes_done: 0,
            bytes_total: None,
            output_path: Some("/other-root/Album/Track.flac".to_string()),
            error: None,
            attempt: 0,
            song_index: 0,
            song_count: 1,
            format: OutputFormat::Flac,
            download_lyrics: true,
        };

        let snapshot = task.to_snapshot("/Users/me/Downloads/SirenMusic");

        assert_eq!(snapshot.output_path, None);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJobSnapshot {
    pub id: String,
    pub kind: DownloadJobKind,
    pub status: DownloadJobStatus,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub options: DownloadOptions,
    pub title: String,
    pub task_count: usize,
    pub completed_task_count: usize,
    pub failed_task_count: usize,
    pub cancelled_task_count: usize,
    pub tasks: Vec<DownloadTaskSnapshot>,
    pub error: Option<DownloadErrorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadManagerSnapshot {
    pub jobs: Vec<DownloadJobSnapshot>,
    pub active_job_id: Option<String>,
    pub queued_job_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDownloadJobRequest {
    pub kind: DownloadJobKind,
    pub song_cids: Vec<String>,
    pub album_cid: Option<String>,
    pub options: DownloadOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTaskProgressEvent {
    pub job_id: String,
    pub task_id: String,
    pub status: DownloadTaskStatus,
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
    pub song_index: usize,
    pub song_count: usize,
    /// Download speed in bytes per second (averaged over recent samples).
    pub speed_bytes_per_sec: f64,
}
