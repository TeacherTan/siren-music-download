export function lazyLoad(
  node: HTMLElement,
  { rootMargin = '150px' }: { rootMargin?: string } = {}
) {
  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          const src = node.dataset.src;
          if (src) {
            const img = node.querySelector('img');
            if (img) {
              img.src = src;
              img.onload = () => node.classList.add('loaded');
              img.onerror = () => {
                node.classList.remove('loaded');
                console.warn(`Failed to load cover: ${src}`);
              };
            }
            node.removeAttribute('data-src');
          }
          observer.unobserve(node);
        }
      });
    },
    { rootMargin, threshold: 0 }
  );

  observer.observe(node);

  return {
    destroy() {
      observer.disconnect();
    },
  };
}
