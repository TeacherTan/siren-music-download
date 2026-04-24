<script lang="ts">
  import * as Sheet from '$lib/components/ui/sheet/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import {
    clearAudioCache,
    getLogFileStatus,
    listLogRecords,
    selectDirectory,
    sendTestNotification,
  } from '$lib/settingsApi';
  import type {
    LogFileKind,
    LogFileStatus,
    LogLevel,
    LogViewerRecord,
    OutputFormat,
  } from '$lib/types';

  interface Props {
    open?: boolean;
    format?: OutputFormat;
    outputDir?: string;
    downloadLyrics?: boolean;
    notifyOnDownloadComplete?: boolean;
    notifyOnPlaybackChange?: boolean;
    logLevel?: LogLevel;
    logRefreshToken?: number;
    notifyInfo: (message: string) => void;
    notifyError: (message: string) => void;
    onOutputDirChange: (outputDir: string) => boolean | Promise<boolean>;
  }

  let {
    open = $bindable(false),
    format = $bindable<OutputFormat>('flac'),
    outputDir = $bindable(''),
    downloadLyrics = $bindable(true),
    notifyOnDownloadComplete = $bindable(true),
    notifyOnPlaybackChange = $bindable(true),
    logLevel = $bindable<LogLevel>('error'),
    logRefreshToken = 0,
    notifyInfo,
    notifyError,
    onOutputDirChange,
  }: Props = $props();

  let logFileKind = $state<LogFileKind>('session');
  let logRecords = $state<LogViewerRecord[]>([]);
  let logFileStatus = $state<LogFileStatus | null>(null);
  let logViewerLoading = $state(false);
  let logViewerError = $state('');
  let logRequestSeq = 0;
  let isSendingTestNotification = $state(false);
  let isClearingAudioCache = $state(false);
  let lastLoadedWhileOpen = $state(false);

  async function refreshLogs(kind = logFileKind) {
    const requestSeq = ++logRequestSeq;
    logViewerLoading = true;
    logViewerError = '';
    try {
      const [page, status] = await Promise.all([
        listLogRecords({ kind, limit: 100 }),
        getLogFileStatus(),
      ]);
      if (requestSeq !== logRequestSeq || !open) {
        return;
      }
      logRecords = page.records;
      logFileStatus = status;
      logFileKind = kind;
    } catch (error) {
      if (requestSeq !== logRequestSeq || !open) {
        return;
      }
      logViewerError = error instanceof Error ? error.message : String(error);
    } finally {
      if (requestSeq !== logRequestSeq) {
        return;
      }
      logViewerLoading = false;
    }
  }

  async function handleSelectDirectory() {
    const currentOutputDir = outputDir;
    const dir = await selectDirectory(currentOutputDir);
    if (!dir || dir === currentOutputDir) {
      return;
    }

    outputDir = dir;
    const saved = await onOutputDirChange(dir);
    if (!saved) {
      outputDir = currentOutputDir;
      notifyError('保存下载目录失败，已恢复为之前的设置。');
    }
  }

  async function handleClearAudioCache() {
    if (isClearingAudioCache) return;
    isClearingAudioCache = true;
    try {
      const removed = await clearAudioCache();
      notifyInfo(
        removed > 0
          ? `已清除 ${removed} 个音频缓存文件`
          : '当前没有可清除的音频缓存'
      );
    } catch (error) {
      notifyError(
        `清除音频缓存失败：${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      isClearingAudioCache = false;
    }
  }

  async function handleSendTestNotification() {
    if (isSendingTestNotification) return;
    isSendingTestNotification = true;
    try {
      await sendTestNotification();
      notifyInfo('测试通知已请求发送，请观察系统通知中心或终端日志。');
    } catch (error) {
      notifyError(
        `发送测试通知失败：${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      isSendingTestNotification = false;
    }
  }

  $effect(() => {
    if (!open) {
      lastLoadedWhileOpen = false;
      logRequestSeq += 1;
      logViewerLoading = false;
      return;
    }

    if (lastLoadedWhileOpen) {
      return;
    }

    lastLoadedWhileOpen = true;
    void refreshLogs(logFileKind);
  });

  $effect(() => {
    const refreshToken = logRefreshToken;
    if (!open || !lastLoadedWhileOpen || refreshToken === 0) {
      return;
    }

    void refreshLogs(logFileKind);
  });
</script>

<Sheet.Root bind:open>
  <Sheet.Content
    class="w-[340px] border-white/50 bg-[var(--surface-sheet)] text-[var(--text-primary)] backdrop-blur-xl"
  >
    <Sheet.Header>
      <Sheet.Title>下载设置</Sheet.Title>
      <Sheet.Description>音频格式、通知和缓存管理</Sheet.Description>
    </Sheet.Header>

    <div class="space-y-6 py-2">
      <div class="space-y-2">
        <label class="text-sm text-[var(--text-secondary)]" for="format-select"
          >输出格式</label
        >
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
        <label
          class="text-sm text-[var(--text-secondary)]"
          for="log-level-select">持久化日志等级</label
        >
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
        <label class="text-sm text-[var(--text-secondary)]" for="output-dir"
          >保存位置</label
        >
        <input
          id="output-dir"
          class="w-full rounded-2xl border border-white/50 bg-white/[0.35] px-3 py-2 text-sm outline-none"
          readonly
          value={outputDir}
        />
        <Button class="w-full" onclick={() => void handleSelectDirectory()}>
          选择文件夹
        </Button>
      </div>

      <div
        class="space-y-4 rounded-[22px] border border-white/[0.45] bg-white/[0.28] p-4"
      >
        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <p class="text-sm font-medium">歌词文件</p>
            <p class="mt-1 text-xs text-[var(--text-secondary)]">
              有歌词时，在音频旁生成同名 `.lrc`。
            </p>
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
            onclick={() => void handleSendTestNotification()}
          >
            {isSendingTestNotification ? '正在发送...' : '发送测试通知'}
          </Button>
        </div>

        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <p class="text-sm font-medium">下载完成通知</p>
            <p class="mt-1 text-xs text-[var(--text-secondary)]">
              下载任务完成时显示通知。
            </p>
          </div>
          <Switch bind:checked={notifyOnDownloadComplete} />
        </div>

        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <p class="text-sm font-medium">播放切换通知</p>
            <p class="mt-1 text-xs text-[var(--text-secondary)]">
              播放新歌曲时显示通知。
            </p>
          </div>
          <Switch bind:checked={notifyOnPlaybackChange} />
        </div>
      </div>

      <div
        class="space-y-3 rounded-[22px] border border-white/45 bg-white/25 p-4"
      >
        <div>
          <p class="text-sm font-medium">音乐缓存</p>
          <p class="mt-1 text-xs text-[var(--text-secondary)]">
            播放时的音频缓存保存在系统缓存目录。
          </p>
        </div>
        <Button
          class="w-full"
          variant="secondary"
          disabled={isClearingAudioCache}
          onclick={() => void handleClearAudioCache()}
        >
          {isClearingAudioCache ? '正在清除缓存...' : '清除音频缓存'}
        </Button>
      </div>

      <div
        class="space-y-3 rounded-[22px] border border-white/45 bg-white/25 p-4"
      >
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
              variant={logFileKind === 'session' ? 'default' : 'secondary'}
              onclick={() => void refreshLogs('session')}
            >
              本次运行
            </Button>
            <Button
              size="sm"
              variant={logFileKind === 'persistent' ? 'default' : 'secondary'}
              onclick={() => void refreshLogs('persistent')}
            >
              持久化
            </Button>
          </div>
        </div>

        <p class="text-[11px] text-[var(--text-secondary)]">
          session: {logFileStatus?.hasSessionLog ? '可用' : '暂无'} · persistent:
          {logFileStatus?.hasPersistentLog ? '可用' : '暂无'}
        </p>

        {#if logViewerLoading}
          <div
            class="rounded-2xl border border-white/[0.30] bg-white/[0.18] px-3 py-4 text-xs text-[var(--text-secondary)]"
          >
            正在加载日志…
          </div>
        {:else if logViewerError}
          <div
            class="rounded-2xl border border-red-400/40 bg-red-500/[0.10] px-3 py-4 text-xs text-red-500/90"
          >
            {logViewerError}
          </div>
        {:else if logRecords.length > 0}
          <div
            class="max-h-[240px] space-y-2 overflow-y-auto rounded-2xl border border-white/[0.30] bg-white/[0.18] p-2"
          >
            {#each logRecords as record (record.id)}
              <div
                class="rounded-xl border border-white/[0.25] bg-white/[0.16] px-3 py-2"
              >
                <div class="flex items-center justify-between gap-2">
                  <span
                    class="text-[11px] font-medium uppercase text-[var(--text-secondary)]"
                    >{record.level}</span
                  >
                  <span class="text-[11px] text-[var(--text-secondary)]"
                    >{record.ts}</span
                  >
                </div>
                <p class="mt-1 text-xs font-medium">{record.message}</p>
                <p class="mt-1 text-[11px] text-[var(--text-secondary)]">
                  {record.domain} · {record.code}
                </p>
                {#if record.details}
                  <p
                    class="mt-1 whitespace-pre-wrap break-all text-[11px] text-[var(--text-secondary)]"
                  >
                    {record.details}
                  </p>
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <div
            class="rounded-2xl border border-white/[0.30] bg-white/[0.18] px-3 py-4 text-xs text-[var(--text-secondary)]"
          >
            当前没有可显示的日志记录。
          </div>
        {/if}
      </div>
    </div>
  </Sheet.Content>
</Sheet.Root>
