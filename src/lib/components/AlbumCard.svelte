<script lang="ts">
  import type { Album } from '$lib/types';
  import { lazyLoad } from '$lib/lazyLoad';

  interface Props {
    album: Album;
    selected?: boolean;
    onclick?: () => void;
  }

  let { album, selected = false, onclick }: Props = $props();
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="album-card"
  class:selected
  role="button"
  tabindex="0"
  onclick={onclick}
>
  <div class="album-cover-wrapper" use:lazyLoad={{ rootMargin: '150px' }} data-src={album.coverUrl}>
    <div class="album-cover-placeholder">♪</div>
    <img class="album-cover-img" alt={album.name} />
  </div>
  <div class="album-info">
    <div class="album-name">{album.name}</div>
    <div class="album-artists">{(album.artists || []).join(', ')}</div>
  </div>
</div>

<style>
  .album-card {
    background: transparent;
    border-radius: 12px;
    padding: 12px;
    margin-bottom: 4px;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .album-card:hover {
    background: var(--hover-bg-elevated);
    transform: scale(1.02);
  }

  .album-card.selected {
    background: var(--accent-light);
  }

  .album-card.selected .album-name {
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
    transition: background 0.3s ease;
  }

  .album-cover-placeholder {
    color: var(--text-tertiary);
    font-size: 20px;
    transition: opacity 0.3s ease;
  }

  :global(.album-cover-wrapper.loaded) .album-cover-placeholder {
    opacity: 0;
  }

  .album-cover-img {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 8px;
    opacity: 0;
    transition: opacity 0.3s ease;
  }

  :global(.album-cover-wrapper.loaded) .album-cover-img {
    opacity: 1;
  }

  .album-info {
    flex: 1;
    min-width: 0;
  }

  .album-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: color 0.2s;
  }

  .album-artists {
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
