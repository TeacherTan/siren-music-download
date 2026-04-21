use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;
use walkdir::WalkDir;

const APP_CACHE_DIR: &str = "siren-music-download";
const AUDIO_CACHE_DIR: &str = "audio";
const AUDIO_CACHE_SOFT_LIMIT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const AUDIO_CACHE_TARGET_BYTES: u64 = AUDIO_CACHE_SOFT_LIMIT_BYTES * 8 / 10;

static AUDIO_CACHE_CLEANUP_RUNNING: AtomicBool = AtomicBool::new(false);

struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        AUDIO_CACHE_CLEANUP_RUNNING.store(false, Ordering::SeqCst);
    }
}

struct CacheFileEntry {
    path: PathBuf,
    size: u64,
    modified_at: SystemTime,
}

pub fn audio_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| std::env::temp_dir().join("cache"))
        .join(APP_CACHE_DIR)
        .join(AUDIO_CACHE_DIR)
}

pub fn ensure_audio_cache_dir() -> Result<PathBuf> {
    let dir = audio_cache_dir();
    fs::create_dir_all(&dir).context("Failed to create audio cache directory")?;
    Ok(dir)
}

pub fn cached_song_path(song_cid: &str, source_url: &str) -> Result<PathBuf> {
    let extension = Path::new(source_url.split('?').next().unwrap_or(source_url))
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("bin");

    Ok(ensure_audio_cache_dir()?.join(format!("{song_cid}.{extension}")))
}

pub fn pending_marker_path(cache_path: &Path) -> PathBuf {
    let mut marker_name = cache_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("audio")
        .to_string();
    marker_name.push_str(".pending");
    cache_path.with_file_name(marker_name)
}

pub fn is_song_cached(cache_path: &Path) -> bool {
    cache_path.is_file() && !pending_marker_path(cache_path).exists()
}

pub fn spawn_cleanup_if_needed() {
    let Ok(dir) = ensure_audio_cache_dir() else {
        return;
    };

    let Ok(total_size) = calculate_cache_size(&dir) else {
        return;
    };

    if total_size <= AUDIO_CACHE_SOFT_LIMIT_BYTES {
        return;
    }

    if AUDIO_CACHE_CLEANUP_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(move || {
        let _guard = CleanupGuard;
        let _ = evict_cache_dir_until_target(
            &dir,
            AUDIO_CACHE_SOFT_LIMIT_BYTES,
            AUDIO_CACHE_TARGET_BYTES,
        );
    });
}

pub fn clear_audio_cache() -> Result<u64> {
    let dir = ensure_audio_cache_dir()?;
    let mut removed = 0_u64;

    for entry in fs::read_dir(&dir).context("Failed to read audio cache directory")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .with_context(|| format!("Failed to remove cache directory {}", path.display()))?;
        } else {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to remove cache file {}", path.display()))?;
        }
        removed += 1;
    }

    Ok(removed)
}

fn is_pending_marker(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.ends_with(".pending"))
        .unwrap_or(false)
}

fn calculate_cache_size(dir: &Path) -> Result<u64> {
    let mut total_size = 0_u64;

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if !entry.file_type().is_file() || is_pending_marker(entry.path()) {
            continue;
        }
        total_size += entry.metadata()?.len();
    }

    Ok(total_size)
}

fn collect_cache_files(dir: &Path) -> Result<Vec<CacheFileEntry>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if !entry.file_type().is_file() || is_pending_marker(entry.path()) {
            continue;
        }

        let metadata = entry.metadata()?;
        files.push(CacheFileEntry {
            path: entry.path().to_path_buf(),
            size: metadata.len(),
            modified_at: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        });
    }

    files.sort_by_key(|entry| entry.modified_at);
    Ok(files)
}

fn evict_cache_dir_until_target(dir: &Path, soft_limit: u64, target_size: u64) -> Result<u64> {
    let mut total_size = calculate_cache_size(dir)?;
    if total_size <= soft_limit {
        return Ok(total_size);
    }

    let files = collect_cache_files(dir)?;
    for file in files {
        if total_size <= target_size {
            break;
        }

        match fs::remove_file(&file.path) {
            Ok(()) => {
                total_size = total_size.saturating_sub(file.size);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("Failed to remove cache file {}", file.path.display())
                })
            }
        }
    }

    Ok(total_size)
}

#[cfg(test)]
mod tests {
    use super::{calculate_cache_size, evict_cache_dir_until_target, pending_marker_path};
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn excludes_pending_marker_size_from_cache_accounting() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_file = temp_dir.path().join("song.flac");
        fs::write(&audio_file, vec![1_u8; 32]).expect("audio file");
        fs::write(pending_marker_path(&audio_file), vec![1_u8; 64]).expect("pending marker");

        let total_size = calculate_cache_size(temp_dir.path()).expect("cache size");
        assert_eq!(total_size, 32);
    }

    #[test]
    fn evicts_oldest_files_until_target_size_is_reached() {
        let temp_dir = tempdir().expect("tempdir");
        let oldest = temp_dir.path().join("oldest.flac");
        fs::write(&oldest, vec![1_u8; 40]).expect("oldest");
        thread::sleep(Duration::from_millis(20));

        let middle = temp_dir.path().join("middle.flac");
        fs::write(&middle, vec![1_u8; 40]).expect("middle");
        thread::sleep(Duration::from_millis(20));

        let newest = temp_dir.path().join("newest.flac");
        fs::write(&newest, vec![1_u8; 40]).expect("newest");

        let remaining_size =
            evict_cache_dir_until_target(temp_dir.path(), 80, 40).expect("eviction");

        assert_eq!(remaining_size, 40);
        assert!(!oldest.exists());
        assert!(!middle.exists());
        assert!(newest.exists());
    }
}
