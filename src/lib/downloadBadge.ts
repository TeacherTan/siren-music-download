import type { LocalTrackDownloadStatus } from './types';

export function shouldShowDownloadBadge(status: LocalTrackDownloadStatus): boolean {
  return status !== 'missing' && status !== 'unknown';
}

export function shouldShowAlbumListDownloadBadge(
  status: LocalTrackDownloadStatus
): boolean {
  return status !== 'partial' && shouldShowDownloadBadge(status);
}

export function getDownloadBadgeLabel(status: LocalTrackDownloadStatus): string {
  switch (status) {
    case 'verified':
      return '已校验';
    case 'mismatch':
      return '校验异常';
    case 'partial':
      return '部分下载';
    case 'unverifiable':
      return '不可校验';
    case 'detected':
      return '已检测到';
    default:
      return '未下载';
  }
}
