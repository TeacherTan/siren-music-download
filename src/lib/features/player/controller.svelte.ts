import type {
  PlaybackContext,
  PlaybackQueueEntry,
  PlayerState,
} from '$lib/types';
import { parseLyricText } from './lyrics';
import { buildPlaybackContext } from './queue';

interface PlayerControllerDeps {
  playSong: (
    songCid: string,
    coverUrl: string | null,
    context: PlaybackContext | null
  ) => Promise<void>;
  pausePlayback: () => Promise<void>;
  resumePlayback: () => Promise<void>;
  seekCurrentPlayback: (positionSecs: number) => Promise<void>;
  getSongLyrics: (songCid: string) => Promise<string | null>;
  notifyError: (message: string) => void;
}

type RepeatMode = 'all' | 'one';

interface PlayerSong {
  cid: string;
  name: string;
  artists: string[];
  coverUrl: string | null;
}

interface LyricLine {
  id: string;
  time: number | null;
  text: string;
}

let initialized = false;

export function createPlayerController(deps: PlayerControllerDeps) {
  let currentSong = $state<PlayerSong | null>(null);
  let isPlaying = $state(false);
  let isPaused = $state(false);
  let isLoading = $state(false);
  let hasPrevious = $state(false);
  let hasNext = $state(false);
  let progress = $state(0);
  let duration = $state(0);
  let shuffleEnabled = $state(false);
  let repeatMode = $state<RepeatMode>('all');
  let playbackEntries = $state<PlaybackQueueEntry[]>([]);
  let playbackOrder = $state<PlaybackQueueEntry[]>([]);
  let playbackIndex = $state(-1);
  let lyricsOpen = $state(false);
  let playlistOpen = $state(false);
  let lyricsLoading = $state(false);
  let lyricsError = $state('');
  let lyricsLines = $state<LyricLine[]>([]);
  let lyricsSongCid = $state<string | null>(null);
  let playingCid = $state<string | null>(null);
  let playbackEndRequestSeq = 0;
  let lastPlaybackSnapshot = {
    cid: null as string | null,
    active: false,
  };
  let lyricRequestSeq = 0;

  function init() {
    if (initialized) return;
    initialized = true;
  }

  function normalizePlayerSong(state: PlayerState): PlayerSong | null {
    if (!state.songCid) return null;

    return {
      cid: state.songCid,
      name: state.songName ?? '',
      artists: state.artists,
      coverUrl: state.coverUrl ?? null,
    };
  }

  function buildSinglePlaybackEntry(song: PlayerSong): PlaybackQueueEntry {
    return {
      cid: song.cid,
      name: song.name,
      artists: song.artists,
      coverUrl: song.coverUrl,
    };
  }

  function shufflePlaybackEntries(
    entries: PlaybackQueueEntry[],
    currentCid: string | null
  ): PlaybackQueueEntry[] {
    if (entries.length <= 1) return [...entries];

    const rest = [...entries];
    let pinnedEntry: PlaybackQueueEntry | null = null;

    if (currentCid) {
      const pinnedIndex = rest.findIndex((entry) => entry.cid === currentCid);
      if (pinnedIndex >= 0) {
        pinnedEntry = rest.splice(pinnedIndex, 1)[0];
      }
    }

    for (let index = rest.length - 1; index > 0; index -= 1) {
      const swapIndex = Math.floor(Math.random() * (index + 1));
      [rest[index], rest[swapIndex]] = [rest[swapIndex], rest[index]];
    }

    return pinnedEntry ? [pinnedEntry, ...rest] : rest;
  }

  function applyPlaybackQueue(
    entries: PlaybackQueueEntry[],
    currentCid: string | null
  ) {
    playbackEntries = [...entries];

    if (!entries.length) {
      playbackOrder = [];
      playbackIndex = -1;
      return;
    }

    playbackOrder = shuffleEnabled
      ? shufflePlaybackEntries(entries, currentCid)
      : [...entries];
    playbackIndex = currentCid
      ? playbackOrder.findIndex((entry) => entry.cid === currentCid)
      : 0;

    if (playbackIndex < 0) {
      playbackIndex = 0;
    }
  }

  function syncPlaybackQueueWithSong(song: PlayerSong | null) {
    if (!song) {
      playbackIndex = -1;
      return;
    }

    const currentOrderIndex = playbackOrder.findIndex(
      (entry) => entry.cid === song.cid
    );
    if (currentOrderIndex >= 0) {
      playbackIndex = currentOrderIndex;
      return;
    }

    const currentSourceIndex = playbackEntries.findIndex(
      (entry) => entry.cid === song.cid
    );
    if (currentSourceIndex >= 0) {
      applyPlaybackQueue(playbackEntries, song.cid);
      return;
    }

    applyPlaybackQueue([buildSinglePlaybackEntry(song)], song.cid);
  }

  async function loadLyrics(songCid: string) {
    const requestSeq = ++lyricRequestSeq;
    lyricsSongCid = songCid;
    lyricsLoading = true;
    lyricsError = '';
    lyricsLines = [];

    try {
      const lyricText = await deps.getSongLyrics(songCid);
      if (requestSeq !== lyricRequestSeq) return;

      if (!lyricText) {
        lyricsLoading = false;
        return;
      }

      lyricsLines = parseLyricText(lyricText);
    } catch (error) {
      if (requestSeq !== lyricRequestSeq) return;
      lyricsError = error instanceof Error ? error.message : String(error);
    } finally {
      if (requestSeq === lyricRequestSeq) {
        lyricsLoading = false;
      }
    }
  }

  function syncPlayerState(state: PlayerState) {
    currentSong = normalizePlayerSong(state);
    isPlaying = state.isPlaying;
    isPaused = state.isPaused;
    isLoading = state.isLoading;
    hasPrevious = state.hasPrevious;
    hasNext = state.hasNext;
    progress = state.progress;
    duration = state.duration;

    if (!state.isLoading) {
      playingCid = null;
    }

    syncPlaybackQueueWithSong(currentSong);
  }

  function syncPlayerProgress(state: PlayerState) {
    progress = state.progress;
    isPlaying = state.isPlaying;
    isPaused = state.isPaused;
    duration = state.duration;
  }

  function syncPlaybackLifecycle() {
    const songCid = currentSong?.cid ?? null;
    const isCurrentActive =
      Boolean(songCid) && (isPlaying || isPaused || isLoading);
    const previousSnapshot = lastPlaybackSnapshot;

    if (
      previousSnapshot.cid &&
      previousSnapshot.active &&
      !isCurrentActive &&
      songCid === previousSnapshot.cid &&
      duration > 0 &&
      progress >= Math.max(0, duration - 0.25)
    ) {
      void handlePlaybackEnded(previousSnapshot.cid);
    }

    if (!songCid) {
      lyricRequestSeq += 1;
      lyricsSongCid = null;
      lyricsLines = [];
      lyricsError = '';
      lyricsLoading = false;
      lyricsOpen = false;
      playlistOpen = false;
      if (previousSnapshot.cid !== null || previousSnapshot.active) {
        lastPlaybackSnapshot = { cid: null, active: false };
      }
      return;
    }

    if (
      previousSnapshot.cid !== songCid ||
      previousSnapshot.active !== isCurrentActive
    ) {
      lastPlaybackSnapshot = {
        cid: songCid,
        active: isCurrentActive,
      };
    }

    if (songCid === lyricsSongCid) {
      return;
    }

    void loadLyrics(songCid);
  }

  async function playQueueEntry(
    entry: PlaybackQueueEntry,
    order = playbackOrder,
    index = order.findIndex((candidate) => candidate.cid === entry.cid),
    options: { forceRestart?: boolean } = {}
  ) {
    if (index < 0) return;

    playbackOrder = [...order];
    playbackIndex = index;

    if (!options.forceRestart) {
      if (currentSong?.cid === entry.cid && isPaused) {
        await deps.resumePlayback();
        return;
      }

      if (currentSong?.cid === entry.cid && (isPlaying || isLoading)) {
        return;
      }

      if (playingCid === entry.cid) {
        return;
      }
    }

    playingCid = entry.cid;
    try {
      const context = buildPlaybackContext(playbackOrder, playbackIndex);
      await deps.playSong(entry.cid, entry.coverUrl ?? null, context ?? null);
    } catch (error) {
      playingCid = null;
      deps.notifyError(
        `播放失败：${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  function resolveWrappedQueueIndex(step: 1 | -1): number {
    if (!playbackOrder.length) return -1;

    const base = playbackIndex >= 0 ? playbackIndex : 0;
    const next = base + step;

    if (next < 0) {
      return playbackOrder.length - 1;
    }
    if (next >= playbackOrder.length) {
      return 0;
    }
    return next;
  }

  function toggleShuffle(next: boolean) {
    const currentCid = currentSong?.cid ?? null;
    shuffleEnabled = next;
    applyPlaybackQueue(playbackEntries, currentCid);
  }

  function toggleRepeat(next: RepeatMode) {
    repeatMode = next;
  }

  function toggleLyrics() {
    if (!currentSong) return;
    lyricsOpen = !lyricsOpen;
    if (lyricsOpen) {
      playlistOpen = false;
    }
  }

  function togglePlaylist() {
    if (!currentSong) return;
    playlistOpen = !playlistOpen;
    if (playlistOpen) {
      lyricsOpen = false;
    }
  }

  async function handlePlaybackEnded(songCid: string) {
    const requestSeq = ++playbackEndRequestSeq;

    if (repeatMode === 'one') {
      const entry = playbackOrder.find((e) => e.cid === songCid);
      if (entry) {
        const index = playbackOrder.indexOf(entry);
        await playQueueEntry(entry, playbackOrder, index, {
          forceRestart: true,
        });
      }
      return;
    }

    if (!playbackOrder.length) return;
    const currentIndex = playbackOrder.findIndex(
      (entry) => entry.cid === songCid
    );
    if (currentIndex < 0) return;

    const nextIndex =
      currentIndex + 1 >= playbackOrder.length ? 0 : currentIndex + 1;
    if (requestSeq !== playbackEndRequestSeq) return;
    await playQueueEntry(playbackOrder[nextIndex], playbackOrder, nextIndex, {
      forceRestart: true,
    });
  }

  async function pause() {
    try {
      await deps.pausePlayback();
    } catch (error) {
      deps.notifyError(
        `暂停播放失败：${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  async function resume() {
    try {
      await deps.resumePlayback();
    } catch (error) {
      deps.notifyError(
        `恢复播放失败：${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  async function seek(positionSecs: number) {
    if (!duration || duration <= 0 || isLoading) return;
    try {
      await deps.seekCurrentPlayback(positionSecs);
    } catch (error) {
      deps.notifyError(
        `跳转播放进度失败：${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  async function playNext() {
    const nextIndex = resolveWrappedQueueIndex(1);
    if (nextIndex < 0) return;
    await playQueueEntry(playbackOrder[nextIndex], playbackOrder, nextIndex);
  }

  async function playPrevious() {
    if (!currentSong) return;

    if (progress > 3 && !isLoading) {
      await seek(0);
      return;
    }

    const previousIndex = resolveWrappedQueueIndex(-1);
    if (previousIndex < 0) return;
    await playQueueEntry(
      playbackOrder[previousIndex],
      playbackOrder,
      previousIndex
    );
  }

  function dispose() {
    initialized = false;
    currentSong = null;
    isPlaying = false;
    isPaused = false;
    isLoading = false;
    hasPrevious = false;
    hasNext = false;
    progress = 0;
    duration = 0;
    shuffleEnabled = false;
    repeatMode = 'all';
    playbackEntries = [];
    playbackOrder = [];
    playbackIndex = -1;
    lyricsOpen = false;
    playlistOpen = false;
    lyricsLoading = false;
    lyricsError = '';
    lyricsLines = [];
    lyricsSongCid = null;
    playingCid = null;
    lastPlaybackSnapshot = { cid: null, active: false };
    lyricRequestSeq += 1;
    playbackEndRequestSeq += 1;
  }

  return {
    get currentSong() {
      return currentSong;
    },
    get isPlaying() {
      return isPlaying;
    },
    get isPaused() {
      return isPaused;
    },
    get isLoading() {
      return isLoading;
    },
    get hasPrevious() {
      return hasPrevious;
    },
    get hasNext() {
      return hasNext;
    },
    get progress() {
      return progress;
    },
    get duration() {
      return duration;
    },
    get shuffleEnabled() {
      return shuffleEnabled;
    },
    get repeatMode() {
      return repeatMode;
    },
    get playbackEntries() {
      return playbackEntries;
    },
    get playbackOrder() {
      return playbackOrder;
    },
    get playbackIndex() {
      return playbackIndex;
    },
    get lyricsOpen() {
      return lyricsOpen;
    },
    get playlistOpen() {
      return playlistOpen;
    },
    get lyricsLoading() {
      return lyricsLoading;
    },
    get lyricsError() {
      return lyricsError;
    },
    get lyricsLines() {
      return lyricsLines;
    },
    get lyricsSongCid() {
      return lyricsSongCid;
    },
    get playingCid() {
      return playingCid;
    },
    get lastPlaybackSnapshot() {
      return lastPlaybackSnapshot;
    },
    get playerHasPrevious() {
      return playbackOrder.length > 1;
    },
    get playerHasNext() {
      return playbackOrder.length > 1;
    },
    init,
    dispose,
    syncPlayerState,
    syncPlayerProgress,
    syncPlaybackLifecycle,
    playQueueEntry,
    toggleShuffle,
    toggleRepeat,
    toggleLyrics,
    togglePlaylist,
    handlePlaybackEnded,
    pause,
    resume,
    seek,
    playNext,
    playPrevious,
    applyPlaybackQueue,
    buildSinglePlaybackEntry,
  };
}

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    initialized = false;
  });
}
