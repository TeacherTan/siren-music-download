<script lang="ts">
  import type { SongEntry } from '$lib/types';

  interface Props {
    song: SongEntry;
    index: number;
    checked?: boolean;
    isPlaying?: boolean;
    onchange?: () => void;
    ontoggleplay?: () => void;
  }

  let { song, index, checked = false, isPlaying = false, onchange, ontoggleplay }: Props = $props();
</script>

<div class="song-row">
  <div class="song-number">{index + 1}</div>
  <div class="song-info">
    <div class="song-name">{song.name}</div>
    <div class="song-artists">{(song.artists || []).join(', ')}</div>
  </div>
  <button
    class="play-btn"
    class:is-playing={isPlaying}
    onclick={(e) => { e.stopPropagation(); ontoggleplay?.(); }}
    aria-label={isPlaying ? '停止试听' : '试听'}
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
  </button>
  <input
    type="checkbox"
    class="checkbox"
    checked={checked}
    onchange={onchange}
  />
</div>

<style>
  .song-row {
    display: flex;
    align-items: center;
    padding: 12px 16px;
    margin: 0 -16px;
    border-radius: 8px;
    transition: all 0.15s cubic-bezier(0.4, 0, 0.2, 1);
    gap: 16px;
  }

  .song-row:hover {
    background: var(--hover-bg);
  }

  .song-number {
    font-size: 14px;
    color: var(--text-tertiary);
    width: 24px;
    text-align: center;
  }

  .song-info {
    flex: 1;
    min-width: 0;
  }

  .song-name {
    font-size: 14px;
    font-weight: 400;
    color: var(--text-primary);
    margin-bottom: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .song-artists {
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .checkbox {
    width: 18px;
    height: 18px;
    cursor: pointer;
    accent-color: var(--accent);
  }

  .play-btn {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: all 0.15s cubic-bezier(0.4, 0, 0.2, 1);
    flex-shrink: 0;
  }

  .song-row:hover .play-btn {
    opacity: 1;
  }

  .play-btn:hover {
    background: var(--accent);
    color: white;
    transform: scale(1.1);
  }

  .play-btn.is-playing {
    opacity: 1;
    background: var(--accent);
    color: white;
  }
</style>
