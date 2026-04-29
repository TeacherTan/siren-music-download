import type {
  Album,
  SeriesGroup,
  HistoryEntry,
  HomepageStatus,
} from '$lib/types';
import { homeStore } from './store.svelte';

interface HomeControllerDeps {
  getLatestAlbums: (limit: number) => Promise<Album[]>;
  getAlbumsBySeriesGroup: () => Promise<SeriesGroup[]>;
  getRecentHistory: (limit: number) => Promise<HistoryEntry[]>;
  getHomepageStatus: () => Promise<HomepageStatus>;
  clearListeningHistory: () => Promise<number>;
  notifyError: (message: string) => void;
}

const CACHE_TTL_MS = 5 * 60 * 1000;
const LATEST_ALBUMS_LIMIT = 12;
const RECENT_HISTORY_LIMIT = 20;

let initialized = false;
let loadRequestSeq = 0;

export function createHomeController(deps: HomeControllerDeps) {
  function init() {
    if (initialized) return;
    initialized = true;
  }

  async function loadHomepageData(options?: { force?: boolean }) {
    const now = Date.now();
    const lastLoaded = homeStore.lastLoadedAt;

    if (
      !options?.force &&
      lastLoaded !== null &&
      now - lastLoaded < CACHE_TTL_MS
    ) {
      return;
    }

    const requestSeq = ++loadRequestSeq;
    homeStore.loading = true;

    const results = await Promise.allSettled([
      deps.getLatestAlbums(LATEST_ALBUMS_LIMIT),
      deps.getAlbumsBySeriesGroup(),
      deps.getRecentHistory(RECENT_HISTORY_LIMIT),
      deps.getHomepageStatus(),
    ]);

    if (requestSeq !== loadRequestSeq) return;

    if (results[0].status === 'fulfilled') {
      homeStore.latestAlbums = results[0].value;
    } else {
      deps.notifyError(`加载最新专辑失败: ${results[0].reason}`);
    }

    if (results[1].status === 'fulfilled') {
      homeStore.seriesGroups = results[1].value;
    }

    if (results[2].status === 'fulfilled') {
      homeStore.recentHistory = results[2].value;
    }

    if (results[3].status === 'fulfilled') {
      homeStore.status = results[3].value;
    }

    homeStore.loading = false;
    homeStore.lastLoadedAt = Date.now();
  }

  async function refreshHomepage() {
    await loadHomepageData({ force: true });
  }

  async function refreshSeriesGroups() {
    const requestSeq = loadRequestSeq;
    try {
      const groups = await deps.getAlbumsBySeriesGroup();
      if (requestSeq !== loadRequestSeq) return;
      homeStore.seriesGroups = groups;
      homeStore.belongReady = true;
    } catch {
      // belong 刷新失败不阻塞
    }
  }

  async function handleClearHistory() {
    try {
      await deps.clearListeningHistory();
      homeStore.recentHistory = [];
    } catch (e) {
      deps.notifyError(`清除收听历史失败: ${e}`);
    }
  }

  function handleBelongReady() {
    homeStore.belongReady = true;
    void refreshSeriesGroups();
  }

  function dispose() {
    loadRequestSeq += 1;
    initialized = false;
    homeStore.reset();
  }

  return {
    get latestAlbums() {
      return homeStore.latestAlbums;
    },
    get seriesGroups() {
      return homeStore.seriesGroups;
    },
    get recentHistory() {
      return homeStore.recentHistory;
    },
    get status() {
      return homeStore.status;
    },
    get loading() {
      return homeStore.loading;
    },
    get belongReady() {
      return homeStore.belongReady;
    },
    init,
    loadHomepageData,
    refreshHomepage,
    refreshSeriesGroups,
    handleClearHistory,
    handleBelongReady,
    dispose,
  };
}

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    loadRequestSeq += 1;
    initialized = false;
    homeStore.reset();
  });
}
