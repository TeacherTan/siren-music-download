# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## 项目概览

- 技术栈：Rust + Tauri 2 + Vite + Svelte 5
- 形态：跨平台桌面应用（macOS / Windows / Linux）
- 当前重点：M1–M5-a/b 已完成，当前主界面已切换到新的 UI shell，并具备系统通知、统一偏好系统与远程封面解析链路

## 常用命令

```bash
pnpm install
pnpm run tauri:dev
pnpm run build
pnpm run tauri:build

cargo check --workspace
cargo fmt --all
cargo clippy --workspace --all-targets

# 文档
cargo doc -p siren_core --no-deps
cargo doc -p siren-music-download --bin siren-music-download --no-deps --document-private-items
```

## 仓库结构

```text
Cargo workspace
├── src-tauri/               # Tauri 后端二进制 crate
│   └── src/
│       ├── main.rs          # Tauri command 入口
│       ├── app_state.rs     # 应用状态组合
│       ├── audio_cache.rs   # 流式播放缓存
│       ├── notification/    # 系统通知（公共入口、封面缓存、平台实现）
│       ├── theme.rs         # 封面取色
│       ├── commands/        # Tauri command 包装层
│       │   ├── mod.rs
│       │   ├── library.rs
│       │   ├── playback.rs
│       │   ├── preferences.rs
│       │   └── downloads.rs
│       ├── downloads/       # 下载桥接层与事件
│       │   ├── mod.rs
│       │   ├── bridge.rs
│       │   └── events.rs
│       └── player/          # 播放器实现
└── crates/
    └── siren-core/          # 共享 Rust 核心库
        └── src/
            ├── lib.rs       # 对外导出
            ├── api.rs       # 上游 HTTP API 客户端
            ├── audio.rs     # 音频格式检测 / 保存 / FLAC 标记
            ├── downloader.rs # 底层下载核心
            └── download/     # 下载任务领域
                ├── mod.rs
                ├── model.rs
                ├── planner.rs
                ├── service.rs
                ├── worker.rs
                └── error.rs
```

前端位于仓库根目录：

- `src/App.svelte`：主界面装配层，负责专辑加载、播放状态同步、下载任务状态同步、滚动舞台和面板编排
- `src/lib/api.ts`：Tauri command bridge
- `src/lib/cache.ts`：专辑详情、歌曲详情、歌词、主题色和远程封面 data URL 缓存
- `src/lib/theme.ts`：动态主题变量应用
- `src/lib/types.ts`：前后端共享数据结构的 TS 版本
- `src/lib/components/AlbumCard.svelte`：左侧专辑卡片
- `src/lib/components/SongRow.svelte`：曲目行，默认点击播放，进入多选模式后切换为勾选交互
- `src/lib/components/AudioPlayer.svelte`：播放器主体，包含进度、乱序 / 循环、歌词 / 队列切换和当前歌曲下载入口
- `src/lib/components/app/AlbumSidebar.svelte`：专辑侧栏容器
- `src/lib/components/app/TopToolbar.svelte`：顶部工具栏，包含刷新、下载任务入口、设置入口
- `src/lib/components/app/AlbumWorkspace.svelte`：主内容区容器
- `src/lib/components/app/PlayerDock.svelte`：底部播放器 Dock 容器
- `src/lib/components/app/SettingsSheet.svelte`：右侧设置面板
- `src/lib/components/app/DownloadTasksSheet.svelte`：右侧下载任务面板
- `src/lib/components/app/StatusToastHost.svelte`：toast 宿主
- `src/lib/features/`：按 `env / library / player / download / shell` 划分的前端状态脚手架与领域逻辑目录
- `src/lib/design/`：设计 token、动效参数和视觉 variant 定义

## 后端 command 清单

`src-tauri/src/main.rs` 当前注册了这些 Tauri command：

- `get_albums`
- `get_album_detail`
- `get_song_detail`
- `get_song_lyrics`
- `extract_image_theme`
- `get_image_data_url`
- `get_default_output_dir`
- `play_song`
- `stop_playback`
- `pause_playback`
- `resume_playback`
- `seek_current_playback`
- `play_next`
- `play_previous`
- `get_player_state`
- `set_playback_volume`
- `clear_audio_cache`
- `create_download_job`
- `list_download_jobs`
- `get_download_job`
- `cancel_download_job`
- `cancel_download_task`
- `retry_download_job`
- `retry_download_task`
- `clear_download_history`
- `get_preferences`
- `set_preferences`
- `export_preferences`
- `import_preferences`
- `get_local_inventory_snapshot`
- `rescan_local_inventory`
- `cancel_local_inventory_scan`
- `get_notification_permission_state`
- `send_test_notification`

播放器事件：

- `player-state-changed`
- `player-progress`

下载事件：

- `download-manager-state-changed`
- `download-job-updated`
- `download-task-progress`

## 文档结构

- `README.md`：项目介绍、使用方式、构建命令
- `doc/BACKEND_API_CONTRACT.md`：后端类型、命令、事件的唯一契约来源
- `doc/BACKEND_ROADMAP.md`：后端未来规划（Phase 6/7）
- `doc/FRONTEND_GUIDE.md`：前端架构、开发约定与验收基线
- `doc/DECISIONS.md`：技术选型决策记录（ADR）
- `doc/RELEASE_PROCESS.md`：CI 与发布流程

## 当前实现状态

### 已完成

- 专辑列表和曲目详情加载
- Tauri command + event 通信链路
- 在线播放、暂停、恢复、拖动进度
- 上一首 / 下一首
- 当前专辑上下文播放
- 播放列表乱序、列表循环 / 单曲循环
- 底部播放器、歌词面板、播放队列面板
- 系统媒体会话同步
- 封面主题色提取
- 流式播放缓存与缓存清理
- 当前播放曲目和专辑曲目行的单曲下载
- 歌词文本拉取与 `.lrc` 同目录保存开关
- FLAC 元数据和封面写入
- **M1** 下载任务领域模型、DownloadService、单曲任务化、新 commands / events
- **M2** 整专下载、专辑封面落盘、下载进度事件推送、前端总进度展示、专辑页批量下载入口、重复创建保护
- **M3** 任务取消、重试、历史清理、结构化错误码与详情、独立下载面板 UI
- **M4** 系统通知集成（下载完成通知、播放切换通知、通知权限检查与测试通知）
- **M5-a** 批量选择管理 UI（全选、清空、反选按钮）
- **M5-b** 流水线下载优化（download/write 两阶段流水线，整专下载吞吐提升）
- **Phase 5** 统一偏好系统（`AppPreferences` + `preferences.toml` 持久化 + 导入导出）

### 未完成

- **Phase 6** 本地已下载盘点与下载标记（当前进行中）
- M5 其他切片：搜索/过滤、下载历史视图增强、session 持久化

## 代码层约定

- 后端“端点”指的是 Tauri command，不是 HTTP server route
- 共享数据结构优先在 Rust 侧定义，再让前端 `types.ts` 保持形状一致
- 如果改了 command 参数、返回值或事件载荷，要同步更新：
  - `src/lib/api.ts`
  - `src/lib/types.ts`
  - `README.md`
  - `src-tauri` / `siren_core` 中对应的 rustdoc
- 如果改了歌词、下载设置或播放器交互，同时检查 `src/App.svelte` 和 `src/lib/components/AudioPlayer.svelte` 的状态同步
