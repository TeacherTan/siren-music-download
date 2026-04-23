# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## 项目概览

- 技术栈：Rust + Tauri 2 + Vite + Svelte 5
- 形态：跨平台桌面应用（macOS / Windows / Linux）
- 当前重点：Phase 1–10 已完成；Phase 11 前端首版下载历史增强已落地，后端 Phase 11 扩展仍按条件触发，当前主线逐步转向 Phase 12 库内搜索 / 过滤

## 常用命令

```bash
bun install
bun run tauri:dev
bun run build
bun run tauri:build

cargo check --workspace
cargo fmt --all
cargo clippy --workspace --all-targets

# 文档
cargo doc -p siren_core --no-deps
cargo doc -p siren-music-download --lib --no-deps --document-private-items
cargo doc -p siren-music-download --bin siren-music-download --no-deps --document-private-items
```

## 仓库结构

```text
Cargo workspace
├── src-tauri/               # Tauri 后端 crate（library + binary）
│   └── src/
│       ├── lib.rs           # 后端模块 crate root，供 binary 与 integration tests 复用
│       ├── main.rs          # Tauri 可执行入口与 wiring
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
- `clear_response_cache`
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
- `list_log_records`
- `get_log_file_status`

播放器事件：

- `player-state-changed`
- `player-progress`

下载事件：

- `download-manager-state-changed`
- `download-job-updated`
- `download-task-progress`
- `local-inventory-state-changed`
- `local-inventory-scan-progress`
- `app-error-recorded`

## 文档结构

- `README.md`：项目介绍、使用方式、构建命令
- `doc/BACKEND_API_CONTRACT.md`：后端类型、命令、事件的唯一契约来源
- `doc/BACKEND_COMPLETED_PHASES.md`：后端已完成阶段（Phase 1~10）
- `doc/BACKEND_PENDING_PHASES.md`：后端待办阶段（Phase 11）
- `doc/FRONTEND_GUIDE.md`：前端架构、开发约定与验收基线
- `doc/DECISIONS.md`：技术选型决策记录（ADR）
- `doc/REVIEW_RULES.md`：测试整理、结构性重构、文档补充的评审与审批规则
- `doc/RELEASE_PROCESS.md`：CI 与发布流程

## 当前实现状态

- **已完成**：Phase 1–10，以及基于现有完整快照的 Phase 11 前端首版下载历史增强
- **进行中 / 未完成**：条件触发的 Phase 11 后端扩展、Phase 12
- **已完成阶段明细**：见 `doc/BACKEND_COMPLETED_PHASES.md`
- **待办阶段明细**：见 `doc/BACKEND_PENDING_PHASES.md`

## 代码层约定

- 后端“端点”指的是 Tauri command，不是 HTTP server route
- 共享数据结构优先在 Rust 侧定义，再让前端 `types.ts` 保持形状一致
- 所有对外暴露的 API 都必须编写函数文档，且文档内容统一使用中文；函数文档至少要说明用途、入参语义、出参/返回值语义以及关键副作用或错误场景；对于层级较高、承担入口职责的 API，还应补充说明适用场景、何时使用、使用注意事项与必要的调用约束；如涉及明确契约边界，还应写清前置条件、状态约束、不变量、是否幂等、是否允许重试等信息；从调用者视角出发，在有必要时补充返回数据的稳定性/兼容性预期、常见调用顺序与最小可用示例；新增或修改对外 API 时同步补齐或更新对应文档；在可行时尽量补充文档测试
- 如果改了 command 参数、返回值或事件载荷，要同步更新：
  - `src/lib/api.ts`
  - `src/lib/types.ts`
  - `README.md`
  - `src-tauri` / `siren_core` 中对应的 rustdoc
- 如果改了歌词、下载设置或播放器交互，同时检查 `src/App.svelte` 和 `src/lib/components/AudioPlayer.svelte` 的状态同步
- 未经用户明确指示，不要新建分支；默认在当前分支上工作，涉及分支切换、新建分支、基于分支的推送或 PR 准备时先确认
- 所有提交、PR 及相关 git / GitHub 协作文案一律使用中文
- 如果本轮改动属于测试整理、结构性重构或审批材料补充，优先对照 `doc/REVIEW_RULES.md` 中的通用规则，而不是把实现细节写进审批文档
