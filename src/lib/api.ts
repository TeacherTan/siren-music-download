import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { getCached, setCached } from './cache';
import type { Album, AlbumDetail, SongDetail, PlayerState } from './types';

// Cache key prefixes for different API endpoints
const CACHE_KEY_ALBUM_DETAIL = 'album_detail:';
const CACHE_KEY_SONG_DETAIL = 'song_detail:';

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
 * Play a song (no caching).
 */
export async function playSong(songCid: string, coverUrl?: string): Promise<number> {
  return invoke('play_song', { songCid, coverUrl: coverUrl ?? null });
}

/**
 * Stop playback (no caching).
 */
export async function stopPlayback(): Promise<void> {
  return invoke('stop_playback');
}

/**
 * Get current player state (no caching).
 */
export async function getPlayerState(): Promise<PlayerState> {
  return invoke('get_player_state');
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