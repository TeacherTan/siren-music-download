import type { ThemePalette } from './types';

export const DEFAULT_THEME_PALETTE: ThemePalette = {
  accentHex: '#fa2d48',
  accentHoverHex: '#ff3b5c',
  accentRgb: [250, 45, 72],
  accentHoverRgb: [255, 59, 92],
};

export function applyThemePalette(
  palette: ThemePalette = DEFAULT_THEME_PALETTE
): void {
  const root = document.documentElement;

  root.style.setProperty('--accent', palette.accentHex);
  root.style.setProperty('--accent-hover', palette.accentHoverHex);
  root.style.setProperty('--accent-rgb', palette.accentRgb.join(', '));
  root.style.setProperty(
    '--accent-hover-rgb',
    palette.accentHoverRgb.join(', ')
  );
}
