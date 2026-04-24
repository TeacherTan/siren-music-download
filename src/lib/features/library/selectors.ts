import type { AlbumDetail, PlaybackQueueEntry } from '$lib/types';

export function getSelectedAlbumCoverUrl(
  album: AlbumDetail | null
): string | null {
  return album?.coverUrl ?? album?.coverDeUrl ?? null;
}

export function buildAlbumPlaybackEntries(
  album: AlbumDetail | null
): PlaybackQueueEntry[] {
  if (!album) return [];

  const coverUrl = album.coverUrl ?? album.coverDeUrl ?? null;
  return album.songs.map((entry) => ({
    cid: entry.cid,
    name: entry.name,
    artists: entry.artists,
    coverUrl,
  }));
}
