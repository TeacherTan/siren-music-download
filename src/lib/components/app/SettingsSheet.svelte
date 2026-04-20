<script lang="ts">
  import * as Sheet from "$lib/components/ui/sheet/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Switch } from "$lib/components/ui/switch/index.js";
  import type {
    LogFileKind,
    LogFileStatus,
    LogLevel,
    LogViewerRecord,
    OutputFormat,
  } from "$lib/types";

  interface Props {
    open?: boolean;
    format?: OutputFormat;
    outputDir?: string;
    downloadLyrics?: boolean;
    notifyOnDownloadComplete?: boolean;
    notifyOnPlaybackChange?: boolean;
    logLevel?: LogLevel;
    logFileKind?: LogFileKind;
    logRecords?: LogViewerRecord[];
    logFileStatus?: LogFileStatus | null;
    logViewerLoading?: boolean;
    logViewerError?: string;
    isSendingTestNotification?: boolean;
    isClearingAudioCache?: boolean;
    onSelectDirectory: () => void | Promise<void>;
    onSendTestNotification: () => void | Promise<void>;
    onClearAudioCache: () => void | Promise<void>;
    onChangeLogFileKind: (kind: LogFileKind) => void | Promise<void>;
  }

  let {
    open = $bindable(false),
    format = $bindable<OutputFormat>("flac"),
    outputDir = "",
    downloadLyrics = $bindable(true),
    notifyOnDownloadComplete = $bindable(true),
    notifyOnPlaybackChange = $bindable(true),
    logLevel = $bindable<LogLevel>("error"),
    logFileKind = "session",
    logRecords = [],
    logFileStatus = null,
    logViewerLoading = false,
    logViewerError = "",
    isSendingTestNotification = false,
    isClearingAudioCache = false,
    onSelectDirectory,
    onSendTestNotification,
    onClearAudioCache,
    onChangeLogFileKind,
  }: Props = $props();
</script>

<Sheet.Root bind:open>
  <Sheet.Content class="w-[340px] border-white/50 bg-[var(--surface-sheet)] text-[var(--text-primary)] backdrop-blur-xl">
    <Sheet.Header>
      <Sheet.Title>下载设置</Sheet.Title>
      <Sheet.Description>音频格式、通知和缓存管理</Sheet.Description>
    </Sheet.Header>

    <div class="space-y-6 py-2">
      <div class="space-y-2">
        <label class="text-sm text-[var(--text-secondary)]" for="format-select">输出格式</label>
        <select
          id="format-select"
          class="w-full rounded-2xl border border-white/50 bg-white/[0.40] px-3 py-2 text-sm outline-none"
          bind:value={format}
        >
          <option value="flac">FLAC（无损压缩）</option>
          <option value="wav">WAV（无损）</option>
          <option value="mp3">MP3</option>
        </select>
      </div>

      <div class="space-y-2">
        <label class="text-sm text-[var(--text-secondary)]" for="log-level-select">持久化日志等级</label>
        <select
          id="log-level-select"
          class="w-full rounded-2xl border border-white/50 bg-white/[0.40] px-3 py-2 text-sm outline-none"
          bind:value={logLevel}
        >
          <option value="error">Error（仅严重错误）</option>
          <option value="warn">Warn（警告及以上）</option>
          <option value="info">Info（信息及以上）</option>
          <option value="debug">Debug（记录全部调试信息）</option>
        </select>
      </div>

      <div class="space-y-2">
        <label class="text-sm text-[var(--text-secondary)]" for="output-dir">保存位置</label>
        <input
          id="output-dir"
          class="w-full rounded-2xl border border-white/50 bg-white/[0.35] px-3 py-2 text-sm outline-none"
          readonly
          value={outputDir}
        />
        <Button class="w-full" onclick={() => void onSelectDirectory()}>
          选择文件夹
        </Button>
      </div>

      <div class="space-y-4 rounded-[22px] border border-white/[0.45] bg-white/[0.28] p-4">
        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <p class="text-sm font-medium">歌词文件</p>
            <p class="mt-1 text-xs text-[var(--text-secondary)]">有歌词时，在音频旁生成同名 `.lrc`。</p>
          </div>
          <Switch bind:checked={downloadLyrics} />
        </div>

        <div class="border-t border-white/40"></div>

        <div class="space-y-3">
          <p class="text-sm font-medium">系统通知</p>
          <p class="text-xs text-[var(--text-secondary)]">
            桌面端权限以系统结果为准，开发环境下可能和打包结果不一致。
          </p>
          <Button
            class="w-full"
            variant="secondary"
            disabled={isSendingTestNotification}
            onclick={() => void onSendTestNotification()}
          >
            {isSendingTestNotification ? "正在发送..." : "发送测试通知"}
          </Button>
        </div>

        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <p class="text-sm font-medium">下载完成通知</p>
            <p class="mt-1 text-xs text-[var(--text-secondary)]">下载任务完成时显示通知。</p>
          </div>
          <Switch bind:checked={notifyOnDownloadComplete} />
        </div>

        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <p class="text-sm font-medium">播放切换通知</p>
            <p class="mt-1 text-xs text-[var(--text-secondary)]">播放新歌曲时显示通知。</p>
          </div>
          <Switch bind:checked={notifyOnPlaybackChange} />
        </div>
      </div>

      <div class="space-y-3 rounded-[22px] border border-white/45 bg-white/25 p-4">
        <div>
          <p class="text-sm font-medium">音乐缓存</p>
          <p class="mt-1 text-xs text-[var(--text-secondary)]">播放时的音频缓存保存在系统缓存目录。</p>
        </div>
        <Button
          class="w-full"
          variant="secondary"
          disabled={isClearingAudioCache}
          onclick={() => void onClearAudioCache()}
        >
          {isClearingAudioCache ? "正在清除缓存..." : "清除音频缓存"}
        </Button>
      </div>

      <div class="space-y-3 rounded-[22px] border border-white/45 bg-white/25 p-4">
        <div class="flex items-center justify-between gap-2">
          <div>
            <p class="text-sm font-medium">日志与诊断</p>
            <p class="mt-1 text-xs text-[var(--text-secondary)]">
              本次运行日志会在正常退出时按等级写入持久化日志。
            </p>
          </div>
          <div class="flex gap-2">
            <Button
              size="sm"
              variant={logFileKind === "session" ? "default" : "secondary"}
              onclick={() => void onChangeLogFileKind("session")}
            >
              本次运行
            </Button>
            <Button
              size="sm"
              variant={logFileKind === "persistent" ? "default" : "secondary"}
              onclick={() => void onChangeLogFileKind("persistent")}
            >
              持久化
            </Button>
          </div>
        </div>

        <p class="text-[11px] text-[var(--text-secondary)]">
          session: {logFileStatus?.hasSessionLog ? "可用" : "暂无"} · persistent:
          {logFileStatus?.hasPersistentLog ? "可用" : "暂无"}
        </p>

        {#if logViewerLoading}
          <div class="rounded-2xl border border-white/[0.30] bg-white/[0.18] px-3 py-4 text-xs text-[var(--text-secondary)]">
            正在加载日志…
          </div>
        {:else if logViewerError}
          <div class="rounded-2xl border border-red-400/40 bg-red-500/[0.10] px-3 py-4 text-xs text-red-500/90">
            {logViewerError}
          </div>
        {:else if logRecords.length > 0}
          <div class="max-h-[240px] space-y-2 overflow-y-auto rounded-2xl border border-white/[0.30] bg-white/[0.18] p-2">
            {#each logRecords as record (record.id)}
              <div class="rounded-xl border border-white/[0.25] bg-white/[0.16] px-3 py-2">
                <div class="flex items-center justify-between gap-2">
                  <span class="text-[11px] font-medium uppercase text-[var(--text-secondary)]">{record.level}</span>
                  <span class="text-[11px] text-[var(--text-secondary)]">{record.ts}</span>
                </div>
                <p class="mt-1 text-xs font-medium">{record.message}</p>
                <p class="mt-1 text-[11px] text-[var(--text-secondary)]">{record.domain} · {record.code}</p>
                {#if record.details}
                  <p class="mt-1 whitespace-pre-wrap break-all text-[11px] text-[var(--text-secondary)]">
                    {record.details}
                  </p>
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <div class="rounded-2xl border border-white/[0.30] bg-white/[0.18] px-3 py-4 text-xs text-[var(--text-secondary)]">
            当前没有可显示的日志记录。
          </div>
        {/if}
      </div>
    </div>
  </Sheet.Content>
</Sheet.Root>
