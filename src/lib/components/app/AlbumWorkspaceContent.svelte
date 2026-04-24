<script lang="ts">
  import { AnimatePresence, motion } from '@humanspeak/svelte-motion';
  import type { MotionTransition } from '@humanspeak/svelte-motion';
  import { OverlayScrollbarsComponent } from 'overlayscrollbars-svelte';
  import type { EventListeners, PartialOptions } from 'overlayscrollbars';
  import type { AlbumDetail, SongEntry } from '$lib/types';
  import MotionSpinner from '$lib/components/MotionSpinner.svelte';
  import AlbumStage from '$lib/components/app/AlbumStage.svelte';
  import AlbumDetailSkeleton from '$lib/components/app/AlbumDetailSkeleton.svelte';
  import AlbumDetailPanel from '$lib/components/app/AlbumDetailPanel.svelte';

  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';
  type MotionTarget = Record<string, string | number>;

  interface Props {
    loadingDetail: boolean;
    showDetailSkeleton: boolean;
    albumRequestSeq: number;
    selectedAlbum: AlbumDetail | null;
    selectedAlbumArtworkUrl: string | null;
    currentSongCid: string | null;
    isPlaybackActive: boolean;
    downloadingAlbumCid: string | null;
    selectionModeEnabled: boolean;
    selectedSongCids: string[];
    reducedMotion: boolean;
    overlayScrollbarOptions: PartialOptions;
    contentScrollbarEvents: EventListeners;
    onContentWheel: (event: WheelEvent) => void;
    albumStageStyle: string;
    albumStageMediaHeight: string;
    albumStageScrimOpacity: number;
    albumStageImageOpacity: number;
    albumStageImageTransform: string;
    albumStageSolidifyOpacity: number;
    albumStageElement?: HTMLElement | null;
    onToggleSelectionMode: () => void;
    onSelectAllSongs: () => void;
    onDeselectAllSongs: () => void;
    onInvertSongSelection: () => void;
    onDownloadAlbum: (albumCid: string) => void | Promise<void>;
    onDownloadSelection: (songCids: string[]) => void | Promise<void>;
    onPlaySong: (song: SongEntry) => void | Promise<void>;
    onDownloadSong: (songCid: string) => void | Promise<void>;
    onToggleSongSelection: (songCid: string) => void;
    isSongSelected: (songCid: string) => boolean;
    getSongDownloadState: (songCid: string) => SongDownloadState;
    isSongDownloadInteractionBlocked: (songCid: string) => boolean;
    hasAlbumDownloadJob: (albumCid: string) => boolean;
    isSelectionDownloadDisabled: (songCids: string[]) => boolean;
    isCurrentSelectionCreating: (songCids: string[]) => boolean;
    hasCurrentSelectionJob: (songCids: string[]) => boolean;
  }

  const PANEL_DURATION = 0.18;
  const CONTENT_MASK_DURATION = 0.14;

  let {
    loadingDetail,
    showDetailSkeleton,
    albumRequestSeq,
    selectedAlbum,
    selectedAlbumArtworkUrl,
    currentSongCid,
    isPlaybackActive,
    downloadingAlbumCid,
    selectionModeEnabled,
    selectedSongCids,
    reducedMotion,
    overlayScrollbarOptions,
    contentScrollbarEvents,
    onContentWheel,
    albumStageStyle,
    albumStageMediaHeight,
    albumStageScrimOpacity,
    albumStageImageOpacity,
    albumStageImageTransform,
    albumStageSolidifyOpacity,
    albumStageElement = $bindable<HTMLElement | null>(null),
    onToggleSelectionMode,
    onSelectAllSongs,
    onDeselectAllSongs,
    onInvertSongSelection,
    onDownloadAlbum,
    onDownloadSelection,
    onPlaySong,
    onDownloadSong,
    onToggleSongSelection,
    isSongSelected,
    getSongDownloadState,
    isSongDownloadInteractionBlocked,
    hasAlbumDownloadJob,
    isSelectionDownloadDisabled,
    isCurrentSelectionCreating,
    hasCurrentSelectionJob,
  }: Props = $props();

  function motionTransition(
    duration: number,
    delay = 0
  ): MotionTransition {
    return {
      duration: reducedMotion ? 0 : duration,
      delay: reducedMotion ? 0 : delay,
      ease: 'easeOut',
    };
  }

  function fadeEnter(opacity = 0): MotionTarget {
    return reducedMotion ? { opacity: 1 } : { opacity };
  }

  function fadeExit(opacity = 0): MotionTarget {
    return { opacity };
  }
</script>

<OverlayScrollbarsComponent
  element="div"
  class="h-full"
  data-overlayscrollbars-initialize
  options={overlayScrollbarOptions}
  events={contentScrollbarEvents}
  defer
  onwheel={onContentWheel}
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
          {reducedMotion}
          stageStyle={albumStageStyle}
          mediaHeight={albumStageMediaHeight}
          scrimOpacity={albumStageScrimOpacity}
          bind:element={albumStageElement}
        />
        <AlbumDetailSkeleton {reducedMotion} />
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
          {reducedMotion}
          stageStyle={albumStageStyle}
          mediaHeight={albumStageMediaHeight}
          scrimOpacity={albumStageScrimOpacity}
          imageOpacity={albumStageImageOpacity}
          imageTransform={albumStageImageTransform}
          solidifyOpacity={albumStageSolidifyOpacity}
          bind:element={albumStageElement}
        />
        <AlbumDetailPanel
          album={selectedAlbum}
          {currentSongCid}
          {isPlaybackActive}
          {downloadingAlbumCid}
          {selectionModeEnabled}
          {selectedSongCids}
          {reducedMotion}
          {onToggleSelectionMode}
          {onSelectAllSongs}
          {onDeselectAllSongs}
          {onInvertSongSelection}
          {onDownloadAlbum}
          {onDownloadSelection}
          {onPlaySong}
          {onDownloadSong}
          {onToggleSongSelection}
          {isSongSelected}
          {getSongDownloadState}
          {isSongDownloadInteractionBlocked}
          {hasAlbumDownloadJob}
          {isSelectionDownloadDisabled}
          {isCurrentSelectionCreating}
          {hasCurrentSelectionJob}
        />
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
          {reducedMotion}
        />
      </motion.div>
    {/if}
  </AnimatePresence>
</OverlayScrollbarsComponent>
