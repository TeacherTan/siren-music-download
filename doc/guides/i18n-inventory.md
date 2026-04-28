# i18n 文件级文案粗清单

> 本文档记录待迁移文件范围和当前状态，不维护独立 key 级清单。
> 具体 message key 以 Paraglide message 文件、后端 Fluent `.ftl` 文件和对应 PR description 为准。
>
> 首轮语言：`zh-CN`（基准）、`en-US`
>
> 最后更新：2026-04-28

## 前端文案清单

| 文件                                                  | 业务域   | 主要类型                 | 估算条数 | 状态 |
| ----------------------------------------------------- | -------- | ------------------------ | -------- | ---- |
| `src/App.svelte`                                      | shell    | toast / dynamic          | 3        | done |
| `src/lib/components/app/TopToolbar.svelte`            | shell    | title / aria             | 3        | done |
| `src/lib/components/app/AlbumWorkspaceContent.svelte` | library  | static                   | 2        | done |
| `src/lib/components/app/AlbumDetailSkeleton.svelte`   | library  | static                   | 1        | done |
| `src/lib/components/app/AlbumSidebar.svelte`          | library  | static / dynamic / aria  | 14       | done |
| `src/lib/components/app/AlbumDetailPanel.svelte`      | library  | static / dynamic         | 12       | done |
| `src/lib/components/SongRow.svelte`                   | library  | aria / title / dynamic   | 8        | done |
| `src/lib/components/AudioPlayer.svelte`               | player   | aria / dynamic           | 21       | done |
| `src/lib/components/app/PlayerFlyoutStack.svelte`     | player   | static / aria            | 4        | done |
| `src/lib/components/app/SettingsSheet.svelte`         | settings | static / toast / dynamic | 28       | done |
| `src/lib/components/app/DownloadTasksSheet.svelte`    | download | static / aria            | 18       | done |
| `src/lib/features/download/controller.svelte.ts`      | download | toast / dynamic          | 30       | done |
| `src/lib/features/download/formatters.ts`             | download | dynamic                  | 9        | done |
| `src/lib/downloadBadge.ts`                            | common   | static                   | 6        | done |
| `src/lib/features/player/controller.svelte.ts`        | player   | toast                    | 4        | done |
| `src/lib/features/shell/settings.svelte.ts`           | settings | toast                    | 1        | done |
| `src/lib/features/shell/store.svelte.ts`              | shell    | toast                    | 2        | done |
| `src/lib/features/library/controller.svelte.ts`       | library  | toast                    | 2        | done |
| `src/lib/components/app/AlbumStage.svelte`            | library  | dynamic                  | 1        | done |

## 后端文案清单

| 文件                                        | 业务域       | 主要类型 | 估算条数 | 状态    |
| ------------------------------------------- | ------------ | -------- | -------- | ------- |
| `src-tauri/src/notification/mod.rs`         | notification | backend  | 5        | done    |
| `src-tauri/src/notification/macos.rs`       | notification | backend  | 2        | done    |
| `src-tauri/src/notification/desktop.rs`     | notification | backend  | 2        | done    |
| `src-tauri/src/preferences.rs`              | preferences  | backend  | 22       | done    |
| `src-tauri/src/commands/preferences.rs`     | preferences  | backend  | 3        | done    |
| `src-tauri/src/app_state.rs`                | app_state    | backend  | 1        | done    |
| `src-tauri/src/download_session.rs`         | download     | backend  | 4        | done    |
| `src-tauri/src/local_inventory.rs`          | inventory    | backend  | 5        | done    |
| `src-tauri/src/search/index.rs`             | search       | backend  | 2        | done    |
| `src-tauri/src/search/service.rs`           | search       | backend  | 3        | done    |
| `crates/siren-core/src/download/service.rs` | download     | backend  | 5        | 不改    |

## 不翻译的内容

- 专辑名、歌曲名、艺术家名、歌词（上游 API 返回的内容数据）
- 日志 key、内部错误 key、Rust / TS 类型名、Tauri command 名称
- rustdoc、开发文档、README
- 构建产物安装包元信息

## Fallback 策略

- 前端：Paraglide message 缺 key 时编译期报错；运行时缺语言回退 `zh-CN`
- 后端：Fluent 目标语言缺 key 时回退 `zh-CN`；`zh-CN` 仍缺失时返回 message id
- 参数缺失：Paraglide 编译期类型检查；Fluent 使用 `{$param}` 原样输出
