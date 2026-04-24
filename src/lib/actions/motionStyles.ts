import { animate } from '@humanspeak/svelte-motion';

export type MotionStyleValue = string | number;
export type MotionStyleTarget = Record<string, MotionStyleValue>;

type MotionStylesParams = {
  animate?: MotionStyleTarget;
  transition?: Record<string, unknown>;
  reducedMotion?: boolean;
};

function applyImmediate(node: HTMLElement, styles: MotionStyleTarget = {}) {
  for (const [key, value] of Object.entries(styles)) {
    if (key === 'x') {
      node.style.transform = `translateX(${typeof value === 'number' ? `${value}px` : value})`;
      continue;
    }
    if (key === 'y') {
      node.style.transform = `translateY(${typeof value === 'number' ? `${value}px` : value})`;
      continue;
    }
    if (key === 'scale') {
      node.style.transform = `scale(${value})`;
      continue;
    }
    // @ts-expect-error dynamic style assignment
    node.style[key] =
      typeof value === 'number' && key !== 'opacity' && key !== 'zIndex'
        ? `${value}px`
        : String(value);
  }
}

export function motionStyles(
  node: HTMLElement,
  params: MotionStylesParams = {}
) {
  let controls: { stop?: () => void; cancel?: () => void } | null = null;

  const run = (next: MotionStylesParams = {}) => {
    controls?.cancel?.();
    controls?.stop?.();

    if (!next.animate) {
      return;
    }

    if (next.reducedMotion) {
      applyImmediate(node, next.animate);
      return;
    }

    controls = animate(node, next.animate, next.transition);
  };

  run(params);

  return {
    update(next: MotionStylesParams = {}) {
      run(next);
    },
    destroy() {
      controls?.cancel?.();
      controls?.stop?.();
    },
  };
}
