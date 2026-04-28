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
import { SvelteMap } from 'svelte/reactivity';
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
import * as m from '$lib/paraglide/messages.js';

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
  const taskSpeedMap = new SvelteMap<string, number>();
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

    taskSpeedMap.set(event.taskId, event.speedBytesPerSec);

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
        deps.notifyInfo(m.download_notify_song_exists());
      }
    } catch (error) {
      deps.notifyError(
        m.download_error_song_failed({
          error: error instanceof Error ? error.message : String(error),
        })
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
        deps.notifyInfo(m.download_notify_album_exists());
      }
    } catch (error) {
      deps.notifyError(
        m.download_error_album_failed({
          error: error instanceof Error ? error.message : String(error),
        })
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
        deps.notifyInfo(m.download_notify_selection_exists());
        return;
      }
      if (jobId) {
        await options?.afterCreated?.();
      }
    } catch (error) {
      deps.notifyError(
        m.download_error_selection_failed({
          error: error instanceof Error ? error.message : String(error),
        })
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
      return m.download_progress_bytes_processed({
        size: formatByteSize(task.bytesDone),
      });
    }

    return task.status === 'writing'
      ? m.download_progress_writing_file()
      : m.download_progress_receiving_data();
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

    const base = m.download_progress_terminal_count({
      done: terminalCount,
      total: job.taskCount,
    });
    if (!activeTask) {
      return base;
    }

    const progressLabel = getTaskProgressLabel(activeTask);
    if (!progressLabel) {
      return `${base} · ${m.download_progress_processing({ name: activeTask.songName })}`;
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
        return m.download_job_status_queued();
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
        return m.download_job_status_running({
          current: currentIndex,
          total: job.taskCount,
        });
      }
      case 'completed':
        return m.download_job_status_completed();
      case 'partiallyFailed':
        return m.download_job_status_partially_failed({
          failed: job.failedTaskCount,
          total: job.taskCount,
        });
      case 'failed':
        return m.download_job_status_failed();
      case 'cancelled':
        return m.download_job_status_cancelled();
      default:
        return job.status;
    }
  }

  function getTaskStatusLabel(task: DownloadTaskSnapshot): string {
    switch (task.status) {
      case 'queued':
        return m.download_job_task_queued();
      case 'preparing':
        return m.download_job_task_preparing();
      case 'downloading': {
        const progressLabel = getTaskProgressLabel(task);
        return progressLabel ?? m.download_job_task_downloading();
      }
      case 'writing': {
        const progressLabel = getTaskProgressLabel(task);
        return progressLabel
          ? m.download_job_task_writing_with_progress({
              progress: progressLabel,
            })
          : m.download_job_task_writing();
      }
      case 'completed':
        return m.download_job_task_completed();
      case 'failed':
        return m.download_job_task_failed();
      case 'cancelled':
        return m.download_job_task_cancelled();
      default:
        return task.status;
    }
  }

  function getJobKindLabel(job: DownloadJobSnapshot): string {
    switch (job.kind) {
      case 'song':
        return m.download_job_kind_song();
      case 'album':
        return m.download_job_kind_album();
      case 'selection':
        return m.download_job_kind_selection();
      default:
        return job.kind;
    }
  }

  function getSelectionJobAlbumCount(job: DownloadJobSnapshot): number {
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    return new Set(job.tasks.map((task) => task.albumCid)).size;
  }

  function getSelectionJobScopeLabel(job: DownloadJobSnapshot): string {
    const albumCount = getSelectionJobAlbumCount(job);
    if (albumCount <= 1) {
      const albumName = job.tasks[0]?.albumName;
      return albumName
        ? m.download_job_scope_from_album({ album: albumName })
        : m.download_job_scope_same_album();
    }

    return m.download_job_scope_cross_albums({ count: albumCount });
  }

  function getJobSummaryLabel(job: DownloadJobSnapshot): string {
    switch (job.kind) {
      case 'song': {
        const task = job.tasks[0];
        return task.albumName
          ? m.download_job_scope_from_album({ album: task.albumName })
          : m.download_job_summary_single_task();
      }
      case 'album':
        return m.download_job_summary_song_count({ count: job.taskCount });
      case 'selection': {
        if (job.taskCount <= 1) {
          return getSelectionJobScopeLabel(job);
        }

        const albumCount = getSelectionJobAlbumCount(job);
        if (albumCount <= 1) {
          return m.download_job_summary_song_count({ count: job.taskCount });
        }

        return m.download_job_summary_song_count_cross_albums({
          count: job.taskCount,
          albumCount,
        });
      }
      default:
        return m.download_job_summary_song_count({ count: job.taskCount });
    }
  }

  function getJobDisplayTitle(job: DownloadJobSnapshot): string {
    if (job.kind !== 'selection') {
      return job.title;
    }
    const albumCount = getSelectionJobAlbumCount(job);
    if (albumCount > 1) {
      return m.download_job_selection_title_cross_albums({
        count: job.taskCount,
        albumCount,
      });
    }
    return m.download_job_selection_title({ count: job.taskCount });
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
        m.download_error_cancel_job_failed({
          error: error instanceof Error ? error.message : String(error),
        })
      );
    }
  }

  async function handleCancelDownloadTask(jobId: string, taskId: string) {
    try {
      await deps.cancelDownloadTask(jobId, taskId);
    } catch (error) {
      deps.notifyError(
        m.download_error_cancel_task_failed({
          error: error instanceof Error ? error.message : String(error),
        })
      );
    }
  }

  async function handleRetryDownloadJob(jobId: string) {
    try {
      await deps.retryDownloadJob(jobId);
    } catch (error) {
      deps.notifyError(
        m.download_error_retry_job_failed({
          error: error instanceof Error ? error.message : String(error),
        })
      );
    }
  }

  async function handleRetryDownloadTask(jobId: string, taskId: string) {
    try {
      await deps.retryDownloadTask(jobId, taskId);
    } catch (error) {
      deps.notifyError(
        m.download_error_retry_task_failed({
          error: error instanceof Error ? error.message : String(error),
        })
      );
    }
  }

  async function handleClearDownloadHistory() {
    try {
      const removed = await deps.clearDownloadHistory();
      if (removed === 0) {
        deps.notifyInfo(m.download_notify_history_empty());
      }
    } catch (error) {
      deps.notifyError(
        m.download_error_clear_history_failed({
          error: error instanceof Error ? error.message : String(error),
        })
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
    taskSpeedMap.clear();
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
    getJobDisplayTitle,
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
