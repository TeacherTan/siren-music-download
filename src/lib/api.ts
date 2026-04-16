import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { getCached, setCached } from './cache';
import type {
  Album, AlbumDetail, SongDetail, ThemePalette, PlayerState, PlaybackContext,
  CreateDownloadJobRequest, DownloadJobSnapshot, DownloadManagerSnapshot,
  NotificationPermissionState, NotificationPreferences,
} from './types';
import type { OutputFormat } from './types';

// Cache key prefixes for different API endpoints
const CACHE_KEY_ALBUM_DETAIL = 'album_detail:';
const CACHE_KEY_SONG_DETAIL = 'song_detail:';
const CACHE_KEY_SONG_LYRICS = 'song_lyrics:';
const CACHE_KEY_IMAGE_THEME = 'image_theme:';

/**
 * Get albums list (no caching - always fetch fresh data).
 */
export async function getAlbums(): Promise<Album[]> {
  return invoke('get_albums');
}

/**
 * Get album detail with caching (6h TTL).
 */
export async function getAlbumDetail(albumCid: string): Promise<AlbumDetail> {
  const cacheKey = `${CACHE_KEY_ALBUM_DETAIL}${albumCid}`;
  const cached = getCached<AlbumDetail>(cacheKey);
  if (cached) {
    return cached;
  }
  const data = await invoke<AlbumDetail>('get_album_detail', { albumCid });
  setCached(cacheKey, data);
  return data;
}

/**
 * Get song detail with caching (6h TTL).
 */
export async function getSongDetail(songCid: string): Promise<SongDetail> {
  const cacheKey = `${CACHE_KEY_SONG_DETAIL}${songCid}`;
  const cached = getCached<SongDetail>(cacheKey);
  if (cached) {
    return cached;
  }
  const data = await invoke<SongDetail>('get_song_detail', { cid: songCid });
  setCached(cacheKey, data);
  return data;
}

/**
 * Get song lyric text with caching (6h TTL).
 */
export async function getSongLyrics(songCid: string): Promise<string | null> {
  const cacheKey = `${CACHE_KEY_SONG_LYRICS}${songCid}`;
  const cached = getCached<{ text: string | null }>(cacheKey);
  if (cached) {
    return cached.text;
  }
  const data = await invoke<string | null>('get_song_lyrics', { cid: songCid });
  setCached(cacheKey, { text: data });
  return data;
}

/**
 * Play a song (no caching).
 */
export async function playSong(
  songCid: string,
  coverUrl?: string,
  playbackContext?: PlaybackContext,
): Promise<number> {
  return invoke('play_song', {
    songCid,
    coverUrl: coverUrl ?? null,
    playbackContext: playbackContext ?? null,
  });
}

/**
 * Stop playback (no caching).
 */
export async function stopPlayback(): Promise<void> {
  return invoke('stop_playback');
}

/**
 * Pause playback.
 */
export async function pausePlayback(): Promise<void> {
  return invoke('pause_playback');
}

/**
 * Resume playback.
 */
export async function resumePlayback(): Promise<void> {
  return invoke('resume_playback');
}

/**
 * Seek current playback to target position in seconds.
 */
export async function seekCurrentPlayback(positionSecs: number): Promise<number> {
  return invoke('seek_current_playback', { positionSecs });
}

/**
 * Play next track in current playback context.
 */
export async function playNext(): Promise<number> {
  return invoke('play_next');
}

/**
 * Play previous track in current playback context.
 */
export async function playPrevious(): Promise<number> {
  return invoke('play_previous');
}

/**
 * Get current player state (no caching).
 */
export async function getPlayerState(): Promise<PlayerState> {
  return invoke('get_player_state');
}

/**
 * Update current playback volume, clamped to 0..1.
 */
export async function setPlaybackVolume(volume: number): Promise<number> {
  return invoke('set_playback_volume', { volume });
}

/**
 * Get default output directory (no caching).
 */
export async function getDefaultOutputDir(): Promise<string> {
  return invoke('get_default_output_dir');
}

/**
 * Open directory selection dialog.
 */
export async function selectDirectory(defaultPath?: string): Promise<string | null> {
  return open({
    directory: true,
    defaultPath,
  });
}

/**
 * Clear audio cache stored by the Tauri backend.
 */
export async function clearAudioCache(): Promise<number> {
  return invoke('clear_audio_cache');
}

/**
 * Download a single song into the selected output directory.
 */
export async function downloadSong(
  songCid: string,
  outputDir: string,
  format: OutputFormat,
  downloadLyrics: boolean,
): Promise<string> {
  return invoke('download_song', {
    songCid,
    outputDir,
    format,
    downloadLyrics,
  });
}

/**
 * Extract a theme palette from artwork and cache it for reuse.
 */
export async function extractImageTheme(imageUrl: string): Promise<ThemePalette> {
  const cacheKey = `${CACHE_KEY_IMAGE_THEME}${imageUrl}`;
  const cached = getCached<ThemePalette>(cacheKey);
  if (cached) {
    return cached;
  }

  const data = await invoke<ThemePalette>('extract_image_theme', { imageUrl });
  setCached(cacheKey, data);
  return data;
}

// ---------------------------------------------------------------------------
// Download job API (mirrors commands in src-tauri/src/commands/downloads.rs)
// ---------------------------------------------------------------------------

/**
 * Create a new download job (song, album, or selection).
 * The job is queued and executed asynchronously; subscribe to
 * download-task-progress / download-job-updated events for progress updates.
 */
export async function createDownloadJob(request: CreateDownloadJobRequest): Promise<DownloadJobSnapshot> {
  return invoke('create_download_job', { request });
}

/**
 * List all download jobs with their current state.
 */
export async function listDownloadJobs(): Promise<DownloadManagerSnapshot> {
  return invoke('list_download_jobs');
}

/**
 * Get a specific download job by ID.
 */
export async function getDownloadJob(jobId: string): Promise<DownloadJobSnapshot | null> {
  return invoke('get_download_job', { jobId });
}

/**
 * Cancel an entire download job.
 */
export async function cancelDownloadJob(jobId: string): Promise<DownloadJobSnapshot | null> {
  return invoke('cancel_download_job', { jobId });
}

/**
 * Cancel a specific task within a download job.
 */
export async function cancelDownloadTask(jobId: string, taskId: string): Promise<DownloadJobSnapshot | null> {
  return invoke('cancel_download_task', { jobId, taskId });
}

/**
 * Retry all failed/cancelled tasks in a download job.
 */
export async function retryDownloadJob(jobId: string): Promise<DownloadJobSnapshot | null> {
  return invoke('retry_download_job', { jobId });
}

/**
 * Retry a specific failed/cancelled task within a download job.
 */
export async function retryDownloadTask(jobId: string, taskId: string): Promise<DownloadJobSnapshot | null> {
  return invoke('retry_download_task', { jobId, taskId });
}

/**
 * Remove completed/failed/cancelled jobs from history.
 * Returns the number of jobs removed.
 */
export async function clearDownloadHistory(): Promise<number> {
  return invoke('clear_download_history');
}

/**
 * Get notification preferences from the Tauri backend.
 */
export async function getNotificationPreferences(): Promise<NotificationPreferences> {
  return invoke('get_notification_preferences');
}

/**
 * Update notification preferences in the Tauri backend.
 */
export async function setNotificationPreferences(
  preferences: NotificationPreferences,
): Promise<NotificationPreferences> {
  return invoke('set_notification_preferences', { preferences });
}

/**
 * Get notification permission state from the Tauri backend.
 */
export async function getNotificationPermissionState(): Promise<NotificationPermissionState> {
  return invoke('get_notification_permission_state');
}

/**
 * Send a test notification through the Tauri backend.
 */
export async function sendTestNotification(): Promise<void> {
  return invoke('send_test_notification');
}
