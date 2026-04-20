//! Download worker: executes individual download tasks.
//!
//! The worker is responsible for the actual file download, format conversion,
//! metadata tagging, and lyric sidecar writing by delegating to the existing
//! `download_song` function.
//!
//! ## Pipeline support
//!
//! In addition to the monolithic [`InternalDownloadTask::execute`], two
//! split-phase methods are provided for pipelined execution:
//!
//! - [`InternalDownloadTask::execute_download_phase`] — network I/O only,
//!   returns a [`WritePayload`].
//! - [`InternalDownloadTask::execute_write_phase`] — disk I/O only, consumes
//!   a [`WritePayload`].

use crate::api::ApiClient;
use crate::download::model::{
    DownloadErrorCode, DownloadErrorInfo, DownloadTaskProgressEvent, InternalDownloadTask,
};
use crate::downloader::{
    download_song, download_song_phase1, write_payload_to_disk, MetaOverride, WritePayload,
};
use anyhow::Error;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Result of executing a download task.
#[derive(Debug)]
pub enum TaskExecutionResult {
    /// Task completed successfully.
    Completed { output_path: String },
    /// Task was cancelled.
    Cancelled,
    /// Task failed with an error.
    Failed(DownloadErrorInfo),
}

impl InternalDownloadTask {
    /// Execute this download task (monolithic: download + write).
    ///
    /// This method downloads the song, converts format if needed, writes metadata,
    /// and optionally saves lyric sidecar.
    ///
    /// The `on_progress` callback is invoked with progress updates.
    pub async fn execute<F>(
        &self,
        api: &ApiClient,
        output_dir: &Path,
        cancellation_flag: Option<Arc<AtomicBool>>,
        on_progress: F,
    ) -> TaskExecutionResult
    where
        F: Fn(DownloadTaskProgressEvent) + Send + Sync + Clone + 'static,
    {
        if is_cancelled(cancellation_flag.as_ref()) {
            return TaskExecutionResult::Cancelled;
        }

        // Fetch song and album details first
        let song = match api.get_song_detail(&self.song_cid).await {
            Ok(s) => s,
            Err(e) => {
                return TaskExecutionResult::Failed(make_error(
                    DownloadErrorCode::Api,
                    "Failed to fetch song detail",
                    e,
                    true,
                ));
            }
        };

        let album = match api.get_album_detail(&self.album_cid).await {
            Ok(a) => a,
            Err(e) => {
                return TaskExecutionResult::Failed(make_error(
                    DownloadErrorCode::Api,
                    "Failed to fetch album detail",
                    e,
                    true,
                ));
            }
        };

        if is_cancelled(cancellation_flag.as_ref()) {
            return TaskExecutionResult::Cancelled;
        }

        // Build progress callback that maps to our event format
        let task_id = self.id.clone();
        let job_id = self.job_id.clone();
        let song_index = self.song_index;
        let song_count = self.song_count;

        // Speed calculation state (using Arc<Mutex> for thread-safe interior mutability)
        let start_time = Instant::now();
        let last_bytes = Arc::new(AtomicU64::new(0));
        let last_time = Arc::new(Mutex::new(start_time));

        // Execute the download using the existing download_song function
        let result = download_song(
            api,
            &song,
            &album,
            output_dir,
            self.format,
            self.download_lyrics,
            &MetaOverride {
                album_name: String::new(),
                artists: Vec::new(),
                album_artists: Vec::new(),
            },
            cancellation_flag.clone(),
            {
                let on_progress = on_progress.clone();
                let last_bytes = Arc::clone(&last_bytes);
                let last_time = Arc::clone(&last_time);
                move |prog| {
                    let now = Instant::now();
                    let prev_time = {
                        let mut time_guard = last_time.lock().unwrap();
                        let prev = *time_guard;
                        *time_guard = now;
                        prev
                    };
                    let elapsed = now.duration_since(prev_time).as_secs_f64();

                    // Calculate speed: bytes downloaded since last update / time elapsed
                    let prev_bytes = last_bytes.swap(prog.bytes_done, Ordering::Relaxed);
                    let speed_bytes_per_sec = if elapsed > 0.0 {
                        let bytes_delta = prog.bytes_done.saturating_sub(prev_bytes);
                        bytes_delta as f64 / elapsed
                    } else {
                        0.0
                    };

                    on_progress(DownloadTaskProgressEvent {
                        job_id: job_id.clone(),
                        task_id: task_id.clone(),
                        status: prog.status,
                        bytes_done: prog.bytes_done,
                        bytes_total: prog.bytes_total,
                        song_index,
                        song_count,
                        speed_bytes_per_sec,
                    });
                }
            },
        )
        .await;

        match result {
            Ok(path) => TaskExecutionResult::Completed {
                output_path: path.to_string_lossy().to_string(),
            },
            Err(e) => {
                // Check if this was a cancellation
                let msg = e.to_string();
                if msg.contains("cancelled") || msg.contains("Canceled") {
                    TaskExecutionResult::Cancelled
                } else {
                    TaskExecutionResult::Failed(classify_error(e))
                }
            }
        }
    }

    /// Execute only the download (network) phase of this task.
    ///
    /// Returns a [`WritePayload`] on success that can later be passed to
    /// [`execute_write_phase`](Self::execute_write_phase) on a write worker.
    /// This enables pipelined execution where song N+1's download overlaps
    /// with song N's disk write.
    pub async fn execute_download_phase<F>(
        &self,
        api: &ApiClient,
        output_dir: &Path,
        cancellation_flag: Option<Arc<AtomicBool>>,
        on_progress: F,
    ) -> Result<WritePayload, TaskExecutionResult>
    where
        F: Fn(DownloadTaskProgressEvent) + Send + Sync + Clone + 'static,
    {
        if is_cancelled(cancellation_flag.as_ref()) {
            return Err(TaskExecutionResult::Cancelled);
        }

        let song = match api.get_song_detail(&self.song_cid).await {
            Ok(s) => s,
            Err(e) => {
                return Err(TaskExecutionResult::Failed(make_error(
                    DownloadErrorCode::Api,
                    "Failed to fetch song detail",
                    e,
                    true,
                )));
            }
        };

        let album = match api.get_album_detail(&self.album_cid).await {
            Ok(a) => a,
            Err(e) => {
                return Err(TaskExecutionResult::Failed(make_error(
                    DownloadErrorCode::Api,
                    "Failed to fetch album detail",
                    e,
                    true,
                )));
            }
        };

        if is_cancelled(cancellation_flag.as_ref()) {
            return Err(TaskExecutionResult::Cancelled);
        }

        let task_id = self.id.clone();
        let job_id = self.job_id.clone();
        let song_index = self.song_index;
        let song_count = self.song_count;

        let start_time = Instant::now();
        let last_bytes = Arc::new(AtomicU64::new(0));
        let last_time = Arc::new(Mutex::new(start_time));

        let result = download_song_phase1(
            api,
            &song,
            &album,
            output_dir,
            self.format,
            self.download_lyrics,
            &MetaOverride {
                album_name: String::new(),
                artists: Vec::new(),
                album_artists: Vec::new(),
            },
            cancellation_flag,
            {
                let on_progress = on_progress.clone();
                let last_bytes = Arc::clone(&last_bytes);
                let last_time = Arc::clone(&last_time);
                move |prog| {
                    let now = Instant::now();
                    let prev_time = {
                        let mut time_guard = last_time.lock().unwrap();
                        let prev = *time_guard;
                        *time_guard = now;
                        prev
                    };
                    let elapsed = now.duration_since(prev_time).as_secs_f64();

                    let prev_bytes = last_bytes.swap(prog.bytes_done, Ordering::Relaxed);
                    let speed_bytes_per_sec = if elapsed > 0.0 {
                        let bytes_delta = prog.bytes_done.saturating_sub(prev_bytes);
                        bytes_delta as f64 / elapsed
                    } else {
                        0.0
                    };

                    on_progress(DownloadTaskProgressEvent {
                        job_id: job_id.clone(),
                        task_id: task_id.clone(),
                        status: prog.status,
                        bytes_done: prog.bytes_done,
                        bytes_total: prog.bytes_total,
                        song_index,
                        song_count,
                        speed_bytes_per_sec,
                    });
                }
            },
        )
        .await;

        match result {
            Ok(payload) => Ok(payload),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("cancelled") || msg.contains("Canceled") {
                    Err(TaskExecutionResult::Cancelled)
                } else {
                    Err(TaskExecutionResult::Failed(classify_error(e)))
                }
            }
        }
    }

    /// Execute only the write (disk I/O) phase using a previously obtained
    /// [`WritePayload`].
    ///
    /// This is the counterpart to [`execute_download_phase`](Self::execute_download_phase).
    pub fn execute_write_phase<F>(
        &self,
        payload: &WritePayload,
        on_progress: F,
    ) -> TaskExecutionResult
    where
        F: Fn(DownloadTaskProgressEvent) + Send + Sync + Clone + 'static,
    {
        let task_id = self.id.clone();
        let job_id = self.job_id.clone();
        let song_index = self.song_index;
        let song_count = self.song_count;

        let progress_adapter = |prog: crate::downloader::DownloadProgress| {
            on_progress(DownloadTaskProgressEvent {
                job_id: job_id.clone(),
                task_id: task_id.clone(),
                status: prog.status,
                bytes_done: prog.bytes_done,
                bytes_total: prog.bytes_total,
                song_index,
                song_count,
                speed_bytes_per_sec: 0.0,
            });
        };

        match write_payload_to_disk(payload, Some(&progress_adapter)) {
            Ok(path) => TaskExecutionResult::Completed {
                output_path: path.to_string_lossy().to_string(),
            },
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("cancelled") || msg.contains("Canceled") {
                    TaskExecutionResult::Cancelled
                } else {
                    TaskExecutionResult::Failed(classify_error(e))
                }
            }
        }
    }
}

fn sanitized_error_details(error: &anyhow::Error) -> Option<String> {
    let message = error.to_string();
    if message.is_empty() {
        return None;
    }

    let normalized = message.replace('\\', "/");
    let looks_like_path = normalized.starts_with('/') || normalized.contains(":/");
    if looks_like_path {
        return Path::new(&normalized)
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .filter(|value| !value.is_empty());
    }

    if normalized.contains('/') {
        return None;
    }

    Some(normalized)
}

fn make_error(
    code: DownloadErrorCode,
    message: &str,
    e: anyhow::Error,
    retryable: bool,
) -> DownloadErrorInfo {
    DownloadErrorInfo {
        code,
        message: message.to_string(),
        retryable,
        details: sanitized_error_details(&e),
    }
}

fn is_cancelled(cancellation_flag: Option<&Arc<AtomicBool>>) -> bool {
    cancellation_flag
        .map(|flag| flag.load(Ordering::SeqCst))
        .unwrap_or(false)
}

fn classify_error(e: anyhow::Error) -> DownloadErrorInfo {
    let msg = e.to_string();
    let lower = msg.to_ascii_lowercase();

    if lower.contains("cancelled") || lower.contains("canceled") {
        return make_error(DownloadErrorCode::Cancelled, "Cancelled by user", e, false);
    }

    if lower.contains("lyric") || lower.contains("lrc") {
        return make_error(DownloadErrorCode::Lyrics, "Lyrics download failed", e, true);
    }

    if lower.contains("metadata") || lower.contains("metaflac") || lower.contains("tag") {
        return make_error(
            DownloadErrorCode::Tagging,
            "Failed to write metadata",
            e,
            false,
        );
    }

    if lower.contains("network")
        || lower.contains("connection")
        || lower.contains("timeout")
        || lower.contains("dns")
        || lower.contains("tls")
    {
        return make_error(DownloadErrorCode::Network, "Network error", e, true);
    }

    if lower.contains("api") || lower.contains("404") || lower.contains("500") {
        return make_error(DownloadErrorCode::Api, "API error", e, true);
    }

    if lower.contains("decode") || lower.contains("encode") || lower.contains("flac") {
        return make_error(DownloadErrorCode::Decode, "Decode/encode error", e, false);
    }

    if is_io_error(&e, &lower) {
        return make_error(DownloadErrorCode::Io, "IO error", e, false);
    }

    make_error(DownloadErrorCode::Internal, "Internal error", e, false)
}

fn is_io_error(error: &Error, lower_message: &str) -> bool {
    error.downcast_ref::<std::io::Error>().is_some()
        || lower_message.contains("write")
        || lower_message.contains("save audio")
        || lower_message.contains("sidecar")
        || lower_message.contains("failed to save")
        || lower_message.contains("failed to write")
        || lower_message.contains("disk")
        || lower_message.contains("permission denied")
        || lower_message.contains("no space left")
}

#[cfg(test)]
mod tests {
    use super::classify_error;
    use crate::download::model::DownloadErrorCode;
    use anyhow::anyhow;

    #[test]
    fn classifies_lyric_errors_explicitly() {
        let error = classify_error(anyhow!("Failed to save lyric sidecar for song"));

        assert!(matches!(error.code, DownloadErrorCode::Lyrics));
        assert!(error.retryable);
    }

    #[test]
    fn classifies_metadata_errors_before_io_errors() {
        let error = classify_error(anyhow!("Failed to write FLAC metadata for song"));

        assert!(matches!(error.code, DownloadErrorCode::Tagging));
        assert!(!error.retryable);
    }

    #[test]
    fn classifies_io_errors_from_underlying_io_type() {
        let io_error =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let error = classify_error(io_error.into());

        assert!(matches!(error.code, DownloadErrorCode::Io));
        assert!(!error.retryable);
    }
}
