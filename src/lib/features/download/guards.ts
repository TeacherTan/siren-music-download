import type {
  DownloadHistoryKindFilter,
  DownloadHistoryScopeFilter,
  DownloadHistoryStatusFilter,
  DownloadJobSnapshot,
} from "$lib/types";

type SortableDownloadJob = {
  job: DownloadJobSnapshot;
  originalIndex: number;
};

export function hasCurrentDownloadOptions(
  job: DownloadJobSnapshot,
  outputDir: string,
  format: string,
  downloadLyrics: boolean,
): boolean {
  return (
    job.options.outputDir === outputDir &&
    job.options.format === format &&
    job.options.downloadLyrics === downloadLyrics
  );
}

export function isJobActive(job: DownloadJobSnapshot): boolean {
  return job.status === "queued" || job.status === "running";
}

export function isTerminalJob(job: DownloadJobSnapshot): boolean {
  return (
    job.status === "completed" ||
    job.status === "failed" ||
    job.status === "cancelled" ||
    job.status === "partiallyFailed"
  );
}

export function sortDownloadJobs(
  jobs: DownloadJobSnapshot[],
): DownloadJobSnapshot[] {
  return jobs
    .map(
      (job, originalIndex): SortableDownloadJob => ({
        job,
        originalIndex,
      }),
    )
    .sort((left, right) => {
      const leftIsTerminal = isTerminalJob(left.job);
      const rightIsTerminal = isTerminalJob(right.job);

      if (leftIsTerminal !== rightIsTerminal) {
        return leftIsTerminal ? 1 : -1;
      }

      if (!leftIsTerminal) {
        return right.originalIndex - left.originalIndex;
      }

      const finishedDiff =
        parseJobTimestamp(right.job.finishedAt) -
        parseJobTimestamp(left.job.finishedAt);
      if (finishedDiff !== 0) {
        return finishedDiff;
      }

      const createdDiff =
        parseJobTimestamp(right.job.createdAt) -
        parseJobTimestamp(left.job.createdAt);
      if (createdDiff !== 0) {
        return createdDiff;
      }

      return left.originalIndex - right.originalIndex;
    })
    .map(({ job }) => job);
}

export function matchesJobSearch(
  job: DownloadJobSnapshot,
  normalizedQuery: string,
): boolean {
  if (!normalizedQuery) return true;
  return job.title.toLocaleLowerCase().includes(normalizedQuery);
}

export function matchesJobScopeFilter(
  job: DownloadJobSnapshot,
  filter: DownloadHistoryScopeFilter,
): boolean {
  if (filter === "all") return true;
  if (filter === "active") return !isTerminalJob(job);
  return isTerminalJob(job);
}

export function matchesJobStatusFilter(
  job: DownloadJobSnapshot,
  filter: DownloadHistoryStatusFilter,
): boolean {
  return filter === "all" ? true : job.status === filter;
}

export function matchesJobKindFilter(
  job: DownloadJobSnapshot,
  filter: DownloadHistoryKindFilter,
): boolean {
  return filter === "all" ? true : job.kind === filter;
}

function parseJobTimestamp(value: string | null): number {
  if (!value) return 0;
  const parsed = Date.parse(value);
  return Number.isNaN(parsed) ? 0 : parsed;
}
