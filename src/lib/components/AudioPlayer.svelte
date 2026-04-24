<script lang="ts">
  import { getImageDataUrl } from '$lib/api';
  type RepeatMode = 'all' | 'one';
  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';

  interface Song {
    cid: string;
    name: string;
    artists: string[];
    coverUrl: string | null;
  }

  interface Props {
    song: Song | null;
    isPlaying: boolean;
    isPaused: boolean;
    hasPrevious: boolean;
    hasNext: boolean;
    progress: number;
    duration: number;
    isLoading?: boolean;
    reducedMotion?: boolean;
    isShuffled?: boolean;
    repeatMode?: RepeatMode;
    lyricsActive?: boolean;
    playlistActive?: boolean;
    downloadState?: SongDownloadState;
    downloadDisabled?: boolean;
    onPrevious?: () => void;
    onTogglePlay?: () => void;
    onSeek?: (positionSecs: number) => void | Promise<void>;
    onNext?: () => void;
    onShuffleChange?: (next: boolean) => void | Promise<void>;
    onRepeatModeChange?: (next: RepeatMode) => void | Promise<void>;
    onToggleLyrics?: () => void;
    onTogglePlaylist?: () => void;
    onDownload?: () => void | Promise<void>;
  }

  let {
    song,
    isPlaying,
    isPaused,
    hasPrevious,
    hasNext,
    progress,
    duration,
    isLoading = false,
    reducedMotion = false,
    isShuffled = false,
    repeatMode = 'all',
    lyricsActive = false,
    playlistActive = false,
    downloadState = 'idle',
    downloadDisabled = false,
    onPrevious,
    onTogglePlay,
    onSeek,
    onNext,
    onShuffleChange,
    onRepeatModeChange,
    onToggleLyrics,
    onTogglePlaylist,
    onDownload,
  }: Props = $props();

  let seekPreview = $state<number | null>(null);
  let draggingSeek = $state(false);
  let activeCid = $state<string | null>(null);
  let resolvedCoverUrl = $state<string | null>(null);
  let coverRequestSeq = 0;

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  function formatTime(seconds: number): string {
    if (!isFinite(seconds) || isNaN(seconds) || seconds < 0) return '0:00';
    const minute = Math.floor(seconds / 60);
    const second = Math.floor(seconds % 60);
    return `${minute}:${second.toString().padStart(2, '0')}`;
  }

  function nextRepeatMode(mode: RepeatMode): RepeatMode {
    return mode === 'all' ? 'one' : 'all';
  }

  function readRangeValue(event: Event): number {
    return Number((event.currentTarget as HTMLInputElement).value);
  }

  const canSeek = $derived.by(
    () => !!song && duration > 0 && !isLoading && !!onSeek
  );
  const canShuffle = $derived.by(
    () => !!song && !isLoading && !!onShuffleChange
  );
  const canRepeat = $derived.by(
    () => !!song && !isLoading && !!onRepeatModeChange
  );
  const shownProgress = $derived.by(() =>
    seekPreview === null ? progress : seekPreview
  );
  const safeDuration = $derived.by(() => (duration > 0 ? duration : 1));
  const remainingProgress = $derived.by(() =>
    Math.max(duration - shownProgress, 0)
  );
  const progressRatio = $derived.by(() =>
    clamp(shownProgress / safeDuration, 0, 1)
  );
  const artistText = $derived.by(() =>
    song?.artists?.length ? song.artists.join(' · ') : '未知艺术家'
  );
  const subtitle = $derived.by(() =>
    isLoading
      ? `${artistText} · 加载中`
      : isPaused
        ? `${artistText} · 已暂停`
        : artistText
  );
  const repeatLabel = $derived.by(() =>
    repeatMode === 'one' ? '单曲循环' : '列表循环'
  );
  const playerState = $derived.by(() =>
    isLoading ? 'loading' : isPlaying ? 'playing' : isPaused ? 'paused' : 'idle'
  );
  const detailPanel = $derived.by(() =>
    lyricsActive ? 'lyrics' : playlistActive ? 'playlist' : 'none'
  );
  const lyricsButtonLabel = $derived.by(() =>
    lyricsActive ? '关闭歌词' : '打开歌词'
  );
  const playlistButtonLabel = $derived.by(() =>
    playlistActive ? '关闭播放列表' : '打开播放列表'
  );
  const downloadButtonLabel = $derived.by(() => {
    if (!song) return '下载当前歌曲';

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
  const canDownload = $derived.by(
    () =>
      !!song &&
      !isLoading &&
      !!onDownload &&
      downloadState === 'idle' &&
      !downloadDisabled
  );
  const remainingLabel = $derived.by(() =>
    duration > 0 ? `-${formatTime(remainingProgress)}` : '0:00'
  );
  const progressStyle = $derived.by(
    () =>
      `--progress-ratio:${progressRatio};--motion-duration:${reducedMotion ? '0ms' : 'var(--motion-base)'}`
  );

  $effect(() => {
    const currentCid = song?.cid ?? null;
    if (currentCid !== activeCid) {
      activeCid = currentCid;
      seekPreview = null;
      draggingSeek = false;
    }
  });

  $effect(() => {
    const coverUrl = song?.coverUrl ?? null;
    const requestSeq = ++coverRequestSeq;

    if (!coverUrl) {
      resolvedCoverUrl = null;
      return;
    }

    void (async () => {
      try {
        const dataUrl = await getImageDataUrl(coverUrl);
        if (requestSeq !== coverRequestSeq) return;
        resolvedCoverUrl = dataUrl;
      } catch (error) {
        if (requestSeq !== coverRequestSeq) return;
        resolvedCoverUrl = null;
      }
    })();
  });

  $effect(() => {
    if (
      !draggingSeek &&
      seekPreview !== null &&
      Math.abs(seekPreview - progress) < 0.25
    ) {
      seekPreview = null;
    }
  });

  async function commitSeek(nextValue: number) {
    draggingSeek = false;
    if (!canSeek) {
      seekPreview = null;
      return;
    }
    const target = clamp(nextValue, 0, duration);
    seekPreview = target;
    if (Math.abs(target - progress) < 0.05) {
      seekPreview = null;
      return;
    }
    try {
      await onSeek?.(target);
    } catch {
      seekPreview = null;
    }
  }

  function handleSeekInput(event: Event) {
    if (!canSeek) return;
    draggingSeek = true;
    seekPreview = clamp(readRangeValue(event), 0, duration || 0);
  }

  function handleSeekChange(event: Event) {
    void commitSeek(readRangeValue(event));
  }

  async function handleShuffleToggle() {
    if (!canShuffle) return;
    try {
      await onShuffleChange?.(!isShuffled);
    } catch {
      return;
    }
  }

  async function handleRepeatToggle() {
    if (!canRepeat) return;
    const next = nextRepeatMode(repeatMode);
    try {
      await onRepeatModeChange?.(next);
    } catch {
      return;
    }
  }
</script>

{#if song}
  <section
    class="am-player"
    aria-label="播放控制条"
    style={progressStyle}
    data-loading={isLoading ? 'true' : 'false'}
    data-state={playerState}
    data-panel={detailPanel}
    data-dragging={draggingSeek ? 'true' : 'false'}
  >
    <div class="left-controls" role="group" aria-label="传输控制">
      <button
        type="button"
        class="icon-button side-toggle"
        aria-label="乱序播放"
        aria-pressed={isShuffled}
        disabled={!canShuffle}
        onclick={handleShuffleToggle}
      >
        <svg class="control-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M5 7h2.2c1.5 0 2.8.6 3.8 1.6L19 16.6"></path>
          <path d="m16.2 16.6 2.8.1-.1-2.8"></path>
          <path d="M5 17h2.2c1.5 0 2.8-.6 3.8-1.6l2-2"></path>
          <path d="m16.2 7.4 2.8-.1-.1 2.8"></path>
        </svg>
      </button>

      <div class="transport-cluster">
        <button
          type="button"
          class="icon-button transport-button"
          aria-label="上一首"
          disabled={!hasPrevious || isLoading}
          onclick={() => onPrevious?.()}
        >
          <svg
            class="control-icon solid-icon"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <rect x="4.75" y="6.15" width="1.95" height="11.7" rx="0.75"></rect>
            <path d="M18.6 6.9v10.2L11.75 12z"></path>
            <path d="M12.2 6.9v10.2L5.35 12z"></path>
          </svg>
        </button>

        <button
          type="button"
          class="icon-button play-button"
          class:playing={isPlaying}
          aria-label={isPlaying ? '暂停播放' : isPaused ? '继续播放' : '播放'}
          disabled={isLoading || !onTogglePlay}
          onclick={() => onTogglePlay?.()}
        >
          <span class="play-glyph" aria-hidden="true">
            <svg
              class="control-icon play-icon play-icon-pause"
              viewBox="0 0 24 24"
            >
              <rect x="7.15" y="5.95" width="3.4" height="12.1" rx="1.25"
              ></rect>
              <rect x="13.45" y="5.95" width="3.4" height="12.1" rx="1.25"
              ></rect>
            </svg>
            <svg
              class="control-icon play-icon play-icon-play"
              viewBox="0 0 24 24"
            >
              <path d="M8.2 6.3v11.4L17.35 12z"></path>
            </svg>
          </span>
        </button>

        <button
          type="button"
          class="icon-button transport-button"
          aria-label="下一首"
          disabled={!hasNext || isLoading}
          onclick={() => onNext?.()}
        >
          <svg
            class="control-icon solid-icon"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <rect x="17.3" y="6.15" width="1.95" height="11.7" rx="0.75"></rect>
            <path d="M5.4 6.9v10.2L12.25 12z"></path>
            <path d="M11.8 6.9v10.2L18.65 12z"></path>
          </svg>
        </button>
      </div>

      <button
        type="button"
        class="icon-button side-toggle"
        aria-label={`切换循环模式，当前${repeatLabel}`}
        aria-pressed={repeatMode === 'one'}
        disabled={!canRepeat}
        onclick={handleRepeatToggle}
      >
        <svg class="control-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M5 8h10.8"></path>
          <path d="m13.3 5.4 2.7 2.6-2.7 2.6"></path>
          <path d="M19 16H8.2"></path>
          <path d="m10.7 18.6-2.7-2.6 2.7-2.6"></path>
          {#if repeatMode === 'one'}
            <circle class="repeat-badge" cx="12" cy="12" r="3.15"></circle>
            <path d="M12 10.3v3.4"></path>
            <path d="m11.4 10.9.6-.6"></path>
          {/if}
        </svg>
      </button>
    </div>

    <div class="center-panel">
      <div class="playback-stage">
        <div class="track-info">
          {#if resolvedCoverUrl}
            <img
              src={resolvedCoverUrl}
              alt={`${song.name} 封面`}
              class="cover"
            />
          {:else}
            <div class="cover fallback" aria-hidden="true">
              <svg viewBox="0 0 24 24"
                ><path d="M12 3v10.5a4 4 0 1 0 2 3.5V7h4V3h-6z" /></svg
              >
            </div>
          {/if}

          <div class="meta meta-stage">
            <p class="title">{song.name}</p>
            <p class="artist">{subtitle}</p>
          </div>
        </div>

        <div class="timeline" role="group" aria-label="播放进度">
          <div class="timeline-timebar" aria-hidden="true">
            <span class="time">{formatTime(shownProgress)}</span>
            <span class="time time-remaining">{remainingLabel}</span>
          </div>
          <div class="progress-track">
            <div class="track-bg" aria-hidden="true">
              <div class="track-fill"></div>
            </div>
            <input
              class="seek-slider"
              type="range"
              min="0"
              max={safeDuration}
              value={shownProgress}
              step="0.1"
              aria-label="调整播放进度"
              disabled={!canSeek}
              oninput={handleSeekInput}
              onchange={handleSeekChange}
            />
          </div>
        </div>
      </div>
    </div>

    <div class="right-controls" role="group" aria-label="附加控制">
      <button
        type="button"
        class="icon-button panel-toggle"
        class:panel-active={lyricsActive}
        aria-label={lyricsButtonLabel}
        aria-pressed={lyricsActive}
        disabled={!song || isLoading || !onToggleLyrics}
        onclick={() => onToggleLyrics?.()}
      >
        <svg
          class="control-icon stateful-icon"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path d="M5.5 7.25h13"></path>
          <path d="M5.5 11h13"></path>
          <path d="M5.5 14.75h9.5"></path>
          <path d="M5.5 18.5h6.25"></path>
          <circle class="toggle-badge" cx="18" cy="6" r="3.1"></circle>
          <path class="toggle-mark" d="m16.55 5.25 1.45 1.45 1.45-1.45"></path>
        </svg>
      </button>

      <button
        type="button"
        class="icon-button panel-toggle"
        class:panel-active={playlistActive}
        aria-label={playlistButtonLabel}
        aria-pressed={playlistActive}
        disabled={!song || isLoading || !onTogglePlaylist}
        onclick={() => onTogglePlaylist?.()}
      >
        <svg
          class="control-icon stateful-icon"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path d="M5.25 7h9.5"></path>
          <path d="M5.25 11.5h9.5"></path>
          <path d="M5.25 16h6.75"></path>
          <path d="M16.6 10.25 20 12.25l-3.4 2z"></path>
          <circle class="toggle-badge" cx="18" cy="6" r="3.1"></circle>
          <path class="toggle-mark" d="m16.55 5.25 1.45 1.45 1.45-1.45"></path>
        </svg>
      </button>

      <button
        type="button"
        class="icon-button"
        class:download-active={downloadState !== 'idle'}
        aria-label={downloadButtonLabel}
        title={downloadButtonLabel}
        disabled={!canDownload}
        onclick={() => onDownload?.()}
      >
        {#if downloadState === 'creating'}
          <svg
            class="control-icon spin-icon"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path d="M21 12a9 9 0 1 1-2.64-6.36"></path>
            <path d="M21 3v6h-6"></path>
          </svg>
        {:else}
          <svg class="control-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 5v9"></path>
            <path d="m8.5 10.5 3.5 3.5 3.5-3.5"></path>
            <path d="M5 18h14"></path>
            {#if downloadState === 'queued'}
              <path d="M8 4.5h8"></path>
            {/if}
          </svg>
        {/if}
      </button>
    </div>
  </section>
{/if}

<style>
  .am-player {
    --surface: var(--player-shell-bg);
    --surface-border: var(--player-shell-border);
    --surface-highlight: var(--player-shell-highlight);
    --text-main: var(--player-title);
    --text-subtle: var(--player-subtitle);
    --icon-default: var(--player-control-color);
    --icon-active: var(--accent);
    --track-bg: var(--player-track-bg);
    --track-fill-end: var(--player-track-fill-end);
    --thumb-border: var(--player-thumb-border);
    --thumb-bg: var(--player-thumb-bg);
    --thumb-shadow: var(--player-thumb-shadow);
    --time-color: var(--player-time);
    --play-text: var(--player-play-text);
    --play-shadow: var(--player-play-shadow);
    --play-shadow-hover: var(--player-play-shadow-hover);
    --group-bg: color-mix(in srgb, var(--surface) 76%, transparent);
    --group-border: color-mix(in srgb, var(--surface-border) 84%, transparent);
    --control-button-size: 30px;
    --control-icon-size: 17px;
    --play-icon-size: 19px;
    --control-icon-stroke: 1.75;
    --seek-thumb-size: 10px;
    --seek-track-size: 3px;
    --seek-track-pad: calc(var(--seek-thumb-size) / 2);
    --thumb-scale: 0.001;
    --thumb-opacity: 0;
    --transport-width: 152px;
    width: min(625px, calc(100vw - 20px));
    min-width: 0;
    min-height: 62px;
    margin: 0 auto;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.55);
    background: transparent;
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
    box-shadow: none;
    display: grid;
    grid-template-columns: var(--transport-width) minmax(0, 1fr) auto;
    gap: 2px;
    align-items: center;
    padding: 5px 7px 5px 5px;
    transition:
      box-shadow var(--motion-duration) var(--ease-standard),
      transform var(--motion-duration) var(--ease-standard);
  }

  .am-player[data-panel='lyrics'],
  .am-player[data-panel='playlist'] {
    box-shadow:
      0 18px 36px rgba(15, 23, 42, 0.14),
      0 8px 20px rgba(var(--accent-rgb), 0.1),
      inset 0 1px 0
        color-mix(in srgb, var(--surface-highlight) 90%, transparent);
  }

  .left-controls {
    display: flex;
    align-items: center;
    gap: 0;
    width: var(--transport-width);
    min-width: 0;
    flex-shrink: 0;
  }

  .transport-cluster {
    display: flex;
    align-items: center;
    gap: 0;
    padding: 0;
  }

  .right-controls {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0;
    padding: 0;
    flex-shrink: 0;
  }

  .center-panel {
    min-width: 0;
    display: flex;
    align-items: center;
    padding: 0;
  }

  .track-info {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .playback-stage {
    min-width: 0;
    width: 100%;
    justify-self: start;
    display: grid;
    gap: 1px;
    transition: gap var(--motion-duration) var(--ease-standard);
  }

  .cover {
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    border-radius: 9px;
    object-fit: cover;
    box-shadow:
      0 12px 24px rgba(16, 18, 28, 0.18),
      0 0 0 1px rgba(255, 255, 255, 0.18);
    transition:
      transform var(--motion-duration) var(--ease-standard),
      box-shadow var(--motion-duration) var(--ease-standard);
  }

  .am-player[data-state='playing'] .cover {
    box-shadow:
      0 14px 28px rgba(16, 18, 28, 0.22),
      0 0 0 1px rgba(var(--accent-rgb), 0.12);
  }

  .fallback {
    display: grid;
    place-items: center;
    background: linear-gradient(
      145deg,
      var(--player-cover-start),
      var(--player-cover-end)
    );
    color: var(--player-placeholder-color);
  }

  .fallback svg {
    width: 15px;
    height: 15px;
    fill: currentColor;
  }

  .meta {
    min-width: 0;
    display: grid;
    gap: 1px;
  }

  .meta-stage {
    position: relative;
    flex: 1 1 auto;
    min-width: 0;
    padding: 0 2px;
    min-height: 24px;
    align-content: start;
    transition: opacity var(--motion-duration) var(--ease-standard);
    isolation: isolate;
    overflow: hidden;
    border-radius: 10px;
  }

  .meta-stage::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: color-mix(
      in srgb,
      var(--surface) 72%,
      rgba(255, 255, 255, 0.14)
    );
    border: 1px solid rgba(255, 255, 255, 0.16);
    backdrop-filter: blur(12px) saturate(1.12);
    -webkit-backdrop-filter: blur(12px) saturate(1.12);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.2);
    opacity: 0;
    transform: scale(0.97);
    transition:
      opacity var(--motion-duration) var(--ease-standard),
      transform var(--motion-duration) var(--ease-standard);
    pointer-events: none;
  }

  .title,
  .artist {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    font-size: 12.5px;
    line-height: 1.14;
    color: var(--text-main);
    font-weight: 700;
    letter-spacing: -0.01em;
    transition:
      color var(--motion-duration) var(--ease-standard),
      transform var(--motion-duration) var(--ease-standard);
  }

  .artist {
    font-size: 10.5px;
    line-height: 1.15;
    color: var(--text-subtle);
    opacity: 1;
    transition: opacity var(--motion-duration) var(--ease-standard);
  }

  .timeline {
    display: grid;
    gap: 0;
    min-width: 0;
    position: relative;
    margin-top: -1px;
  }

  .center-panel:hover .timeline,
  .center-panel:focus-within .timeline,
  .timeline:hover,
  .timeline:focus-within,
  .am-player[data-dragging='true'] .timeline {
    --thumb-scale: 1;
    --thumb-opacity: 1;
    --seek-track-size: 4px;
  }

  .center-panel:hover .title,
  .center-panel:focus-within .title,
  .am-player[data-dragging='true'] .title {
    color: color-mix(in srgb, var(--text-main) 92%, black);
  }

  .center-panel:hover .meta-stage::after,
  .center-panel:focus-within .meta-stage::after,
  .am-player[data-dragging='true'] .meta-stage::after {
    opacity: 1;
    transform: scale(1);
  }

  .center-panel:hover .artist,
  .center-panel:focus-within .artist,
  .am-player[data-dragging='true'] .artist {
    opacity: 0.22;
  }

  .timeline-timebar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: calc(100% - 2px);
    z-index: 3;
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-width: 0;
    opacity: 0;
    transform: translateY(6px);
    pointer-events: none;
    transition:
      opacity var(--motion-duration) var(--ease-standard),
      transform var(--motion-duration) var(--ease-standard);
  }

  .center-panel:hover .timeline-timebar,
  .center-panel:focus-within .timeline-timebar,
  .timeline:hover .timeline-timebar,
  .timeline:focus-within .timeline-timebar,
  .am-player[data-dragging='true'] .timeline-timebar {
    opacity: 1;
    transform: translateY(0);
  }

  .time {
    min-width: 0;
    font-size: 10px;
    font-weight: 600;
    color: color-mix(in srgb, var(--text-main) 68%, var(--text-subtle));
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    transition: color var(--motion-duration) var(--ease-standard);
  }

  .center-panel:hover .time,
  .center-panel:focus-within .time,
  .timeline:hover .time,
  .timeline:focus-within .time,
  .am-player[data-dragging='true'] .time {
    color: var(--text-main);
  }

  .time-remaining {
    text-align: right;
  }

  .progress-track {
    position: relative;
    height: 6px;
    display: flex;
    align-items: flex-end;
    min-width: 0;
  }

  .track-bg {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: var(--seek-track-size);
    border-radius: 999px;
    background: color-mix(in srgb, var(--text-main) 10%, transparent);
    overflow: hidden;
    transition: height var(--motion-duration) var(--ease-standard);
  }

  .track-fill {
    position: relative;
    height: 100%;
    width: 100%;
    border-radius: inherit;
    background: color-mix(in srgb, var(--text-main) 82%, black);
    transform: scaleX(var(--progress-ratio));
    transform-origin: left center;
    transition: background-color var(--motion-duration) var(--ease-standard);
  }

  .track-fill::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: none;
    opacity: 0;
  }

  .seek-slider {
    appearance: none;
    -webkit-appearance: none;
    width: 100%;
    margin: 0;
    background: transparent;
    height: 6px;
    position: relative;
    z-index: 2;
    cursor: pointer;
  }

  .seek-slider::-webkit-slider-runnable-track {
    height: var(--seek-track-size);
    background: transparent;
    border-radius: 999px;
  }

  .seek-slider:disabled {
    cursor: not-allowed;
  }

  .seek-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: calc(var(--seek-thumb-size) * var(--thumb-scale));
    height: calc(var(--seek-thumb-size) * var(--thumb-scale));
    margin-top: calc(
      (var(--seek-track-size) - (var(--seek-thumb-size) * var(--thumb-scale))) /
        2
    );
    border-radius: 50%;
    border: 1.5px solid rgba(255, 255, 255, 0.92);
    background: color-mix(in srgb, var(--text-main) 92%, black);
    box-shadow: 0 1px 3px rgba(15, 23, 42, 0.18);
    opacity: var(--thumb-opacity);
    transition:
      width var(--motion-duration) var(--ease-standard),
      height var(--motion-duration) var(--ease-standard),
      margin-top var(--motion-duration) var(--ease-standard),
      box-shadow var(--motion-duration) var(--ease-standard),
      opacity var(--motion-duration) var(--ease-standard);
  }

  .seek-slider::-moz-range-track {
    height: var(--seek-track-size);
    background: transparent;
    border: 0;
    border-radius: 999px;
  }

  .seek-slider::-moz-range-progress {
    background: transparent;
    border: 0;
  }

  .seek-slider::-moz-range-thumb {
    width: calc(var(--seek-thumb-size) * var(--thumb-scale));
    height: calc(var(--seek-thumb-size) * var(--thumb-scale));
    border-radius: 50%;
    border: 1.5px solid rgba(255, 255, 255, 0.92);
    background: color-mix(in srgb, var(--text-main) 92%, black);
    box-shadow: 0 1px 3px rgba(15, 23, 42, 0.18);
    opacity: var(--thumb-opacity);
    transition:
      width var(--motion-duration) var(--ease-standard),
      height var(--motion-duration) var(--ease-standard),
      box-shadow var(--motion-duration) var(--ease-standard),
      opacity var(--motion-duration) var(--ease-standard);
  }

  .icon-button {
    position: relative;
    width: var(--control-button-size);
    height: var(--control-button-size);
    border-radius: 50%;
    border: 1px solid transparent;
    display: inline-grid;
    place-items: center;
    cursor: pointer;
    color: var(--icon-default);
    background: transparent;
    transition:
      background-color var(--motion-duration) var(--ease-standard),
      border-color var(--motion-duration) var(--ease-standard),
      box-shadow var(--motion-duration) var(--ease-standard),
      color var(--motion-duration) var(--ease-standard),
      transform var(--motion-duration) var(--ease-standard);
  }

  .icon-button::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.2), transparent);
    opacity: 0;
    transition: opacity var(--motion-duration) var(--ease-standard);
    pointer-events: none;
  }

  .control-icon {
    width: var(--control-icon-size);
    height: var(--control-icon-size);
    fill: none;
    stroke: currentColor;
    stroke-width: var(--control-icon-stroke);
    stroke-linecap: round;
    stroke-linejoin: round;
    flex-shrink: 0;
    transition:
      transform var(--motion-duration) var(--ease-standard),
      opacity var(--motion-duration) var(--ease-standard);
  }

  .control-icon.solid-icon {
    fill: currentColor;
    stroke: none;
  }

  .control-icon .repeat-badge {
    fill: color-mix(in srgb, currentColor 12%, transparent);
    stroke: currentColor;
  }

  .stateful-icon .toggle-badge,
  .stateful-icon .toggle-mark {
    transform-origin: 18px 6px;
    transition:
      transform var(--motion-duration) var(--ease-standard),
      opacity var(--motion-duration) var(--ease-standard);
  }

  .stateful-icon .toggle-badge {
    fill: rgba(var(--accent-rgb), 0.12);
    stroke: rgba(var(--accent-rgb), 0.24);
    opacity: 0;
    transform: scale(0.72);
  }

  .stateful-icon .toggle-mark {
    opacity: 0;
    transform: scale(0.72);
    stroke-width: 2.15;
  }

  .icon-button:hover:not(:disabled),
  .icon-button[aria-pressed='true'] {
    background: rgba(var(--accent-rgb), 0.08);
    color: var(--icon-active);
    border-color: rgba(var(--accent-rgb), 0.08);
    box-shadow: none;
  }

  .icon-button:hover:not(:disabled)::before,
  .icon-button[aria-pressed='true']::before {
    opacity: 1;
  }

  .icon-button[aria-pressed='true'] .stateful-icon .toggle-badge,
  .icon-button[aria-pressed='true'] .stateful-icon .toggle-mark {
    opacity: 1;
    transform: scale(1);
  }

  .panel-toggle.panel-active {
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.2),
      0 8px 18px rgba(var(--accent-rgb), 0.12);
  }

  .icon-button.download-active {
    background: var(--player-control-hover-bg);
    color: var(--icon-active);
    border-color: rgba(var(--accent-rgb), 0.14);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.18);
  }

  .icon-button.download-active::before {
    opacity: 1;
  }

  .spin-icon {
    animation: player-download-spin 0.9s linear infinite;
  }

  .icon-button:active:not(:disabled) {
    transform: scale(0.96);
  }

  .play-button {
    color: var(--icon-default);
  }

  .play-button.playing {
    color: var(--icon-active);
  }

  .play-glyph {
    position: relative;
    width: var(--play-icon-size);
    height: var(--play-icon-size);
    display: grid;
    place-items: center;
  }

  .play-icon {
    position: absolute;
    inset: 0;
    width: var(--play-icon-size);
    height: var(--play-icon-size);
    fill: currentColor;
    stroke: none;
  }

  .play-icon-play {
    transform: translateX(0.5px) scale(1);
    opacity: 1;
  }

  .play-icon-pause {
    transform: scale(0.82);
    opacity: 0;
  }

  .play-button.playing .play-icon-play {
    transform: translateX(0.5px) scale(0.82);
    opacity: 0;
  }

  .play-button.playing .play-icon-pause {
    transform: scale(1);
    opacity: 1;
  }

  .icon-button:focus-visible,
  .seek-slider:focus-visible {
    outline: none;
    box-shadow:
      0 0 0 2px color-mix(in srgb, var(--surface-highlight) 86%, white 14%),
      0 0 0 4px rgba(var(--accent-rgb), 0.28);
    border-radius: 999px;
  }

  .icon-button:disabled,
  .seek-slider:disabled {
    opacity: 0.42;
  }

  .icon-button:disabled {
    cursor: not-allowed;
    box-shadow: none;
  }

  @keyframes player-download-spin {
    from {
      transform: rotate(0deg);
    }

    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 900px) {
    .am-player {
      border-radius: 999px;
      grid-template-columns: 1fr;
      gap: 8px;
      padding: 7px 10px;
    }

    .left-controls,
    .right-controls {
      justify-content: center;
      flex-wrap: wrap;
    }

    .center-panel {
      order: -1;
      display: flex;
    }

    .playback-stage {
      width: 100%;
    }

    .left-controls {
      width: auto;
    }

    .timeline {
      gap: 0;
    }
  }

  @media (max-width: 640px) {
    .am-player {
      --control-button-size: 28px;
      --control-icon-size: 16px;
      --play-icon-size: 18px;
      width: calc(100vw - 12px);
      min-height: 62px;
      padding: 6px 8px;
      gap: 6px;
    }

    .left-controls {
      gap: 0;
    }

    .track-info {
      gap: 6px;
    }

    .transport-cluster {
      gap: 0;
    }

    .right-controls {
      gap: 0;
    }

    .cover {
      width: 34px;
      height: 34px;
      border-radius: 8px;
    }

    .title {
      font-size: 12px;
    }

    .artist,
    .time {
      font-size: 10px;
    }
  }

  @media (hover: none) {
    .timeline {
      --timeline-time-width: 34px;
      --timeline-gap: 6px;
      --thumb-scale: 1;
      --thumb-opacity: 1;
    }

    .timeline-timebar {
      opacity: 1;
      transform: translateY(0);
    }

    .playback-stage {
      width: 100%;
      gap: 1px;
    }

    .title {
      transform: none;
    }

    .artist {
      opacity: 1;
    }

    .meta-stage::after {
      opacity: 0;
      transform: scale(1);
    }
  }
</style>
