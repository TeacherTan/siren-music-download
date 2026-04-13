<script lang="ts">
  interface Props {
    song: { cid: string; name: string; artists: string[]; coverUrl: string | null } | null;
    progress: number;
    duration: number;
    isLoading?: boolean;
    onStop?: () => void;
  }

  let { song, progress, duration, isLoading = false, onStop }: Props = $props();

  function formatTime(seconds: number): string {
    if (!isFinite(seconds) || isNaN(seconds)) return '0:00';
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  }
</script>

<div class="player">
  <div class="player-inner">
    {#if song}
      <div class="player-left">
        {#if song.coverUrl}
          <img
            class="cover-art"
            src={song.coverUrl}
            alt="{song.name} cover"
          />
        {:else}
          <div class="cover-placeholder">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z"/>
            </svg>
          </div>
        {/if}
        <div class="player-info">
          <div class="player-title">{song.name}</div>
          <div class="player-artists">{(song.artists || []).join(', ')}</div>
        </div>
      </div>

      <div class="player-center">
        <div class="time-row">
          <button class="stop-btn" onclick={() => onStop?.()} aria-label="停止播放">
            <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
              <rect x="2" y="2" width="8" height="8" rx="1" />
            </svg>
          </button>
          <span class="time">{formatTime(progress)}</span>
          <div
            class="progress-bar"
            role="slider"
            tabindex="0"
            aria-label="播放进度"
            aria-valuenow={progress}
            aria-valuemin={0}
            aria-valuemax={duration || 100}
          >
            <div class="progress-track">
              <div
                class="progress-fill"
                style="width: {duration > 0 ? (progress / duration) * 100 : 0}%"
              ></div>
            </div>
          </div>
          {#if isLoading}
            <span class="time time-loading">
              <span class="loading-dots">···</span>
            </span>
          {:else}
            <span class="time">{formatTime(duration)}</span>
          {/if}
        </div>
      </div>

      <div class="player-spacer"></div>
    {/if}
  </div>
</div>

<style>
  .player {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    z-index: 100;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    transition: background-color 0.3s ease;
  }

  .player-inner {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    padding: 10px 24px;
    max-width: 1200px;
    margin: 0 auto;
    gap: 16px;
  }

  .player-left {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .cover-art {
    width: 48px;
    height: 48px;
    border-radius: 6px;
    object-fit: cover;
    flex-shrink: 0;
  }

  .cover-placeholder {
    width: 48px;
    height: 48px;
    border-radius: 6px;
    background: var(--bg-tertiary);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .player-info {
    min-width: 0;
  }

  .player-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .player-artists {
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-top: 2px;
  }

  .player-center {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    min-width: 320px;
  }

  .time-row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  .stop-btn {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: none;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.15s cubic-bezier(0.4, 0, 0.2, 1);
    flex-shrink: 0;
  }

  .stop-btn:hover {
    background: var(--accent);
    color: white;
  }

  .time {
    font-size: 11px;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
    min-width: 32px;
    text-align: center;
    flex-shrink: 0;
  }

  .time-loading {
    color: var(--accent);
  }

  .loading-dots {
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 1; }
  }

  .progress-bar {
    flex: 1;
    height: 20px;
    display: flex;
    align-items: center;
  }

  .progress-track {
    width: 100%;
    height: 4px;
    background: var(--bg-tertiary);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    transition: width 0.1s linear;
  }

  .player-spacer {
    /* spacer for layout balance */
  }
</style>
