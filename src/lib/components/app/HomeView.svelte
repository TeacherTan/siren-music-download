<script lang="ts">
  import { OverlayScrollbarsComponent } from 'overlayscrollbars-svelte';
  import HomeLatestAlbums from './HomeLatestAlbums.svelte';
  import HomeSeriesGroups from './HomeSeriesGroups.svelte';
  import HomeRecentHistory from './HomeRecentHistory.svelte';
  import HomeStatusDashboard from './HomeStatusDashboard.svelte';
  import type { PartialOptions } from 'overlayscrollbars';
  import type {
    Album,
    SeriesGroup,
    HistoryEntry,
    HomepageStatus,
    SongEntry,
  } from '$lib/types';

  interface Props {
    runtime: {
      homeController: {
        latestAlbums: Album[];
        seriesGroups: SeriesGroup[];
        recentHistory: HistoryEntry[];
        status: HomepageStatus | null;
        loading: boolean;
        belongReady: boolean;
        handleClearHistory: () => Promise<void>;
      };
      playerController: {
        currentSong: {
          cid: string;
          name: string;
          artists: string[];
          coverUrl: string | null;
        } | null;
        isPlaying: boolean;
      };
      downloadController: {
        activeDownloadCount: number;
      };
      handleSelectAlbum: (album: Album) => void | Promise<void>;
      handlePlay: (song: SongEntry) => void | Promise<void>;
      overlayScrollbarOptions: PartialOptions;
    };
  }

  let { runtime }: Props = $props();
</script>

<OverlayScrollbarsComponent
  class="home-scroll-container"
  options={runtime.overlayScrollbarOptions}
  defer
>
  <div class="home-view">
    <HomeLatestAlbums
      albums={runtime.homeController.latestAlbums}
      loading={runtime.homeController.loading}
      onSelect={runtime.handleSelectAlbum}
    />

    <HomeSeriesGroups
      groups={runtime.homeController.seriesGroups}
      belongReady={runtime.homeController.belongReady}
      onSelectSeries={() => {}}
    />

    <HomeRecentHistory
      entries={runtime.homeController.recentHistory}
      onPlay={(entry) => {
        void runtime.handlePlay({
          cid: entry.songCid,
          name: entry.songName,
          artists: entry.artists,
          download: {
            isDownloaded: false,
            downloadStatus: 'unknown',
            inventoryVersion: '',
          },
        });
      }}
      onClear={runtime.homeController.handleClearHistory}
    />

    <HomeStatusDashboard
      status={runtime.homeController.status}
      currentSong={runtime.playerController.currentSong}
      isPlaying={runtime.playerController.isPlaying}
      activeDownloadCount={runtime.downloadController.activeDownloadCount}
    />
  </div>
</OverlayScrollbarsComponent>

<style>
  :global(.home-scroll-container) {
    flex: 1;
    overflow: hidden;
  }

  .home-view {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    padding: 1.25rem 1.5rem 2rem;
  }
</style>
