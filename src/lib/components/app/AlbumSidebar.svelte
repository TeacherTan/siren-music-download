<script lang="ts">
  import * as Select from '$lib/components/ui/select/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import AlbumCard from '$lib/components/AlbumCard.svelte';
  import MotionSpinner from '$lib/components/MotionSpinner.svelte';
  import type {
    Album,
    LibraryIndexState,
    SearchLibraryResponse,
    SearchLibraryResultItem,
    LibrarySearchScope,
  } from '$lib/types';

  interface Props {
    albums: Album[];
    selectedAlbumCid: string | null;
    reducedMotion: boolean;
    loadingAlbums?: boolean;
    errorMsg?: string;
    searchQuery?: string;
    searchScope?: LibrarySearchScope;
    searchLoading?: boolean;
    searchResponse?: SearchLibraryResponse | null;
    onSearchQueryChange: (query: string) => void;
    onSearchScopeChange: (scope: LibrarySearchScope) => void;
    onSelect: (album: Album) => void;
    onSelectSearchResult: (item: SearchLibraryResultItem) => void;
  }

  const scopeOptions: Array<{ value: LibrarySearchScope; label: string }> = [
    { value: 'all', label: '全部' },
    { value: 'albums', label: '专辑' },
    { value: 'songs', label: '歌曲' },
  ];

  let {
    albums,
    selectedAlbumCid,
    reducedMotion,
    loadingAlbums = false,
    errorMsg = '',
    searchQuery = '',
    searchScope = 'all',
    searchLoading = false,
    searchResponse = null,
    onSearchQueryChange,
    onSearchScopeChange,
    onSelect,
    onSelectSearchResult,
  }: Props = $props();

  const trimmedSearchQuery = $derived.by(() => searchQuery.trim());
  const isSearchMode = $derived.by(() => trimmedSearchQuery.length > 0);
  const searchIndexState = $derived.by<LibraryIndexState>(
    () => searchResponse?.indexState ?? 'notReady'
  );
  const isSearchIndexBuilding = $derived.by(
    () => isSearchMode && !searchLoading && searchIndexState === 'building'
  );
  const searchStatusMessage = $derived.by(() => {
    if (!isSearchMode) return '';
    if (searchLoading) return '正在搜索…';
    switch (searchIndexState) {
      case 'stale':
        return '索引正在刷新，暂时不可用。';
      case 'notReady':
        return '搜索索引尚未就绪。';
      default:
        return '';
    }
  });
</script>

<div class="h-full">
  <h2 class="section-title">专辑</h2>
  <div class="mb-3 grid gap-2">
    <Input
      value={searchQuery}
      placeholder="搜索专辑 / 歌曲 / 艺术家"
      aria-label="搜索专辑、歌曲或艺术家"
      class="border-white/35 bg-white/20"
      oninput={(event) => {
        const target = event.currentTarget as HTMLInputElement;
        onSearchQueryChange(target.value);
      }}
    />

    <Select.Root
      type="single"
      value={searchScope}
      onValueChange={(value) =>
        onSearchScopeChange(value as LibrarySearchScope)}
    >
      <Select.Trigger class="w-full border-white/35 bg-white/20">
        {scopeOptions.find((option) => option.value === searchScope)?.label ??
          '全部'}
      </Select.Trigger>
      <Select.Content>
        {#each scopeOptions as option (option.value)}
          <Select.Item value={option.value} label={option.label} />
        {/each}
      </Select.Content>
    </Select.Root>
  </div>

  {#if loadingAlbums}
    <div class="loading">
      <span>正在加载专辑...</span><MotionSpinner
        className="inline-loading-spinner"
        {reducedMotion}
      />
    </div>
  {:else if errorMsg && albums.length === 0}
    <div class="empty-state">
      <div class="empty-icon">⚠️</div>
      <div class="empty-text">加载失败</div>
      <div class="empty-text" style="margin-top: 8px; font-size: 12px;">
        {errorMsg}
      </div>
    </div>
  {:else if isSearchMode}
    {#if isSearchIndexBuilding}
      <div class="search-status-card" aria-live="polite">
        <div class="search-status-title">正在构建搜索索引</div>
        <div
          class="search-status-progress"
          role="progressbar"
          aria-label="搜索索引构建进度"
          aria-valuetext="索引正在构建中"
        >
          <div
            class={`search-status-progress-bar${reducedMotion ? ' is-reduced-motion' : ''}`}
          ></div>
        </div>
        <div class="search-status-hint">首次扫描完成后即可看到搜索结果。</div>
      </div>
    {:else if searchStatusMessage}
      <div class="empty-state">
        <div class="empty-text">{searchStatusMessage}</div>
      </div>
    {:else if searchResponse && searchResponse.items.length > 0}
      <div class="album-list">
        {#each searchResponse.items as item, index (`${item.kind}:${item.albumCid}:${item.songCid ?? 'album'}:${index}`)}
          <button
            type="button"
            class={`search-result${selectedAlbumCid === item.albumCid ? ' is-selected' : ''}`}
            onclick={() => onSelectSearchResult(item)}
          >
            <div class="search-result-kind">
              {item.kind === 'album' ? '专辑' : '歌曲'}
            </div>
            <div class="search-result-title">
              {item.kind === 'song' && item.songTitle
                ? item.songTitle
                : item.albumTitle}
            </div>
            <div class="search-result-subtitle">
              {#if item.kind === 'song'}
                <span>{item.albumTitle}</span>
              {/if}
              {#if item.artistLine}
                <span>{item.artistLine}</span>
              {/if}
            </div>
          </button>
        {/each}
      </div>
    {:else}
      <div class="empty-state">
        <div class="empty-text">没有匹配的搜索结果</div>
      </div>
    {/if}
  {:else}
    <div class="album-list">
      {#each albums as album (album.cid)}
        <AlbumCard
          {album}
          selected={selectedAlbumCid === album.cid}
          {reducedMotion}
          onclick={() => onSelect(album)}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .search-status-card {
    display: grid;
    gap: 10px;
    padding: 16px 14px;
    border-radius: 20px;
    border: 1px solid rgba(255, 255, 255, 0.22);
    background: rgba(255, 255, 255, 0.16);
  }

  .search-status-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .search-status-hint {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .search-status-progress {
    position: relative;
    overflow: hidden;
    height: 8px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.16);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.12);
  }

  .search-status-progress-bar {
    position: absolute;
    inset: 0;
    width: 42%;
    border-radius: inherit;
    background: linear-gradient(
      90deg,
      rgba(var(--accent-rgb), 0.28) 0%,
      rgba(var(--accent-rgb), 0.9) 45%,
      rgba(var(--accent-rgb), 0.32) 100%
    );
    animation: search-progress-slide 1.2s ease-in-out infinite;
  }

  .search-status-progress-bar.is-reduced-motion {
    width: 100%;
    opacity: 0.72;
    animation: none;
  }

  @keyframes search-progress-slide {
    0% {
      transform: translateX(-100%);
    }

    100% {
      transform: translateX(240%);
    }
  }

  .search-result {
    width: 100%;
    display: grid;
    gap: 4px;
    padding: 12px 14px;
    border-radius: 18px;
    border: 1px solid rgba(255, 255, 255, 0.28);
    background: rgba(255, 255, 255, 0.22);
    text-align: left;
    transition:
      background-color 0.16s ease,
      border-color 0.16s ease;
  }

  .search-result:hover,
  .search-result.is-selected {
    background: rgba(var(--accent-rgb), 0.1);
    border-color: rgba(var(--accent-rgb), 0.22);
  }

  .search-result-kind {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .search-result-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .search-result-subtitle {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    font-size: 12px;
    color: var(--text-secondary);
  }
</style>
