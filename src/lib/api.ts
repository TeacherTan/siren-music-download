import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import {
  cacheManager,
  createAlbumCacheTag,
  createInventoryCacheTag,
  createSongCacheTag,
} from './cache';
import type {
  Album,
  AlbumDetail,
  SongDetail,
  ThemePalette,
  PlayerState,
  PlaybackContext,
  CreateDownloadJobRequest,
  DownloadJobSnapshot,
  DownloadManagerSnapshot,
  NotificationPermissionState,
  AppPreferences,
  LocalInventorySnapshot,
  VerificationMode,
  LogViewerPage,
  LogViewerQuery,
  LogFileStatus,
  SearchLibraryRequest,
  SearchLibraryResponse,
} from './types';
import type { OutputFormat } from './types';

const CACHE_KEY_ALBUM_DETAIL = 'album_detail:';
const CACHE_KEY_SONG_DETAIL = 'song_detail:';
const CACHE_KEY_SONG_LYRICS = 'song_lyrics:';
const CACHE_KEY_IMAGE_THEME = 'image_theme:';
const CACHE_KEY_IMAGE_DATA_URL = 'image_data_url:';

export async function getAlbums(): Promise<Album[]> {
  return invoke('get_albums');
}

export async function getAlbumDetail(
  albumCid: string,
  inventoryVersion?: string | null
): Promise<AlbumDetail> {
  const cacheScope = inventoryVersion ?? 'unversioned';
  const cacheKey = `${CACHE_KEY_ALBUM_DETAIL}${cacheScope}:${albumCid}`;
  const cached = await cacheManager.albums.get(cacheKey);
  if (cached.found) {
    return cached.data as AlbumDetail;
  }

  const data = await invoke<AlbumDetail>('get_album_detail', { albumCid });
  await cacheManager.albums.set(cacheKey, data, [
    createAlbumCacheTag(albumCid),
    createInventoryCacheTag(inventoryVersion),
  ]);
  return data;
}

export async function getSongDetail(
  songCid: string,
  inventoryVersion?: string | null
): Promise<SongDetail> {
  const cacheScope = inventoryVersion ?? 'unversioned';
  const cacheKey = `${CACHE_KEY_SONG_DETAIL}${cacheScope}:${songCid}`;
  const cached = await cacheManager.songs.get(cacheKey);
  if (cached.found) {
    return cached.data as SongDetail;
  }

  const data = await invoke<SongDetail>('get_song_detail', { cid: songCid });
  await cacheManager.songs.set(cacheKey, data, [
    createSongCacheTag(songCid),
    createAlbumCacheTag(data.albumCid),
    createInventoryCacheTag(inventoryVersion),
  ]);
  return data;
}

export async function getSongLyrics(songCid: string): Promise<string | null> {
  const cacheKey = `${CACHE_KEY_SONG_LYRICS}${songCid}`;
  const cached = await cacheManager.lyrics.get(cacheKey);
  if (cached.found) {
    return cached.data;
  }

  const songDetail = await getSongDetail(songCid);
  const data = await invoke<string | null>('get_song_lyrics', { cid: songCid });
  await cacheManager.lyrics.set(cacheKey, data, [
    createSongCacheTag(songCid),
    createAlbumCacheTag(songDetail.albumCid),
  ]);
  return data;
}

export async function searchLibrary(
  request: SearchLibraryRequest
): Promise<SearchLibraryResponse> {
  return invoke<SearchLibraryResponse>('search_library', { request });
}

export async function playSong(
  songCid: string,
  coverUrl?: string,
  playbackContext?: PlaybackContext
): Promise<number> {
  return invoke('play_song', {
    songCid,
    coverUrl: coverUrl ?? null,
    playbackContext: playbackContext ?? null,
  });
}

export async function stopPlayback(): Promise<void> {
  return invoke('stop_playback');
}

export async function pausePlayback(): Promise<void> {
  return invoke('pause_playback');
}

export async function resumePlayback(): Promise<void> {
  return invoke('resume_playback');
}

export async function seekCurrentPlayback(
  positionSecs: number
): Promise<number> {
  return invoke('seek_current_playback', { positionSecs });
}

export async function playNext(): Promise<number> {
  return invoke('play_next');
}

export async function playPrevious(): Promise<number> {
  return invoke('play_previous');
}

export async function getPlayerState(): Promise<PlayerState> {
  return invoke('get_player_state');
}

export async function setPlaybackVolume(volume: number): Promise<number> {
  return invoke('set_playback_volume', { volume });
}

export async function getDefaultOutputDir(): Promise<string> {
  return invoke('get_default_output_dir');
}

export async function selectDirectory(
  defaultPath?: string
): Promise<string | null> {
  return open({
    directory: true,
    defaultPath,
  });
}

export async function clearAudioCache(): Promise<number> {
  return invoke('clear_audio_cache');
}

export async function clearResponseCache(): Promise<void> {
  return invoke('clear_response_cache');
}

export async function extractImageTheme(
  imageUrl: string
): Promise<ThemePalette> {
  const cacheKey = `${CACHE_KEY_IMAGE_THEME}${imageUrl}`;
  const cached = await cacheManager.themes.get(cacheKey);
  if (cached.found) {
    return cached.data as ThemePalette;
  }

  const data = await invoke<ThemePalette>('extract_image_theme', { imageUrl });
  await cacheManager.themes.set(cacheKey, data);
  return data;
}

export async function getImageDataUrl(imageUrl: string): Promise<string> {
  const cacheKey = `${CACHE_KEY_IMAGE_DATA_URL}${imageUrl}`;
  const cached = await cacheManager.covers.get(cacheKey);
  if (cached.found) {
    return cached.data as string;
  }

  const data = await invoke<string>('get_image_data_url', { imageUrl });
  await cacheManager.covers.set(cacheKey, data);
  return data;
}

export async function createDownloadJob(
  request: CreateDownloadJobRequest
): Promise<DownloadJobSnapshot> {
  return invoke('create_download_job', { request });
}

export async function listDownloadJobs(): Promise<DownloadManagerSnapshot> {
  return invoke('list_download_jobs');
}

export async function getDownloadJob(
  jobId: string
): Promise<DownloadJobSnapshot | null> {
  return invoke('get_download_job', { jobId });
}

export async function cancelDownloadJob(
  jobId: string
): Promise<DownloadJobSnapshot | null> {
  return invoke('cancel_download_job', { jobId });
}

export async function cancelDownloadTask(
  jobId: string,
  taskId: string
): Promise<DownloadJobSnapshot | null> {
  return invoke('cancel_download_task', { jobId, taskId });
}

export async function retryDownloadJob(
  jobId: string
): Promise<DownloadJobSnapshot | null> {
  return invoke('retry_download_job', { jobId });
}

export async function retryDownloadTask(
  jobId: string,
  taskId: string
): Promise<DownloadJobSnapshot | null> {
  return invoke('retry_download_task', { jobId, taskId });
}

export async function clearDownloadHistory(): Promise<number> {
  return invoke('clear_download_history');
}

export async function getNotificationPermissionState(): Promise<NotificationPermissionState> {
  return invoke('get_notification_permission_state');
}

export async function sendTestNotification(): Promise<void> {
  return invoke('send_test_notification');
}

export async function getLocalInventorySnapshot(): Promise<LocalInventorySnapshot> {
  return invoke<LocalInventorySnapshot>('get_local_inventory_snapshot');
}

export async function rescanLocalInventory(
  verificationMode?: VerificationMode
): Promise<LocalInventorySnapshot> {
  return invoke<LocalInventorySnapshot>('rescan_local_inventory', {
    verificationMode: verificationMode ?? null,
  });
}

export async function cancelLocalInventoryScan(): Promise<LocalInventorySnapshot> {
  return invoke<LocalInventorySnapshot>('cancel_local_inventory_scan');
}

export async function getPreferences(): Promise<AppPreferences> {
  return invoke<AppPreferences>('get_preferences');
}

export async function setPreferences(
  preferences: AppPreferences
): Promise<AppPreferences> {
  return invoke<AppPreferences>('set_preferences', { preferences });
}

export async function exportPreferences(
  outputPath: string
): Promise<AppPreferences> {
  return invoke<AppPreferences>('export_preferences', { outputPath });
}

export async function importPreferences(
  inputPath: string
): Promise<AppPreferences> {
  return invoke<AppPreferences>('import_preferences', { inputPath });
}

export async function listLogRecords(
  query: LogViewerQuery
): Promise<LogViewerPage> {
  return invoke<LogViewerPage>('list_log_records', { query });
}

export async function getLogFileStatus(): Promise<LogFileStatus> {
  return invoke<LogFileStatus>('get_log_file_status');
}
