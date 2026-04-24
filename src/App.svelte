<script lang="ts">
  import { tick } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { AnimatePresence, motion } from "@humanspeak/svelte-motion";
  import { OverlayScrollbarsComponent } from "overlayscrollbars-svelte";
  import type {
    EventListeners,
    OverlayScrollbars,
    PartialOptions,
  } from "overlayscrollbars";
  import {
    getAlbums,
    getAlbumDetail,
    getDefaultOutputDir,
    playSong,
    pausePlayback,
    resumePlayback,
    seekCurrentPlayback,
    getPlayerState,
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
    getPreferences,
    setPreferences,
    getLocalInventorySnapshot,
    searchLibrary,
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
    LibrarySearchScope,
    SearchLibraryResultItem,
  } from "$lib/types";
  import { applyThemePalette, DEFAULT_THEME_PALETTE } from "$lib/theme";
  import { envStore } from "$lib/features/env/store.svelte";
  import { shellStore } from "$lib/features/shell/store.svelte";
  import { createLibraryController } from "$lib/features/library/controller.svelte";
  import { createPlayerController } from "$lib/features/player/controller.svelte";
  import { createDownloadController } from "$lib/features/download/controller.svelte";
  import { clamp, preloadImage } from "$lib/features/library/helpers";
  import { buildAlbumPlaybackEntries, getSelectedAlbumCoverUrl } from "$lib/features/library/selectors";
  import { toast } from "svelte-sonner";
  import MotionSpinner from "$lib/components/MotionSpinner.svelte";
  import MotionPulseBlock from "$lib/components/MotionPulseBlock.svelte";
  import TopToolbar from "$lib/components/app/TopToolbar.svelte";
  import StatusToastHost from "$lib/components/app/StatusToastHost.svelte";
  import AlbumSidebar from "$lib/components/app/AlbumSidebar.svelte";
  import AlbumWorkspace from "$lib/components/app/AlbumWorkspace.svelte";
  import AlbumStage from "$lib/components/app/AlbumStage.svelte";
  import AlbumDetailSkeleton from "$lib/components/app/AlbumDetailSkeleton.svelte";
  import AlbumDetailPanel from "$lib/components/app/AlbumDetailPanel.svelte";
  import PlayerFlyoutStack from "$lib/components/app/PlayerFlyoutStack.svelte";

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
  const DEFAULT_ALBUM_STAGE_ASPECT_RATIO = 16 / 9;
  const ALBUM_STAGE_BASE_VIEWPORT_RATIO = 1 / 3;
  const ALBUM_STAGE_COLLAPSE_SCROLL_RANGE = 260;
  const ALBUM_STAGE_SOLIDIFY_SCROLL_RANGE = 220;

  const delay = (ms: number) =>
    new Promise((resolve) => setTimeout(resolve, ms));

  type SettingsSheetComponent = typeof import("$lib/components/app/SettingsSheet.svelte").default;
  type DownloadTasksSheetComponent = typeof import("$lib/components/app/DownloadTasksSheet.svelte").default;

  const libraryController = createLibraryController({
    delay,
    detailSkeletonDelayMs: DETAIL_SKELETON_DELAY_MS,
    minDetailDisplayMs: MIN_DISPLAY_MS,
    getAlbums,
    getAlbumDetail,
    searchLibrary,
    preloadAlbumArtwork,
    setAlbumStageAspectRatio,
    notifyError,
  });

  const playerController = createPlayerController({
    playSong: async (songCid, coverUrl, context) => {
      await playSong(songCid, coverUrl ?? undefined, context ?? undefined);
    },
    pausePlayback,
    resumePlayback,
    seekCurrentPlayback,
    getSongLyrics,
    notifyError,
  });

  const downloadController = createDownloadController({
    createDownloadJob,
    cancelDownloadJob,
    cancelDownloadTask,
    retryDownloadJob,
    retryDownloadTask,
    clearDownloadHistory,
    openDownloadPanel: async (resetFilters = false) => {
      if (resetFilters) {
        downloadController.resetFilters();
      }

      const loaded = await ensureDownloadTasksSheetLoaded();
      if (!loaded) {
        return;
      }

      downloadPanelOpen = true;
      settingsOpen = false;
    },
    getDownloadOptions: () => ({
      outputDir,
      format,
      downloadLyrics,
    }),
    notifyInfo,
    notifyError,
  });

  let outputDir = $state("");
  let format = $state<OutputFormat>("flac");
  let selectedSongCids = $state<string[]>([]);
  let selectionModeEnabled = $state(false);
  // Download job system state
  let downloadPanelOpen = $state(false);
  let SettingsSheetView = $state<SettingsSheetComponent | null>(null);
  let DownloadTasksSheetView = $state<DownloadTasksSheetComponent | null>(null);
  let settingsSheetLoader = $state<Promise<SettingsSheetComponent> | null>(null);
  let downloadTasksSheetLoader = $state<Promise<DownloadTasksSheetComponent> | null>(null);
  let themeRequestSeq = 0;
  let artworkRequestSeq = 0;
  let playerStateInitSeq = 0;
  let playerStateHydratedFromEvent = false;
  const prefersReducedMotion = $derived(envStore.prefersReducedMotion);
  const albums = $derived(libraryController.albums);
  const selectedAlbum = $derived(libraryController.selectedAlbum);
  const selectedAlbumCid = $derived(libraryController.selectedAlbumCid);
  const loadingAlbums = $derived(libraryController.loadingAlbums);
  const loadingDetail = $derived(libraryController.loadingDetail);
  const errorMsg = $derived(libraryController.errorMsg);
  const librarySearchQuery = $derived(libraryController.librarySearchQuery);
  const librarySearchScope = $derived(libraryController.librarySearchScope);
  const librarySearchLoading = $derived(libraryController.librarySearchLoading);
  const librarySearchResponse = $derived(libraryController.librarySearchResponse);
  const pendingScrollToSongCid = $derived(libraryController.pendingScrollToSongCid);
  const showDetailSkeleton = $derived(libraryController.showDetailSkeleton);
  const albumRequestSeq = $derived(libraryController.albumRequestSeq);
  const currentSong = $derived(playerController.currentSong);
  const isPlaying = $derived(playerController.isPlaying);
  const isPaused = $derived(playerController.isPaused);
  const isLoading = $derived(playerController.isLoading);
  const progress = $derived(playerController.progress);
  const duration = $derived(playerController.duration);
  const shuffleEnabled = $derived(playerController.shuffleEnabled);
  const repeatMode = $derived(playerController.repeatMode);
  const playbackOrder = $derived(playerController.playbackOrder);
  const lyricsOpen = $derived(playerController.lyricsOpen);
  const playlistOpen = $derived(playerController.playlistOpen);
  const lyricsLoading = $derived(playerController.lyricsLoading);
  const lyricsError = $derived(playerController.lyricsError);
  const lyricsLines = $derived(playerController.lyricsLines);
  const downloadingSongCid = $derived(downloadController.downloadingSongCid);
  const downloadingAlbumCid = $derived(downloadController.downloadingAlbumCid);
  const activeDownloadCount = $derived(downloadController.activeDownloadCount);
  const filteredDownloadJobs = $derived(downloadController.filteredJobs);
  const hasDownloadHistory = $derived(downloadController.hasDownloadHistory);
  let contentEl = $state<HTMLElement | null>(null);
  let albumStageEl = $state<HTMLElement | null>(null);
  let selectedAlbumArtworkUrl = $state<string | null>(null);
  const isMacOS = $derived(envStore.isMacOS);
  let albumStageAspectRatio = $state(DEFAULT_ALBUM_STAGE_ASPECT_RATIO);
  let albumStageWidth = $state(0);
  const viewportHeight = $derived(envStore.viewportHeight);
  let albumStageCollapseOffset = $state(0);
  let albumStageScrollTop = $state(0);
  let albumStageMotionFrame = 0;
  let pendingAlbumStageCollapseOffset = 0;
  let pendingAlbumStageScrollTop = 0;

  const playerHasPrevious = $derived(playerController.playerHasPrevious);
  const playerHasNext = $derived(playerController.playerHasNext);

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
      } catch {
        if (paletteRequestSeq !== themeRequestSeq) return;
        applyThemePalette(DEFAULT_THEME_PALETTE);
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
      } catch {
        if (requestSeq !== artworkRequestSeq) return;
        selectedAlbumArtworkUrl = null;
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

  async function bootstrapApp(shouldDispose: () => boolean) {
    try {
      await warmCacheManager();
    } catch {
      // Keep startup usable if IndexedDB warm start is unavailable.
    }

    if (shouldDispose()) {
      return;
    }

    try {
      const prefs = await getPreferences();
      if (shouldDispose()) {
        return;
      }
      outputDir = prefs.outputDir || outputDir;
      format = prefs.outputFormat || format;
      downloadLyrics = prefs.downloadLyrics;
      notifyOnDownloadComplete = prefs.notifyOnDownloadComplete;
      notifyOnPlaybackChange = prefs.notifyOnPlaybackChange;
      logLevel = prefs.logLevel;
      prefsReady = true;
    } catch {
      if (!shouldDispose()) {
        prefsReady = true;
      }
    }

    const defaultDirPromise = outputDir
      ? Promise.resolve("")
      : getDefaultOutputDir().catch(() => "");

    try {
      const albumList = await libraryController.loadAlbums({ shouldDispose });

      const defaultDir = await defaultDirPromise;
      if (shouldDispose()) {
        return;
      }
      if (defaultDir && !outputDir) {
        outputDir = defaultDir;
      }

      try {
        const snapshot = await getLocalInventorySnapshot();
        if (shouldDispose()) {
          return;
        }
        libraryController.initializeInventory(snapshot);
      } catch {
        if (!shouldDispose()) {
          libraryController.initializeInventory(null);
        }
      }

      if (albumList.length > 0 && !libraryController.selectedAlbumCid) {
        clearSongSelection();
        selectionModeEnabled = false;
        await libraryController.selectAlbum(albumList[0], {
          shouldDispose,
          afterSelect: async () => {
            await tick();
            resetContentScroll();
          },
        });
        if (shouldDispose()) {
          return;
        }
      }
    } catch {
      const defaultDir = await defaultDirPromise;
      if (shouldDispose()) {
        return;
      }
      if (defaultDir && !outputDir) {
        outputDir = defaultDir;
      }
    }

    try {
      const requestSeq = downloadController.beginHydrationAttempt();
      const manager = await listDownloadJobs();
      if (shouldDispose()) {
        return;
      }
      downloadController.applyManagerSnapshot(manager, requestSeq);
    } catch {
      // Download manager not available
    }

    try {
      const requestSeq = ++playerStateInitSeq;
      const playerState = await getPlayerState();
      if (shouldDispose()) {
        return;
      }
      if (requestSeq === playerStateInitSeq && !playerStateHydratedFromEvent) {
        playerController.syncPlayerState(playerState);
      }
    } catch {
      // Player not playing on startup
    }
  }

  async function subscribeToTauriEvents(shouldDispose: () => boolean) {
    const unlisteners: Array<() => void> = [];

    const cleanup = () => {
      while (unlisteners.length > 0) {
        unlisteners.pop()?.();
      }
    };

    async function register<T>(
      eventName: string,
      handler: (event: { payload: T }) => void | Promise<void>,
    ) {
      const unlisten = await listen<T>(eventName, async (event) => {
        if (shouldDispose()) {
          return;
        }
        await handler(event);
      });

      if (shouldDispose()) {
        unlisten();
        return false;
      }

      unlisteners.push(unlisten);
      return true;
    }

    try {
      if (!(await register<PlayerState>("player-state-changed", (event) => {
        playerStateHydratedFromEvent = true;
        playerController.syncPlayerState(event.payload);
      }))) {
        return cleanup;
      }

      if (!(await register<PlayerState>("player-progress", (event) => {
        playerController.syncPlayerProgress(event.payload);
      }))) {
        return cleanup;
      }

      if (!(await register<DownloadManagerSnapshot>(
        "download-manager-state-changed",
        (event) => {
          downloadController.applyManagerEvent(event.payload);
        },
      ))) {
        return cleanup;
      }

      if (!(await register<DownloadJobSnapshot>("download-job-updated", (event) => {
        downloadController.applyJobUpdate(event.payload);
      }))) {
        return cleanup;
      }

      if (!(await register<DownloadTaskProgressEvent>("download-task-progress", (event) => {
        downloadController.applyTaskProgress(event.payload);
      }))) {
        return cleanup;
      }

      if (!(await register<AppErrorEvent>("app-error-recorded", (event) => {
        handleAppErrorEvent(event.payload);
      }))) {
        return cleanup;
      }

      if (!(await register<LocalInventorySnapshot>(
        "local-inventory-state-changed",
        async (event) => {
          await libraryController.handleInventoryStateChanged(event.payload, {
            shouldDispose,
            invalidateInventoryCaches,
            onSelectionInvalidated: () => {
              clearSongSelection();
              selectionModeEnabled = false;
            },
          });
        },
      ))) {
        return cleanup;
      }

      return cleanup;
    } catch (error) {
      cleanup();
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(`failed to subscribe tauri events: ${message}`);
    }
  }

  function teardownAppRuntime(unsubscribe: (() => void) | null) {
    shellStore.dispose();
    envStore.dispose();
    libraryController.dispose();
    playerController.dispose();
    downloadController.dispose();
    playerStateInitSeq += 1;
    playerStateHydratedFromEvent = false;
    if (albumStageMotionFrame) {
      cancelAnimationFrame(albumStageMotionFrame);
    }
    unsubscribe?.();
  }

  $effect(() => {
    libraryController.init();
    playerController.init();
    downloadController.init();
    envStore.init();
    shellStore.init();

    let disposed = false;
    let unsubscribe: (() => void) | null = null;

    void (async () => {
      try {
        const nextUnsubscribe = await subscribeToTauriEvents(() => disposed);
        if (disposed) {
          nextUnsubscribe();
          return;
        }
        unsubscribe = nextUnsubscribe;

        await bootstrapApp(() => disposed);
      } catch (error) {
        if (disposed) {
          return;
        }
        notifyError(
          `初始化应用失败：${error instanceof Error ? error.message : String(error)}`,
        );
      }
    })();

    return () => {
      disposed = true;
      teardownAppRuntime(unsubscribe);
    };
  });

  $effect(() => {
    playerController.syncPlaybackLifecycle();
  });

  $effect(() => {
    if (!pendingScrollToSongCid || !selectedAlbum || loadingDetail) {
      return;
    }

    const expectedSongCid = pendingScrollToSongCid;
    void tick().then(() => {
      if (pendingScrollToSongCid !== expectedSongCid || !contentEl) {
        return;
      }

      const row = contentEl.querySelector<HTMLElement>(
        `[data-song-cid="${CSS.escape(expectedSongCid)}"]`,
      );
      if (!row) {
        return;
      }

      row.scrollIntoView({
        behavior: prefersReducedMotion ? "auto" : "smooth",
        block: "center",
      });
      libraryController.clearPendingScrollToSong(expectedSongCid);
    });
  });

  async function handleSelectSearchResult(item: SearchLibraryResultItem) {
    const album = albums.find((candidate) => candidate.cid === item.albumCid);
    if (!album) {
      notifyError("未找到对应专辑，可能需要先刷新列表。");
      return;
    }

    libraryController.setPendingScrollToSong(item.kind === "song" ? item.songCid : null);
    clearSongSelection();
    selectionModeEnabled = false;
    await libraryController.selectAlbum(album, {
      afterSelect: async () => {
        await tick();
        resetContentScroll();
      },
    });
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
  let downloadLyrics = $state(true);
  let notifyOnDownloadComplete = $state(true);
  let notifyOnPlaybackChange = $state(true);
  let logLevel = $state<LogLevel>("error");
  let isFormatHovered = $state(false);
  let isFormatFocused = $state(false);
  let isOutputDirHovered = $state(false);
  let isOutputDirFocused = $state(false);
  let settingsLogRefreshToken = $state(0);
  let prefsReady = $state(false);

  function handleAppErrorEvent(event: AppErrorEvent) {
    notifyError(event.message);
    if (settingsOpen) {
      settingsLogRefreshToken += 1;
    }
  }

  async function invalidateInventoryCaches(
    inventoryVersion: string | null | undefined,
  ) {
    await invalidateByTag(createInventoryCacheTag(inventoryVersion));
  }

  async function savePreferences(): Promise<boolean> {
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
      return true;
    } catch (e) {
      notifyError(
        `保存设置失败：${e instanceof Error ? e.message : String(e)}`,
      );
      return false;
    }
  }

  function notifyInfo(message: string) {
    toast(message);
  }

  function notifyError(message: string) {
    toast.error(message);
  }

  async function ensureSettingsSheetLoaded(): Promise<boolean> {
    if (SettingsSheetView) {
      return true;
    }

    if (!settingsSheetLoader) {
      settingsSheetLoader = import("$lib/components/app/SettingsSheet.svelte")
        .then((module) => {
          SettingsSheetView = module.default;
          return module.default;
        })
        .finally(() => {
          settingsSheetLoader = null;
        });
    }

    try {
      await settingsSheetLoader;
      return true;
    } catch (error) {
      notifyError(
        `打开设置面板失败：${error instanceof Error ? error.message : String(error)}`,
      );
      return false;
    }
  }

  async function ensureDownloadTasksSheetLoaded(): Promise<boolean> {
    if (DownloadTasksSheetView) {
      return true;
    }

    if (!downloadTasksSheetLoader) {
      downloadTasksSheetLoader = import("$lib/components/app/DownloadTasksSheet.svelte")
        .then((module) => {
          DownloadTasksSheetView = module.default;
          return module.default;
        })
        .finally(() => {
          downloadTasksSheetLoader = null;
        });
    }

    try {
      await downloadTasksSheetLoader;
      return true;
    } catch (error) {
      notifyError(
        `打开下载任务面板失败：${error instanceof Error ? error.message : String(error)}`,
      );
      return false;
    }
  }

  async function openSettingsPanel() {
    if (settingsOpen) {
      settingsOpen = false;
      return;
    }

    const loaded = await ensureSettingsSheetLoaded();
    if (!loaded) {
      return;
    }

    settingsOpen = true;
    downloadPanelOpen = false;
  }


  async function handlePlay(song: SongEntry) {
    const sourceEntries = buildAlbumPlaybackEntries(selectedAlbum);
    const fallbackEntry: PlaybackQueueEntry = {
      cid: song.cid,
      name: song.name,
      artists: song.artists,
      coverUrl: getSelectedAlbumCoverUrl(selectedAlbum),
    };
    const entries = sourceEntries.length ? sourceEntries : [fallbackEntry];

    playerController.applyPlaybackQueue(entries, song.cid);

    const nextOrder = shuffleEnabled ? [...playbackOrder] : [...entries];
    const nextIndex = nextOrder.findIndex((entry) => entry.cid === song.cid);
    if (nextIndex < 0) return;

    await playerController.playQueueEntry(nextOrder[nextIndex], nextOrder, nextIndex);
  }

  // Refresh cache and reload current album
  let isRefreshing = $state(false);

  async function handleRefresh() {
    if (isRefreshing) return;
    isRefreshing = true;

    clearSongSelection();
    selectionModeEnabled = false;

    try {
      await clearCache();
      await clearResponseCache();
      await libraryController.reloadAlbumsAndRefreshCurrentSelection({
        afterSelect: async () => {
          await tick();
          resetContentScroll();
        },
      });
    } catch (e) {
      notifyError(`刷新专辑列表失败：${e instanceof Error ? e.message : String(e)}`);
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
      searchQuery={librarySearchQuery}
      searchScope={librarySearchScope}
      searchLoading={librarySearchLoading}
      searchResponse={librarySearchResponse}
      onSearchQueryChange={libraryController.setSearchQuery}
      onSearchScopeChange={libraryController.setSearchScope}
      onSelect={(album) => {
        clearSongSelection();
        selectionModeEnabled = false;
        return libraryController.selectAlbum(album, {
          afterSelect: async () => {
            await tick();
            resetContentScroll();
          },
        });
      }}
      onSelectSearchResult={handleSelectSearchResult}
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
      onOpenDownloads={async () => {
        const nextOpen = !downloadPanelOpen;
        if (nextOpen) {
          await downloadController.openPanel();
          return;
        }
        downloadPanelOpen = false;
      }}
      onOpenSettings={async () => {
        await openSettingsPanel();
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
            <AlbumStage
              loading={true}
              reducedMotion={prefersReducedMotion}
              stageStyle={albumStageStyle}
              mediaHeight={albumStageMediaHeight}
              scrimOpacity={albumStageScrimOpacity}
              bind:element={albumStageEl}
            />
            <AlbumDetailSkeleton reducedMotion={prefersReducedMotion} />
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
            <AlbumStage
              albumName={selectedAlbum.name}
              artworkUrl={selectedAlbumArtworkUrl}
              reducedMotion={prefersReducedMotion}
              stageStyle={albumStageStyle}
              mediaHeight={albumStageMediaHeight}
              scrimOpacity={albumStageScrimOpacity}
              imageOpacity={albumStageImageOpacity}
              imageTransform={albumStageImageTransform}
              solidifyOpacity={albumStageSolidifyOpacity}
              bind:element={albumStageEl}
            />
            <AlbumDetailPanel
              album={selectedAlbum}
              currentSongCid={currentSong?.cid ?? null}
              isPlaybackActive={isPlaying || isPaused}
              {downloadingAlbumCid}
              {selectionModeEnabled}
              {selectedSongCids}
              reducedMotion={prefersReducedMotion}
              onToggleSelectionMode={toggleSelectionMode}
              onSelectAllSongs={selectAllSongs}
              onDeselectAllSongs={deselectAllSongs}
              onInvertSongSelection={invertSongSelection}
              onDownloadAlbum={downloadController.handleAlbumDownload}
              onDownloadSelection={(songCids) =>
                downloadController.handleSelectionDownload(songCids, {
                  afterCreated: () => {
                    clearSongSelection();
                    selectionModeEnabled = false;
                  },
                })}
              onPlaySong={handlePlay}
              onDownloadSong={downloadController.handleSongDownload}
              onToggleSongSelection={toggleSongSelection}
              {isSongSelected}
              getSongDownloadState={downloadController.getSongDownloadState}
              isSongDownloadInteractionBlocked={downloadController.isSongDownloadInteractionBlocked}
              hasAlbumDownloadJob={(albumCid) =>
                !!downloadController.findAlbumDownloadJob(albumCid)}
              isSelectionDownloadDisabled={downloadController.isSelectionDownloadActionDisabled}
              isCurrentSelectionCreating={downloadController.isCurrentSelectionCreating}
              hasCurrentSelectionJob={(songCids) =>
                !!downloadController.getCurrentSelectionJob(songCids)}
            />          </motion.section>
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

    <PlayerFlyoutStack
      song={currentSong}
      {isPlaying}
      {isPaused}
      hasPrevious={playerHasPrevious}
      hasNext={playerHasNext}
      {progress}
      {duration}
      {isLoading}
      reducedMotion={prefersReducedMotion}
      isShuffled={shuffleEnabled}
      {repeatMode}
      {lyricsOpen}
      {playlistOpen}
      {lyricsLoading}
      {lyricsError}
      {lyricsLines}
      {activeLyricIndex}
      {playbackOrder}
      downloadState={currentSong
        ? downloadController.getSongDownloadState(currentSong.cid)
        : "idle"}
      downloadDisabled={currentSong
        ? downloadController.isSongDownloadInteractionBlocked(currentSong.cid)
        : false}
      onPrevious={playerController.playPrevious}
      onTogglePlay={isPlaying
        ? playerController.pause
        : playerController.resume}
      onSeek={playerController.seek}
      onNext={playerController.playNext}
      onShuffleChange={playerController.toggleShuffle}
      onRepeatModeChange={playerController.toggleRepeat}
      onToggleLyrics={playerController.toggleLyrics}
      onTogglePlaylist={playerController.togglePlaylist}
      onDownload={() => {
        if (currentSong) {
          return downloadController.handleSongDownload(currentSong.cid);
        }
      }}
      onPlayQueueEntry={playerController.playQueueEntry}
    />
  </section>

  {#if SettingsSheetView}
    <SettingsSheetView
      bind:open={settingsOpen}
      bind:format
      bind:outputDir
      bind:downloadLyrics
      bind:notifyOnDownloadComplete
      bind:notifyOnPlaybackChange
      bind:logLevel
      logRefreshToken={settingsLogRefreshToken}
      {notifyInfo}
      {notifyError}
      onOutputDirChange={() => savePreferences()}
    />
  {/if}

  {#if DownloadTasksSheetView}
    <DownloadTasksSheetView
      bind:open={downloadPanelOpen}
      jobs={filteredDownloadJobs}
      hasDownloadHistory={hasDownloadHistory}
      bind:searchQuery={downloadController.searchQuery}
      bind:scopeFilter={downloadController.scopeFilter}
      bind:statusFilter={downloadController.statusFilter}
      bind:kindFilter={downloadController.kindFilter}
      canClearDownloadHistory={downloadController.canClearDownloadHistory}
      getJobProgress={downloadController.getJobProgress}
      getJobProgressText={downloadController.getJobProgressText}
      getJobStatusLabel={downloadController.getJobStatusLabel}
      getJobKindLabel={downloadController.getJobKindLabel}
      getJobSummaryLabel={downloadController.getJobSummaryLabel}
      getJobErrorSummary={downloadController.getJobErrorSummary}
      isJobActive={downloadController.isJobActive}
      canCancelTask={downloadController.canCancelTask}
      canRetryTask={downloadController.canRetryTask}
      getTaskErrorLabel={downloadController.getTaskErrorLabel}
      getTaskStatusLabel={downloadController.getTaskStatusLabel}
      onClearDownloadHistory={downloadController.handleClearDownloadHistory}
      onCancelDownloadJob={downloadController.handleCancelDownloadJob}
      onRetryDownloadJob={downloadController.handleRetryDownloadJob}
      onCancelDownloadTask={downloadController.handleCancelDownloadTask}
      onRetryDownloadTask={downloadController.handleRetryDownloadTask}
    />
  {/if}

</div>
