let isMacOS = $state(false);
let prefersReducedMotion = $state(false);
let viewportHeight = $state(0);

let mediaQuery: MediaQueryList | null = null;
let changeHandler: (() => void) | null = null;
let resizeHandler: (() => void) | null = null;
let initialized = false;

function updateReducedMotionPreference() {
  prefersReducedMotion = mediaQuery?.matches ?? false;
}

function updateViewportHeight() {
  viewportHeight = window.innerHeight || 0;
}

function init() {
  if (initialized || typeof window === 'undefined') return;
  initialized = true;

  isMacOS =
    /Mac|iPhone|iPad|iPod/.test(navigator.platform) ||
    navigator.userAgent.includes('Mac');

  mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
  changeHandler = () => updateReducedMotionPreference();
  resizeHandler = () => updateViewportHeight();

  updateReducedMotionPreference();
  updateViewportHeight();

  mediaQuery.addEventListener('change', changeHandler);
  window.addEventListener('resize', resizeHandler, { passive: true });
}

function dispose() {
  if (!initialized) return;
  initialized = false;

  if (mediaQuery && changeHandler) {
    mediaQuery.removeEventListener('change', changeHandler);
  }

  if (resizeHandler) {
    window.removeEventListener('resize', resizeHandler);
  }

  mediaQuery = null;
  changeHandler = null;
  resizeHandler = null;
}

export const envStore = {
  get isMacOS() {
    return isMacOS;
  },
  get prefersReducedMotion() {
    return prefersReducedMotion;
  },
  get viewportHeight() {
    return viewportHeight;
  },
  init,
  dispose,
};

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    dispose();
  });
}
