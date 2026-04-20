<script lang="ts">
  import { motion } from '@humanspeak/svelte-motion';
  import { motionStyles } from '$lib/actions/motionStyles';
  import type { Album } from '$lib/types';
  import { lazyLoad } from '$lib/lazyLoad';

  interface Props {
    album: Album;
    selected?: boolean;
    reducedMotion?: boolean;
    onclick?: () => void;
  }

  let { album, selected = false, reducedMotion = false, onclick }: Props = $props();

  let isHovered = $state(false);
  let isFocused = $state(false);

  const motionTransition = $derived.by(() => ({
    duration: reducedMotion ? 0 : 0.16,
    ease: 'easeOut',
  } as const));

  const showCoverLift = $derived.by(() => isHovered || isFocused);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<motion.div
  class={`album-card${selected ? ' selected' : ''}`}
  role="button"
  tabindex="0"
  animate={selected
    ? {
        backgroundColor: 'var(--accent-light)',
        boxShadow: 'inset 0 0 0 1px rgba(var(--accent-rgb), 0.12)',
        y: 0,
      }
    : {
        backgroundColor: 'rgba(255, 255, 255, 0)',
        boxShadow: 'inset 0 0 0 1px rgba(var(--accent-rgb), 0)',
        y: 0,
      }}
  whileHover={selected
    ? (reducedMotion ? {} : { y: -1 })
    : {
        backgroundColor: 'var(--hover-bg-elevated)',
        boxShadow: '0 2px 8px rgba(15, 23, 42, 0.05)',
        ...(reducedMotion ? {} : { y: -1 }),
      }}
  whileFocus={selected
    ? (reducedMotion ? {} : { y: -1 })
    : {
        backgroundColor: 'var(--hover-bg-elevated)',
        boxShadow: '0 2px 8px rgba(15, 23, 42, 0.05)',
        ...(reducedMotion ? {} : { y: -1 }),
      }}
  whileTap={reducedMotion ? undefined : { scale: 0.99, y: 0 }}
  transition={motionTransition}
  onclick={onclick}
  onmouseenter={() => { isHovered = true; }}
  onmouseleave={() => { isHovered = false; }}
  onfocusin={() => { isFocused = true; }}
  onfocusout={() => { isFocused = false; }}
>
  <div
    class="album-cover-wrapper"
    use:lazyLoad={{ rootMargin: '150px', reducedMotion }}
    use:motionStyles={{
      animate: {
        boxShadow: showCoverLift ? '0 8px 18px rgba(var(--accent-rgb), 0.16)' : '0 0 0 rgba(var(--accent-rgb), 0)',
      },
      transition: motionTransition,
      reducedMotion,
    }}
    data-src={album.coverUrl}
  >
    <div class="album-cover-placeholder">♪</div>
    <img class="album-cover-img" alt={album.name} />
  </div>
  <div class="album-info">
    <div class="album-name-row">
      <div class="album-name">{album.name}</div>
      {#if album.download.hasDownloadedTracks}
        <span class="album-download-badge">
          {album.download.downloadedTrackCount > 0
            ? `${album.download.downloadedTrackCount} 首已下载`
            : "已下载"}
        </span>
      {/if}
    </div>
    <div class="album-artists">{(album.artists || []).join(', ')}</div>
  </div>
</motion.div>

<style>
  :global(.album-card) {
    background: transparent;
    border-radius: 12px;
    padding: 12px;
    margin-bottom: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 12px;
    outline: none;
    box-shadow: inset 0 0 0 1px transparent;
  }

  :global(.album-card:focus-visible) {
    box-shadow:
      inset 0 0 0 1px rgba(var(--accent-rgb), 0.18),
      0 0 0 4px rgba(var(--accent-rgb), 0.08);
  }

  :global(.album-card.selected) .album-name {
    color: var(--accent);
  }

  .album-cover-wrapper {
    width: 48px;
    height: 48px;
    border-radius: 8px;
    background: linear-gradient(135deg, var(--bg-tertiary) 0%, var(--bg-secondary) 100%);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    overflow: hidden;
    box-shadow: 0 0 0 rgba(var(--accent-rgb), 0);
  }

  .album-cover-placeholder {
    color: var(--text-tertiary);
    font-size: 20px;
    opacity: 1;
  }

  .album-cover-img {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center;
    border-radius: 8px;
    opacity: 0;
    transform: scale(1.04);
  }

  .album-info {
    flex: 1;
    min-width: 0;
  }

  .album-name-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    margin-bottom: 2px;
  }

  .album-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .album-download-badge {
    flex-shrink: 0;
    font-size: 11px;
    line-height: 1;
    color: var(--accent);
    background: rgba(var(--accent-rgb), 0.1);
    border: 1px solid rgba(var(--accent-rgb), 0.12);
    border-radius: 999px;
    padding: 4px 8px;
  }

  .album-artists {
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
