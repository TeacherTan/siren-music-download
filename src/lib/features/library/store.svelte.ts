import type { Album, AlbumDetail } from '$lib/types';

let albums = $state<Album[]>([]);
let selectedAlbum = $state<AlbumDetail | null>(null);
let selectedAlbumCid = $state<string | null>(null);
let loadingAlbums = $state(false);
let loadingDetail = $state(false);
let initialized = false;

async function init() {
  if (initialized) return;
  initialized = true;
}

function dispose() {
  initialized = false;
}

export const libraryStore = {
  get albums() {
    return albums;
  },
  get selectedAlbum() {
    return selectedAlbum;
  },
  get selectedAlbumCid() {
    return selectedAlbumCid;
  },
  get loadingAlbums() {
    return loadingAlbums;
  },
  get loadingDetail() {
    return loadingDetail;
  },
  init,
  dispose,
};

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    dispose();
  });
}
