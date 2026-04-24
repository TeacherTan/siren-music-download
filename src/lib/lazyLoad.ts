import { animate } from '@humanspeak/svelte-motion';
import { getImageDataUrl } from './api';

export function lazyLoad(
  node: HTMLElement,
  {
    rootMargin = '150px',
    reducedMotion = false,
  }: { rootMargin?: string; reducedMotion?: boolean } = {}
) {
  let imageAnimation: { cancel?: () => void; stop?: () => void } | null = null;
  let placeholderAnimation: { cancel?: () => void; stop?: () => void } | null =
    null;
  let loadSeq = 0;

  const stopAnimations = () => {
    imageAnimation?.cancel?.();
    imageAnimation?.stop?.();
    placeholderAnimation?.cancel?.();
    placeholderAnimation?.stop?.();
  };

  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) {
          return;
        }

        const src = node.dataset.src;
        if (!src) {
          observer.unobserve(node);
          return;
        }

        const img = node.querySelector('img');
        const placeholder = node.querySelector<HTMLElement>(
          '.album-cover-placeholder'
        );
        if (!img) {
          observer.unobserve(node);
          return;
        }

        const seq = ++loadSeq;

        void (async () => {
          try {
            const resolvedSrc = await getImageDataUrl(src);
            if (seq !== loadSeq) return;

            img.style.opacity = '0';
            img.style.transform = reducedMotion ? 'scale(1)' : 'scale(1.04)';
            img.onload = () => {
              stopAnimations();
              if (placeholder) {
                placeholderAnimation = animate(
                  placeholder,
                  { opacity: 0 },
                  { duration: reducedMotion ? 0 : 0.18, ease: 'easeOut' }
                );
              }
              imageAnimation = animate(
                img,
                { opacity: 1, scale: 1 },
                { duration: reducedMotion ? 0 : 0.2, ease: 'easeOut' }
              );
            };
            img.onerror = () => {
              stopAnimations();
              if (placeholder) {
                placeholder.style.opacity = '1';
              }
              console.warn(`Failed to load cover: ${src}`);
            };
            img.src = resolvedSrc;
          } catch (error) {
            stopAnimations();
            if (placeholder) {
              placeholder.style.opacity = '1';
            }
            console.warn(`Failed to resolve cover: ${src}`, error);
          } finally {
            if (seq === loadSeq) {
              node.removeAttribute('data-src');
              observer.unobserve(node);
            }
          }
        })();
      });
    },
    { rootMargin, threshold: 0 }
  );

  observer.observe(node);

  return {
    update(next: { rootMargin?: string; reducedMotion?: boolean } = {}) {
      reducedMotion = next.reducedMotion ?? false;
    },
    destroy() {
      stopAnimations();
      observer.disconnect();
    },
  };
}
