import type { DownloadTaskSnapshot } from '$lib/types';

export function buildSelectionKey(songCids: string[]): string {
  return [...songCids].sort().join(',');
}

export function formatByteSize(bytes: number | null | undefined): string {
  if (
    bytes === null ||
    bytes === undefined ||
    !Number.isFinite(bytes) ||
    bytes < 0
  ) {
    return '未知大小';
  }

  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let unitIndex = -1;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  const precision = value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(precision)} ${units[unitIndex]}`;
}

export function formatSpeed(bytesPerSec: number): string {
  if (!Number.isFinite(bytesPerSec) || bytesPerSec < 0) {
    return '未知速度';
  }

  if (bytesPerSec < 1024) return `${bytesPerSec.toFixed(0)} B/s`;
  const units = ['KB/s', 'MB/s', 'GB/s'];
  let value = bytesPerSec;
  let unitIndex = -1;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  const precision = value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(precision)} ${units[unitIndex]}`;
}

export function getTaskStatusLabel(task: DownloadTaskSnapshot): string {
  switch (task.status) {
    case 'queued':
      return '排队中';
    case 'preparing':
      return '准备中';
    case 'downloading':
      return '下载中...';
    case 'writing':
      return '写入中';
    case 'completed':
      return '已完成';
    case 'failed':
      return '失败';
    case 'cancelled':
      return '已取消';
    default:
      return task.status;
  }
}
