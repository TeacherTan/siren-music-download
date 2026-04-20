<script lang="ts">
  import { motion } from '@humanspeak/svelte-motion';
  import type { SongEntry } from '$lib/types';

  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';

  interface Props {
    song: SongEntry;
    index: number;
    isPlaying?: boolean;
    downloadState?: SongDownloadState;
    downloadDisabled?: boolean;
    selectionMode?: boolean;
    isSelected?: boolean;
    selectionDisabled?: boolean;
    reducedMotion?: boolean;
    onclick?: () => void;
    onDownload?: () => void;
    onToggleSelection?: () => void;
  }

  let {
    song,
    index,
    isPlaying = false,
    downloadState = 'idle',
    downloadDisabled = false,
    selectionMode = false,
    isSelected = false,
    selectionDisabled = false,
    reducedMotion = false,
    onclick,
    onDownload,
    onToggleSelection,
  }: Props = $props();

  let isHovered = $state(false);
  let isFocused = $state(false);

  const showEmphasis = $derived.by(() => isPlaying || isHovered || isFocused || isSelected);
  const showPlayIndicator = $derived.by(() => isPlaying || isHovered || isFocused);
  const showDownloadedBadge = $derived.by(() => song.download?.isDownloaded ?? false);
  const downloadedBadgeLabel = $derived.by(() => {
    switch (song.download?.downloadStatus) {
      case 'verified':
        return '已校验';
      case 'unverifiable':
        return '已下载';
      case 'partial':
        return '部分下载';
      default:
        return '已下载';
    }
  });
  const isBusy = $derived.by(() => downloadState !== 'idle');
  const isDownloadDisabled = $derived.by(() => isBusy || downloadDisabled || selectionMode);
  const downloadButtonLabel = $derived.by(() => {
    switch (downloadState) {
      case 'creating':
        return `正在创建 ${song.name} 的下载任务`;
      case 'queued':
        return `${song.name} 已在下载队列中`;
      case 'running':
        return `${song.name} 正在下载中`;
      default:
        return `下载 ${song.name}`;
    }
  });
  const downloadButtonTitle = $derived.by(() => {
    switch (downloadState) {
      case 'creating':
        return '正在创建任务...';
      case 'queued':
        return '已在队列中';
      case 'running':
        return '下载中';
      default:
        return '下载';
    }
  });
  const rowSurface = $derived.by(() => {
    if (isSelected) {
      return {
        backgroundColor: 'rgba(var(--accent-rgb), 0.12)',
        boxShadow: 'inset 0 0 0 1px rgba(var(--accent-rgb), 0.12)',
      };
    }

    if (isPlaying) {
      return {
        backgroundColor: 'rgba(var(--accent-rgb), 0.1)',
        boxShadow: 'inset 0 0 0 1px rgba(var(--accent-rgb), 0.08)',
      };
    }

    if (isHovered || isFocused) {
      return {
        backgroundColor: 'rgba(15, 23, 42, 0.04)',
        boxShadow: 'inset 0 0 0 1px rgba(var(--accent-rgb), 0)',
      };
    }

    return {
      backgroundColor: 'rgba(15, 23, 42, 0)',
      boxShadow: 'inset 0 0 0 1px rgba(var(--accent-rgb), 0)',
    };
  });

  const indicatorState = $derived.by(() => {
    if (isPlaying) {
      return {
        opacity: 1,
        scale: 1,
        backgroundColor: 'var(--accent)',
        color: '#ffffff',
        boxShadow: '0 10px 20px rgba(var(--accent-rgb), 0.18)',
      };
    }

    return {
      opacity: showPlayIndicator ? 1 : 0,
      scale: reducedMotion ? 1 : showPlayIndicator ? 1 : 0.92,
      backgroundColor: showPlayIndicator ? 'rgba(var(--accent-rgb), 0.1)' : 'rgba(15, 23, 42, 0.05)',
      color: showPlayIndicator ? 'var(--accent)' : 'var(--text-secondary)',
      boxShadow: '0 0 0 rgba(var(--accent-rgb), 0)',
    };
  });

  const motionTransition = $derived.by(() => ({
    duration: reducedMotion ? 0 : 0.16,
    ease: 'easeOut',
  } as const));

  function handleRowActivate() {
    if (selectionMode) {
      if (!selectionDisabled) {
        onToggleSelection?.();
      }
      return;
    }

    onclick?.();
  }
</script>

<motion.div
  class={`song-row${selectionMode ? ' is-selection-mode' : ''}${isSelected ? ' is-selected' : ''}`}
  role="button"
  tabindex="0"
  animate={rowSurface}
  whileTap={reducedMotion ? undefined : { scale: 0.996 }}
  transition={motionTransition}
  onclick={handleRowActivate}
  onmouseenter={() => { isHovered = true; }}
  onmouseleave={() => { isHovered = false; }}
  onfocusin={() => { isFocused = true; }}
  onfocusout={() => { isFocused = false; }}
  onkeydown={(e: KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handleRowActivate();
    }
  }}
>
  {#if selectionMode}
    <button
      type="button"
      class="song-selection-toggle"
      class:is-selected={isSelected}
      disabled={selectionDisabled}
      aria-label={isSelected ? `取消选择 ${song.name}` : `选择 ${song.name}`}
      aria-pressed={isSelected}
      onclick={(event: MouseEvent) => {
        event.stopPropagation();
        onToggleSelection?.();
      }}
    >
      <span class="song-selection-dot"></span>
    </button>
  {/if}
  <motion.div
    class="song-number"
    animate={{
      color: showEmphasis ? 'var(--accent)' : 'var(--text-tertiary)',
      opacity: showEmphasis ? 0.86 : 1,
    }}
    transition={motionTransition}
  >
    {index + 1}
  </motion.div>
  <div class="song-info">
    <motion.div
      class="song-name"
      animate={{ color: showEmphasis ? 'var(--accent)' : 'var(--text-primary)' }}
      transition={motionTransition}
    >
      {song.name}
    </motion.div>
    <motion.div
      class="song-artists"
      animate={{
        color: 'var(--text-secondary)',
        opacity: showEmphasis ? 0.92 : 1,
      }}
      transition={motionTransition}
    >
      {(song.artists || []).join(', ')}
    </motion.div>
    {#if showDownloadedBadge}
      <span class="song-download-badge">{downloadedBadgeLabel}</span>
    {/if}
  </div>
  <div class="song-actions">
    <motion.div
      class="play-indicator"
      animate={indicatorState}
      transition={motionTransition}
    >
      {#if isPlaying}
        <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor">
          <rect x="2" y="2" width="4" height="10" rx="1" />
          <rect x="8" y="2" width="4" height="10" rx="1" />
        </svg>
      {:else}
        <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor">
          <path d="M4 2.5l7 4.5-7 4.5V2.5z" />
        </svg>
      {/if}
    </motion.div>
    <motion.button
      type="button"
      class="download-button"
      aria-label={downloadButtonLabel}
      title={downloadButtonTitle}
      disabled={!onDownload || isDownloadDisabled}
      animate={{
        opacity: isDownloadDisabled ? 0.52 : isBusy ? 0.78 : 1,
        scale: 1,
        backgroundColor: isBusy ? 'rgba(var(--accent-rgb), 0.12)' : showEmphasis ? 'rgba(var(--accent-rgb), 0.08)' : 'rgba(15, 23, 42, 0.04)',
        color: isBusy ? 'var(--accent)' : showEmphasis ? 'var(--accent)' : 'var(--text-secondary)',
      }}
      transition={motionTransition}
      onclick={(event: MouseEvent) => {
        event.stopPropagation();
        onDownload?.();
      }}
    >
      {#if downloadState === 'creating'}
        <motion.svg
          width="15"
          height="15"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.1"
          stroke-linecap="round"
          stroke-linejoin="round"
          animate={reducedMotion ? undefined : { rotate: 360 }}
          transition={{ duration: 0.9, ease: 'linear', repeat: reducedMotion ? 0 : Infinity }}
        >
          <path d="M21 12a9 9 0 1 1-2.64-6.36"></path>
          <path d="M21 3v6h-6"></path>
        </motion.svg>
      {:else if downloadState === 'queued'}
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M12 7v5"></path>
          <path d="m9.5 10.5 2.5 2.5 2.5-2.5"></path>
          <path d="M5 18h14"></path>
          <path d="M8 4.5h8"></path>
        </svg>
      {:else if downloadState === 'running'}
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M12 5v9"></path>
          <path d="m8.5 10.5 3.5 3.5 3.5-3.5"></path>
          <path d="M5 18h14"></path>
        </svg>
      {:else}
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
          <polyline points="7 10 12 15 17 10"></polyline>
          <line x1="12" y1="15" x2="12" y2="3"></line>
        </svg>
      {/if}
    </motion.button>
  </div>
</motion.div>

<style>
  :global(.song-row) {
    display: flex;
    align-items: center;
    padding: 14px 10px;
    margin: 0;
    border-radius: 18px;
    gap: 18px;
    cursor: pointer;
    user-select: none;
    outline: none;
    box-shadow: inset 0 0 0 1px transparent;
    background: transparent;
  }

  :global(.song-row.is-selection-mode) {
    gap: 14px;
  }

  :global(.song-row:focus-visible) {
    box-shadow:
      inset 0 0 0 1px rgba(var(--accent-rgb), 0.16),
      0 0 0 4px rgba(var(--accent-rgb), 0.08);
  }

  :global(.song-number) {
    width: 28px;
    font-size: 15px;
    font-variant-numeric: tabular-nums;
    text-align: center;
    color: var(--text-tertiary);
  }

  .song-info {
    flex: 1;
    min-width: 0;
  }

  .song-download-badge {
    display: inline-flex;
    align-items: center;
    margin-top: 6px;
    padding: 4px 8px;
    border-radius: 999px;
    font-size: 11px;
    line-height: 1;
    color: var(--accent);
    background: rgba(var(--accent-rgb), 0.1);
    border: 1px solid rgba(var(--accent-rgb), 0.12);
  }

  .song-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .song-selection-toggle {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 1px solid rgba(var(--accent-rgb), 0.16);
    background: rgba(15, 23, 42, 0.03);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.16s ease, border-color 0.16s ease, opacity 0.16s ease;
  }

  .song-selection-toggle.is-selected {
    background: rgba(var(--accent-rgb), 0.12);
    border-color: rgba(var(--accent-rgb), 0.3);
  }

  .song-selection-toggle:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .song-selection-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: transparent;
    box-shadow: inset 0 0 0 1.5px rgba(var(--accent-rgb), 0.4);
    transition: background 0.16s ease, box-shadow 0.16s ease, transform 0.16s ease;
  }

  .song-selection-toggle.is-selected .song-selection-dot {
    background: var(--accent);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.24);
    transform: scale(1.02);
  }

  :global(.song-name) {
    margin-bottom: 4px;
    font-size: 16px;
    font-weight: 600;
    letter-spacing: -0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-primary);
  }

  :global(.song-artists) {
    font-size: 13px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  :global(.play-indicator) {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(15, 23, 42, 0.05);
    color: var(--text-secondary);
    box-shadow: 0 0 0 rgba(var(--accent-rgb), 0);
    flex-shrink: 0;
  }

  :global(.download-button) {
    width: 34px;
    height: 34px;
    border: 0;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(15, 23, 42, 0.04);
    color: var(--text-secondary);
    cursor: pointer;
    flex-shrink: 0;
  }

  :global(.download-button:disabled) {
    cursor: not-allowed;
  }

  @media (max-width: 560px) {
    :global(.song-row) {
      padding: 12px 6px;
      gap: 14px;
    }

    :global(.song-number) {
      width: 22px;
      font-size: 14px;
    }

    :global(.song-name) {
      font-size: 15px;
    }
  }
</style>
