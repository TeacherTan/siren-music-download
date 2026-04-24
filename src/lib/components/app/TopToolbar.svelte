<script lang="ts">
  import { Button } from '$lib/components/ui/button/index.js';
  import { toolbarIconButton } from '$lib/design/variants';

  interface Props {
    activeDownloadCount: number;
    isRefreshing?: boolean;
    settingsOpen?: boolean;
    downloadPanelOpen?: boolean;
    onRefresh: () => void;
    onOpenDownloads: () => void;
    onOpenSettings: () => void;
  }

  let {
    activeDownloadCount,
    isRefreshing = false,
    settingsOpen = false,
    downloadPanelOpen = false,
    onRefresh,
    onOpenDownloads,
    onOpenSettings,
  }: Props = $props();
</script>

<div class="top-actions">
  <div
    class="flex items-center gap-2 rounded-full border border-white/50 bg-white/[0.62] p-2 shadow-[0_16px_36px_rgba(15,23,42,0.12)] backdrop-blur-xl"
  >
    <Button
      size="icon-sm"
      variant="ghost"
      class={toolbarIconButton({ active: false })}
      onclick={onRefresh}
      disabled={isRefreshing}
      aria-label="刷新缓存"
      title="刷新缓存"
    >
      ↻
    </Button>

    <Button
      size="icon-sm"
      variant="ghost"
      class={`relative ${toolbarIconButton({ active: downloadPanelOpen })}`}
      onclick={onOpenDownloads}
      aria-label="下载任务"
      aria-pressed={downloadPanelOpen}
      title="下载任务"
    >
      ↓
      {#if activeDownloadCount > 0}
        <span class="toolbar-badge">{activeDownloadCount}</span>
      {/if}
    </Button>

    <Button
      size="icon-sm"
      variant="ghost"
      class={toolbarIconButton({ active: settingsOpen })}
      onclick={onOpenSettings}
      aria-label="下载设置"
      aria-pressed={settingsOpen}
      title="下载设置"
    >
      ⚙
    </Button>
  </div>
</div>
