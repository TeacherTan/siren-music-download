<script lang="ts">
  import * as Sheet from '$lib/components/ui/sheet/index.js';
  import * as Select from '$lib/components/ui/select/index.js';
  import { Badge } from '$lib/components/ui/badge/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Progress } from '$lib/components/ui/progress/index.js';
  import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
  import SearchIcon from '@lucide/svelte/icons/search';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import XIcon from '@lucide/svelte/icons/x';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import type {
    DownloadHistoryKindFilter,
    DownloadHistoryScopeFilter,
    DownloadHistoryStatusFilter,
    DownloadJobSnapshot,
    DownloadTaskSnapshot,
  } from '$lib/types';
  interface Props {
    open?: boolean;
    jobs: DownloadJobSnapshot[];
    hasDownloadHistory: boolean;
    searchQuery?: string;
    scopeFilter?: DownloadHistoryScopeFilter;
    statusFilter?: DownloadHistoryStatusFilter;
    kindFilter?: DownloadHistoryKindFilter;
    canClearDownloadHistory: () => boolean;
    getJobProgress: (job: DownloadJobSnapshot) => number;
    getJobProgressText: (job: DownloadJobSnapshot) => string;
    getJobStatusLabel: (job: DownloadJobSnapshot) => string;
    getJobKindLabel: (job: DownloadJobSnapshot) => string;
    getJobSummaryLabel: (job: DownloadJobSnapshot) => string;
    getJobDisplayTitle: (job: DownloadJobSnapshot) => string;
    getJobErrorSummary: (job: DownloadJobSnapshot) => string | null;
    isJobActive: (jobId: string) => boolean;
    canCancelTask: (task: DownloadTaskSnapshot) => boolean;
    canRetryTask: (task: DownloadTaskSnapshot) => boolean;
    getTaskErrorLabel: (task: DownloadTaskSnapshot) => string | null;
    getTaskStatusLabel: (task: DownloadTaskSnapshot) => string;
    onClearDownloadHistory: () => void | Promise<void>;
    onCancelDownloadJob: (jobId: string) => void | Promise<void>;
    onRetryDownloadJob: (jobId: string) => void | Promise<void>;
    onCancelDownloadTask: (
      jobId: string,
      taskId: string
    ) => void | Promise<void>;
    onRetryDownloadTask: (
      jobId: string,
      taskId: string
    ) => void | Promise<void>;
  }
  let {
    open = $bindable(false),
    jobs,
    hasDownloadHistory,
    searchQuery = $bindable(''),
    scopeFilter = $bindable('all'),
    statusFilter = $bindable('all'),
    kindFilter = $bindable('all'),
    canClearDownloadHistory,
    getJobProgress,
    getJobProgressText,
    getJobStatusLabel,
    getJobKindLabel,
    getJobSummaryLabel,
    getJobDisplayTitle,
    getJobErrorSummary,
    isJobActive,
    canCancelTask,
    canRetryTask,
    getTaskErrorLabel,
    getTaskStatusLabel,
    onClearDownloadHistory,
    onCancelDownloadJob,
    onRetryDownloadJob,
    onCancelDownloadTask,
    onRetryDownloadTask,
  }: Props = $props();
  const labels = $derived.by(() => {
    void localeState.current;
    return {
      title: m.download_sheet_title(),
      description: m.download_sheet_description(),
      searchLabel: m.download_search_label(),
      clearHistory: m.download_clear_history(),
      cancel: m.download_action_cancel(),
      retry: m.download_action_retry(),
      emptyFilteredTitle: m.download_empty_filtered_title(),
      emptyFilteredHint: m.download_empty_filtered_hint(),
      emptyTitle: m.download_empty_title(),
      emptyHint: m.download_empty_hint(),
    };
  });
  const scopeOptions = $derived.by(() => {
    void localeState.current;
    return [
      {
        value: 'all' as DownloadHistoryScopeFilter,
        label: m.download_scope_all(),
      },
      {
        value: 'active' as DownloadHistoryScopeFilter,
        label: m.download_scope_active(),
      },
      {
        value: 'history' as DownloadHistoryScopeFilter,
        label: m.download_scope_history(),
      },
    ];
  });
  const statusOptions = $derived.by(() => {
    void localeState.current;
    return [
      {
        value: 'all' as DownloadHistoryStatusFilter,
        label: m.download_status_all(),
      },
      {
        value: 'queued' as DownloadHistoryStatusFilter,
        label: m.download_status_queued(),
      },
      {
        value: 'running' as DownloadHistoryStatusFilter,
        label: m.download_status_running(),
      },
      {
        value: 'completed' as DownloadHistoryStatusFilter,
        label: m.download_status_completed(),
      },
      {
        value: 'partiallyFailed' as DownloadHistoryStatusFilter,
        label: m.download_status_partially_failed(),
      },
      {
        value: 'failed' as DownloadHistoryStatusFilter,
        label: m.download_status_failed(),
      },
      {
        value: 'cancelled' as DownloadHistoryStatusFilter,
        label: m.download_status_cancelled(),
      },
    ];
  });
  const kindOptions = $derived.by(() => {
    void localeState.current;
    return [
      {
        value: 'all' as DownloadHistoryKindFilter,
        label: m.download_kind_all(),
      },
      {
        value: 'song' as DownloadHistoryKindFilter,
        label: m.download_kind_song(),
      },
      {
        value: 'album' as DownloadHistoryKindFilter,
        label: m.download_kind_album(),
      },
      {
        value: 'selection' as DownloadHistoryKindFilter,
        label: m.download_kind_selection(),
      },
    ];
  });
  const currentScopeLabel = $derived(
    scopeOptions.find((o) => o.value === scopeFilter)?.label ?? ''
  );
  const currentStatusLabel = $derived(
    statusOptions.find((o) => o.value === statusFilter)?.label ?? ''
  );
  const currentKindLabel = $derived(
    kindOptions.find((o) => o.value === kindFilter)?.label ?? ''
  );
</script>

<Sheet.Root bind:open>
  <Sheet.Content
    class="app-side-sheet download-sheet gap-0 overflow-hidden border-[var(--download-border)] bg-[var(--surface-sheet)] p-0 text-[var(--text-primary)] shadow-[0_24px_64px_rgba(15,23,42,0.18)] backdrop-blur-xl"
  >
    <Sheet.Header class="download-sheet-header">
      <Sheet.Title>{labels.title}</Sheet.Title>
      <Sheet.Description>{labels.description}</Sheet.Description>
    </Sheet.Header>
    <div class="download-sheet-body">
      <section class="download-filter-section">
        <div class="download-search-field">
          <SearchIcon aria-hidden="true" />
          <Input
            bind:value={searchQuery}
            placeholder={labels.searchLabel}
            aria-label={labels.searchLabel}
            class="download-search-input h-9 border-[var(--download-border)] bg-[var(--download-control-bg)]"
            style="padding-left: 38px;"
          />
        </div>
        <div class="download-filter-grid">
          <Select.Root type="single" bind:value={scopeFilter}
            ><Select.Trigger
              class="download-filter-trigger h-9 w-full border-[var(--download-border)] bg-[var(--download-control-bg)]"
              >{currentScopeLabel}</Select.Trigger
            ><Select.Content class="download-filter-select-content"
              >{#each scopeOptions as option (option.value)}<Select.Item
                  value={option.value}
                  label={option.label}
                />{/each}</Select.Content
            ></Select.Root
          >
          <Select.Root type="single" bind:value={statusFilter}
            ><Select.Trigger
              class="download-filter-trigger h-9 w-full border-[var(--download-border)] bg-[var(--download-control-bg)]"
              >{currentStatusLabel}</Select.Trigger
            ><Select.Content class="download-filter-select-content"
              >{#each statusOptions as option (option.value)}<Select.Item
                  value={option.value}
                  label={option.label}
                />{/each}</Select.Content
            ></Select.Root
          >
          <Select.Root type="single" bind:value={kindFilter}
            ><Select.Trigger
              class="download-filter-trigger h-9 w-full border-[var(--download-border)] bg-[var(--download-control-bg)]"
              >{currentKindLabel}</Select.Trigger
            ><Select.Content class="download-filter-select-content"
              >{#each kindOptions as option (option.value)}<Select.Item
                  value={option.value}
                  label={option.label}
                />{/each}</Select.Content
            ></Select.Root
          >
        </div>
        <Button
          class="download-clear-history"
          variant="secondary"
          disabled={!canClearDownloadHistory()}
          onclick={() => void onClearDownloadHistory()}
          ><Trash2Icon data-icon="inline-start" />{labels.clearHistory}</Button
        >
      </section>
      {#if jobs.length > 0}
        <div class="download-job-list">
          {#each jobs as job (job.id)}
            {@const progress = getJobProgress(job)}
            {@const progressText = getJobProgressText(job)}
            {@const statusLabel = getJobStatusLabel(job)}
            {@const kindLabel = getJobKindLabel(job)}
            {@const summaryLabel = getJobSummaryLabel(job)}
            {@const errorSummary = getJobErrorSummary(job)}
            <section class="download-job-card" data-status={job.status}>
              <div class="download-job-header">
                <div class="download-job-copy">
                  <div class="download-job-meta">
                    <Badge variant="secondary" class="download-kind-badge"
                      >{kindLabel}</Badge
                    ><span class="download-status-pill">{statusLabel}</span>
                  </div>
                  <h3>{getJobDisplayTitle(job)}</h3>
                  <p>{summaryLabel}</p>
                </div>
                <div class="download-job-actions">
                  {#if job.status === 'running' || job.status === 'queued'}
                    <Button
                      size="sm"
                      variant="ghost"
                      onclick={() => void onCancelDownloadJob(job.id)}
                      ><XIcon data-icon="inline-start" />{labels.cancel}</Button
                    >
                  {:else if (job.status === 'failed' || job.status === 'partiallyFailed' || job.status === 'cancelled') && !isJobActive(job.id)}
                    <Button
                      size="sm"
                      variant="ghost"
                      onclick={() => void onRetryDownloadJob(job.id)}
                      ><RotateCcwIcon
                        data-icon="inline-start"
                      />{labels.retry}</Button
                    >
                  {/if}
                </div>
              </div>
              <div class="download-progress-block">
                <Progress class="download-progress" value={progress * 100} />
                <p>{progressText}</p>
              </div>
              {#if errorSummary}<p class="download-error-summary">
                  {errorSummary}
                </p>{/if}
              <div class="download-task-list">
                {#each job.tasks as task (task.id)}
                  {@const taskError = getTaskErrorLabel(task)}
                  <div class="download-task-row" data-status={task.status}>
                    <div class="download-task-copy">
                      <p>{task.songName}</p>
                      {#if taskError}<small>{taskError}</small>{/if}
                    </div>
                    <div class="download-task-side">
                      <span>{getTaskStatusLabel(task)}</span>
                      {#if canCancelTask(task)}
                        <Button
                          size="icon-sm"
                          variant="ghost"
                          title={labels.cancel}
                          aria-label={m.download_task_cancel_aria({
                            name: task.songName,
                          })}
                          onclick={() =>
                            void onCancelDownloadTask(job.id, task.id)}
                          ><XIcon /></Button
                        >
                      {:else if canRetryTask(task) && !isJobActive(job.id)}
                        <Button
                          size="icon-sm"
                          variant="ghost"
                          title={labels.retry}
                          aria-label={m.download_task_retry_aria({
                            name: task.songName,
                          })}
                          onclick={() =>
                            void onRetryDownloadTask(job.id, task.id)}
                          ><RotateCcwIcon /></Button
                        >
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            </section>
          {/each}
        </div>
      {:else if hasDownloadHistory}
        <div class="download-empty-state">
          <h3>{labels.emptyFilteredTitle}</h3>
          <p>{labels.emptyFilteredHint}</p>
        </div>
      {:else}
        <div class="download-empty-state">
          <h3>{labels.emptyTitle}</h3>
          <p>{labels.emptyHint}</p>
        </div>
      {/if}
    </div>
  </Sheet.Content>
</Sheet.Root>

<style>
  :global(.download-sheet) {
    --download-border: color-mix(in srgb, var(--border) 78%, white 22%);
    --download-section-bg: color-mix(
      in srgb,
      var(--bg-secondary) 76%,
      transparent
    );
    --download-control-bg: color-mix(
      in srgb,
      var(--bg-primary) 54%,
      transparent
    );
    --download-row-bg: color-mix(in srgb, var(--bg-primary) 42%, transparent);
    --download-row-hover-bg: color-mix(
      in srgb,
      var(--bg-primary) 56%,
      transparent
    );
  }
  :global(.download-sheet-header) {
    padding: 18px 48px 14px 18px;
    border-bottom: 1px solid var(--download-border);
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--surface-tint-strong) 72%, transparent),
      transparent
    );
  }
  .download-sheet-body {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    gap: 12px;
    overflow-y: auto;
    padding: 14px 14px 18px;
  }
  .download-filter-section {
    display: grid;
    gap: 10px;
    border: 1px solid var(--download-border);
    border-radius: 8px;
    background: var(--download-section-bg);
    padding: 12px;
  }
  .download-search-field {
    position: relative;
  }
  :global(.download-search-field svg) {
    position: absolute;
    top: 50%;
    left: 11px;
    z-index: 1;
    width: 15px;
    height: 15px;
    color: var(--text-secondary);
    transform: translateY(-50%);
    pointer-events: none;
  }
  :global(.download-search-input) {
    padding-left: 38px !important;
  }
  .download-filter-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }
  :global(.download-filter-trigger) {
    min-width: 0;
    border-radius: 7px;
    background: color-mix(in srgb, var(--download-row-bg) 88%, transparent);
    box-shadow: inset 0 1px 0 color-mix(in srgb, white 28%, transparent);
    color: var(--text-primary);
    padding-inline: 10px 8px;
    transition:
      background var(--motion-fast) var(--ease-standard),
      border-color var(--motion-fast) var(--ease-standard),
      box-shadow var(--motion-fast) var(--ease-standard);
  }
  :global(.download-filter-trigger:hover),
  :global(.download-filter-trigger[data-state='open']) {
    border-color: color-mix(in srgb, var(--accent) 18%, var(--download-border));
    background: color-mix(
      in srgb,
      var(--download-row-hover-bg) 92%,
      transparent
    );
  }
  :global(.download-filter-trigger:focus-visible) {
    border-color: color-mix(in srgb, var(--accent) 32%, var(--download-border));
    box-shadow:
      inset 0 1px 0 color-mix(in srgb, white 28%, transparent),
      0 0 0 3px color-mix(in srgb, var(--accent) 14%, transparent);
  }
  :global(.download-filter-trigger svg) {
    color: var(--text-tertiary);
  }
  :global(.download-filter-select-content) {
    z-index: 210;
    min-width: var(--bits-select-anchor-width);
    border: 1px solid var(--download-border);
    border-radius: 8px;
    background: color-mix(in srgb, var(--surface-sheet) 86%, transparent);
    color: var(--text-primary);
    padding: 4px;
    box-shadow:
      0 18px 40px rgba(15, 23, 42, 0.16),
      inset 0 1px 0 color-mix(in srgb, white 38%, transparent);
    transform-origin: var(--bits-select-content-transform-origin);
    backdrop-filter: blur(18px) saturate(1.16);
    will-change: opacity, transform;
  }
  :global(.download-filter-select-content [data-slot='select-item']) {
    min-height: 28px;
    border-radius: 6px;
    color: var(--text-primary);
    padding: 5px 30px 5px 9px;
    font-size: 12px;
    line-height: 1.35;
  }
  :global(
    .download-filter-select-content [data-slot='select-item'][data-highlighted]
  ),
  :global(.download-filter-select-content [data-slot='select-item']:focus) {
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    color: var(--text-primary);
  }
  :global(
    .download-filter-select-content
      [data-slot='select-item'][data-state='checked']
  ) {
    color: var(--text-primary);
    font-weight: 600;
  }
  :global(.download-filter-select-content [data-slot='select-item'] svg) {
    color: var(--accent);
  }
  :global(.download-filter-select-content[data-state='open']) {
    animation-duration: var(--motion-fast);
    animation-fill-mode: both;
    animation-timing-function: var(--ease-decelerate);
  }
  :global(
    .download-filter-select-content[data-state='open'][data-side='bottom']
  ) {
    animation-name: download-filter-select-in-bottom;
  }
  :global(.download-filter-select-content[data-state='open'][data-side='top']) {
    animation-name: download-filter-select-in-top;
  }
  :global(.download-filter-select-content[data-state='closed']) {
    animation-duration: var(--motion-fast);
    animation-fill-mode: both;
    animation-timing-function: var(--ease-standard);
  }
  :global(
    .download-filter-select-content[data-state='closed'][data-side='bottom']
  ) {
    animation-name: download-filter-select-out-bottom;
  }
  :global(
    .download-filter-select-content[data-state='closed'][data-side='top']
  ) {
    animation-name: download-filter-select-out-top;
  }
  :global(.download-clear-history) {
    justify-self: end;
  }
  .download-job-list {
    display: grid;
    gap: 12px;
  }
  .download-job-card {
    display: grid;
    gap: 12px;
    border: 1px solid var(--download-border);
    border-radius: 8px;
    background: var(--download-section-bg);
    padding: 12px;
  }
  .download-job-card[data-status='running'],
  .download-job-card[data-status='queued'] {
    border-color: color-mix(in srgb, var(--accent) 28%, var(--download-border));
  }
  .download-job-card[data-status='failed'],
  .download-job-card[data-status='partiallyFailed'] {
    border-color: color-mix(
      in srgb,
      var(--destructive) 42%,
      var(--download-border)
    );
  }
  .download-job-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  .download-job-copy {
    display: grid;
    gap: 5px;
    min-width: 0;
  }
  .download-job-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  :global(.download-kind-badge) {
    background: var(--download-row-bg);
    color: var(--text-primary);
  }
  .download-status-pill {
    overflow: hidden;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .download-job-copy h3 {
    margin: 0;
    overflow: hidden;
    color: var(--text-primary);
    font-size: 14px;
    font-weight: 700;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .download-job-copy p,
  .download-progress-block p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.45;
  }
  .download-job-actions {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    gap: 6px;
  }
  .download-progress-block {
    display: grid;
    gap: 7px;
  }
  :global(.download-progress) {
    height: 5px;
    background: color-mix(in srgb, var(--bg-tertiary) 74%, transparent);
  }
  :global(.download-progress [data-slot='progress-indicator']) {
    background: linear-gradient(
      90deg,
      var(--accent),
      color-mix(in srgb, var(--accent) 72%, white 28%)
    );
  }
  .download-error-summary {
    margin: 0;
    border: 1px solid color-mix(in srgb, var(--destructive) 36%, transparent);
    border-radius: 7px;
    background: color-mix(in srgb, var(--destructive) 10%, transparent);
    color: var(--destructive);
    padding: 8px 10px;
    font-size: 12px;
    line-height: 1.45;
  }
  .download-task-list {
    overflow: hidden;
    border: 1px solid var(--download-border);
    border-radius: 8px;
    background: var(--download-row-bg);
  }
  .download-task-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 9px 10px;
    transition: background var(--motion-fast) var(--ease-standard);
  }
  .download-task-row + .download-task-row {
    border-top: 1px solid var(--download-border);
  }
  .download-task-row:hover {
    background: var(--download-row-hover-bg);
  }
  .download-task-copy {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .download-task-copy p {
    margin: 0;
    overflow: hidden;
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 600;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .download-task-copy small {
    overflow: hidden;
    color: var(--destructive);
    font-size: 11px;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .download-task-side {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
    min-width: 0;
  }
  .download-task-side span {
    max-width: 40vw;
    overflow: hidden;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.35;
    text-align: right;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .download-task-row[data-status='failed'] .download-task-side span,
  .download-task-row[data-status='cancelled'] .download-task-side span {
    color: var(--destructive);
  }
  .download-task-row[data-status='completed'] .download-task-side span {
    color: var(--text-tertiary);
  }
  .download-empty-state {
    display: grid;
    place-items: center;
    align-content: center;
    min-height: 260px;
    border: 1px solid var(--download-border);
    border-radius: 8px;
    background: var(--download-section-bg);
    padding: 28px 18px;
    text-align: center;
  }
  .download-empty-state h3 {
    margin: 0;
    color: var(--text-primary);
    font-size: 14px;
    font-weight: 700;
  }
  .download-empty-state p {
    max-width: 72%;
    margin: 7px 0 0;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.5;
  }
  @media (max-width: 440px) {
    .download-filter-grid,
    .download-job-header,
    .download-task-row {
      grid-template-columns: 1fr;
    }
    .download-job-header,
    .download-task-row {
      display: grid;
    }
    .download-job-actions,
    .download-task-side {
      justify-content: flex-start;
    }
    .download-task-side span {
      max-width: none;
      text-align: left;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    :global(.download-filter-select-content[data-state]) {
      animation: none;
    }
  }
  @keyframes -global-download-filter-select-in-bottom {
    from {
      opacity: 0;
      transform: translateY(-5px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
  @keyframes -global-download-filter-select-in-top {
    from {
      opacity: 0;
      transform: translateY(5px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
  @keyframes -global-download-filter-select-out-bottom {
    from {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
    to {
      opacity: 0;
      transform: translateY(-5px) scale(0.98);
    }
  }
  @keyframes -global-download-filter-select-out-top {
    from {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
    to {
      opacity: 0;
      transform: translateY(5px) scale(0.98);
    }
  }
</style>
