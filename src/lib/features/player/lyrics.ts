export type LyricLine = {
  id: string;
  time: number | null;
  text: string;
};

export function parseLyricText(source: string): LyricLine[] {
  const lines = source
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  const parsed: LyricLine[] = [];

  for (const rawLine of lines) {
    const matches = [
      ...rawLine.matchAll(/\[(\d{1,2}):(\d{2})(?:\.(\d{1,3}))?\]/g),
    ];
    const text =
      rawLine.replace(/\[(\d{1,2}):(\d{2})(?:\.(\d{1,3}))?\]/g, '').trim() ||
      '♪';

    if (!matches.length) {
      parsed.push({
        id: `plain-${parsed.length}`,
        time: null,
        text,
      });
      continue;
    }

    for (const match of matches) {
      const minutes = Number(match[1]);
      const seconds = Number(match[2]);
      // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- regex group 3 is optional
      const fractionText = match[3] ?? '0';
      const fraction = Number(`0.${fractionText.padEnd(3, '0')}`);
      parsed.push({
        id: `${minutes}:${seconds}:${fractionText}:${parsed.length}`,
        time: minutes * 60 + seconds + fraction,
        text,
      });
    }
  }

  return parsed.sort((left, right) => {
    if (left.time === null && right.time === null) return 0;
    if (left.time === null) return 1;
    if (right.time === null) return -1;
    return left.time - right.time;
  });
}
