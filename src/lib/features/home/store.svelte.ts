import type {
  Album,
  SeriesGroup,
  HistoryEntry,
  HomepageStatus,
} from '$lib/types';

export interface HomeState {
  latestAlbums: Album[];
  seriesGroups: SeriesGroup[];
  recentHistory: HistoryEntry[];
  status: HomepageStatus | null;
  loading: boolean;
  belongReady: boolean;
  lastLoadedAt: number | null;
}

let latestAlbums = $state<Album[]>([]);
let seriesGroups = $state<SeriesGroup[]>([]);
let recentHistory = $state<HistoryEntry[]>([]);
let status = $state<HomepageStatus | null>(null);
let loading = $state(false);
let belongReady = $state(false);
let lastLoadedAt = $state<number | null>(null);

function reset() {
  latestAlbums = [];
  seriesGroups = [];
  recentHistory = [];
  status = null;
  loading = false;
  belongReady = false;
  lastLoadedAt = null;
}

export const homeStore = {
  get latestAlbums() {
    return latestAlbums;
  },
  set latestAlbums(value: Album[]) {
    latestAlbums = value;
  },
  get seriesGroups() {
    return seriesGroups;
  },
  set seriesGroups(value: SeriesGroup[]) {
    seriesGroups = value;
  },
  get recentHistory() {
    return recentHistory;
  },
  set recentHistory(value: HistoryEntry[]) {
    recentHistory = value;
  },
  get status() {
    return status;
  },
  set status(value: HomepageStatus | null) {
    status = value;
  },
  get loading() {
    return loading;
  },
  set loading(value: boolean) {
    loading = value;
  },
  get belongReady() {
    return belongReady;
  },
  set belongReady(value: boolean) {
    belongReady = value;
  },
  get lastLoadedAt() {
    return lastLoadedAt;
  },
  set lastLoadedAt(value: number | null) {
    lastLoadedAt = value;
  },
  reset,
};

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    reset();
  });
}
