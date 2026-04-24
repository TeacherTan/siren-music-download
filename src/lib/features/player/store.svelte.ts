import type { PlayerState } from '$lib/types';

let playerState = $state<PlayerState | null>(null);
let initialized = false;

async function init() {
  if (initialized) return;
  initialized = true;
}

function dispose() {
  initialized = false;
}

export const playerStore = {
  get playerState() {
    return playerState;
  },
  init,
  dispose,
};

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    dispose();
  });
}
