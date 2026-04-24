import type { DownloadManagerSnapshot } from '$lib/types';

let manager = $state<DownloadManagerSnapshot | null>(null);
let initialized = false;

async function init() {
  if (initialized) return;
  initialized = true;
}

function dispose() {
  initialized = false;
}

export const downloadStore = {
  get manager() {
    return manager;
  },
  init,
  dispose,
};

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    dispose();
  });
}
