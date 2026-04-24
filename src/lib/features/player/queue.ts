import type { PlaybackContext, PlaybackQueueEntry } from '$lib/types';

export function buildPlaybackContext(
  order: PlaybackQueueEntry[],
  currentIndex: number
): PlaybackContext | undefined {
  if (!order.length || currentIndex < 0 || currentIndex >= order.length) {
    return undefined;
  }

  return {
    currentIndex,
    entries: order.map((entry) => ({
      cid: entry.cid,
      name: entry.name,
      artists: entry.artists,
      coverUrl: entry.coverUrl,
    })),
  };
}
