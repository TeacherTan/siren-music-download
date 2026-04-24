import { tv } from 'tailwind-variants';

export const toolbarIconButton = tv({
  base: 'inline-flex items-center justify-center rounded-full border border-transparent text-[var(--text-primary)] transition-colors',
  variants: {
    active: {
      true: 'bg-[var(--surface-state)] text-[var(--accent)]',
      false: 'bg-transparent hover:bg-[var(--surface-state)]',
    },
  },
});

export const sheetSurface = tv({
  base: 'backdrop-blur-xl border border-white/40 shadow-[0_24px_64px_rgba(15,23,42,0.16)]',
});

export const tonalBadge = tv({
  base: 'inline-flex items-center rounded-full px-2.5 py-1 text-[11px] font-medium text-[var(--text-secondary)] bg-[var(--surface-state)]',
});
