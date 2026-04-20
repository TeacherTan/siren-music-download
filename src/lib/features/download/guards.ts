import type { DownloadJobSnapshot } from "$lib/types";

export function hasCurrentDownloadOptions(
  job: DownloadJobSnapshot,
  _outputDir: string,
  format: string,
  downloadLyrics: boolean,
): boolean {
  return (
    job.options.format === format &&
    job.options.downloadLyrics === downloadLyrics
  );
}

export function isJobActive(job: DownloadJobSnapshot): boolean {
  return job.status === "queued" || job.status === "running";
}
