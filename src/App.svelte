<script lang="ts">
  import { tick } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import type { PartialOptions } from 'overlayscrollbars';
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
  } from '$lib/api';
  import {
    clearCache,
    createInventoryCacheTag,
    invalidateByTag,
    warmCacheManager,
  } from '$lib/cache';
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
    LocalInventorySnapshot,
    AppErrorEvent,
    LogLevel,
    LibrarySearchScope,
    SearchLibraryResultItem,
  } from '$lib/types';
  import { applyThemePalette, DEFAULT_THEME_PALETTE } from '$lib/theme';
  import { envStore } from '$lib/features/env/store.svelte';
  import { shellStore } from '$lib/features/shell/store.svelte';
  import { createSettingsController } from '$lib/features/shell/settings.svelte';
  import { createAlbumStageMotionController } from '$lib/features/shell/albumStageMotion.svelte';
  import { createLibraryController } from '$lib/features/library/controller.svelte';
  import { createPlayerController } from '$lib/features/player/controller.svelte';
  import { createDownloadController } from '$lib/features/download/controller.svelte';
  import { preloadImage } from '$lib/features/library/helpers';
  import {
    buildAlbumPlaybackEntries,
    getSelectedAlbumCoverUrl,
  } from '$lib/features/library/selectors';
  import { toast } from 'svelte-sonner';
  import TopToolbar from '$lib/components/app/TopToolbar.svelte';
  import StatusToastHost from '$lib/components/app/StatusToastHost.svelte';
  import AlbumSidebarContainer from '$lib/components/app/AlbumSidebarContainer.svelte';
  import AlbumWorkspace from '$lib/components/app/AlbumWorkspace.svelte';
  import AlbumWorkspaceContent from '$lib/components/app/AlbumWorkspaceContent.svelte';
  import PlayerFlyoutStack from '$lib/components/app/PlayerFlyoutStack.svelte';
  import AppSideSheets from '$lib/components/app/AppSideSheets.svelte';

  // Minimum display time (ms) to prevent animation flash on fast loads
  const MIN_DISPLAY_MS = 260;
  const DETAIL_SKELETON_DELAY_MS = 140;
  const OVERLAY_DURATION = 0.16;
  const SHEET_DURATION = 0.22;

  const delay = (ms: number): Promise<void> =>
    new Promise((resolve) => {
      setTimeout(resolve, ms);
    });

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
    seekCurrentPlayback: async (positionSecs) => {
      await seekCurrentPlayback(positionSecs);
    },
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
      await shellStore.openDownloads({
        notifyError,
        beforeOpen: resetFilters
          ? () => {
              downloadController.resetFilters();
            }
          : undefined,
      });
    },
    getDownloadOptions: () => ({
      outputDir: settingsState.outputDir,
      format: settingsState.format,
      downloadLyrics: settingsState.downloadLyrics,
    }),
    notifyInfo,
    notifyError,
  });

  const settingsController = createSettingsController({
    getPreferences,
    setPreferences,
    notifyError,
  });

  const albumStageMotionController = createAlbumStageMotionController({
    getReducedMotion: () => envStore.prefersReducedMotion,
    getViewportHeight: () => envStore.viewportHeight,
    getLoadingDetail: () => libraryController.loadingDetail,
  });

  let selectedSongCids = $state<string[]>([]);
  let selectionModeEnabled = $state(false);
  let themeRequestSeq = 0;
  let artworkRequestSeq = 0;
  let playerStateInitSeq = 0;
  let playerStateHydratedFromEvent = false;
  const settingsOpen = $derived(shellStore.settingsOpen);
  const downloadPanelOpen = $derived(shellStore.downloadPanelOpen);
  const SettingsSheetView = $derived(shellStore.SettingsSheetView);
  const DownloadTasksSheetView = $derived(shellStore.DownloadTasksSheetView);
  const contentScrollbarEvents = $derived(
    albumStageMotionController.contentScrollbarEvents
  );
  const albumStageStyle = $derived(albumStageMotionController.albumStageStyle);
  const albumStageMediaHeight = $derived(
    albumStageMotionController.albumStageMediaHeight
  );
  const albumStageScrimOpacity = $derived(
    albumStageMotionController.albumStageScrimOpacity
  );
  const albumStageImageOpacity = $derived(
    albumStageMotionController.albumStageImageOpacity
  );
  const albumStageImageTransform = $derived(
    albumStageMotionController.albumStageImageTransform
  );
  const albumStageSolidifyOpacity = $derived(
    albumStageMotionController.albumStageSolidifyOpacity
  );
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
  const librarySearchResponse = $derived(
    libraryController.librarySearchResponse
  );
  const pendingScrollToSongCid = $derived(
    libraryController.pendingScrollToSongCid
  );
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
  const contentEl = $derived(albumStageMotionController.contentElement);
  const isMacOS = $derived(envStore.isMacOS);
  const settingsState = $state({
    format: 'flac' as OutputFormat,
    outputDir: '',
    downloadLyrics: true,
    notifyOnDownloadComplete: true,
    notifyOnPlaybackChange: true,
    logLevel: 'error' as LogLevel,
    settingsLogRefreshToken: 0,
    prefsReady: false,
    isSaving: false,
    persistedSnapshot: '',
    lastSaveFailedSnapshot: '',
    dirty: {
      format: false,
      outputDir: false,
      downloadLyrics: false,
      notifyOnDownloadComplete: false,
      notifyOnPlaybackChange: false,
      logLevel: false,
    },
    suspendDirtyTracking: 0,
  });
  let selectedAlbumArtworkUrl = $state<string | null>(null);
  let albumStageElement = $state<HTMLElement | null>(null);
  const lastObservedSettings = {
    format: settingsState.format,
    outputDir: settingsState.outputDir,
    downloadLyrics: settingsState.downloadLyrics,
    notifyOnDownloadComplete: settingsState.notifyOnDownloadComplete,
    notifyOnPlaybackChange: settingsState.notifyOnPlaybackChange,
    logLevel: settingsState.logLevel,
  };

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

  const overlayScrollbarOptions = $derived.by(
    (): PartialOptions => ({
      scrollbars: {
        theme: 'os-theme-app',
        autoHide: prefersReducedMotion ? 'leave' : 'move',
        autoHideDelay: prefersReducedMotion ? 160 : 720,
        autoHideSuspend: true,
        dragScroll: true,
        clickScroll: false,
      },
    })
  );

  function resetContentScroll() {
    albumStageMotionController.resetContentScroll();
  }

  async function preloadAlbumArtwork(
    album: AlbumDetail
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
    albumStageMotionController.setAspectRatio(value);
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
    const allCids = new Set(selectedAlbum.songs.map((s: SongEntry) => s.cid));
    const currentSelected = new Set(selectedSongCids);
    selectedSongCids = [...allCids].filter((cid) => !currentSelected.has(cid));
  }

  function toggleSelectionMode() {
    selectionModeEnabled = !selectionModeEnabled;
    if (!selectionModeEnabled) {
      clearSongSelection();
    }
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
    const sourceUrl =
      selectedAlbum?.coverDeUrl ?? selectedAlbum?.coverUrl ?? null;
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

  function getSettingsSnapshot() {
    return JSON.stringify({
      format: settingsState.format,
      outputDir: settingsState.outputDir,
      downloadLyrics: settingsState.downloadLyrics,
      notifyOnDownloadComplete: settingsState.notifyOnDownloadComplete,
      notifyOnPlaybackChange: settingsState.notifyOnPlaybackChange,
      logLevel: settingsState.logLevel,
    });
  }

  $effect(() => {
    const value = settingsState.format;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.format = value;
      return;
    }
    if (value !== lastObservedSettings.format) {
      settingsState.dirty.format = true;
      lastObservedSettings.format = value;
    }
  });

  $effect(() => {
    const value = settingsState.outputDir;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.outputDir = value;
      return;
    }
    if (value !== lastObservedSettings.outputDir) {
      settingsState.dirty.outputDir = true;
      lastObservedSettings.outputDir = value;
    }
  });

  $effect(() => {
    const value = settingsState.downloadLyrics;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.downloadLyrics = value;
      return;
    }
    if (value !== lastObservedSettings.downloadLyrics) {
      settingsState.dirty.downloadLyrics = true;
      lastObservedSettings.downloadLyrics = value;
    }
  });

  $effect(() => {
    const value = settingsState.notifyOnDownloadComplete;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.notifyOnDownloadComplete = value;
      return;
    }
    if (value !== lastObservedSettings.notifyOnDownloadComplete) {
      settingsState.dirty.notifyOnDownloadComplete = true;
      lastObservedSettings.notifyOnDownloadComplete = value;
    }
  });

  $effect(() => {
    const value = settingsState.notifyOnPlaybackChange;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.notifyOnPlaybackChange = value;
      return;
    }
    if (value !== lastObservedSettings.notifyOnPlaybackChange) {
      settingsState.dirty.notifyOnPlaybackChange = true;
      lastObservedSettings.notifyOnPlaybackChange = value;
    }
  });

  $effect(() => {
    const value = settingsState.logLevel;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.logLevel = value;
      return;
    }
    if (value !== lastObservedSettings.logLevel) {
      settingsState.dirty.logLevel = true;
      lastObservedSettings.logLevel = value;
    }
  });

  $effect(() => {
    settingsState.persistedSnapshot;
    settingsState.isSaving;
    settingsState.lastSaveFailedSnapshot;
    if (!settingsState.prefsReady || settingsState.isSaving) {
      return;
    }

    const currentSnapshot = getSettingsSnapshot();

    if (currentSnapshot === settingsState.persistedSnapshot) {
      return;
    }

    if (currentSnapshot === settingsState.lastSaveFailedSnapshot) {
      return;
    }

    void settingsController.savePreferences(settingsState);
  });

  $effect(() => {
    albumStageMotionController.albumStageElement = albumStageElement;
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
      await settingsController.hydratePreferences(settingsState, {
        shouldDispose,
      });
    } catch {
      // Preferences hydration failure is already tolerated in controller.
    }

    const defaultDirPromise = settingsState.outputDir
      ? Promise.resolve('')
      : getDefaultOutputDir().catch(() => '');

    try {
      const albumList = await libraryController.loadAlbums({ shouldDispose });

      const defaultDir = await defaultDirPromise;
      if (shouldDispose()) {
        return;
      }
      if (defaultDir) {
        settingsController.applyDefaultOutputDir(settingsState, defaultDir);
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
      if (defaultDir) {
        settingsController.applyDefaultOutputDir(settingsState, defaultDir);
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
      handler: (event: { payload: T }) => void | Promise<void>
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
      if (
        !(await register<PlayerState>('player-state-changed', (event) => {
          playerStateHydratedFromEvent = true;
          playerController.syncPlayerState(event.payload);
        }))
      ) {
        return cleanup;
      }

      if (
        !(await register<PlayerState>('player-progress', (event) => {
          playerController.syncPlayerProgress(event.payload);
        }))
      ) {
        return cleanup;
      }

      if (
        !(await register<DownloadManagerSnapshot>(
          'download-manager-state-changed',
          (event) => {
            downloadController.applyManagerEvent(event.payload);
          }
        ))
      ) {
        return cleanup;
      }

      if (
        !(await register<DownloadJobSnapshot>(
          'download-job-updated',
          (event) => {
            downloadController.applyJobUpdate(event.payload);
          }
        ))
      ) {
        return cleanup;
      }

      if (
        !(await register<DownloadTaskProgressEvent>(
          'download-task-progress',
          (event) => {
            downloadController.applyTaskProgress(event.payload);
          }
        ))
      ) {
        return cleanup;
      }

      if (
        !(await register<AppErrorEvent>('app-error-recorded', (event) => {
          handleAppErrorEvent(event.payload);
        }))
      ) {
        return cleanup;
      }

      if (
        !(await register<LocalInventorySnapshot>(
          'local-inventory-state-changed',
          async (event) => {
            await libraryController.handleInventoryStateChanged(event.payload, {
              shouldDispose,
              invalidateInventoryCaches,
              onSelectionInvalidated: () => {
                clearSongSelection();
                selectionModeEnabled = false;
              },
            });
          }
        ))
      ) {
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
    albumStageMotionController.dispose();
    playerStateInitSeq += 1;
    playerStateHydratedFromEvent = false;
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
          `初始化应用失败：${error instanceof Error ? error.message : String(error)}`
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
        `[data-song-cid="${CSS.escape(expectedSongCid)}"]`
      );
      if (!row) {
        return;
      }

      row.scrollIntoView({
        behavior: prefersReducedMotion ? 'auto' : 'smooth',
        block: 'center',
      });
      libraryController.clearPendingScrollToSong(expectedSongCid);
    });
  });

  async function handleSelectSearchResult(item: SearchLibraryResultItem) {
    const album = albums.find((candidate) => candidate.cid === item.albumCid);
    if (!album) {
      notifyError('未找到对应专辑，可能需要先刷新列表。');
      return;
    }

    libraryController.setPendingScrollToSong(
      item.kind === 'song' ? item.songCid : null
    );
    clearSongSelection();
    selectionModeEnabled = false;
    await libraryController.selectAlbum(album, {
      afterSelect: async () => {
        await tick();
        resetContentScroll();
      },
    });
  }

  function handleContentWheel(event: WheelEvent) {
    albumStageMotionController.handleContentWheel(event);
  }

  function handleAppErrorEvent(event: AppErrorEvent) {
    notifyError(event.message);
    settingsController.handleAppError(settingsState, settingsOpen);
  }

  async function invalidateInventoryCaches(
    inventoryVersion: string | null | undefined
  ) {
    await invalidateByTag(createInventoryCacheTag(inventoryVersion));
  }

  function notifyInfo(message: string) {
    toast(message);
  }

  function notifyError(message: string) {
    toast.error(message);
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

    await playerController.playQueueEntry(
      nextOrder[nextIndex],
      nextOrder,
      nextIndex
    );
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
      notifyError(
        `刷新专辑列表失败：${e instanceof Error ? e.message : String(e)}`
      );
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
  <AlbumSidebarContainer
    {isMacOS}
    {albums}
    {selectedAlbumCid}
    reducedMotion={prefersReducedMotion}
    {loadingAlbums}
    {errorMsg}
    searchQuery={librarySearchQuery}
    searchScope={librarySearchScope}
    searchLoading={librarySearchLoading}
    searchResponse={librarySearchResponse}
    {overlayScrollbarOptions}
    onSearchQueryChange={libraryController.setSearchQuery}
    onSearchScopeChange={libraryController.setSearchScope}
    onSelect={(album: Album) => {
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
        await shellStore.toggleDownloads({ notifyError });
      }}
      onOpenSettings={async () => {
        await shellStore.toggleSettings({ notifyError });
      }}
    />

    <!-- 歌曲列表内容区 -->
    <AlbumWorkspace {currentSong} {loadingDetail} {selectedAlbum}>
      {#snippet children()}
        <AlbumWorkspaceContent
          {loadingDetail}
          {showDetailSkeleton}
          {albumRequestSeq}
          {selectedAlbum}
          {selectedAlbumArtworkUrl}
          currentSongCid={currentSong?.cid ?? null}
          isPlaybackActive={isPlaying || isPaused}
          {downloadingAlbumCid}
          {selectionModeEnabled}
          {selectedSongCids}
          reducedMotion={prefersReducedMotion}
          {overlayScrollbarOptions}
          {contentScrollbarEvents}
          onContentWheel={handleContentWheel}
          {albumStageStyle}
          {albumStageMediaHeight}
          {albumStageScrimOpacity}
          {albumStageImageOpacity}
          {albumStageImageTransform}
          {albumStageSolidifyOpacity}
          bind:albumStageElement
          onToggleSelectionMode={toggleSelectionMode}
          onSelectAllSongs={selectAllSongs}
          onDeselectAllSongs={deselectAllSongs}
          onInvertSongSelection={invertSongSelection}
          onDownloadAlbum={downloadController.handleAlbumDownload}
          onDownloadSelection={(songCids: string[]) =>
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
          hasAlbumDownloadJob={(albumCid: string) =>
            !!downloadController.findAlbumDownloadJob(albumCid)}
          isSelectionDownloadDisabled={downloadController.isSelectionDownloadActionDisabled}
          isCurrentSelectionCreating={downloadController.isCurrentSelectionCreating}
          hasCurrentSelectionJob={(songCids: string[]) =>
            !!downloadController.getCurrentSelectionJob(songCids)}
        />
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
        : 'idle'}
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

    <AppSideSheets
      {SettingsSheetView}
      {DownloadTasksSheetView}
      bind:settingsOpen={shellStore.settingsOpen}
      bind:downloadPanelOpen={shellStore.downloadPanelOpen}
      bind:format={settingsState.format}
      bind:outputDir={settingsState.outputDir}
      bind:downloadLyrics={settingsState.downloadLyrics}
      bind:notifyOnDownloadComplete={settingsState.notifyOnDownloadComplete}
      bind:notifyOnPlaybackChange={settingsState.notifyOnPlaybackChange}
      bind:logLevel={settingsState.logLevel}
      settingsLogRefreshToken={settingsState.settingsLogRefreshToken}
      {notifyInfo}
      {notifyError}
      onOutputDirChange={() =>
        settingsController.savePreferences(settingsState)}
      jobs={filteredDownloadJobs}
      {hasDownloadHistory}
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
  </section>
</div>
