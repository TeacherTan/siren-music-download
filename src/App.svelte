<script lang="ts">
  import { onMount, tick } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { AnimatePresence, motion } from "@humanspeak/svelte-motion";
  import { OverlayScrollbarsComponent } from "overlayscrollbars-svelte";
  import type { OverlayScrollbarsComponentRef } from "overlayscrollbars-svelte";
  import type {
    EventListeners,
    OverlayScrollbars,
    PartialOptions,
  } from "overlayscrollbars";
  import {
    getAlbums,
    getAlbumDetail,
    getDefaultOutputDir,
    selectDirectory,
    playSong,
    pausePlayback,
    resumePlayback,
    seekCurrentPlayback,
    getPlayerState,
    clearAudioCache,
    clearResponseCache,
    extractImageTheme,
    getImageDataUrl,
    getSongLyrics,
    createDownloadJob,
    listDownloadJobs,
    cancelDownloadJob,
    cancelDownloadTask,
    retryDownloadJob,
    retryDownloadTask,
    clearDownloadHistory,
    sendTestNotification,
    getPreferences,
    setPreferences,
    getLocalInventorySnapshot,
    listLogRecords,
    getLogFileStatus,
  } from "$lib/api";
  import {
    clearCache,
    createInventoryCacheTag,
    invalidateByTag,
    warmCacheManager,
  } from "$lib/cache";
  import type {
    Album,
    AlbumDetail,
    OutputFormat,
    SongEntry,
    PlayerState,
    PlaybackContext,
    PlaybackQueueEntry,
    DownloadJobSnapshot,
    DownloadManagerSnapshot,
    DownloadTaskProgressEvent,
    CreateDownloadJobRequest,
    DownloadTaskSnapshot,
    AppPreferences,
    LocalInventorySnapshot,
    AppErrorEvent,
    LogLevel,
    LogFileKind,
    LogFileStatus,
    LogViewerRecord,
  } from "$lib/types";
  import { applyThemePalette, DEFAULT_THEME_PALETTE } from "$lib/theme";
  import { getDownloadBadgeLabel, shouldShowDownloadBadge } from "$lib/downloadBadge";
  import { motionStyles } from "$lib/actions/motionStyles";
  import { envStore } from "$lib/features/env/store.svelte";
  import { shellStore } from "$lib/features/shell/store.svelte";
  import { toast } from "svelte-sonner";
  import AlbumCard from "$lib/components/AlbumCard.svelte";
  import SongRow from "$lib/components/SongRow.svelte";
  import AudioPlayer from "$lib/components/AudioPlayer.svelte";
  import MotionSpinner from "$lib/components/MotionSpinner.svelte";
  import MotionPulseBlock from "$lib/components/MotionPulseBlock.svelte";
  import TopToolbar from "$lib/components/app/TopToolbar.svelte";
  import SettingsSheet from "$lib/components/app/SettingsSheet.svelte";
  import DownloadTasksSheet from "$lib/components/app/DownloadTasksSheet.svelte";
  import StatusToastHost from "$lib/components/app/StatusToastHost.svelte";
  import AlbumSidebar from "$lib/components/app/AlbumSidebar.svelte";
  import AlbumWorkspace from "$lib/components/app/AlbumWorkspace.svelte";
  import PlayerDock from "$lib/components/app/PlayerDock.svelte";

  // Minimum display time (ms) to prevent animation flash on fast loads
  const MIN_DISPLAY_MS = 260;
  const DETAIL_SKELETON_DELAY_MS = 140;
  const PANEL_DURATION = 0.18;
  const HERO_DURATION = 0.22;
  const HERO_DELAY = 0.03;
  const LIST_DURATION = 0.2;
  const LIST_DELAY = 0.07;
  const CONTENT_MASK_DURATION = 0.14;
  const OVERLAY_DURATION = 0.16;
  const SHEET_DURATION = 0.22;
  const PLAYER_DOCK_DURATION = 0.22;
  const DEFAULT_ALBUM_STAGE_ASPECT_RATIO = 16 / 9;
  const ALBUM_STAGE_BASE_VIEWPORT_RATIO = 1 / 3;
  const ALBUM_STAGE_COLLAPSE_SCROLL_RANGE = 260;
  const ALBUM_STAGE_SOLIDIFY_SCROLL_RANGE = 220;

  const delay = (ms: number) =>
    new Promise((resolve) => setTimeout(resolve, ms));

  type RepeatMode = "all" | "one";
  type SongDownloadState = "idle" | "creating" | "queued" | "running";

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

  let albums = $state<Album[]>([]);
  let selectedAlbum = $state<AlbumDetail | null>(null);
  let selectedAlbumCid = $state<string | null>(null);
  let outputDir = $state("");
  let format = $state<OutputFormat>("flac");
  let loadingAlbums = $state(false);
  let loadingDetail = $state(false);
  let errorMsg = $state("");

  // Audio player state (synced from Rust backend via Tauri events)
  let currentSong = $state<PlayerSong | null>(null);
  let isPlaying = $state(false);
  let isPaused = $state(false);
  let isLoading = $state(false);
  let hasPrevious = $state(false);
  let hasNext = $state(false);
  let progress = $state(0);
  let duration = $state(0);
  let shuffleEnabled = $state(false);
  let repeatMode = $state<RepeatMode>("all");
  let playbackEntries = $state<PlaybackQueueEntry[]>([]);
  let playbackOrder = $state<PlaybackQueueEntry[]>([]);
  let playbackIndex = $state(-1);
  let lyricsOpen = $state(false);
  let playlistOpen = $state(false);
  let lyricsLoading = $state(false);
  let lyricsError = $state("");
  let lyricsLines = $state<LyricLine[]>([]);
  let lyricsSongCid = $state<string | null>(null);
  let downloadingSongCid = $state<string | null>(null);
  let downloadingAlbumCid = $state<string | null>(null);
  let creatingSelectionKey = $state<string | null>(null);
  let selectedSongCids = $state<string[]>([]);
  let selectionModeEnabled = $state(false);
  // Download job system state
  let downloadManager = $state<DownloadManagerSnapshot | null>(null);
  let downloadPanelOpen = $state(false);
  // Track current download speed for active tasks
  let taskSpeedMap = $state<Map<string, number>>(new Map());

  // Computed: number of active/queued/running jobs
  let activeDownloadCount = $derived(
    downloadManager
      ? downloadManager.jobs.filter(
          (j) => j.status === "running" || j.status === "queued",
        ).length
      : 0,
  );
  let downloadJobs = $derived(downloadManager?.jobs ?? []);
  // Track which song is currently being loaded to prevent duplicate play calls
  let playingCid = $state<string | null>(null);
  let albumRequestSeq = $state(0);
  let themeRequestSeq = 0;
  let artworkRequestSeq = 0;
  let lyricRequestSeq = 0;
  let playbackEndRequestSeq = 0;
  let lastPlaybackSnapshot = {
    cid: null as string | null,
    active: false,
  };
  let prefersReducedMotion = $state(false);
  let showDetailSkeleton = $state(false);
  let contentEl = $state<HTMLElement | null>(null);
  let contentScrollbar = $state<OverlayScrollbarsComponentRef<"main"> | null>(
    null,
  );
  let albumStageEl = $state<HTMLElement | null>(null);
  let selectedAlbumArtworkUrl = $state<string | null>(null);
  let isMacOS = $state(false);
  let detailSkeletonTimer: ReturnType<typeof setTimeout> | null = null;
  let albumStageAspectRatio = $state(DEFAULT_ALBUM_STAGE_ASPECT_RATIO);
  let albumStageWidth = $state(0);
  let viewportHeight = $state(0);
  let albumStageCollapseOffset = $state(0);
  let albumStageScrollTop = $state(0);
  let albumStageMotionFrame = 0;
  let pendingAlbumStageCollapseOffset = 0;
  let pendingAlbumStageScrollTop = 0;

  const playerHasPrevious = $derived.by(() => playbackOrder.length > 1);
  const playerHasNext = $derived.by(() => playbackOrder.length > 1);

  const activeLyricIndex = $derived.by(() => {
    let activeIndex = -1;

    for (let index = 0; index < lyricsLines.length; index += 1) {
      const lineTime = lyricsLines[index]?.time;
      if (lineTime === null || lineTime === undefined) continue;
      if (progress + 0.08 >= lineTime) {
        activeIndex = index;
      } else {
        break;
      }
    }

    return activeIndex;
  });

  const selectedSongCount = $derived.by(() => selectedSongCids.length);
  const selectedSongsLabel = $derived.by(() => {
    if (selectedSongCount === 0) return "未选择歌曲";
    if (selectedSongCount === 1) return "已选择 1 首";
    return `已选择 ${selectedSongCount} 首`;
  });

  function setContentViewport(instance: OverlayScrollbars) {
    const viewport = instance.elements().viewport;
    if (contentEl !== viewport) {
      contentEl = viewport;
    }
  }

  const overlayScrollbarOptions = $derived.by(
    (): PartialOptions => ({
      scrollbars: {
        theme: "os-theme-app",
        autoHide: prefersReducedMotion ? "leave" : "move",
        autoHideDelay: prefersReducedMotion ? 160 : 720,
        autoHideSuspend: true,
        dragScroll: true,
        clickScroll: false,
      },
    }),
  );

  const contentScrollbarEvents = $derived.by(
    (): EventListeners => ({
      initialized(instance) {
        setContentViewport(instance);
        handleContentScroll();
      },
      updated(instance) {
        setContentViewport(instance);
      },
      destroyed() {
        contentEl = null;
      },
      scroll(instance) {
        setContentViewport(instance);
        handleContentScroll();
      },
    }),
  );

  function resetContentScroll() {
    resetAlbumStageMotion();
    contentEl?.scrollTo({
      top: 0,
      behavior: prefersReducedMotion ? "auto" : "smooth",
    });
  }

  interface ImageMeta {
    aspectRatio: number;
  }

  function getImageMeta(image: HTMLImageElement): ImageMeta | null {
    const width = image.naturalWidth || image.width;
    const height = image.naturalHeight || image.height;

    if (!width || !height) {
      return null;
    }

    return {
      aspectRatio: width / height,
    };
  }

  function preloadImage(
    src: string | null | undefined,
  ): Promise<ImageMeta | null> {
    if (!src) return Promise.resolve(null);

    return new Promise((resolve) => {
      const image = new Image();
      let settled = false;

      const finish = (meta: ImageMeta | null) => {
        if (settled) return;
        settled = true;
        resolve(meta);
      };

      image.decoding = "async";
      image.onload = () => finish(getImageMeta(image));
      image.onerror = () => finish(null);
      image.src = src;

      if (image.complete) {
        queueMicrotask(() => finish(getImageMeta(image)));
      }
    });
  }

  async function preloadAlbumArtwork(
    album: AlbumDetail,
  ): Promise<number | null> {
    const sourceUrl = album.coverDeUrl ?? album.coverUrl ?? null;
    if (!sourceUrl) return null;

    let resolvedUrl = sourceUrl;
    try {
      resolvedUrl = await getImageDataUrl(sourceUrl);
    } catch {
      resolvedUrl = sourceUrl;
    }

    const meta = await preloadImage(resolvedUrl);
    return meta?.aspectRatio ?? null;
  }

  function setAlbumStageAspectRatio(value: number | null | undefined) {
    if (value && Number.isFinite(value) && value > 0) {
      albumStageAspectRatio = value;
      return;
    }

    albumStageAspectRatio = DEFAULT_ALBUM_STAGE_ASPECT_RATIO;
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  function getSelectedAlbumCoverUrl(): string | null {
    return selectedAlbum?.coverUrl ?? selectedAlbum?.coverDeUrl ?? null;
  }

  function normalizePlayerSong(state: PlayerState): PlayerSong | null {
    if (!state.songCid) return null;

    return {
      cid: state.songCid,
      name: state.songName ?? "",
      artists: state.artists,
      coverUrl: state.coverUrl ?? null,
    };
  }

  function buildAlbumPlaybackEntries(
    album: AlbumDetail | null,
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
    currentCid: string | null,
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
    currentCid: string | null,
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

  function buildPlaybackContext(
    order: PlaybackQueueEntry[],
    currentIndex: number,
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

  function syncPlaybackQueueWithSong(song: PlayerSong | null) {
    if (!song) {
      playbackIndex = -1;
      return;
    }

    const currentOrderIndex = playbackOrder.findIndex(
      (entry) => entry.cid === song.cid,
    );
    if (currentOrderIndex >= 0) {
      playbackIndex = currentOrderIndex;
      return;
    }

    const currentSourceIndex = playbackEntries.findIndex(
      (entry) => entry.cid === song.cid,
    );
    if (currentSourceIndex >= 0) {
      applyPlaybackQueue(playbackEntries, song.cid);
      return;
    }

    applyPlaybackQueue([buildSinglePlaybackEntry(song)], song.cid);
  }

  function parseLyricText(source: string): LyricLine[] {
    const lines = source
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean);
    const parsed: LyricLine[] = [];

    for (const rawLine of lines) {
      const matches = [
        ...rawLine.matchAll(/\[(\d{1,2}):(\d{2})(?:\.(\d{1,3}))?\]/g),
      ];
      const text =
        rawLine.replace(/\[(\d{1,2}):(\d{2})(?:\.(\d{1,3}))?\]/g, "").trim() ||
        "♪";

      if (!matches.length) {
        parsed.push({
          id: `plain-${parsed.length}`,
          time: null,
          text,
        });
        continue;
      }

      for (const match of matches) {
        const minutes = Number(match[1] ?? 0);
        const seconds = Number(match[2] ?? 0);
        const fractionText = match[3] ?? "0";
        const fraction = Number(`0.${fractionText.padEnd(3, "0")}`);
        parsed.push({
          id: `${minutes}:${seconds}:${fractionText}:${parsed.length}`,
          time: minutes * 60 + seconds + fraction,
          text,
        });
      }
    }

    return parsed.sort((left, right) => {
      if (left.time === null && right.time === null) return 0;
      if (left.time === null) return 1;
      if (right.time === null) return -1;
      return left.time - right.time;
    });
  }

  async function loadLyrics(songCid: string) {
    const requestSeq = ++lyricRequestSeq;
    lyricsSongCid = songCid;
    lyricsLoading = true;
    lyricsError = "";
    lyricsLines = [];

    try {
      const lyricText = await getSongLyrics(songCid);
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

  async function playQueueEntry(
    entry: PlaybackQueueEntry,
    order = playbackOrder,
    index = order.findIndex((candidate) => candidate.cid === entry.cid),
    options: { forceRestart?: boolean } = {},
  ) {
    if (index < 0) return;

    playbackOrder = [...order];
    playbackIndex = index;

    if (!options.forceRestart) {
      if (currentSong?.cid === entry.cid && isPaused) {
        await handleResumePlayback();
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
      await playSong(
        entry.cid,
        entry.coverUrl ?? undefined,
        buildPlaybackContext(order, index),
      );
    } catch (error) {
      playingCid = null;
      console.error("[ERROR] Failed to play song from queue:", error);
    }
  }

  function resolveWrappedQueueIndex(step: 1 | -1): number {
    if (!playbackOrder.length) return -1;
    if (playbackIndex < 0) return step > 0 ? 0 : playbackOrder.length - 1;
    return (playbackIndex + step + playbackOrder.length) % playbackOrder.length;
  }

  function handleShuffleChange(next: boolean) {
    shuffleEnabled = next;
    if (!playbackEntries.length) return;

    const currentCid = currentSong?.cid ?? playbackEntries[0]?.cid ?? null;
    applyPlaybackQueue(playbackEntries, currentCid);
  }

  function handleRepeatModeChange(next: RepeatMode) {
    repeatMode = next;
  }

  function toggleLyricsPanel() {
    if (!currentSong) return;
    lyricsOpen = !lyricsOpen;
    if (lyricsOpen) {
      playlistOpen = false;
    }
  }

  function togglePlaylistPanel() {
    if (!currentSong) return;
    playlistOpen = !playlistOpen;
    if (playlistOpen) {
      lyricsOpen = false;
    }
  }

  function buildSelectionKey(songCids: string[]): string {
    return [...songCids].sort().join(",");
  }

  function isSongSelected(songCid: string): boolean {
    return selectedSongCids.includes(songCid);
  }

  function toggleSongSelection(songCid: string) {
    if (selectedSongCids.includes(songCid)) {
      selectedSongCids = selectedSongCids.filter((cid) => cid !== songCid);
      return;
    }

    selectedSongCids = [...selectedSongCids, songCid];
  }

  function clearSongSelection() {
    selectedSongCids = [];
  }

  function selectAllSongs() {
    if (!selectedAlbum) return;
    selectedSongCids = selectedAlbum.songs.map((s: SongEntry) => s.cid);
  }

  function deselectAllSongs() {
    selectedSongCids = [];
  }

  function invertSongSelection() {
    if (!selectedAlbum) return;
    const allCids = new Set(
      selectedAlbum.songs.map((s: SongEntry) => s.cid),
    );
    const currentSelected = new Set(selectedSongCids);
    selectedSongCids = [...allCids].filter(
      (cid) => !currentSelected.has(cid),
    );
  }

  function toggleSelectionMode() {
    selectionModeEnabled = !selectionModeEnabled;
    if (!selectionModeEnabled) {
      clearSongSelection();
    }
  }

  function hasCurrentDownloadOptions(job: DownloadJobSnapshot): boolean {
    return (
      job.options.format === format &&
      job.options.downloadLyrics === downloadLyrics
    );
  }

  function findSelectionDownloadJob(
    songCids: string[],
  ): DownloadJobSnapshot | null {
    if (!downloadManager || songCids.length === 0) return null;

    const targetKey = buildSelectionKey(songCids);
    return (
      downloadManager.jobs.find((job) => {
        if (job.kind !== "selection") return false;
        if (job.status !== "queued" && job.status !== "running") return false;
        if (!hasCurrentDownloadOptions(job)) return false;
        return (
          buildSelectionKey(job.tasks.map((task) => task.songCid)) === targetKey
        );
      }) ?? null
    );
  }

  function getCurrentSelectionKey(): string | null {
    return selectedSongCids.length > 0
      ? buildSelectionKey(selectedSongCids)
      : null;
  }

  function isCurrentSelectionCreating(): boolean {
    const selectionKey = getCurrentSelectionKey();
    return selectionKey !== null && creatingSelectionKey === selectionKey;
  }

  function getCurrentSelectionJob(): DownloadJobSnapshot | null {
    return findSelectionDownloadJob(selectedSongCids);
  }

  function isSelectionDownloadActionDisabled(): boolean {
    return (
      selectedSongCount === 0 ||
      isCurrentSelectionCreating() ||
      !!getCurrentSelectionJob()
    );
  }

  function findAlbumDownloadJob(albumCid: string): DownloadJobSnapshot | null {
    if (!downloadManager) return null;

    return (
      downloadManager.jobs.find((job) => {
        if (job.kind !== "album") return false;
        if (job.status !== "queued" && job.status !== "running") return false;
        if (!hasCurrentDownloadOptions(job)) return false;
        return job.tasks.some((task) => task.albumCid === albumCid);
      }) ?? null
    );
  }

  function findSongDownloadTask(songCid: string): DownloadTaskSnapshot | null {
    if (!downloadManager) return null;

    for (const job of downloadManager.jobs) {
      if (job.status !== "queued" && job.status !== "running") continue;
      const task = job.tasks.find((candidate) => candidate.songCid === songCid);
      if (task) return task;
    }

    return null;
  }

  function isSongDownloadInteractionBlocked(songCid: string): boolean {
    return downloadingSongCid !== null && downloadingSongCid !== songCid;
  }

  function getSongDownloadState(songCid: string): SongDownloadState {
    if (downloadingSongCid === songCid) {
      return "creating";
    }

    const task = findSongDownloadTask(songCid);
    if (!task) {
      return "idle";
    }

    if (task.status === "queued") {
      return "queued";
    }

    if (
      task.status === "preparing" ||
      task.status === "downloading" ||
      task.status === "writing"
    ) {
      return "running";
    }

    return "idle";
  }

  function getSongDownloadJob(songCid: string): DownloadJobSnapshot | null {
    if (!downloadManager) return null;

    return (
      downloadManager.jobs.find(
        (job) =>
          (job.status === "queued" || job.status === "running") &&
          hasCurrentDownloadOptions(job) &&
          job.tasks.some((task) => task.songCid === songCid),
      ) ?? null
    );
  }

  async function performSongDownload(songCid: string) {
    const existingJob = getSongDownloadJob(songCid);
    if (existingJob) {
      downloadPanelOpen = true;
      return existingJob.id;
    }

    if (downloadingSongCid) return null;

    downloadingSongCid = songCid;
    try {
      const request: CreateDownloadJobRequest = {
        kind: "song",
        songCids: [songCid],
        albumCid: null,
        options: {
          outputDir: "",
          format,
          downloadLyrics,
        },
      };
      const job = await createDownloadJob(request);
      downloadPanelOpen = true;
      return job.id;
    } finally {
      if (downloadingSongCid === songCid) {
        downloadingSongCid = null;
      }
    }
  }

  async function handleCurrentSongDownload() {
    if (!currentSong) return;
    try {
      const existingJob = getSongDownloadJob(currentSong.cid);
      await performSongDownload(currentSong.cid);
      if (existingJob) {
        notifyInfo("这首歌的下载任务已在队列中或正在执行。");
      }
    } catch (error) {
      console.error("[ERROR] Failed to download current song:", error);
      notifyError(
        `下载失败：${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async function performAlbumDownload(album: AlbumDetail) {
    const existingJob = findAlbumDownloadJob(album.cid);
    if (existingJob) {
      downloadPanelOpen = true;
      return existingJob.id;
    }

    if (downloadingAlbumCid === album.cid) {
      return null;
    }

    downloadingAlbumCid = album.cid;
    try {
      const request: CreateDownloadJobRequest = {
        kind: "album",
        songCids: [],
        albumCid: album.cid,
        options: {
          outputDir: "",
          format,
          downloadLyrics,
        },
      };
      const job = await createDownloadJob(request);
      downloadPanelOpen = true;
      return job.id;
    } finally {
      if (downloadingAlbumCid === album.cid) {
        downloadingAlbumCid = null;
      }
    }
  }

  async function performSelectionDownload(songCids: string[]) {
    if (songCids.length === 0) return null;

    const existingJob = findSelectionDownloadJob(songCids);
    if (existingJob) {
      downloadPanelOpen = true;
      return existingJob.id;
    }

    const selectionKey = buildSelectionKey(songCids);
    if (creatingSelectionKey === selectionKey) {
      return null;
    }

    creatingSelectionKey = selectionKey;
    try {
      const request: CreateDownloadJobRequest = {
        kind: "selection",
        songCids,
        albumCid: null,
        options: {
          outputDir: "",
          format,
          downloadLyrics,
        },
      };
      const job = await createDownloadJob(request);
      downloadPanelOpen = true;
      clearSongSelection();
      selectionModeEnabled = false;
      return job.id;
    } finally {
      if (creatingSelectionKey === selectionKey) {
        creatingSelectionKey = null;
      }
    }
  }

  async function handleAlbumDownload() {
    if (!selectedAlbum) return;

    try {
      const existingJob = findAlbumDownloadJob(selectedAlbum.cid);
      await performAlbumDownload(selectedAlbum);
      if (existingJob) {
        notifyInfo("这张专辑的下载任务已在队列中或正在执行。");
      }
    } catch (error) {
      console.error("[ERROR] Failed to download album:", error);
      notifyError(
        `整专下载失败：${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async function handleSelectionDownload() {
    if (!selectedAlbum || selectedSongCids.length === 0) return;

    try {
      const existingJob = findSelectionDownloadJob(selectedSongCids);
      await performSelectionDownload(selectedSongCids);
      if (existingJob) {
        notifyInfo("这组歌曲的下载任务已在队列中或正在执行。");
      }
    } catch (error) {
      console.error("[ERROR] Failed to create selection download job:", error);
      notifyError(
        `批量下载失败：${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async function handlePlaybackEnded(songCid: string) {
    const requestSeq = ++playbackEndRequestSeq;
    const currentIndex = playbackOrder.findIndex(
      (entry) => entry.cid === songCid,
    );
    if (currentIndex < 0) return;

    if (repeatMode === "one") {
      await playQueueEntry(
        playbackOrder[currentIndex],
        playbackOrder,
        currentIndex,
        { forceRestart: true },
      );
      return;
    }

    const nextIndex = (currentIndex + 1) % playbackOrder.length;
    if (requestSeq !== playbackEndRequestSeq) return;
    await playQueueEntry(playbackOrder[nextIndex], playbackOrder, nextIndex, {
      forceRestart: true,
    });
  }

  function flushAlbumStageMotion() {
    albumStageMotionFrame = 0;

    if (albumStageCollapseOffset !== pendingAlbumStageCollapseOffset) {
      albumStageCollapseOffset = pendingAlbumStageCollapseOffset;
    }

    if (albumStageScrollTop !== pendingAlbumStageScrollTop) {
      albumStageScrollTop = pendingAlbumStageScrollTop;
    }
  }

  function scheduleAlbumStageMotion(
    next: {
      collapseOffset?: number;
      scrollTop?: number;
    },
    immediate = false,
  ) {
    pendingAlbumStageCollapseOffset =
      next.collapseOffset ?? pendingAlbumStageCollapseOffset;
    pendingAlbumStageScrollTop = next.scrollTop ?? pendingAlbumStageScrollTop;

    if (immediate || prefersReducedMotion || typeof window === "undefined") {
      if (albumStageMotionFrame) {
        cancelAnimationFrame(albumStageMotionFrame);
        albumStageMotionFrame = 0;
      }
      flushAlbumStageMotion();
      return;
    }

    if (albumStageMotionFrame) {
      return;
    }

    albumStageMotionFrame = requestAnimationFrame(() => {
      flushAlbumStageMotion();
    });
  }

  function resetAlbumStageMotion() {
    if (albumStageMotionFrame) {
      cancelAnimationFrame(albumStageMotionFrame);
      albumStageMotionFrame = 0;
    }

    pendingAlbumStageCollapseOffset = 0;
    pendingAlbumStageScrollTop = 0;
    albumStageCollapseOffset = 0;
    albumStageScrollTop = 0;
  }

  function syncAlbumStageWidth() {
    albumStageWidth = albumStageEl?.clientWidth ?? 0;
  }

  function syncViewportHeight() {
    viewportHeight = window.innerHeight || 0;
  }

  const albumStageFullHeight = $derived.by(() => {
    if (!albumStageWidth || !albumStageAspectRatio) {
      return 0;
    }

    return albumStageWidth / albumStageAspectRatio;
  });

  const albumStageBaseHeight = $derived.by(() => {
    if (!albumStageWidth) {
      return 0;
    }

    return Math.min(
      albumStageFullHeight,
      viewportHeight * ALBUM_STAGE_BASE_VIEWPORT_RATIO,
    );
  });

  const albumStageCollapseProgress = $derived.by(() =>
    clamp(albumStageCollapseOffset / ALBUM_STAGE_COLLAPSE_SCROLL_RANGE, 0, 1),
  );

  const albumStageRevealProgress = $derived.by(
    () => 1 - albumStageCollapseProgress,
  );

  const albumStageSolidifyProgress = $derived.by(() =>
    Math.max(
      albumStageCollapseProgress,
      clamp(albumStageScrollTop / ALBUM_STAGE_SOLIDIFY_SCROLL_RANGE, 0, 1),
    ),
  );

  const albumStageHeight = $derived.by(() => {
    if (!albumStageBaseHeight) {
      return 0;
    }

    return (
      albumStageBaseHeight +
      (albumStageFullHeight - albumStageBaseHeight) * albumStageRevealProgress
    );
  });

  const albumStageStyle = $derived.by(
    () => `--album-stage-aspect-ratio: ${albumStageAspectRatio}`,
  );

  type MotionTarget = Record<string, string | number>;

  function motionTransition(duration: number, delay = 0): any {
    const transition: any = {
      duration: prefersReducedMotion ? 0 : duration,
      delay: prefersReducedMotion ? 0 : delay,
      ease: "easeOut" as const,
    };

    return transition;
  }

  function fadeEnter(opacity = 0): MotionTarget {
    return prefersReducedMotion ? { opacity: 1 } : { opacity };
  }

  function fadeExit(opacity = 0): MotionTarget {
    return { opacity };
  }

  function axisEnter(axis: "x" | "y", offset: number): MotionTarget {
    return prefersReducedMotion
      ? { opacity: 1 }
      : { opacity: 0, [axis]: offset };
  }

  function axisAnimate(axis: "x" | "y"): MotionTarget {
    return { opacity: 1, [axis]: 0 };
  }

  function axisExit(axis: "x" | "y", offset: number): MotionTarget {
    return prefersReducedMotion
      ? { opacity: 0 }
      : { opacity: 0, [axis]: offset };
  }

  const interactiveTransition = $derived.by(
    () =>
      ({
        duration: prefersReducedMotion ? 0 : 0.16,
        ease: "easeOut",
      }) as const,
  );

  const albumStageMotionHeight = $derived.by(() =>
    albumStageHeight > 0
      ? albumStageHeight
      : Math.max(albumStageBaseHeight || 0, 280),
  );

  const albumStageMediaHeight = $derived.by(
    () => `${albumStageMotionHeight}px`,
  );
  const albumStageScrimOpacity = $derived.by(() =>
    Math.max(0.58, 1 - albumStageSolidifyProgress * 0.34),
  );
  const albumStageImageOpacity = $derived.by(
    () => 1 - albumStageSolidifyProgress * 0.54,
  );
  const albumStageImageTransform = $derived.by(() =>
    prefersReducedMotion
      ? "translateZ(0) scale(1)"
      : `translateZ(0) scale(${1 + albumStageRevealProgress * 0.006 + albumStageSolidifyProgress * 0.012})`,
  );
  const albumStageSolidifyOpacity = $derived.by(
    () => albumStageSolidifyProgress,
  );

  function modeButtonAnimate(active: boolean): MotionTarget {
    return active
      ? {
          backgroundColor: "var(--accent)",
          color: "#ffffff",
          boxShadow: "0 10px 22px rgba(var(--accent-rgb), 0.22)",
        }
      : {
          backgroundColor: "rgba(15, 23, 42, 0)",
          color: "rgba(31, 41, 55, 0.72)",
          boxShadow: "0 0 0 rgba(var(--accent-rgb), 0)",
        };
  }

  function modeButtonHover(active: boolean): MotionTarget | undefined {
    if (active) {
      return prefersReducedMotion ? undefined : { y: -1 };
    }

    return {
      backgroundColor: "rgba(15, 23, 42, 0.06)",
      color: "var(--text-primary)",
      ...(prefersReducedMotion ? {} : { y: -1 }),
    };
  }

  function toolbarButtonAnimate(
    active = false,
    accented = false,
    disabled = false,
  ): MotionTarget {
    return {
      opacity: disabled ? 0.42 : 1,
      backgroundColor: active
        ? "var(--accent-light)"
        : "rgba(255, 255, 255, 0.78)",
      color: active || accented ? "var(--accent)" : "var(--text-primary)",
      boxShadow: "inset 0 1px 0 rgba(255, 255, 255, 0.94)",
    };
  }

  function toolbarButtonHover(disabled = false): MotionTarget | undefined {
    if (disabled) {
      return undefined;
    }

    return {
      backgroundColor: "rgba(var(--accent-rgb), 0.1)",
      boxShadow: "0 10px 22px rgba(var(--accent-rgb), 0.14)",
      ...(prefersReducedMotion ? {} : { y: -1 }),
    };
  }

  function appButtonAnimate(primary = false, disabled = false): MotionTarget {
    return primary
      ? {
          backgroundColor: disabled ? "var(--bg-tertiary)" : "var(--accent)",
          color: disabled ? "var(--text-tertiary)" : "#ffffff",
          boxShadow: disabled
            ? "0 0 0 rgba(var(--accent-rgb), 0)"
            : "0 10px 24px rgba(var(--accent-rgb), 0.16)",
          opacity: disabled ? 0.72 : 1,
        }
      : {
          backgroundColor: "var(--bg-tertiary)",
          color: "var(--text-primary)",
          boxShadow: "0 0 0 rgba(var(--accent-rgb), 0)",
          opacity: disabled ? 0.42 : 1,
        };
  }

  function appButtonHover(
    primary = false,
    disabled = false,
  ): MotionTarget | undefined {
    if (disabled) return undefined;

    return primary
      ? {
          backgroundColor: "var(--accent-hover)",
          boxShadow: "0 10px 24px rgba(var(--accent-rgb), 0.2)",
          ...(prefersReducedMotion ? {} : { y: -1 }),
        }
      : {
          backgroundColor: "var(--hover-bg-elevated)",
          boxShadow: "0 8px 20px rgba(15, 23, 42, 0.08)",
          ...(prefersReducedMotion ? {} : { y: -1 }),
        };
  }

  function settingsCloseAnimate(): MotionTarget {
    return {
      backgroundColor: "var(--bg-tertiary)",
      color: "var(--text-secondary)",
    };
  }

  function settingsCloseHover(): MotionTarget {
    return {
      backgroundColor: "var(--hover-bg-elevated)",
      color: "var(--text-primary)",
      ...(prefersReducedMotion ? {} : { y: -1 }),
    };
  }

  function fieldAnimate(): MotionTarget {
    return {
      backgroundColor: "var(--bg-tertiary)",
      borderColor: "var(--border)",
      color: "var(--text-primary)",
      boxShadow: "0 0 0 0 rgba(var(--accent-rgb), 0)",
    };
  }

  function fieldHover(): MotionTarget {
    return {
      borderColor: "var(--text-tertiary)",
    };
  }

  function fieldFocus(): MotionTarget {
    return {
      borderColor: "var(--accent)",
      backgroundColor: "var(--accent-light)",
      boxShadow: "0 0 0 3px rgba(var(--accent-rgb), 0.12)",
    };
  }

  function fieldMotion(hovered: boolean, focused: boolean): MotionTarget {
    if (focused) return fieldFocus();
    if (hovered) return fieldHover();
    return fieldAnimate();
  }

  function armDetailSkeleton() {
    if (detailSkeletonTimer) {
      clearTimeout(detailSkeletonTimer);
    }

    showDetailSkeleton = false;
    detailSkeletonTimer = setTimeout(() => {
      if (loadingDetail) {
        showDetailSkeleton = true;
      }
    }, DETAIL_SKELETON_DELAY_MS);
  }

  function clearDetailSkeleton() {
    if (detailSkeletonTimer) {
      clearTimeout(detailSkeletonTimer);
      detailSkeletonTimer = null;
    }
    showDetailSkeleton = false;
  }

  $effect(() => {
    const paletteRequestSeq = ++themeRequestSeq;
    const artworkUrl =
      selectedAlbum?.coverUrl ?? selectedAlbum?.coverDeUrl ?? null;

    if (!artworkUrl) {
      applyThemePalette(DEFAULT_THEME_PALETTE);
      return;
    }

    void (async () => {
      try {
        const palette = await extractImageTheme(artworkUrl);
        if (paletteRequestSeq !== themeRequestSeq) return;
        applyThemePalette(palette);
      } catch (e) {
        if (paletteRequestSeq !== themeRequestSeq) return;
        applyThemePalette(DEFAULT_THEME_PALETTE);
        console.error("[ERROR] Failed to extract album theme:", e);
      }
    })();
  });

  $effect(() => {
    const sourceUrl = selectedAlbum?.coverDeUrl ?? selectedAlbum?.coverUrl ?? null;
    const requestSeq = ++artworkRequestSeq;

    if (!sourceUrl) {
      selectedAlbumArtworkUrl = null;
      return;
    }

    void (async () => {
      try {
        const dataUrl = await getImageDataUrl(sourceUrl);
        if (requestSeq !== artworkRequestSeq) return;
        selectedAlbumArtworkUrl = dataUrl;
      } catch (error) {
        if (requestSeq !== artworkRequestSeq) return;
        selectedAlbumArtworkUrl = null;
        console.warn("[WARN] Failed to resolve album artwork:", error);
      }
    })();
  });

  // Auto-save preferences via unified preferences API (after initialization)
  $effect(() => {
    const _fmt = format;
    if (!prefsReady) return;
    void savePreferences();
  });

  $effect(() => {
    const _lyrics = downloadLyrics;
    if (!prefsReady) return;
    void savePreferences();
  });

  $effect(() => {
    const _notif = notifyOnDownloadComplete;
    if (!prefsReady) return;
    void savePreferences();
  });

  $effect(() => {
    const _playback = notifyOnPlaybackChange;
    if (!prefsReady) return;
    void savePreferences();
  });

  $effect(() => {
    const _logLevel = logLevel;
    if (!prefsReady) return;
    void savePreferences();
  });

  $effect(() => {
    if (!albumStageEl) return;

    syncAlbumStageWidth();

    if (typeof ResizeObserver === "undefined") return;

    const observer = new ResizeObserver(() => {
      syncAlbumStageWidth();
    });

    observer.observe(albumStageEl);

    return () => observer.disconnect();
  });

  $effect(() => {
    if (!settingsOpen) return;
    void refreshLogs(logFileKind);
  });

  onMount(() => {
    envStore.init();
    shellStore.init();

    isMacOS =
      /Mac|iPhone|iPad|iPod/.test(navigator.platform) ||
      navigator.userAgent.includes("Mac");

    let unlistenState: (() => void) | null = null;
    let unlistenProgress: (() => void) | null = null;
    let unlistenDownloadManager: (() => void) | null = null;
    let unlistenDownloadJob: (() => void) | null = null;
    let unlistenDownloadProgress: (() => void) | null = null;
    let unlistenLocalInventory: (() => void) | null = null;
    let unlistenAppError: (() => void) | null = null;
    const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");

    function updateReducedMotionPreference() {
      prefersReducedMotion = mediaQuery.matches;
    }

    function handleWindowResize() {
      syncViewportHeight();
      syncAlbumStageWidth();
    }

    async function initialize() {
      loadingAlbums = true;

      // Load unified preferences from backend (non-blocking on failure)
      try {
        await warmCacheManager();
      } catch {
        // Keep startup usable if IndexedDB warm start is unavailable.
      }

      try {
        const prefs = await getPreferences();
        outputDir = prefs.outputDir || outputDir;
        format = prefs.outputFormat || format;
        downloadLyrics = prefs.downloadLyrics;
        notifyOnDownloadComplete = prefs.notifyOnDownloadComplete;
        notifyOnPlaybackChange = prefs.notifyOnPlaybackChange;
        logLevel = prefs.logLevel;
        prefsReady = true;
      } catch {
        prefsReady = true;
      }

      try {
        localInventory = await getLocalInventorySnapshot();
      } catch {
        localInventory = null;
      }

      try {
        // Load albums and get default output dir in parallel.
        // outputDir from preferences takes precedence; getDefaultOutputDir() is only
        // a fallback when preferences has no saved output dir.
        const [albumsData, defaultDir] = await Promise.all([
          getAlbums(),
          outputDir ? Promise.resolve("") : getDefaultOutputDir(),
        ]);
        if (defaultDir && !outputDir) {
          outputDir = defaultDir;
        }
        albums = albumsData;
        // Auto-select the first album on startup
        if (albums.length > 0) {
          await handleSelectAlbum(albums[0]);
        }
      } catch (e) {
        errorMsg = e instanceof Error ? e.message : String(e);
        console.error("[ERROR] Failed to load albums:", e);
      } finally {
        loadingAlbums = false;
      }

      unlistenState = await listen<PlayerState>(
        "player-state-changed",
        (event) => {
          syncPlayerState(event.payload);
        },
      );

      unlistenProgress = await listen<PlayerState>(
        "player-progress",
        (event) => {
          const state = event.payload;
          progress = state.progress;
          isPlaying = state.isPlaying;
          isPaused = state.isPaused;
          duration = state.duration;
        },
      );

      // Subscribe to download events
      unlistenDownloadManager = await listen<DownloadManagerSnapshot>(
        "download-manager-state-changed",
        (event) => {
          downloadManager = event.payload;
        },
      );

      unlistenDownloadJob = await listen<DownloadJobSnapshot>(
        "download-job-updated",
        (event) => {
          const job = event.payload;
          if (!downloadManager) return;
          const jobs = downloadManager.jobs.map((j) =>
            j.id === job.id ? job : j,
          );
          downloadManager = { ...downloadManager, jobs };
        },
      );

      unlistenDownloadProgress = await listen<DownloadTaskProgressEvent>(
        "download-task-progress",
        (event) => {
          // Progress events update individual tasks in the manager state.
          // The job-updated event carries the full snapshot, so we just
          // need to update the task's bytes fields in-place.
          if (!downloadManager) return;
          const progress = event.payload;

          // Update speed map (reassign to trigger Svelte 5 reactivity)
          taskSpeedMap = new Map(taskSpeedMap).set(progress.taskId, progress.speedBytesPerSec);

          const jobIdx = downloadManager.jobs.findIndex(
            (j) => j.id === progress.jobId,
          );
          if (jobIdx < 0) return;
          const job = downloadManager.jobs[jobIdx];
          const taskIdx = job.tasks.findIndex((t) => t.id === progress.taskId);
          if (taskIdx < 0) return;
          const updatedTasks = [...job.tasks];
          updatedTasks[taskIdx] = { ...updatedTasks[taskIdx], ...progress };
          const updatedJob = { ...job, tasks: updatedTasks };
          const updatedJobs = [...downloadManager.jobs];
          updatedJobs[jobIdx] = updatedJob;
          downloadManager = { ...downloadManager, jobs: updatedJobs };
        },
      );

      unlistenAppError = await listen<AppErrorEvent>(
        "app-error-recorded",
        (event) => {
          handleAppErrorEvent(event.payload);
        },
      );

      unlistenLocalInventory = await listen<LocalInventorySnapshot>(
        "local-inventory-state-changed",
        async (event) => {
          const previousVersion = localInventory?.inventoryVersion ?? null;
          const previousStatus = localInventory?.status ?? null;
          localInventory = event.payload;
          const inventoryVersionChanged =
            previousVersion !== event.payload.inventoryVersion;
          const scanJustCompleted =
            event.payload.status === "completed" && previousStatus !== "completed";

          if (inventoryVersionChanged) {
            await invalidateInventoryCaches(previousVersion);
          }

          if (scanJustCompleted) {
            try {
              await refreshAlbumsList();
            } catch {
              // Keep current album list if refresh fails.
            }

            const currentSelectedAlbumCid = selectedAlbumCid;
            if (!currentSelectedAlbumCid) {
              return;
            }

            try {
              const detail = await getAlbumDetail(
                currentSelectedAlbumCid,
                event.payload.inventoryVersion,
              );
              if (selectedAlbumCid !== currentSelectedAlbumCid) {
                return;
              }
              selectedAlbum = detail;
            } catch {
              // Keep current UI state if refresh fails.
            }
          }
        },
      );

      // Initialize download manager state
      try {
        downloadManager = await listDownloadJobs();
      } catch {
        // Download manager not available
      }

      try {
        await refreshLogs("session");
      } catch {
        // Keep settings usable if logs are unavailable.
      }

      try {
        syncPlayerState(await getPlayerState());
      } catch {
        // Player not playing on startup
      }
    }

    syncViewportHeight();
    updateReducedMotionPreference();
    mediaQuery.addEventListener("change", updateReducedMotionPreference);
    window.addEventListener("resize", handleWindowResize, { passive: true });
    void initialize();

    return () => {
      shellStore.dispose();
      envStore.dispose();
      clearDetailSkeleton();
      if (albumStageMotionFrame) {
        cancelAnimationFrame(albumStageMotionFrame);
      }
      unlistenState?.();
      unlistenProgress?.();
      unlistenDownloadManager?.();
      unlistenDownloadJob?.();
      unlistenDownloadProgress?.();
      unlistenLocalInventory?.();
      unlistenAppError?.();
      mediaQuery.removeEventListener("change", updateReducedMotionPreference);
      window.removeEventListener("resize", handleWindowResize);
    };
  });

  $effect(() => {
    const songCid = currentSong?.cid ?? null;

    if (!songCid) {
      lyricRequestSeq += 1;
      lyricsSongCid = null;
      lyricsLines = [];
      lyricsError = "";
      lyricsLoading = false;
      lyricsOpen = false;
      playlistOpen = false;
      lastPlaybackSnapshot = { cid: null, active: false };
      return;
    }

    if (songCid === lyricsSongCid) {
      return;
    }

    void loadLyrics(songCid);
  });

  $effect(() => {
    const songCid = currentSong?.cid ?? null;
    const playbackActive = isPlaying || isPaused || isLoading;
    const hasReachedEnd =
      !!songCid && duration > 0 && progress >= Math.max(0, duration - 0.35);
    const shouldAutoAdvance =
      !!songCid &&
      songCid === lastPlaybackSnapshot.cid &&
      lastPlaybackSnapshot.active &&
      !playbackActive &&
      hasReachedEnd;

    lastPlaybackSnapshot = {
      cid: songCid,
      active: playbackActive,
    };

    if (shouldAutoAdvance) {
      void handlePlaybackEnded(songCid);
    }
  });

  async function handleSelectAlbum(album: Album) {
    if (album.cid === selectedAlbumCid && !loadingDetail) {
      return;
    }

    clearSongSelection();
    selectionModeEnabled = false;

    const requestSeq = ++albumRequestSeq;
    selectedAlbumCid = album.cid;
    loadingDetail = true;
    if (!selectedAlbum) {
      armDetailSkeleton();
    } else {
      clearDetailSkeleton();
    }

    const startTime = Date.now();
    try {
      const detail = await getAlbumDetail(
        album.cid,
        localInventory?.inventoryVersion ?? null,
      );
      if (requestSeq !== albumRequestSeq) return;
      const artworkAspectRatio = await preloadAlbumArtwork(detail);
      if (requestSeq !== albumRequestSeq) return;
      selectedAlbum = detail;
      setAlbumStageAspectRatio(artworkAspectRatio);
      errorMsg = "";
      await tick();
      resetContentScroll();
    } catch (e) {
      if (requestSeq !== albumRequestSeq) return;
      errorMsg = e instanceof Error ? e.message : String(e);
      console.error("[ERROR] Failed to load album detail:", e);
    } finally {
      if (requestSeq !== albumRequestSeq) return;
      // Ensure minimum display time so animations aren't interrupted
      const elapsed = Date.now() - startTime;
      if (elapsed < MIN_DISPLAY_MS) {
        await delay(MIN_DISPLAY_MS - elapsed);
      }
      if (requestSeq !== albumRequestSeq) return;
      clearDetailSkeleton();
      loadingDetail = false;
    }
  }

  function handleContentScroll() {
    if (loadingDetail) {
      scheduleAlbumStageMotion({ scrollTop: 0 }, true);
      return;
    }

    const nextScrollTop = Math.max(0, contentEl?.scrollTop ?? 0);
    const nextCollapseOffset =
      nextScrollTop > 0 &&
      pendingAlbumStageCollapseOffset < ALBUM_STAGE_COLLAPSE_SCROLL_RANGE
        ? ALBUM_STAGE_COLLAPSE_SCROLL_RANGE
        : undefined;

    scheduleAlbumStageMotion({
      scrollTop: nextScrollTop,
      collapseOffset: nextCollapseOffset,
    });
  }

  function handleContentWheel(event: WheelEvent) {
    if (loadingDetail || !contentEl) {
      return;
    }

    const atTop = (contentEl.scrollTop ?? 0) <= 0.5;

    if (!atTop) {
      return;
    }

    if (
      event.deltaY > 0 &&
      pendingAlbumStageCollapseOffset < ALBUM_STAGE_COLLAPSE_SCROLL_RANGE
    ) {
      event.preventDefault();
      scheduleAlbumStageMotion({
        collapseOffset: clamp(
          pendingAlbumStageCollapseOffset + event.deltaY,
          0,
          ALBUM_STAGE_COLLAPSE_SCROLL_RANGE,
        ),
        scrollTop: 0,
      });
      return;
    }

    if (event.deltaY < 0 && pendingAlbumStageCollapseOffset > 0) {
      event.preventDefault();
      scheduleAlbumStageMotion({
        collapseOffset: clamp(
          pendingAlbumStageCollapseOffset + event.deltaY,
          0,
          ALBUM_STAGE_COLLAPSE_SCROLL_RANGE,
        ),
        scrollTop: 0,
      });
    }
  }

  let settingsOpen = $state(false);
  let isClearingAudioCache = $state(false);
  let downloadLyrics = $state(true);
  let notifyOnDownloadComplete = $state(true);
  let notifyOnPlaybackChange = $state(true);
  let logLevel = $state<LogLevel>("error");
  let logFileKind = $state<LogFileKind>("session");
  let logRecords = $state<LogViewerRecord[]>([]);
  let logFileStatus = $state<LogFileStatus | null>(null);
  let logViewerLoading = $state(false);
  let logViewerError = $state("");
  let isSendingTestNotification = $state(false);
  let isFormatHovered = $state(false);
  let isFormatFocused = $state(false);
  let isOutputDirHovered = $state(false);
  let isOutputDirFocused = $state(false);
  let prefsReady = $state(false);
  let localInventory = $state<LocalInventorySnapshot | null>(null);

  async function refreshLogs(kind = logFileKind) {
    logViewerLoading = true;
    logViewerError = "";
    try {
      const [page, status] = await Promise.all([
        listLogRecords({ kind, limit: 100 }),
        getLogFileStatus(),
      ]);
      logRecords = page.records;
      logFileStatus = status;
      logFileKind = kind;
    } catch (error) {
      logViewerError = error instanceof Error ? error.message : String(error);
    } finally {
      logViewerLoading = false;
    }
  }

  function handleAppErrorEvent(event: AppErrorEvent) {
    notifyError(event.message);
    if (settingsOpen) {
      void refreshLogs(logFileKind);
    }
  }

  async function invalidateInventoryCaches(
    inventoryVersion: string | null | undefined,
  ) {
    await invalidateByTag(createInventoryCacheTag(inventoryVersion));
  }

  async function refreshAlbumsList() {
    albums = await getAlbums();
  }

  async function handleSelectDirectory() {
    const dir = await selectDirectory(outputDir);
    if (dir) {
      outputDir = dir;
      void savePreferences();
    }
  }

  async function handleClearAudioCache() {
    if (isClearingAudioCache) return;
    isClearingAudioCache = true;
    try {
      const removed = await clearAudioCache();
      notifyInfo(
        removed > 0
          ? `已清除 ${removed} 个音频缓存文件`
          : "当前没有可清除的音频缓存",
      );
    } catch (e) {
      console.error("[ERROR] Failed to clear audio cache:", e);
      notifyError(`清除音频缓存失败：${e instanceof Error ? e.message : String(e)}`);
    } finally {
      isClearingAudioCache = false;
    }
  }

  async function handleSendTestNotification() {
    if (isSendingTestNotification) return;
    isSendingTestNotification = true;
    try {
      await sendTestNotification();
      notifyInfo("测试通知已请求发送，请观察系统通知中心或终端日志。");
    } catch (e) {
      console.error("[ERROR] Failed to send test notification:", e);
      notifyError(`发送测试通知失败：${e instanceof Error ? e.message : String(e)}`);
    } finally {
      isSendingTestNotification = false;
    }
  }

  async function savePreferences() {
    const prefs: AppPreferences = {
      outputFormat: format,
      outputDir,
      downloadLyrics,
      notifyOnDownloadComplete,
      notifyOnPlaybackChange,
      logLevel,
    };
    try {
      const updated = await setPreferences(prefs);
      // Sync from returned values (backend may have normalized them)
      format = updated.outputFormat;
      outputDir = updated.outputDir;
      downloadLyrics = updated.downloadLyrics;
      notifyOnDownloadComplete = updated.notifyOnDownloadComplete;
      notifyOnPlaybackChange = updated.notifyOnPlaybackChange;
      logLevel = updated.logLevel;
    } catch (e) {
      console.error("[ERROR] Failed to save preferences:", e);
    }
  }

  function notifyInfo(message: string) {
    toast(message);
  }

  function notifyError(message: string) {
    toast.error(message);
  }

  // Download job helper functions
  function getActiveDownloadJob(): DownloadJobSnapshot | null {
    if (!downloadManager) return null;
    const manager = downloadManager;

    if (manager.activeJobId) {
      return (
        manager.jobs.find((j) => j.id === manager.activeJobId) ?? null
      );
    }
    // Fallback: find first running job
    return manager.jobs.find((j) => j.status === "running") ?? null;
  }

  function formatByteSize(bytes: number | null | undefined): string {
    if (
      bytes === null ||
      bytes === undefined ||
      !Number.isFinite(bytes) ||
      bytes < 0
    ) {
      return "未知大小";
    }

    if (bytes < 1024) return `${bytes} B`;
    const units = ["KB", "MB", "GB", "TB"];
    let value = bytes;
    let unitIndex = -1;

    while (value >= 1024 && unitIndex < units.length - 1) {
      value /= 1024;
      unitIndex += 1;
    }

    const precision = value >= 100 ? 0 : value >= 10 ? 1 : 2;
    return `${value.toFixed(precision)} ${units[unitIndex]}`;
  }

  function formatSpeed(bytesPerSec: number): string {
    if (bytesPerSec < 1024) return `${bytesPerSec.toFixed(0)} B/s`;
    const units = ["KB/s", "MB/s", "GB/s"];
    let value = bytesPerSec;
    let unitIndex = -1;

    while (value >= 1024 && unitIndex < units.length - 1) {
      value /= 1024;
      unitIndex += 1;
    }

    const precision = value >= 100 ? 0 : value >= 10 ? 1 : 2;
    return `${value.toFixed(precision)} ${units[unitIndex]}`;
  }

  function getTaskProgressLabel(task: DownloadTaskSnapshot): string | null {
    if (task.status !== "downloading" && task.status !== "writing") {
      return null;
    }

    if (
      task.status === "downloading" &&
      task.bytesTotal &&
      task.bytesTotal > 0
    ) {
      const percent = Math.min(
        Math.round((task.bytesDone / task.bytesTotal) * 100),
        100,
      );
      const speed = taskSpeedMap.get(task.id);
      const speedText = speed && speed > 0 ? ` · ${formatSpeed(speed)}` : "";
      return `${formatByteSize(task.bytesDone)} / ${formatByteSize(task.bytesTotal)} · ${percent}%${speedText}`;
    }

    if (task.bytesDone > 0) {
      return `${formatByteSize(task.bytesDone)} 已处理`;
    }

    return task.status === "writing" ? "正在整理文件..." : "正在接收数据...";
  }

  function getTaskErrorLabel(task: DownloadTaskSnapshot): string | null {
    if (!task.error) return null;

    if (task.error.details && task.error.details !== task.error.message) {
      return `${task.error.message} · ${task.error.details}`;
    }

    return task.error.message;
  }

  function getJobErrorSummary(job: DownloadJobSnapshot): string | null {
    const firstFailedTask = job.tasks.find(
      (task) => task.status === "failed" && task.error,
    );
    if (firstFailedTask) {
      return getTaskErrorLabel(firstFailedTask);
    }

    const firstCancelledTask = job.tasks.find(
      (task) => task.status === "cancelled" && task.error,
    );
    if (firstCancelledTask) {
      return getTaskErrorLabel(firstCancelledTask);
    }

    if (!job.error) return null;

    if (job.error.details && job.error.details !== job.error.message) {
      return `${job.error.message} · ${job.error.details}`;
    }

    return job.error.message;
  }

  function getJobProgressText(job: DownloadJobSnapshot): string {
    const terminalCount =
      job.completedTaskCount + job.failedTaskCount + job.cancelledTaskCount;
    const activeTask = job.tasks.find(
      (task) =>
        task.status === "preparing" ||
        task.status === "downloading" ||
        task.status === "writing",
    );

    const base = `${terminalCount}/${job.taskCount} 首已结束`;
    if (!activeTask) {
      return base;
    }

    const progressLabel = getTaskProgressLabel(activeTask);
    if (!progressLabel) {
      return `${base} · 正在处理 ${activeTask.songName}`;
    }

    return `${base} · ${activeTask.songName} · ${progressLabel}`;
  }

  function getJobProgress(job: DownloadJobSnapshot): number {
    if (job.taskCount === 0) return 0;

    const terminalCount =
      job.completedTaskCount + job.failedTaskCount + job.cancelledTaskCount;
    const activeTask = job.tasks.find(
      (task) =>
        task.status === "preparing" ||
        task.status === "downloading" ||
        task.status === "writing",
    );

    if (!activeTask) {
      return terminalCount / job.taskCount;
    }

    const activeTaskProgress =
      activeTask.status === "downloading" && activeTask.bytesTotal
        ? activeTask.bytesDone / activeTask.bytesTotal
        : activeTask.status === "writing"
          ? 1
          : 0;

    return Math.min((terminalCount + activeTaskProgress) / job.taskCount, 1);
  }

  function getJobStatusLabel(job: DownloadJobSnapshot): string {
    switch (job.status) {
      case "queued":
        return "排队中";
      case "running": {
        const activeTask = job.tasks.find(
          (task) =>
            task.status === "preparing" ||
            task.status === "downloading" ||
            task.status === "writing",
        );
        const currentIndex = activeTask
          ? activeTask.songIndex + 1
          : job.completedTaskCount;
        return `下载中 (${currentIndex}/${job.taskCount})`;
      }
      case "completed":
        return "已完成";
      case "partiallyFailed":
        return `部分失败 (${job.failedTaskCount}/${job.taskCount})`;
      case "failed":
        return "失败";
      case "cancelled":
        return "已取消";
      default:
        return job.status;
    }
  }

  function getTaskStatusLabel(task: DownloadTaskSnapshot): string {
    switch (task.status) {
      case "queued":
        return "排队中";
      case "preparing":
        return "准备中";
      case "downloading": {
        const progressLabel = getTaskProgressLabel(task);
        return progressLabel ?? "下载中...";
      }
      case "writing": {
        const progressLabel = getTaskProgressLabel(task);
        return progressLabel ? `写入中 · ${progressLabel}` : "写入中";
      }
      case "completed":
        return "已完成";
      case "failed":
        return "失败";
      case "cancelled":
        return "已取消";
      default:
        return task.status;
    }
  }

  function getJobKindLabel(job: DownloadJobSnapshot): string {
    switch (job.kind) {
      case "song":
        return "单曲下载";
      case "album":
        return "整专下载";
      case "selection":
        return "多选下载";
      default:
        return job.kind;
    }
  }

  function getSelectionJobAlbumCount(job: DownloadJobSnapshot): number {
    return new Set(job.tasks.map((task) => task.albumCid)).size;
  }

  function getSelectionJobScopeLabel(job: DownloadJobSnapshot): string {
    const albumCount = getSelectionJobAlbumCount(job);
    if (albumCount <= 1) {
      const albumName = job.tasks[0]?.albumName;
      return albumName ? `来自《${albumName}》` : "来自同一张专辑";
    }

    return `跨 ${albumCount} 张专辑`;
  }

  function getJobSummaryLabel(job: DownloadJobSnapshot): string {
    switch (job.kind) {
      case "song": {
        const task = job.tasks[0];
        return task?.albumName ? `来自《${task.albumName}》` : "单曲任务";
      }
      case "album":
        return `${job.taskCount} 首歌曲`;
      case "selection": {
        if (job.taskCount <= 1) {
          return getSelectionJobScopeLabel(job);
        }

        const albumCount = getSelectionJobAlbumCount(job);
        if (albumCount <= 1) {
          return `${job.taskCount} 首歌曲`;
        }

        return `${job.taskCount} 首歌曲 · 跨 ${albumCount} 张专辑`;
      }
      default:
        return `${job.taskCount} 首歌曲`;
    }
  }

  function isJobActive(jobId: string): boolean {
    return downloadManager?.activeJobId === jobId;
  }

  function canCancelTask(task: DownloadTaskSnapshot): boolean {
    return (
      task.status === "queued" ||
      task.status === "preparing" ||
      task.status === "downloading" ||
      task.status === "writing"
    );
  }

  function canRetryTask(task: DownloadTaskSnapshot): boolean {
    return task.status === "failed" || task.status === "cancelled";
  }

  function canClearDownloadHistory(): boolean {
    return !!downloadManager?.jobs.some(
      (job) =>
        job.status === "completed" ||
        job.status === "failed" ||
        job.status === "cancelled" ||
        job.status === "partiallyFailed",
    );
  }

  async function handleCancelDownloadJob(jobId: string) {
    try {
      await cancelDownloadJob(jobId);
    } catch (e) {
      console.error("[ERROR] Failed to cancel download job:", e);
    }
  }

  async function handleCancelDownloadTask(jobId: string, taskId: string) {
    try {
      await cancelDownloadTask(jobId, taskId);
    } catch (e) {
      console.error("[ERROR] Failed to cancel download task:", e);
    }
  }

  async function handleRetryDownloadJob(jobId: string) {
    try {
      await retryDownloadJob(jobId);
    } catch (e) {
      console.error("[ERROR] Failed to retry download job:", e);
    }
  }

  async function handleRetryDownloadTask(jobId: string, taskId: string) {
    try {
      await retryDownloadTask(jobId, taskId);
    } catch (e) {
      console.error("[ERROR] Failed to retry download task:", e);
    }
  }

  async function handleClearDownloadHistory() {
    try {
      const removed = await clearDownloadHistory();
      if (removed === 0) {
        notifyInfo("当前没有可清理的下载历史。");
      }
    } catch (e) {
      console.error("[ERROR] Failed to clear download history:", e);
      notifyError(`清理下载历史失败：${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function handleSongDownload(song: SongEntry) {
    try {
      const existingJob = getSongDownloadJob(song.cid);
      await performSongDownload(song.cid);
      if (existingJob) {
        notifyInfo("这首歌的下载任务已在队列中或正在执行。");
      }
    } catch (error) {
      console.error("[ERROR] Failed to download song:", error);
      notifyError(
        `下载失败：${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async function handlePlay(song: SongEntry) {
    const sourceEntries = buildAlbumPlaybackEntries(selectedAlbum);
    const fallbackEntry: PlaybackQueueEntry = {
      cid: song.cid,
      name: song.name,
      artists: song.artists,
      coverUrl: getSelectedAlbumCoverUrl(),
    };
    const entries = sourceEntries.length ? sourceEntries : [fallbackEntry];

    applyPlaybackQueue(entries, song.cid);

    const nextOrder = shuffleEnabled ? [...playbackOrder] : [...entries];
    const nextIndex = nextOrder.findIndex((entry) => entry.cid === song.cid);
    if (nextIndex < 0) return;

    await playQueueEntry(nextOrder[nextIndex], nextOrder, nextIndex);
  }

  async function handlePausePlayback() {
    try {
      await pausePlayback();
    } catch (e) {
      console.error("[ERROR] Failed to pause playback:", e);
    }
  }

  async function handleResumePlayback() {
    try {
      await resumePlayback();
    } catch (e) {
      console.error("[ERROR] Failed to resume playback:", e);
    }
  }

  async function handleSeekPlayback(positionSecs: number) {
    if (!duration || duration <= 0 || isLoading) return;
    try {
      await seekCurrentPlayback(positionSecs);
    } catch (e) {
      console.error("[ERROR] Failed to seek playback:", e);
    }
  }

  async function handlePlayNext() {
    if (!playerHasNext) return;

    const nextIndex = resolveWrappedQueueIndex(1);
    if (nextIndex < 0) return;

    await playQueueEntry(playbackOrder[nextIndex], playbackOrder, nextIndex);
  }

  async function handlePlayPrevious() {
    if (!currentSong) return;

    if (progress > 3 && !isLoading) {
      await handleSeekPlayback(0);
      return;
    }

    const previousIndex = resolveWrappedQueueIndex(-1);
    if (previousIndex < 0) return;

    await playQueueEntry(
      playbackOrder[previousIndex],
      playbackOrder,
      previousIndex,
    );
  }

  // Refresh cache and reload current album
  let isRefreshing = $state(false);

  async function handleRefresh() {
    if (isRefreshing) return;
    isRefreshing = true;
    const requestSeq = ++albumRequestSeq;

    clearSongSelection();
    selectionModeEnabled = false;

    try {
      await clearCache();
      await clearResponseCache();

      const nextAlbums = await getAlbums();
      albums = nextAlbums;
      if (selectedAlbumCid) {
        const currentAlbumCid = selectedAlbumCid;
        loadingDetail = true;
        if (!selectedAlbum) {
          armDetailSkeleton();
        } else {
          clearDetailSkeleton();
        }
        const refreshedAlbum = nextAlbums.find((album) => album.cid === currentAlbumCid);
        if (refreshedAlbum) {
          try {
            const detail = await getAlbumDetail(
              currentAlbumCid,
              localInventory?.inventoryVersion ?? null,
            );
            if (requestSeq === albumRequestSeq) {
              const artworkAspectRatio = await preloadAlbumArtwork(detail);
              if (requestSeq === albumRequestSeq) {
                setAlbumStageAspectRatio(artworkAspectRatio);
              }
            }
            if (requestSeq === albumRequestSeq) {
              selectedAlbum = detail;
              await tick();
              resetContentScroll();
            }
          } catch (e) {
            if (requestSeq === albumRequestSeq) {
              console.error("[ERROR] Failed to reload album:", e);
            }
          } finally {
            if (requestSeq === albumRequestSeq) {
              clearDetailSkeleton();
              loadingDetail = false;
            }
          }
        } else if (requestSeq === albumRequestSeq) {
          selectedAlbum = null;
          clearDetailSkeleton();
          loadingDetail = false;
        }
      }
    } catch (e) {
      console.error("[ERROR] Failed to refresh album list:", e);
    } finally {
      await delay(400);
      isRefreshing = false;
    }
  }
</script>

{#if isMacOS}
  <div
    class="macos-window-drag-region"
    data-tauri-drag-region
    aria-hidden="true"
  ></div>
{/if}

<StatusToastHost />

<div class="container" class:macos-overlay={isMacOS}>
  <!-- 专辑列表侧边栏 -->
  <OverlayScrollbarsComponent
    element="aside"
    class="sidebar"
    data-overlayscrollbars-initialize
    options={overlayScrollbarOptions}
    defer
  >
    {#if isMacOS}
      <div
        class="sidebar-drag-region"
        data-tauri-drag-region
        aria-hidden="true"
      ></div>
    {/if}
    <AlbumSidebar
      {albums}
      {selectedAlbumCid}
      reducedMotion={prefersReducedMotion}
      {loadingAlbums}
      {errorMsg}
      onSelect={handleSelectAlbum}
    />
  </OverlayScrollbarsComponent>

  <section class="main-region">
    {#if isMacOS}
      <div
        class="main-drag-region"
        data-tauri-drag-region
        aria-hidden="true"
      ></div>
    {/if}

    <TopToolbar
      {activeDownloadCount}
      {isRefreshing}
      {settingsOpen}
      {downloadPanelOpen}
      onRefresh={handleRefresh}
      onOpenDownloads={() => {
        downloadPanelOpen = !downloadPanelOpen;
        if (downloadPanelOpen) settingsOpen = false;
      }}
      onOpenSettings={() => {
        settingsOpen = !settingsOpen;
        if (settingsOpen) downloadPanelOpen = false;
      }}
    />


    <!-- 歌曲列表内容区 -->
    <AlbumWorkspace
      {currentSong}
      {loadingDetail}
      {selectedAlbum}
    >
      {#snippet children()}
    <OverlayScrollbarsComponent
      element="div"
      class="h-full"
      data-overlayscrollbars-initialize
      bind:this={contentScrollbar}
      options={overlayScrollbarOptions}
      events={contentScrollbarEvents}
      defer
      onwheel={handleContentWheel}
      aria-busy={loadingDetail}
    >
      <AnimatePresence mode="wait">
        {#if loadingDetail && showDetailSkeleton}
          <motion.section
            key={`loading-${albumRequestSeq}`}
            class="album-panel album-panel-loading"
            initial={fadeEnter()}
            animate={{ opacity: 1 }}
            exit={fadeExit()}
            transition={motionTransition(PANEL_DURATION)}
          >
            <div
              class="album-stage"
              bind:this={albumStageEl}
              style={albumStageStyle}
            >
              <div class="album-stage-frame">
                <div
                  class="album-stage-media album-stage-media-loading"
                  style:height={albumStageMediaHeight}
                >
                  <div class="album-stage-media-content">
                    <MotionPulseBlock
                      className="album-stage-skeleton loading-cover"
                      reducedMotion={prefersReducedMotion}
                    />
                  </div>
                  <div
                    class="album-stage-media-scrim"
                    aria-hidden="true"
                    style:opacity={albumStageScrimOpacity}
                  ></div>
                  <div
                    class="album-stage-media-border"
                    aria-hidden="true"
                  ></div>
                  <div class="album-stage-divider" aria-hidden="true"></div>
                </div>
              </div>
            </div>
            <motion.div
              class="album-detail-card"
              initial={fadeEnter()}
              animate={{ opacity: 1 }}
              exit={fadeExit()}
              transition={motionTransition(PANEL_DURATION)}
            >
              <div class="album-hero">
                <motion.div
                  class="album-hero-info"
                  initial={fadeEnter()}
                  animate={{ opacity: 1 }}
                  exit={fadeExit()}
                  transition={motionTransition(HERO_DURATION, HERO_DELAY)}
                >
                  <MotionPulseBlock
                    className="album-hero-title loading-text"
                    reducedMotion={prefersReducedMotion}
                  />
                  <MotionPulseBlock
                    className="album-hero-sub loading-text-sub"
                    reducedMotion={prefersReducedMotion}
                    delay={0.14}
                  />
                </motion.div>
              </div>
              <motion.div
                class="loading album-loading"
                initial={fadeEnter()}
                animate={{ opacity: 1 }}
                exit={fadeExit()}
                transition={motionTransition(LIST_DURATION, LIST_DELAY)}
              >
                <span>正在加载歌曲...</span><MotionSpinner
                  className="inline-loading-spinner"
                  reducedMotion={prefersReducedMotion}
                />
              </motion.div>
            </motion.div>
          </motion.section>
        {:else if selectedAlbum}
          <motion.section
            key={selectedAlbum.cid}
            class="album-panel"
            initial={fadeEnter()}
            animate={{ opacity: 1 }}
            exit={fadeExit()}
            transition={motionTransition(PANEL_DURATION)}
          >
            <div
              class="album-stage"
              bind:this={albumStageEl}
              style={albumStageStyle}
            >
              <div class="album-stage-frame">
                <div
                  class="album-stage-media"
                  style:height={albumStageMediaHeight}
                >
                  <div class="album-stage-media-content">
                    <img
                      class="album-stage-image"
                      src={selectedAlbumArtworkUrl ?? undefined}
                      alt="{selectedAlbum.name} banner"
                      loading="eager"
                      style:opacity={albumStageImageOpacity}
                      style:transform={albumStageImageTransform}
                    />
                    <div
                      class="album-stage-solidify"
                      aria-hidden="true"
                      style:opacity={albumStageSolidifyOpacity}
                    ></div>
                  </div>
                  <div
                    class="album-stage-media-scrim"
                    aria-hidden="true"
                    style:opacity={albumStageScrimOpacity}
                  ></div>
                  <div
                    class="album-stage-media-border"
                    aria-hidden="true"
                  ></div>
                  <div class="album-stage-divider" aria-hidden="true"></div>
                </div>
              </div>
            </div>
            <motion.div
              class="album-detail-card"
              initial={fadeEnter()}
              animate={{ opacity: 1 }}
              exit={fadeExit()}
              transition={motionTransition(PANEL_DURATION)}
            >
              <div class="album-hero">
                <motion.div
                  class="album-hero-info"
                  initial={axisEnter("y", 14)}
                  animate={axisAnimate("y")}
                  exit={axisExit("y", 8)}
                  transition={motionTransition(HERO_DURATION, HERO_DELAY)}
                >
                  {#if selectedAlbum.belong}
                    <span class="album-belong-tag"
                      >{selectedAlbum.belong.toUpperCase()}</span
                    >
                  {/if}
                  <h1 class="album-hero-title">{selectedAlbum.name}</h1>
                  {#if selectedAlbum.artists && selectedAlbum.artists.length > 0}
                    <p class="album-hero-artists">
                      {selectedAlbum.artists.join(", ")}
                    </p>
                  {/if}
                  {#if selectedAlbum.intro}
                    <p class="album-hero-intro">{selectedAlbum.intro}</p>
                  {/if}
                  <div class="album-hero-meta">
                    <span class="album-song-count"
                      >{selectedAlbum.songs.length} 首歌曲</span
                    >
                    {#if shouldShowDownloadBadge(selectedAlbum.download.downloadStatus)}
                      <span class="album-download-status-badge">
                        {getDownloadBadgeLabel(selectedAlbum.download.downloadStatus)}
                      </span>
                    {/if}
                  </div>
                  <div class="controls album-hero-actions">
                    <motion.button
                      class="btn btn-primary"
                      onclick={handleAlbumDownload}
                      disabled={downloadingAlbumCid === selectedAlbum.cid ||
                        !!findAlbumDownloadJob(selectedAlbum.cid)}
                      animate={appButtonAnimate(
                        true,
                        downloadingAlbumCid === selectedAlbum.cid ||
                          !!findAlbumDownloadJob(selectedAlbum.cid),
                      )}
                      whileHover={appButtonHover(
                        true,
                        downloadingAlbumCid === selectedAlbum.cid ||
                          !!findAlbumDownloadJob(selectedAlbum.cid),
                      )}
                      whileTap={!prefersReducedMotion &&
                      !(
                        downloadingAlbumCid === selectedAlbum.cid ||
                        !!findAlbumDownloadJob(selectedAlbum.cid)
                      )
                        ? { y: 0, scale: 0.98, opacity: 0.94 }
                        : undefined}
                      transition={interactiveTransition}
                    >
                      {#if downloadingAlbumCid === selectedAlbum.cid}
                        正在创建任务...
                      {:else if findAlbumDownloadJob(selectedAlbum.cid)}
                        已在队列中
                      {:else}
                        下载整张专辑
                      {/if}
                    </motion.button>
                    <motion.button
                      class="btn"
                      onclick={toggleSelectionMode}
                      animate={appButtonAnimate(false, false)}
                      whileHover={appButtonHover(false, false)}
                      whileTap={prefersReducedMotion
                        ? undefined
                        : { y: 0, scale: 0.98, opacity: 0.94 }}
                      transition={interactiveTransition}
                    >
                      {selectionModeEnabled ? "取消多选" : "多选下载"}
                    </motion.button>
                    {#if selectionModeEnabled}
                      <motion.button
                        class="btn"
                        onclick={selectAllSongs}
                        disabled={!selectedAlbum ||
                          selectedSongCount === selectedAlbum.songs.length}
                        animate={appButtonAnimate(
                          false,
                          !selectedAlbum ||
                            selectedSongCount === selectedAlbum.songs.length,
                        )}
                        whileHover={appButtonHover(
                          false,
                          !selectedAlbum ||
                            selectedSongCount === selectedAlbum.songs.length,
                        )}
                        whileTap={!prefersReducedMotion &&
                        selectedAlbum &&
                        selectedSongCount !== selectedAlbum.songs.length
                          ? { y: 0, scale: 0.98, opacity: 0.94 }
                          : undefined}
                        transition={interactiveTransition}
                      >
                        全选
                      </motion.button>
                      <motion.button
                        class="btn"
                        onclick={deselectAllSongs}
                        disabled={selectedSongCount === 0}
                        animate={appButtonAnimate(false, selectedSongCount === 0)}
                        whileHover={appButtonHover(false, selectedSongCount === 0)}
                        whileTap={!prefersReducedMotion && selectedSongCount > 0
                          ? { y: 0, scale: 0.98, opacity: 0.94 }
                          : undefined}
                        transition={interactiveTransition}
                      >
                        清空
                      </motion.button>
                      <motion.button
                        class="btn"
                        onclick={invertSongSelection}
                        disabled={!selectedAlbum ||
                          selectedAlbum.songs.length === 0}
                        animate={appButtonAnimate(
                          false,
                          !selectedAlbum || selectedAlbum.songs.length === 0,
                        )}
                        whileHover={appButtonHover(
                          false,
                          !selectedAlbum || selectedAlbum.songs.length === 0,
                        )}
                        whileTap={!prefersReducedMotion &&
                        selectedAlbum &&
                        selectedAlbum.songs.length > 0
                          ? { y: 0, scale: 0.98, opacity: 0.94 }
                          : undefined}
                        transition={interactiveTransition}
                      >
                        反选
                      </motion.button>
                      <motion.button
                        class="btn btn-primary"
                        onclick={handleSelectionDownload}
                        disabled={isSelectionDownloadActionDisabled()}
                        animate={appButtonAnimate(
                          true,
                          isSelectionDownloadActionDisabled(),
                        )}
                        whileHover={appButtonHover(
                          true,
                          isSelectionDownloadActionDisabled(),
                        )}
                        whileTap={!prefersReducedMotion &&
                        !isSelectionDownloadActionDisabled()
                          ? { y: 0, scale: 0.98, opacity: 0.94 }
                          : undefined}
                        transition={interactiveTransition}
                      >
                        {#if isCurrentSelectionCreating()}
                          正在创建批量任务...
                        {:else if getCurrentSelectionJob()}
                          已在队列中
                        {:else}
                          下载所选歌曲
                        {/if}
                      </motion.button>
                      <span class="album-selection-summary"
                        >{selectedSongsLabel}</span
                      >
                    {/if}
                  </div>
                </motion.div>
              </div>
              <motion.div
                class="song-list"
                initial={axisEnter("y", 10)}
                animate={axisAnimate("y")}
                exit={fadeExit()}
                transition={motionTransition(LIST_DURATION, LIST_DELAY)}
              >
                {#each selectedAlbum.songs as song, i (song.cid)}
                  <SongRow
                    {song}
                    index={i}
                    isPlaying={currentSong?.cid === song.cid &&
                      (isPlaying || isPaused)}
                    downloadState={getSongDownloadState(song.cid)}
                    downloadDisabled={isSongDownloadInteractionBlocked(
                      song.cid,
                    )}
                    selectionMode={selectionModeEnabled}
                    isSelected={isSongSelected(song.cid)}
                    selectionDisabled={isCurrentSelectionCreating()}
                    reducedMotion={prefersReducedMotion}
                    onclick={() => handlePlay(song)}
                    onDownload={() => handleSongDownload(song)}
                    onToggleSelection={() => toggleSongSelection(song.cid)}
                  />
                {/each}
              </motion.div>
            </motion.div>
          </motion.section>
        {/if}
      </AnimatePresence>

      {#if !loadingDetail && !selectedAlbum}
        <h1 class="page-title">选择专辑</h1>
        <p class="page-subtitle">从左侧选择一个专辑以查看歌曲</p>
      {/if}

      <AnimatePresence>
        {#if loadingDetail && selectedAlbum}
          <motion.div
            key={`content-mask-${albumRequestSeq}`}
            class="content-loading-mask"
            aria-hidden="true"
            initial={fadeEnter()}
            animate={{ opacity: 1 }}
            exit={fadeExit()}
            transition={motionTransition(CONTENT_MASK_DURATION)}
          >
            <MotionSpinner
              className="content-loading-mask-spinner"
              reducedMotion={prefersReducedMotion}
            />
          </motion.div>
        {/if}
      </AnimatePresence>
    </OverlayScrollbarsComponent>
      {/snippet}
    </AlbumWorkspace>

    <AnimatePresence>
      {#if currentSong}
        <motion.div
          key="player-dock"
          initial={axisEnter("y", 18)}
          animate={axisAnimate("y")}
          exit={fadeExit()}
          transition={motionTransition(PLAYER_DOCK_DURATION)}
        >
          <div
            class="player-dock-stack"
            data-panel={lyricsOpen
              ? "lyrics"
              : playlistOpen
                ? "playlist"
                : "none"}
          >
            <AnimatePresence initial={false}>
              {#if lyricsOpen}
                <motion.section
                  key="player-lyrics"
                  class="player-flyout"
                  data-panel="lyrics"
                  initial={axisEnter("y", 12)}
                  animate={axisAnimate("y")}
                  exit={axisExit("y", 8)}
                  transition={motionTransition(0.18)}
                >
                  <div class="player-flyout-header">
                    <div>
                      <p class="player-flyout-eyebrow">歌词</p>
                      <h3 class="player-flyout-title">{currentSong.name}</h3>
                    </div>
                    <span class="player-flyout-count"
                      >{lyricsLines.length > 0
                        ? `${lyricsLines.length} 行`
                        : "歌词"}</span
                    >
                  </div>

                  {#if lyricsLoading}
                    <div class="player-flyout-empty">正在加载歌词…</div>
                  {:else if lyricsError}
                    <div class="player-flyout-empty">{lyricsError}</div>
                  {:else if lyricsLines.length > 0}
                    <div class="player-lyrics-list">
                      {#each lyricsLines as line, index (line.id)}
                        <p
                          class={`player-lyric-line${index === activeLyricIndex ? " active" : ""}`}
                        >
                          {line.text}
                        </p>
                      {/each}
                    </div>
                  {:else}
                    <div class="player-flyout-empty">这首歌暂时没有歌词。</div>
                  {/if}
                </motion.section>
              {:else if playlistOpen}
                <motion.section
                  key="player-playlist"
                  class="player-flyout"
                  data-panel="playlist"
                  initial={axisEnter("y", 12)}
                  animate={axisAnimate("y")}
                  exit={axisExit("y", 8)}
                  transition={motionTransition(0.18)}
                >
                  <div class="player-flyout-header">
                    <div>
                      <p class="player-flyout-eyebrow">播放列表</p>
                      <h3 class="player-flyout-title">当前队列</h3>
                    </div>
                    <span class="player-flyout-count"
                      >{playbackOrder.length} 首</span
                    >
                  </div>

                  {#if playbackOrder.length > 0}
                    <div class="player-playlist-list">
                      {#each playbackOrder as entry, index (entry.cid)}
                        <button
                          type="button"
                          class={`player-playlist-item${entry.cid === currentSong?.cid ? " active" : ""}`}
                          onclick={() => {
                            void playQueueEntry(entry, playbackOrder, index);
                          }}
                        >
                          <span class="player-playlist-index"
                            >{String(index + 1).padStart(2, "0")}</span
                          >
                          <span class="player-playlist-meta">
                            <span class="player-playlist-name"
                              >{entry.name}</span
                            >
                            <span class="player-playlist-artists"
                              >{entry.artists.join(" · ")}</span
                            >
                          </span>
                        </button>
                      {/each}
                    </div>
                  {:else}
                    <div class="player-flyout-empty">
                      当前没有可播放的队列。
                    </div>
                  {/if}
                </motion.section>
              {/if}
            </AnimatePresence>

            <PlayerDock
              song={currentSong}
              {isPlaying}
              {isPaused}
              hasPrevious={playerHasPrevious}
              hasNext={playerHasNext}
              {progress}
              {duration}
              {isLoading}
              isShuffled={shuffleEnabled}
              {repeatMode}
              lyricsActive={lyricsOpen}
              playlistActive={playlistOpen}
              downloadState={currentSong
                ? getSongDownloadState(currentSong.cid)
                : "idle"}
              downloadDisabled={currentSong
                ? isSongDownloadInteractionBlocked(currentSong.cid)
                : false}
              reducedMotion={prefersReducedMotion}
              onPrevious={handlePlayPrevious}
              onTogglePlay={isPlaying
                ? handlePausePlayback
                : handleResumePlayback}
              onSeek={handleSeekPlayback}
              onNext={handlePlayNext}
              onShuffleChange={handleShuffleChange}
              onRepeatModeChange={handleRepeatModeChange}
              onToggleLyrics={toggleLyricsPanel}
              onTogglePlaylist={togglePlaylistPanel}
              onDownload={handleCurrentSongDownload}
            />
          </div>
        </motion.div>
      {/if}
    </AnimatePresence>
  </section>

  <SettingsSheet
    bind:open={settingsOpen}
    bind:format
    {outputDir}
    bind:downloadLyrics
    bind:notifyOnDownloadComplete
    bind:notifyOnPlaybackChange
    bind:logLevel
    {logFileKind}
    {logRecords}
    {logFileStatus}
    {logViewerLoading}
    {logViewerError}
    {isSendingTestNotification}
    {isClearingAudioCache}
    onSelectDirectory={handleSelectDirectory}
    onSendTestNotification={handleSendTestNotification}
    onClearAudioCache={handleClearAudioCache}
    onChangeLogFileKind={refreshLogs}
  />

  <DownloadTasksSheet
    bind:open={downloadPanelOpen}
    {downloadManager}
    {canClearDownloadHistory}
    {getJobProgress}
    {getJobProgressText}
    {getJobStatusLabel}
    {getJobKindLabel}
    {getJobSummaryLabel}
    {getJobErrorSummary}
    {isJobActive}
    {canCancelTask}
    {canRetryTask}
    {getTaskErrorLabel}
    {getTaskStatusLabel}
    onClearDownloadHistory={handleClearDownloadHistory}
    onCancelDownloadJob={handleCancelDownloadJob}
    onRetryDownloadJob={handleRetryDownloadJob}
    onCancelDownloadTask={handleCancelDownloadTask}
    onRetryDownloadTask={handleRetryDownloadTask}
  />

</div>
