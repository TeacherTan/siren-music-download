<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { fade, fly } from 'svelte/transition';
  import {
    getAlbums, getAlbumDetail, getDefaultOutputDir, selectDirectory,
    playSong, stopPlayback, getPlayerState
  } from '$lib/api';
  import { clearCache } from '$lib/cache';
  import type { Album, AlbumDetail, OutputFormat, SongEntry, PlayerState } from '$lib/types';
  import AlbumCard from '$lib/components/AlbumCard.svelte';
  import SongRow from '$lib/components/SongRow.svelte';
  import AudioPlayer from '$lib/components/AudioPlayer.svelte';

  // Minimum display time (ms) to prevent animation flash on fast loads
  const MIN_DISPLAY_MS = 400;

  const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

  let albums = $state<Album[]>([]);
  let selectedAlbum = $state<AlbumDetail | null>(null);
  let selectedSongs = $state.raw(new Set<string>());
  let outputDir = $state('');
  let format = $state<OutputFormat>('flac');
  let loadingAlbums = $state(false);
  let loadingDetail = $state(false);
  let errorMsg = $state('');

  // Audio player state (synced from Rust backend via Tauri events)
  let currentSong = $state<{ cid: string; name: string; artists: string[]; coverUrl: string | null } | null>(null);
  let isPlaying = $state(false);
  let isLoading = $state(false);
  let progress = $state(0);
  let duration = $state(0);
  // Track which song is currently being loaded to prevent duplicate play calls
  let playingCid = $state<string | null>(null);

  onMount(() => {
    let unlistenState: (() => void) | null = null;
    let unlistenProgress: (() => void) | null = null;

    async function initialize() {
      loadingAlbums = true;
      try {
        [albums, outputDir] = await Promise.all([
          getAlbums(),
          getDefaultOutputDir(),
        ]);
        // Auto-select the first album on startup
        if (albums.length > 0) {
          await handleSelectAlbum(albums[0]);
        }
      } catch (e) {
        errorMsg = e instanceof Error ? e.message : String(e);
        console.error('[ERROR] Failed to load albums:', e);
      } finally {
        loadingAlbums = false;
      }

      unlistenState = await listen<PlayerState>('player-state-changed', (event) => {
        const state = event.payload;
        currentSong = state.songCid ? {
          cid: state.songCid,
          name: state.songName ?? '',
          artists: state.artists,
          coverUrl: state.coverUrl ?? null,
        } : null;
        isPlaying = state.isPlaying;
        isLoading = state.isLoading;
        progress = state.progress;
        duration = state.duration;
        // Clear playingCid when loading finishes or song changes
        if (!state.isLoading) {
          playingCid = null;
        }
      });

      unlistenProgress = await listen<PlayerState>('player-progress', (event) => {
        const state = event.payload;
        progress = state.progress;
        isPlaying = state.isPlaying;
      });

      try {
        const state = await getPlayerState();
        currentSong = state.songCid ? {
          cid: state.songCid,
          name: state.songName ?? '',
          artists: state.artists,
          coverUrl: state.coverUrl ?? null,
        } : null;
        isPlaying = state.isPlaying;
        isLoading = state.isLoading;
        progress = state.progress;
        duration = state.duration;
      } catch {
        // Player not playing on startup
      }
    }

    void initialize();

    return () => {
      unlistenState?.();
      unlistenProgress?.();
    };
  });

  async function handleSelectAlbum(album: Album) {
    loadingDetail = true;
    selectedSongs = new Set();

    const startTime = Date.now();
    try {
      selectedAlbum = await getAlbumDetail(album.cid);
      errorMsg = '';
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : String(e);
      console.error('[ERROR] Failed to load album detail:', e);
    } finally {
      // Ensure minimum display time so animations aren't interrupted
      const elapsed = Date.now() - startTime;
      if (elapsed < MIN_DISPLAY_MS) {
        await delay(MIN_DISPLAY_MS - elapsed);
      }
      loadingDetail = false;
    }
  }

  function toggleSong(cid: string) {
    const newSet = new Set(selectedSongs);
    if (newSet.has(cid)) {
      newSet.delete(cid);
    } else {
      newSet.add(cid);
    }
    selectedSongs = newSet;
  }

  function selectAll() {
    if (!selectedAlbum) return;
    selectedSongs = new Set(selectedAlbum.songs.map(s => s.cid));
  }

  function selectNone() {
    selectedSongs = new Set();
  }

  let settingsOpen = $state(false);

  async function handleSelectDirectory() {
    const dir = await selectDirectory(outputDir);
    if (dir) outputDir = dir;
  }

  async function handleDownload() {
    if (selectedSongs.size === 0) return;
    alert(`下载功能开发中...\n格式: ${format}\n目录: ${outputDir}\n歌曲数: ${selectedSongs.size}`);
  }

  async function handlePlay(song: SongEntry) {
    // Prevent duplicate play calls for the same song
    if (currentSong?.cid === song.cid && (isPlaying || isLoading)) {
      return;
    }
    // Prevent clicking the same song while it's loading
    if (playingCid === song.cid) {
      return;
    }
    playingCid = song.cid;
    try {
      await playSong(song.cid, selectedAlbum?.coverUrl);
    } catch (e) {
      playingCid = null;
      console.error('[ERROR] Failed to play song:', e);
    }
  }

  async function handleStopPlayback() {
    try {
      await stopPlayback();
    } catch (e) {
      console.error('[ERROR] Failed to stop playback:', e);
    }
  }

  // Refresh cache and reload current album
  let isRefreshing = $state(false);

  async function handleRefresh() {
    if (isRefreshing) return;
    isRefreshing = true;

    // Clear cache
    clearCache();

    // Reload current album if selected
    if (selectedAlbum) {
      const currentAlbumCid = selectedAlbum.cid;
      loadingDetail = true;
      selectedSongs = new Set();
      try {
        selectedAlbum = await getAlbumDetail(currentAlbumCid);
      } catch (e) {
        console.error('[ERROR] Failed to reload album:', e);
      } finally {
        loadingDetail = false;
      }
    }

    // Brief delay to show spinning state
    await delay(400);
    isRefreshing = false;
  }
</script>

<!-- Floating refresh button (top-right corner) -->
<button
  class="refresh-btn"
  class:spinning={isRefreshing}
  onclick={handleRefresh}
  disabled={isRefreshing}
  aria-label="刷新缓存"
>
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
    <path d="M21 12a9 9 0 1 1-6.86-8.72"/>
    <polyline points="21 3 21 12 12 12"/>
  </svg>
</button>

<div class="container">
  <!-- 专辑列表侧边栏 -->
  <aside class="sidebar">
    <h2 class="section-title">专辑</h2>
    {#if loadingAlbums}
      <div class="loading">正在加载专辑...</div>
    {:else if errorMsg && albums.length === 0}
      <div class="empty-state">
        <div class="empty-icon">⚠️</div>
        <div class="empty-text">加载失败</div>
        <div class="empty-text" style="margin-top: 8px; font-size: 12px;">{errorMsg}</div>
      </div>
    {:else}
      <div class="album-list">
        {#each albums as album}
          <AlbumCard
            {album}
            selected={selectedAlbum?.cid === album.cid}
            onclick={() => handleSelectAlbum(album)}
          />
        {/each}
      </div>
    {/if}
  </aside>

  <!-- 歌曲列表内容区 -->
  <main class="content">
    {#if loadingDetail}
      {#key selectedAlbum?.cid ?? 'loading'}
        <div class="album-hero" transition:fade|global={{ duration: 200 }}>
          <div class="album-hero-cover-wrapper" transition:fly|global={{ x: -16, duration: 240 }}>
            <div class="album-hero-cover loading-cover"></div>
          </div>
          <div class="album-hero-info" transition:fly|global={{ x: 16, duration: 240, delay: 60 }}>
            <div class="album-hero-title loading-text"></div>
            <div class="album-hero-sub loading-text-sub"></div>
          </div>
        </div>
      {/key}
      <div class="loading" style="margin-top: 24px;">正在加载歌曲...</div>
    {:else if selectedAlbum}
      {#key selectedAlbum.cid}
        <div class="album-hero" transition:fade|global={{ duration: 280 }}>
          <div class="album-hero-cover-wrapper" transition:fly|global={{ x: -20, duration: 320 }}>
            <img
              class="album-hero-cover"
              src={selectedAlbum.coverDeUrl ?? selectedAlbum.coverUrl}
              alt="{selectedAlbum.name} cover"
              loading="eager"
            />
          </div>
          <div class="album-hero-info" transition:fly|global={{ x: 20, duration: 320, delay: 80 }}>
            {#if selectedAlbum.belong}
              <span class="album-belong-tag">{selectedAlbum.belong.toUpperCase()}</span>
            {/if}
            <h1 class="album-hero-title">{selectedAlbum.name}</h1>
            {#if selectedAlbum.artists && selectedAlbum.artists.length > 0}
              <p class="album-hero-artists">{selectedAlbum.artists.join(', ')}</p>
            {/if}
            {#if selectedAlbum.intro}
              <p class="album-hero-intro">{selectedAlbum.intro}</p>
            {/if}
            <div class="album-hero-meta">
              <span class="album-song-count">{selectedAlbum.songs.length} 首歌曲</span>
            </div>
            <div class="controls">
              <button class="btn" onclick={selectAll}>✓ 全选</button>
              <button class="btn" onclick={selectNone}>✕ 取消全选</button>
            </div>
          </div>
        </div>

        <div class="song-list" transition:fly|global={{ x: 40, duration: 280 }}>
          {#each selectedAlbum.songs as song, i (song.cid)}
            <div transition:fly|global={{ x: 40, duration: 220, delay: Math.min(i * 35, 400) }}>
              <SongRow
                {song}
                index={i}
                checked={selectedSongs.has(song.cid)}
                isPlaying={currentSong?.cid === song.cid && isPlaying}
                onchange={() => toggleSong(song.cid)}
                ontoggleplay={() => handlePlay(song)}
              />
            </div>
          {/each}
        </div>
      {/key}
    {:else}
      <h1 class="page-title">选择专辑</h1>
      <p class="page-subtitle">从左侧选择一个专辑以查看歌曲</p>
    {/if}
  </main>

  <!-- Download settings floating button -->
  <button
    class="settings-btn"
    class:active={settingsOpen}
    onclick={() => settingsOpen = !settingsOpen}
    aria-label="下载设置"
  >
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/>
      <circle cx="12" cy="12" r="3"/>
    </svg>
    {#if selectedSongs.size > 0}
      <span class="settings-badge">{selectedSongs.size}</span>
    {/if}
  </button>

  <!-- Download settings panel (slide-in from right) -->
  {#if settingsOpen}
    <div class="settings-overlay" role="button" tabindex="-1" onclick={() => settingsOpen = false} onkeydown={(e) => e.key === 'Escape' && (settingsOpen = false)} transition:fade|global={{ duration: 150 }}></div>
    <aside class="settings-panel" transition:fly|global={{ x: 320, duration: 220 }}>
      <div class="settings-header">
        <h2 class="settings-title">下载设置</h2>
        <button class="settings-close" onclick={() => settingsOpen = false} aria-label="关闭">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>

      <div class="form-group">
        <label class="form-label" for="format-select">输出格式</label>
        <select id="format-select" class="form-select" bind:value={format}>
          <option value="flac">FLAC（无损压缩）</option>
          <option value="wav">WAV（无损）</option>
          <option value="mp3">MP3</option>
        </select>
      </div>

      <div class="form-group">
        <label class="form-label" for="output-dir">保存位置</label>
        <input id="output-dir" type="text" class="form-input" readonly value={outputDir} />
        <button class="btn" onclick={handleSelectDirectory} style="width: 100%; margin-top: 8px;">
          📁 选择文件夹
        </button>
      </div>

      <button
        class="btn btn-primary btn-block"
        disabled={selectedSongs.size === 0}
        onclick={handleDownload}
      >
        {selectedSongs.size > 0 ? `下载 ${selectedSongs.size} 首歌曲` : '下载选中曲目'}
      </button>
    </aside>
  {/if}
</div>

<!-- Audio Player Bar (Rust backend playback via rodio) -->
{#if currentSong}
  <AudioPlayer
    song={currentSong}
    {progress}
    {duration}
    {isLoading}
    onStop={handleStopPlayback}
  />
{/if}