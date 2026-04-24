<script lang="ts">
  import { motion } from "@humanspeak/svelte-motion";
  import SongRow from "$lib/components/SongRow.svelte";
  import { getDownloadBadgeLabel, shouldShowDownloadBadge } from "$lib/downloadBadge";
  import type { AlbumDetail, SongEntry } from "$lib/types";

  type SongDownloadState = "idle" | "creating" | "queued" | "running";
  type MotionTarget = Record<string, string | number>;

  interface Props {
    album: AlbumDetail;
    currentSongCid: string | null;
    isPlaybackActive: boolean;
    downloadingAlbumCid: string | null;
    selectionModeEnabled: boolean;
    selectedSongCids: string[];
    reducedMotion: boolean;
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

  const HERO_DURATION = 0.22;
  const HERO_DELAY = 0.03;
  const LIST_DURATION = 0.2;
  const LIST_DELAY = 0.07;

  let props: Props = $props();

  const interactiveTransition = $derived.by(
    () =>
      ({
        duration: props.reducedMotion ? 0 : 0.16,
        ease: "easeOut",
      }) as const,
  );

  const selectedSongCount = $derived.by(() => props.selectedSongCids.length);
  const selectedSongsLabel = $derived.by(() => {
    if (selectedSongCount === 0) return "未选择歌曲";
    if (selectedSongCount === 1) return "已选择 1 首";
    return `已选择 ${selectedSongCount} 首`;
  });
  const isAlbumDownloadCreating = $derived.by(
    () => props.downloadingAlbumCid === props.album.cid,
  );
  const hasAlbumDownloadJob = $derived.by(() => props.hasAlbumDownloadJob(props.album.cid));
  const isAlbumDownloadDisabled = $derived.by(
    () => isAlbumDownloadCreating || hasAlbumDownloadJob,
  );
  const isAllSongsSelected = $derived.by(
    () => selectedSongCount === props.album.songs.length,
  );
  const canInvertSelection = $derived.by(() => props.album.songs.length > 0);
  const isSelectionCreating = $derived.by(
    () => props.isCurrentSelectionCreating(props.selectedSongCids),
  );
  const hasCurrentSelectionJob = $derived.by(
    () => props.hasCurrentSelectionJob(props.selectedSongCids),
  );
  const isSelectionDownloadDisabled = $derived.by(
    () => props.isSelectionDownloadDisabled(props.selectedSongCids),
  );

  function motionTransition(duration: number, delay = 0): any {
    const transition: any = {
      duration: props.reducedMotion ? 0 : duration,
      delay: props.reducedMotion ? 0 : delay,
      ease: "easeOut" as const,
    };

    return transition;
  }

  function fadeExit(opacity = 0): MotionTarget {
    return { opacity };
  }

  function axisEnter(axis: "x" | "y", offset: number): MotionTarget {
    return props.reducedMotion ? { opacity: 1 } : { opacity: 0, [axis]: offset };
  }

  function axisAnimate(axis: "x" | "y"): MotionTarget {
    return { opacity: 1, [axis]: 0 };
  }

  function axisExit(axis: "x" | "y", offset: number): MotionTarget {
    return props.reducedMotion ? { opacity: 0 } : { opacity: 0, [axis]: offset };
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
          ...(props.reducedMotion ? {} : { y: -1 }),
        }
      : {
          backgroundColor: "var(--hover-bg-elevated)",
          boxShadow: "0 8px 20px rgba(15, 23, 42, 0.08)",
          ...(props.reducedMotion ? {} : { y: -1 }),
        };
  }
</script>

<motion.div
  class="album-detail-card"
  initial={{ opacity: 0 }}
  animate={{ opacity: 1 }}
  exit={fadeExit()}
  transition={motionTransition(HERO_DURATION)}
>
  <div class="album-hero">
    <motion.div
      class="album-hero-info"
      initial={axisEnter("y", 14)}
      animate={axisAnimate("y")}
      exit={axisExit("y", 8)}
      transition={motionTransition(HERO_DURATION, HERO_DELAY)}
    >
      {#if props.album.belong}
        <span class="album-belong-tag">{props.album.belong.toUpperCase()}</span>
      {/if}
      <h1 class="album-hero-title">{props.album.name}</h1>
      {#if props.album.artists && props.album.artists.length > 0}
        <p class="album-hero-artists">{props.album.artists.join(", ")}</p>
      {/if}
      {#if props.album.intro}
        <p class="album-hero-intro">{props.album.intro}</p>
      {/if}
      <div class="album-hero-meta">
        <span class="album-song-count">{props.album.songs.length} 首歌曲</span>
        {#if shouldShowDownloadBadge(props.album.download.downloadStatus)}
          <span class="album-download-status-badge">
            {getDownloadBadgeLabel(props.album.download.downloadStatus)}
          </span>
        {/if}
      </div>
      <div class="controls album-hero-actions">
        <motion.button
          class="btn btn-primary"
          onclick={() => props.onDownloadAlbum(props.album.cid)}
          disabled={isAlbumDownloadDisabled}
          animate={appButtonAnimate(true, isAlbumDownloadDisabled)}
          whileHover={appButtonHover(true, isAlbumDownloadDisabled)}
          whileTap={!props.reducedMotion && !isAlbumDownloadDisabled
            ? { y: 0, scale: 0.98, opacity: 0.94 }
            : undefined}
          transition={interactiveTransition}
        >
          {#if isAlbumDownloadCreating}
            正在创建任务...
          {:else if hasAlbumDownloadJob}
            已在队列中
          {:else}
            下载整张专辑
          {/if}
        </motion.button>
        <motion.button
          class="btn"
          onclick={props.onToggleSelectionMode}
          animate={appButtonAnimate(false, false)}
          whileHover={appButtonHover(false, false)}
          whileTap={props.reducedMotion
            ? undefined
            : { y: 0, scale: 0.98, opacity: 0.94 }}
          transition={interactiveTransition}
        >
          {props.selectionModeEnabled ? "取消多选" : "多选下载"}
        </motion.button>
        {#if props.selectionModeEnabled}
          <motion.button
            class="btn"
            onclick={props.onSelectAllSongs}
            disabled={isAllSongsSelected}
            animate={appButtonAnimate(false, isAllSongsSelected)}
            whileHover={appButtonHover(false, isAllSongsSelected)}
            whileTap={!props.reducedMotion && !isAllSongsSelected
              ? { y: 0, scale: 0.98, opacity: 0.94 }
              : undefined}
            transition={interactiveTransition}
          >
            全选
          </motion.button>
          <motion.button
            class="btn"
            onclick={props.onDeselectAllSongs}
            disabled={selectedSongCount === 0}
            animate={appButtonAnimate(false, selectedSongCount === 0)}
            whileHover={appButtonHover(false, selectedSongCount === 0)}
            whileTap={!props.reducedMotion && selectedSongCount > 0
              ? { y: 0, scale: 0.98, opacity: 0.94 }
              : undefined}
            transition={interactiveTransition}
          >
            清空
          </motion.button>
          <motion.button
            class="btn"
            onclick={props.onInvertSongSelection}
            disabled={!canInvertSelection}
            animate={appButtonAnimate(false, !canInvertSelection)}
            whileHover={appButtonHover(false, !canInvertSelection)}
            whileTap={!props.reducedMotion && canInvertSelection
              ? { y: 0, scale: 0.98, opacity: 0.94 }
              : undefined}
            transition={interactiveTransition}
          >
            反选
          </motion.button>
          <motion.button
            class="btn btn-primary"
            onclick={() => props.onDownloadSelection(props.selectedSongCids)}
            disabled={isSelectionDownloadDisabled}
            animate={appButtonAnimate(true, isSelectionDownloadDisabled)}
            whileHover={appButtonHover(true, isSelectionDownloadDisabled)}
            whileTap={!props.reducedMotion && !isSelectionDownloadDisabled
              ? { y: 0, scale: 0.98, opacity: 0.94 }
              : undefined}
            transition={interactiveTransition}
          >
            {#if isSelectionCreating}
              正在创建批量任务...
            {:else if hasCurrentSelectionJob}
              已在队列中
            {:else}
              下载所选歌曲
            {/if}
          </motion.button>
          <span class="album-selection-summary">{selectedSongsLabel}</span>
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
    {#each props.album.songs as song, index (song.cid)}
      <SongRow
        {song}
        {index}
        isPlaying={props.currentSongCid === song.cid && props.isPlaybackActive}
        downloadState={props.getSongDownloadState(song.cid)}
        downloadDisabled={props.isSongDownloadInteractionBlocked(song.cid)}
        selectionMode={props.selectionModeEnabled}
        isSelected={props.isSongSelected(song.cid)}
        selectionDisabled={isSelectionCreating}
        reducedMotion={props.reducedMotion}
        onclick={() => props.onPlaySong(song)}
        onDownload={() => props.onDownloadSong(song.cid)}
        onToggleSelection={() => props.onToggleSongSelection(song.cid)}
      />
    {/each}
  </motion.div>
</motion.div>
