export const surfaceTokens = {
  window: 'bg-[var(--surface-window)]',
  sidebar: 'bg-[var(--surface-sidebar)]',
  workspace: 'bg-[var(--surface-workspace)]',
  sheet: 'bg-[var(--surface-sheet)]',
  dock: 'bg-[var(--surface-dock)]',
  flyout: 'bg-[var(--surface-flyout)]',
  state: 'bg-[var(--surface-state)]',
} as const;

export const textTokens = {
  primary: 'text-[var(--text-primary)]',
  secondary: 'text-[var(--text-secondary)]',
  tertiary: 'text-[var(--text-tertiary)]',
} as const;

export const materialTokens = {
  hairline: 'border border-white/40',
  glass: 'backdrop-blur-xl',
  glassSoft: 'backdrop-blur-lg',
} as const;
