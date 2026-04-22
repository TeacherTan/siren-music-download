export type LocalTrackDownloadStatus =
  | 'missing'
  | 'detected'
  | 'verified'
  | 'mismatch'
  | 'partial'
  | 'unverifiable'
  | 'unknown';

export interface TrackDownloadBadge {
  isDownloaded: boolean;
  downloadStatus: LocalTrackDownloadStatus;
  inventoryVersion: string;
}

export type LocalInventoryStatus = 'idle' | 'scanning' | 'completed' | 'failed';

export type VerificationMode = 'none' | 'whenAvailable' | 'strict';

export interface LocalInventorySnapshot {
  rootOutputDir: string;
  status: LocalInventoryStatus;
  inventoryVersion: string;
  startedAt: string | null;
  finishedAt: string | null;
  scannedFileCount: number;
  matchedTrackCount: number;
  verifiedTrackCount: number;
  lastError: string | null;
}

export interface LocalInventoryScanProgressEvent {
  rootOutputDir: string;
  inventoryVersion: string;
  filesScanned: number;
  matchedTrackCount: number;
  verifiedTrackCount: number;
  currentPath: string | null;
}

export interface AlbumDownloadBadge {
  isDownloaded: boolean;
  downloadStatus: LocalTrackDownloadStatus;
  inventoryVersion: string;
}

export interface Album {
  cid: string;
  name: string;
  coverUrl: string;
  artists: string[];
  download: AlbumDownloadBadge;
}

export interface SongEntry {
  cid: string;
  name: string;
  artists: string[];
  download: TrackDownloadBadge;
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
  download: TrackDownloadBadge;
}

export interface AlbumDetail {
  cid: string;
  name: string;
  intro: string | null;
  belong: string;
  coverUrl: string;
  coverDeUrl: string | null;
  artists: string[] | null;
  download: AlbumDownloadBadge;
  songs: SongEntry[];
}

export type LibrarySearchScope = 'all' | 'albums' | 'songs';

export type LibrarySearchHitField = 'title' | 'artist';

export type LibraryIndexState = 'notReady' | 'building' | 'stale' | 'ready';

export type SearchLibraryResultKind = 'album' | 'song';

export interface SearchLibraryRequest {
  query: string;
  scope: LibrarySearchScope;
  limit?: number;
  offset?: number;
}

export interface SearchLibraryResultItem {
  kind: SearchLibraryResultKind;
  albumCid: string;
  songCid: string | null;
  albumTitle: string;
  songTitle: string | null;
  artistLine: string | null;
  matchedFields: LibrarySearchHitField[];
}

export interface SearchLibraryResponse {
  items: SearchLibraryResultItem[];
  total: number;
  query: string;
  scope: LibrarySearchScope;
  indexState: LibraryIndexState;
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

export type DownloadHistoryScopeFilter = 'all' | 'active' | 'history';

export type DownloadHistoryStatusFilter = 'all' | DownloadJobStatus;

export type DownloadHistoryKindFilter = 'all' | DownloadJobKind;

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

export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

export interface AppPreferences {
  outputFormat: OutputFormat;
  outputDir: string;
  downloadLyrics: boolean;
  notifyOnDownloadComplete: boolean;
  notifyOnPlaybackChange: boolean;
  logLevel: LogLevel;
}

export type AppErrorLevel = 'warn' | 'error';

export interface AppErrorEvent {
  id: string;
  ts: string;
  level: AppErrorLevel;
  domain: string;
  code: string;
  message: string;
}

export type LogFileKind = 'session' | 'persistent';

export interface LogViewerQuery {
  kind: LogFileKind;
  level?: LogLevel | null;
  domain?: string | null;
  search?: string | null;
  limit?: number | null;
  offset?: number | null;
}

export interface LogViewerRecord {
  id: string;
  ts: string;
  level: LogLevel;
  domain: string;
  code: string;
  message: string;
  details: string | null;
}

export interface LogViewerPage {
  records: LogViewerRecord[];
  total: number;
  kind: LogFileKind;
}

export interface LogFileStatus {
  hasSessionLog: boolean;
  hasPersistentLog: boolean;
}

export type NotificationPermissionState =
  | 'granted'
  | 'denied'
  | 'prompt'
  | 'prompt-with-rationale';
