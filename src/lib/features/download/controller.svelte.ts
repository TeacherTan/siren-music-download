import type {
  CreateDownloadJobRequest,
  DownloadHistoryKindFilter,
  DownloadHistoryScopeFilter,
  DownloadHistoryStatusFilter,
  DownloadJobSnapshot,
  DownloadManagerSnapshot,
  DownloadTaskProgressEvent,
  DownloadTaskSnapshot,
  OutputFormat,
} from '$lib/types';
import { buildSelectionKey, formatByteSize, formatSpeed } from './formatters';
import {
  hasCurrentDownloadOptions,
  isJobActive as isActiveDownloadJob,
  isTerminalJob,
  matchesJobKindFilter,
  matchesJobScopeFilter,
  matchesJobSearch,
  matchesJobStatusFilter,
  sortDownloadJobs,
} from './guards';

interface DownloadControllerDeps {
  createDownloadJob: (
    request: CreateDownloadJobRequest
  ) => Promise<DownloadJobSnapshot>;
  cancelDownloadJob: (jobId: string) => Promise<DownloadJobSnapshot | null>;
  cancelDownloadTask: (
    jobId: string,
    taskId: string
  ) => Promise<DownloadJobSnapshot | null>;
  retryDownloadJob: (jobId: string) => Promise<DownloadJobSnapshot | null>;
  retryDownloadTask: (
    jobId: string,
    taskId: string
  ) => Promise<DownloadJobSnapshot | null>;
  clearDownloadHistory: () => Promise<number>;
  openDownloadPanel: (resetFilters?: boolean) => Promise<void>;
  getDownloadOptions: () => {
    outputDir: string;
    format: OutputFormat;
    downloadLyrics: boolean;
  };
  notifyInfo: (message: string) => void;
  notifyError: (message: string) => void;
}

type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';

let initialized = false;

export function createDownloadController(deps: DownloadControllerDeps) {
  let manager = $state<DownloadManagerSnapshot | null>(null);
  let downloadingSongCid = $state<string | null>(null);
  let downloadingAlbumCid = $state<string | null>(null);
  let creatingSelectionKey = $state<string | null>(null);
  let searchQuery = $state('');
  let scopeFilter = $state<DownloadHistoryScopeFilter>('all');
  let statusFilter = $state<DownloadHistoryStatusFilter>('all');
  let kindFilter = $state<DownloadHistoryKindFilter>('all');
  let taskSpeedMap = $state<Map<string, number>>(new Map());
  let managerInitSeq = 0;
  let managerHydratedFromEvent = false;

  function init() {
    if (initialized) return;
    initialized = true;
  }

  function beginHydrationAttempt(): number {
    return ++managerInitSeq;
  }

  function applyManagerSnapshot(
    snapshot: DownloadManagerSnapshot,
    requestSeq: number
  ) {
    if (requestSeq !== managerInitSeq || managerHydratedFromEvent) {
      return;
    }
    manager = snapshot;
  }

  function applyManagerEvent(snapshot: DownloadManagerSnapshot) {
    managerHydratedFromEvent = true;
    manager = snapshot;
  }

  function applyJobUpdate(job: DownloadJobSnapshot) {
    if (!manager) return;
    const jobs = manager.jobs.map((candidate) =>
      candidate.id === job.id ? job : candidate
    );
    manager = { ...manager, jobs };
  }

  function applyTaskProgress(event: DownloadTaskProgressEvent) {
    if (!manager) return;

    taskSpeedMap = new Map(taskSpeedMap).set(
      event.taskId,
      event.speedBytesPerSec
    );

    const jobIdx = manager.jobs.findIndex((job) => job.id === event.jobId);
    if (jobIdx < 0) return;

    const job = manager.jobs[jobIdx];
    const taskIdx = job.tasks.findIndex((task) => task.id === event.taskId);
    if (taskIdx < 0) return;

    const updatedTasks = [...job.tasks];
    updatedTasks[taskIdx] = { ...updatedTasks[taskIdx], ...event };
    const updatedJobs = [...manager.jobs];
    updatedJobs[jobIdx] = { ...job, tasks: updatedTasks };
    manager = { ...manager, jobs: updatedJobs };
  }

  function resetFilters() {
    searchQuery = '';
    scopeFilter = 'all';
    statusFilter = 'all';
    kindFilter = 'all';
  }

  function getFilteredJobs(): DownloadJobSnapshot[] {
    const normalizedQuery = searchQuery.trim().toLocaleLowerCase();
    return sortDownloadJobs(manager?.jobs ?? []).filter((job) => {
      return (
        matchesJobSearch(job, normalizedQuery) &&
        matchesJobScopeFilter(job, scopeFilter) &&
        matchesJobStatusFilter(job, statusFilter) &&
        matchesJobKindFilter(job, kindFilter)
      );
    });
  }

  function getDownloadOptions() {
    return deps.getDownloadOptions();
  }

  function findSelectionDownloadJob(
    songCids: string[]
  ): DownloadJobSnapshot | null {
    if (!manager || songCids.length === 0) return null;

    const options = getDownloadOptions();
    const targetKey = buildSelectionKey(songCids);
    return (
      manager.jobs.find((job) => {
        if (job.kind !== 'selection') return false;
        if (!isActiveDownloadJob(job)) return false;
        if (
          !hasCurrentDownloadOptions(
            job,
            options.outputDir,
            options.format,
            options.downloadLyrics
          )
        ) {
          return false;
        }
        return (
          buildSelectionKey(job.tasks.map((task) => task.songCid)) === targetKey
        );
      }) ?? null
    );
  }

  function getCurrentSelectionKey(songCids: string[]): string | null {
    return songCids.length > 0 ? buildSelectionKey(songCids) : null;
  }

  function isCurrentSelectionCreating(songCids: string[]): boolean {
    const selectionKey = getCurrentSelectionKey(songCids);
    return selectionKey !== null && creatingSelectionKey === selectionKey;
  }

  function getCurrentSelectionJob(
    songCids: string[]
  ): DownloadJobSnapshot | null {
    return findSelectionDownloadJob(songCids);
  }

  function isSelectionDownloadActionDisabled(songCids: string[]): boolean {
    return (
      songCids.length === 0 ||
      isCurrentSelectionCreating(songCids) ||
      !!getCurrentSelectionJob(songCids)
    );
  }

  function findAlbumDownloadJob(albumCid: string): DownloadJobSnapshot | null {
    if (!manager) return null;

    const options = getDownloadOptions();
    return (
      manager.jobs.find((job) => {
        if (job.kind !== 'album') return false;
        if (!isActiveDownloadJob(job)) return false;
        if (
          !hasCurrentDownloadOptions(
            job,
            options.outputDir,
            options.format,
            options.downloadLyrics
          )
        ) {
          return false;
        }
        return job.tasks.some((task) => task.albumCid === albumCid);
      }) ?? null
    );
  }

  function findSongDownloadTask(songCid: string): DownloadTaskSnapshot | null {
    if (!manager) return null;

    for (const job of manager.jobs) {
      if (!isActiveDownloadJob(job)) continue;
      const task = job.tasks.find((candidate) => candidate.songCid === songCid);
      if (task) return task;
    }

    return null;
  }

  function isSongDownloadInteractionBlocked(songCid: string): boolean {
    return downloadingSongCid !== null && downloadingSongCid !== songCid;
  }

  function getSongDownloadState(songCid: string): SongDownloadState {
    if (downloadingSongCid === songCid) {
      return 'creating';
    }

    const task = findSongDownloadTask(songCid);
    if (!task) {
      return 'idle';
    }

    if (task.status === 'queued') {
      return 'queued';
    }

    if (
      task.status === 'preparing' ||
      task.status === 'downloading' ||
      task.status === 'writing'
    ) {
      return 'running';
    }

    return 'idle';
  }

  function getSongDownloadJob(songCid: string): DownloadJobSnapshot | null {
    if (!manager) return null;

    const options = getDownloadOptions();
    return (
      manager.jobs.find(
        (job) =>
          isActiveDownloadJob(job) &&
          hasCurrentDownloadOptions(
            job,
            options.outputDir,
            options.format,
            options.downloadLyrics
          ) &&
          job.tasks.some((task) => task.songCid === songCid)
      ) ?? null
    );
  }

  async function performSongDownload(songCid: string): Promise<string | null> {
    const existingJob = getSongDownloadJob(songCid);
    if (existingJob) {
      await deps.openDownloadPanel();
      return existingJob.id;
    }

    if (downloadingSongCid) return null;

    const options = getDownloadOptions();
    downloadingSongCid = songCid;
    try {
      const request: CreateDownloadJobRequest = {
        kind: 'song',
        songCids: [songCid],
        albumCid: null,
        options,
      };
      const job = await deps.createDownloadJob(request);
      await deps.openDownloadPanel(true);
      return job.id;
    } finally {
      if (downloadingSongCid === songCid) {
        downloadingSongCid = null;
      }
    }
  }

  async function handleSongDownload(songCid: string) {
    try {
      const existingJob = getSongDownloadJob(songCid);
      await performSongDownload(songCid);
      if (existingJob) {
        deps.notifyInfo('这首歌的下载任务已在队列中或正在执行。');
      }
    } catch (error) {
      deps.notifyError(
        `下载失败：${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  async function performAlbumDownload(
    albumCid: string
  ): Promise<string | null> {
    const existingJob = findAlbumDownloadJob(albumCid);
    if (existingJob) {
      await deps.openDownloadPanel();
      return existingJob.id;
    }

    if (downloadingAlbumCid === albumCid) {
      return null;
    }

    const options = getDownloadOptions();
    downloadingAlbumCid = albumCid;
    try {
      const request: CreateDownloadJobRequest = {
        kind: 'album',
        songCids: [],
        albumCid,
        options,
      };
      const job = await deps.createDownloadJob(request);
      await deps.openDownloadPanel(true);
      return job.id;
    } finally {
      if (downloadingAlbumCid === albumCid) {
        downloadingAlbumCid = null;
      }
    }
  }

  async function handleAlbumDownload(albumCid: string | null) {
    if (!albumCid) return;

    try {
      const existingJob = findAlbumDownloadJob(albumCid);
      await performAlbumDownload(albumCid);
      if (existingJob) {
        deps.notifyInfo('这张专辑的下载任务已在队列中或正在执行。');
      }
    } catch (error) {
      deps.notifyError(
        `整专下载失败：${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  async function performSelectionDownload(
    songCids: string[]
  ): Promise<string | null> {
    if (songCids.length === 0) return null;

    const existingJob = findSelectionDownloadJob(songCids);
    if (existingJob) {
      await deps.openDownloadPanel();
      return existingJob.id;
    }

    const selectionKey = buildSelectionKey(songCids);
    if (creatingSelectionKey === selectionKey) {
      return null;
    }

    const options = getDownloadOptions();
    creatingSelectionKey = selectionKey;
    try {
      const request: CreateDownloadJobRequest = {
        kind: 'selection',
        songCids,
        albumCid: null,
        options,
      };
      const job = await deps.createDownloadJob(request);
      await deps.openDownloadPanel(true);
      return job.id;
    } finally {
      if (creatingSelectionKey === selectionKey) {
        creatingSelectionKey = null;
      }
    }
  }

  async function handleSelectionDownload(
    songCids: string[],
    options?: { afterCreated?: () => void | Promise<void> }
  ) {
    if (songCids.length === 0) return;

    try {
      const existingJob = findSelectionDownloadJob(songCids);
      const jobId = await performSelectionDownload(songCids);
      if (existingJob) {
        deps.notifyInfo('这组歌曲的下载任务已在队列中或正在执行。');
        return;
      }
      if (jobId) {
        await options?.afterCreated?.();
      }
    } catch (error) {
      deps.notifyError(
        `批量下载失败：${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  function getTaskProgressLabel(task: DownloadTaskSnapshot): string | null {
    if (task.status !== 'downloading' && task.status !== 'writing') {
      return null;
    }

    if (
      task.status === 'downloading' &&
      task.bytesTotal &&
      task.bytesTotal > 0
    ) {
      const percent = Math.min(
        Math.round((task.bytesDone / task.bytesTotal) * 100),
        100
      );
      const speed = taskSpeedMap.get(task.id);
      const speedText = speed && speed > 0 ? ` · ${formatSpeed(speed)}` : '';
      return `${formatByteSize(task.bytesDone)} / ${formatByteSize(task.bytesTotal)} · ${percent}%${speedText}`;
    }

    if (task.bytesDone > 0) {
      return `${formatByteSize(task.bytesDone)} 已处理`;
    }

    return task.status === 'writing' ? '正在整理文件...' : '正在接收数据...';
  }

  function getTaskErrorLabel(task: DownloadTaskSnapshot): string | null {
    if (!task.error) return null;

    if (task.error.details && task.error.details !== task.error.message) {
      return `${task.error.message} · ${task.error.details}`;
    }

    return task.error.message;
  }

  function getJobErrorSummary(job: DownloadJobSnapshot): string | null {
    const firstFailedTask = job.tasks.find(
      (task) => task.status === 'failed' && task.error
    );
    if (firstFailedTask) {
      return getTaskErrorLabel(firstFailedTask);
    }

    const firstCancelledTask = job.tasks.find(
      (task) => task.status === 'cancelled' && task.error
    );
    if (firstCancelledTask) {
      return getTaskErrorLabel(firstCancelledTask);
    }

    if (!job.error) return null;

    if (job.error.details && job.error.details !== job.error.message) {
      return `${job.error.message} · ${job.error.details}`;
    }

    return job.error.message;
  }

  function getJobProgressText(job: DownloadJobSnapshot): string {
    const terminalCount =
      job.completedTaskCount + job.failedTaskCount + job.cancelledTaskCount;
    const activeTask = job.tasks.find(
      (task) =>
        task.status === 'preparing' ||
        task.status === 'downloading' ||
        task.status === 'writing'
    );

    const base = `${terminalCount}/${job.taskCount} 首已结束`;
    if (!activeTask) {
      return base;
    }

    const progressLabel = getTaskProgressLabel(activeTask);
    if (!progressLabel) {
      return `${base} · 正在处理 ${activeTask.songName}`;
    }

    return `${base} · ${activeTask.songName} · ${progressLabel}`;
  }

  function getJobProgress(job: DownloadJobSnapshot): number {
    if (job.taskCount === 0) return 0;

    const terminalCount =
      job.completedTaskCount + job.failedTaskCount + job.cancelledTaskCount;
    const activeTask = job.tasks.find(
      (task) =>
        task.status === 'preparing' ||
        task.status === 'downloading' ||
        task.status === 'writing'
    );

    if (!activeTask) {
      return terminalCount / job.taskCount;
    }

    const activeTaskProgress =
      activeTask.status === 'downloading' && activeTask.bytesTotal
        ? activeTask.bytesDone / activeTask.bytesTotal
        : activeTask.status === 'writing'
          ? 1
          : 0;

    return Math.min((terminalCount + activeTaskProgress) / job.taskCount, 1);
  }

  function getJobStatusLabel(job: DownloadJobSnapshot): string {
    switch (job.status) {
      case 'queued':
        return '排队中';
      case 'running': {
        const activeTask = job.tasks.find(
          (task) =>
            task.status === 'preparing' ||
            task.status === 'downloading' ||
            task.status === 'writing'
        );
        const currentIndex = activeTask
          ? activeTask.songIndex + 1
          : job.completedTaskCount;
        return `下载中 (${currentIndex}/${job.taskCount})`;
      }
      case 'completed':
        return '已完成';
      case 'partiallyFailed':
        return `部分失败 (${job.failedTaskCount}/${job.taskCount})`;
      case 'failed':
        return '失败';
      case 'cancelled':
        return '已取消';
      default:
        return job.status;
    }
  }

  function getTaskStatusLabel(task: DownloadTaskSnapshot): string {
    switch (task.status) {
      case 'queued':
        return '排队中';
      case 'preparing':
        return '准备中';
      case 'downloading': {
        const progressLabel = getTaskProgressLabel(task);
        return progressLabel ?? '下载中...';
      }
      case 'writing': {
        const progressLabel = getTaskProgressLabel(task);
        return progressLabel ? `写入中 · ${progressLabel}` : '写入中';
      }
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

  function getJobKindLabel(job: DownloadJobSnapshot): string {
    switch (job.kind) {
      case 'song':
        return '单曲下载';
      case 'album':
        return '整专下载';
      case 'selection':
        return '多选下载';
      default:
        return job.kind;
    }
  }

  function getSelectionJobAlbumCount(job: DownloadJobSnapshot): number {
    return new Set(job.tasks.map((task) => task.albumCid)).size;
  }

  function getSelectionJobScopeLabel(job: DownloadJobSnapshot): string {
    const albumCount = getSelectionJobAlbumCount(job);
    if (albumCount <= 1) {
      const albumName = job.tasks[0]?.albumName;
      return albumName ? `来自《${albumName}》` : '来自同一张专辑';
    }

    return `跨 ${albumCount} 张专辑`;
  }

  function getJobSummaryLabel(job: DownloadJobSnapshot): string {
    switch (job.kind) {
      case 'song': {
        const task = job.tasks[0];
        return task?.albumName ? `来自《${task.albumName}》` : '单曲任务';
      }
      case 'album':
        return `${job.taskCount} 首歌曲`;
      case 'selection': {
        if (job.taskCount <= 1) {
          return getSelectionJobScopeLabel(job);
        }

        const albumCount = getSelectionJobAlbumCount(job);
        if (albumCount <= 1) {
          return `${job.taskCount} 首歌曲`;
        }

        return `${job.taskCount} 首歌曲 · 跨 ${albumCount} 张专辑`;
      }
      default:
        return `${job.taskCount} 首歌曲`;
    }
  }

  function isJobActive(jobId: string): boolean {
    return manager?.activeJobId === jobId;
  }

  function canCancelTask(task: DownloadTaskSnapshot): boolean {
    return (
      task.status === 'queued' ||
      task.status === 'preparing' ||
      task.status === 'downloading' ||
      task.status === 'writing'
    );
  }

  function canRetryTask(task: DownloadTaskSnapshot): boolean {
    return task.status === 'failed' || task.status === 'cancelled';
  }

  function canClearDownloadHistory(): boolean {
    return !!manager?.jobs.some((job) => isTerminalJob(job));
  }

  async function handleCancelDownloadJob(jobId: string) {
    try {
      await deps.cancelDownloadJob(jobId);
    } catch (error) {
      deps.notifyError(
        `取消下载任务失败：${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  async function handleCancelDownloadTask(jobId: string, taskId: string) {
    try {
      await deps.cancelDownloadTask(jobId, taskId);
    } catch (error) {
      deps.notifyError(
        `取消下载子任务失败：${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  async function handleRetryDownloadJob(jobId: string) {
    try {
      await deps.retryDownloadJob(jobId);
    } catch (error) {
      deps.notifyError(
        `重试下载任务失败：${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  async function handleRetryDownloadTask(jobId: string, taskId: string) {
    try {
      await deps.retryDownloadTask(jobId, taskId);
    } catch (error) {
      deps.notifyError(
        `重试下载子任务失败：${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  async function handleClearDownloadHistory() {
    try {
      const removed = await deps.clearDownloadHistory();
      if (removed === 0) {
        deps.notifyInfo('当前没有可清理的下载历史。');
      }
    } catch (error) {
      deps.notifyError(
        `清理下载历史失败：${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  async function openPanel(resetFilters = false) {
    await deps.openDownloadPanel(resetFilters);
  }

  function dispose() {
    initialized = false;
    manager = null;
    downloadingSongCid = null;
    downloadingAlbumCid = null;
    creatingSelectionKey = null;
    searchQuery = '';
    scopeFilter = 'all';
    statusFilter = 'all';
    kindFilter = 'all';
    taskSpeedMap = new Map();
    managerInitSeq += 1;
    managerHydratedFromEvent = false;
  }

  return {
    get manager() {
      return manager;
    },
    get searchQuery() {
      return searchQuery;
    },
    set searchQuery(value: string) {
      searchQuery = value;
    },
    get scopeFilter() {
      return scopeFilter;
    },
    set scopeFilter(value: DownloadHistoryScopeFilter) {
      scopeFilter = value;
    },
    get statusFilter() {
      return statusFilter;
    },
    set statusFilter(value: DownloadHistoryStatusFilter) {
      statusFilter = value;
    },
    get kindFilter() {
      return kindFilter;
    },
    set kindFilter(value: DownloadHistoryKindFilter) {
      kindFilter = value;
    },
    get downloadingSongCid() {
      return downloadingSongCid;
    },
    get downloadingAlbumCid() {
      return downloadingAlbumCid;
    },
    get activeDownloadCount() {
      return manager
        ? manager.jobs.filter((job) => isActiveDownloadJob(job)).length
        : 0;
    },
    get filteredJobs() {
      return getFilteredJobs();
    },
    get hasDownloadHistory() {
      return (manager?.jobs.length ?? 0) > 0;
    },
    init,
    dispose,
    beginHydrationAttempt,
    applyManagerSnapshot,
    applyManagerEvent,
    applyJobUpdate,
    applyTaskProgress,
    resetFilters,
    findSelectionDownloadJob,
    getCurrentSelectionKey,
    isCurrentSelectionCreating,
    getCurrentSelectionJob,
    isSelectionDownloadActionDisabled,
    findAlbumDownloadJob,
    findSongDownloadTask,
    isSongDownloadInteractionBlocked,
    getSongDownloadState,
    getSongDownloadJob,
    handleSongDownload,
    handleAlbumDownload,
    handleSelectionDownload,
    getTaskErrorLabel,
    getJobErrorSummary,
    getJobProgressText,
    getJobProgress,
    getJobStatusLabel,
    getTaskStatusLabel,
    getJobKindLabel,
    getJobSummaryLabel,
    isJobActive,
    canCancelTask,
    canRetryTask,
    canClearDownloadHistory,
    handleCancelDownloadJob,
    handleCancelDownloadTask,
    handleRetryDownloadJob,
    handleRetryDownloadTask,
    handleClearDownloadHistory,
    openPanel,
  };
}

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    initialized = false;
  });
}
