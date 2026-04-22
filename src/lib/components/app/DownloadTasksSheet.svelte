<script lang="ts">
  import * as Sheet from "$lib/components/ui/sheet/index.js";
  import * as Select from "$lib/components/ui/select/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import type {
    DownloadHistoryKindFilter,
    DownloadHistoryScopeFilter,
    DownloadHistoryStatusFilter,
    DownloadJobSnapshot,
    DownloadTaskSnapshot,
  } from "$lib/types";

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
    getJobErrorSummary: (job: DownloadJobSnapshot) => string | null;
    isJobActive: (jobId: string) => boolean;
    canCancelTask: (task: DownloadTaskSnapshot) => boolean;
    canRetryTask: (task: DownloadTaskSnapshot) => boolean;
    getTaskErrorLabel: (task: DownloadTaskSnapshot) => string | null;
    getTaskStatusLabel: (task: DownloadTaskSnapshot) => string;
    onClearDownloadHistory: () => void | Promise<void>;
    onCancelDownloadJob: (jobId: string) => void | Promise<void>;
    onRetryDownloadJob: (jobId: string) => void | Promise<void>;
    onCancelDownloadTask: (jobId: string, taskId: string) => void | Promise<void>;
    onRetryDownloadTask: (jobId: string, taskId: string) => void | Promise<void>;
  }

  const scopeOptions: Array<{
    value: DownloadHistoryScopeFilter;
    label: string;
  }> = [
    { value: "all", label: "全部范围" },
    { value: "active", label: "仅进行中" },
    { value: "history", label: "仅历史" },
  ];

  const statusOptions: Array<{
    value: DownloadHistoryStatusFilter;
    label: string;
  }> = [
    { value: "all", label: "全部状态" },
    { value: "queued", label: "排队中" },
    { value: "running", label: "下载中" },
    { value: "completed", label: "已完成" },
    { value: "partiallyFailed", label: "部分失败" },
    { value: "failed", label: "失败" },
    { value: "cancelled", label: "已取消" },
  ];

  const kindOptions: Array<{
    value: DownloadHistoryKindFilter;
    label: string;
  }> = [
    { value: "all", label: "全部类型" },
    { value: "song", label: "单曲下载" },
    { value: "album", label: "整专下载" },
    { value: "selection", label: "多选下载" },
  ];

  let {
    open = $bindable(false),
    jobs,
    hasDownloadHistory,
    searchQuery = $bindable(""),
    scopeFilter = $bindable("all"),
    statusFilter = $bindable("all"),
    kindFilter = $bindable("all"),
    canClearDownloadHistory,
    getJobProgress,
    getJobProgressText,
    getJobStatusLabel,
    getJobKindLabel,
    getJobSummaryLabel,
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
</script>

<Sheet.Root bind:open>
  <Sheet.Content class="w-[420px] border-white/50 bg-[var(--surface-sheet)] text-[var(--text-primary)] backdrop-blur-xl">
    <Sheet.Header>
      <Sheet.Title>下载任务</Sheet.Title>
      <Sheet.Description>查看进度、错误和历史记录</Sheet.Description>
    </Sheet.Header>

    <div class="space-y-3 py-2">
      <div class="grid gap-2">
        <Input
          bind:value={searchQuery}
          placeholder="按任务标题搜索"
          aria-label="按任务标题搜索"
          class="border-white/35 bg-white/20"
        />

        <div class="grid grid-cols-1 gap-2 sm:grid-cols-3">
          <Select.Root type="single" bind:value={scopeFilter}>
            <Select.Trigger class="w-full border-white/35 bg-white/20">
              {scopeOptions.find((option) => option.value === scopeFilter)?.label ?? "全部范围"}
            </Select.Trigger>
            <Select.Content>
              {#each scopeOptions as option (option.value)}
                <Select.Item value={option.value} label={option.label} />
              {/each}
            </Select.Content>
          </Select.Root>

          <Select.Root type="single" bind:value={statusFilter}>
            <Select.Trigger class="w-full border-white/35 bg-white/20">
              {statusOptions.find((option) => option.value === statusFilter)?.label ?? "全部状态"}
            </Select.Trigger>
            <Select.Content>
              {#each statusOptions as option (option.value)}
                <Select.Item value={option.value} label={option.label} />
              {/each}
            </Select.Content>
          </Select.Root>

          <Select.Root type="single" bind:value={kindFilter}>
            <Select.Trigger class="w-full border-white/35 bg-white/20">
              {kindOptions.find((option) => option.value === kindFilter)?.label ?? "全部类型"}
            </Select.Trigger>
            <Select.Content>
              {#each kindOptions as option (option.value)}
                <Select.Item value={option.value} label={option.label} />
              {/each}
            </Select.Content>
          </Select.Root>
        </div>
      </div>

      <div class="flex items-center justify-end">
        <Button
          variant="secondary"
          disabled={!canClearDownloadHistory()}
          onclick={() => void onClearDownloadHistory()}
        >
          清理历史
        </Button>
      </div>
    </div>

    {#if jobs.length > 0}
      <div class="space-y-3 py-2">
        {#each jobs as job (job.id)}
          {@const progress = getJobProgress(job)}
          {@const progressText = getJobProgressText(job)}
          {@const statusLabel = getJobStatusLabel(job)}
          {@const kindLabel = getJobKindLabel(job)}
          {@const summaryLabel = getJobSummaryLabel(job)}
          {@const errorSummary = getJobErrorSummary(job)}
          <section class="rounded-[22px] border border-white/[0.40] bg-white/[0.28] p-4">
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0 space-y-1">
                <div class="flex items-center gap-2">
                  <Badge>{kindLabel}</Badge>
                  <span class="text-xs text-[var(--text-secondary)]">{statusLabel}</span>
                </div>
                <p class="truncate text-sm font-medium">{job.title}</p>
                <p class="text-xs text-[var(--text-secondary)]">{summaryLabel}</p>
              </div>

              <div class="flex items-center gap-2">
                {#if job.status === "running" || job.status === "queued"}
                  <Button
                    size="sm"
                    variant="ghost"
                    onclick={() => void onCancelDownloadJob(job.id)}
                  >
                    取消
                  </Button>
                {:else if (job.status === "failed" || job.status === "partiallyFailed" || job.status === "cancelled") && !isJobActive(job.id)}
                  <Button
                    size="sm"
                    variant="ghost"
                    onclick={() => void onRetryDownloadJob(job.id)}
                  >
                    重试
                  </Button>
                {/if}
              </div>
            </div>

            <div class="mt-3 space-y-2">
              <Progress value={progress * 100} />
              <p class="text-xs text-[var(--text-secondary)]">{progressText}</p>
            </div>

            {#if errorSummary}
              <p class="mt-2 text-xs text-red-500/90">{errorSummary}</p>
            {/if}

            <div class="mt-3 space-y-2">
              {#each job.tasks as task (task.id)}
                {@const taskError = getTaskErrorLabel(task)}
                <div class="flex items-start justify-between gap-3 rounded-2xl border border-white/[0.30] bg-white/[0.22] px-3 py-2">
                  <div class="min-w-0">
                    <p class="truncate text-xs font-medium">{task.songName}</p>
                    {#if taskError}
                      <p class="mt-1 text-[11px] text-red-500/90">{taskError}</p>
                    {/if}
                  </div>

                  <div class="flex shrink-0 items-center gap-2">
                    <span class="max-w-[140px] text-right text-[11px] text-[var(--text-secondary)]">
                      {getTaskStatusLabel(task)}
                    </span>
                    {#if canCancelTask(task)}
                      <Button
                        size="sm"
                        variant="ghost"
                        onclick={() => void onCancelDownloadTask(job.id, task.id)}
                      >
                        取消
                      </Button>
                    {:else if canRetryTask(task) && !isJobActive(job.id)}
                      <Button
                        size="sm"
                        variant="ghost"
                        onclick={() => void onRetryDownloadTask(job.id, task.id)}
                      >
                        重试
                      </Button>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          </section>
        {/each}
      </div>
    {:else if hasDownloadHistory}
      <div class="flex min-h-[240px] flex-col items-center justify-center gap-2 py-8 text-center">
        <p class="text-sm font-medium">没有匹配的下载任务</p>
        <p class="max-w-[24rem] text-xs text-[var(--text-secondary)]">
          请调整搜索关键字或筛选条件后重试。
        </p>
      </div>
    {:else}
      <div class="flex min-h-[240px] flex-col items-center justify-center gap-2 py-8 text-center">
        <p class="text-sm font-medium">暂无下载任务</p>
        <p class="max-w-[24rem] text-xs text-[var(--text-secondary)]">
          点击专辑页的“下载整张专辑”或曲目右侧下载按钮开始下载。
        </p>
      </div>
    {/if}
  </Sheet.Content>
</Sheet.Root>
