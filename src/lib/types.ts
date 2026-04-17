export interface Album {
  cid: string;
  name: string;
  coverUrl: string;
  artists: string[];
}

export interface SongEntry {
  cid: string;
  name: string;
  artists: string[];
}

export interface PlaybackQueueEntry {
  cid: string;
  name: string;
  artists: string[];
  coverUrl: string | null;
}

export interface PlaybackContext {
  entries: PlaybackQueueEntry[];
  currentIndex: number;
}

export interface SongDetail {
  cid: string;
  name: string;
  albumCid: string;
  sourceUrl: string;
  lyricUrl: string | null;
  mvUrl: string | null;
  mvCoverUrl: string | null;
  artists: string[];
}

export interface AlbumDetail {
  cid: string;
  name: string;
  intro: string | null;
  belong: string;
  coverUrl: string;
  coverDeUrl: string | null;
  artists: string[] | null;
  songs: SongEntry[];
}

export interface ThemePalette {
  accentHex: string;
  accentHoverHex: string;
  accentRgb: [number, number, number];
  accentHoverRgb: [number, number, number];
}

export type OutputFormat = 'flac' | 'wav' | 'mp3';

// ---------------------------------------------------------------------------
// Download job types (mirrors siren-core/src/download/model.rs)
// ---------------------------------------------------------------------------

export type DownloadJobKind = 'song' | 'album' | 'selection';

export type DownloadJobStatus =
  | 'queued'
  | 'running'
  | 'completed'
  | 'partiallyFailed'
  | 'failed'
  | 'cancelled';

export type DownloadTaskStatus =
  | 'queued'
  | 'preparing'
  | 'downloading'
  | 'writing'
  | 'completed'
  | 'failed'
  | 'cancelled';

export type DownloadErrorCode =
  | 'network'
  | 'api'
  | 'io'
  | 'decode'
  | 'tagging'
  | 'lyrics'
  | 'cancelled'
  | 'invalidRequest'
  | 'internal';

export interface DownloadErrorInfo {
  code: DownloadErrorCode;
  message: string;
  retryable: boolean;
  details: string | null;
}

export interface DownloadOptions {
  outputDir: string;
  format: OutputFormat;
  downloadLyrics: boolean;
}

export interface DownloadTaskSnapshot {
  id: string;
  jobId: string;
  songCid: string;
  songName: string;
  artists: string[];
  albumCid: string;
  albumName: string;
  status: DownloadTaskStatus;
  bytesDone: number;
  bytesTotal: number | null;
  outputPath: string | null;
  error: DownloadErrorInfo | null;
  attempt: number;
  songIndex: number;
  songCount: number;
}

export interface DownloadJobSnapshot {
  id: string;
  kind: DownloadJobKind;
  status: DownloadJobStatus;
  createdAt: string;
  startedAt: string | null;
  finishedAt: string | null;
  options: DownloadOptions;
  title: string;
  taskCount: number;
  completedTaskCount: number;
  failedTaskCount: number;
  cancelledTaskCount: number;
  tasks: DownloadTaskSnapshot[];
  error: DownloadErrorInfo | null;
}

export interface DownloadManagerSnapshot {
  jobs: DownloadJobSnapshot[];
  activeJobId: string | null;
  queuedJobIds: string[];
}

export interface DownloadTaskProgressEvent {
  jobId: string;
  taskId: string;
  status: DownloadTaskStatus;
  bytesDone: number;
  bytesTotal: number | null;
  songIndex: number;
  songCount: number;
  speedBytesPerSec: number;
}

export interface CreateDownloadJobRequest {
  kind: DownloadJobKind;
  songCids: string[];
  albumCid: string | null;
  options: DownloadOptions;
}

export interface PlayerState {
  songCid: string | null;
  songName: string | null;
  artists: string[];
  coverUrl: string | null;
  isPlaying: boolean;
  isPaused: boolean;
  isLoading: boolean;
  hasPrevious: boolean;
  hasNext: boolean;
  progress: number;
  duration: number;
  volume: number;
}

export interface NotificationPreferences {
  notifyOnDownloadComplete: boolean;
  notifyOnPlaybackChange: boolean;
}

export interface AppPreferences {
  outputFormat: OutputFormat;
  outputDir: string;
  downloadLyrics: boolean;
  notifyOnDownloadComplete: boolean;
  notifyOnPlaybackChange: boolean;
}

export type NotificationPermissionState =
  | 'granted'
  | 'denied'
  | 'prompt'
  | 'prompt-with-rationale';
