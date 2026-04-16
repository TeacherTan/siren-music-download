# 塞壬音乐下载器

基于 [Rust](https://www.rust-lang.org/) + [Tauri 2](https://tauri.app/) + Svelte 5 的桌面应用，用来浏览 [塞壬唱片](https://monster-siren.hypergryph.com/)公开曲库、在线播放，并将当前曲目下载为 WAV / FLAC / MP3。

## 功能亮点

- 桌面端一体化体验：专辑浏览、歌曲详情、在线播放、下载都在一个窗口里完成
- 流式播放链路：边下边播，带本地缓存和缓存清理
- 完整播放器能力：暂停、继续、拖动进度、上一首 / 下一首、乱序、列表循环 / 单曲循环、媒体会话控制
- 歌词与队列面板：底部播放器可展开歌词和当前播放列表
- 下载结果更完整：FLAC 可写入标题、艺术家、专辑、曲序、封面和同名 `.lrc` 歌词
- 动态界面主题：根据专辑封面自动提取强调色

## 当前实现

- 已完成：专辑列表、曲目详情、在线播放、暂停 / 继续、拖动进度、上一首 / 下一首、系统媒体会话
- 已完成：播放器歌词面板、播放队列面板、乱序播放、列表循环 / 单曲循环、封面取色动态主题、流式音频缓存与缓存清理
- 已完成：当前播放曲目和专辑歌曲列表中的单曲下载，支持 `wav` / `flac` / `mp3`
- 已完成：歌词文本拉取；下载时可选同目录生成同名 `.lrc`
- 已完成：FLAC 输出写入标题、艺术家、专辑名、专辑艺术家、曲序和封面元数据
- 已完成（M1）：下载任务领域模型、DownloadService、单曲任务化、新 commands / events
- 已完成（M2）：整专下载、专辑封面落盘、下载进度事件推送、前端总进度展示、专辑页批量下载入口、重复创建保护
- 已完成（M3）：任务取消、重试、历史清理、结构化错误码与详情、独立下载面板 UI
- 已完成（M4）：系统通知集成（下载完成通知、播放切换通知、通知偏好开关）

## 依赖

| 工具 | 说明 |
| --- | --- |
| Rust 1.70+ | Rust 编译环境，建议通过 [rustup](https://rustup.rs/) 安装 |
| Node 18+ | 前端构建与 Tauri CLI 运行环境 |

说明：
- FLAC 编码使用纯 Rust `flacenc`，不依赖 `ffmpeg`
- WAV 和 MP3 输出也不需要外部转码工具

## 开发与构建

```bash
npm install

# 前端 + Tauri 开发模式
npm run tauri:dev

# 仅构建前端
npm run build

# 打包桌面应用
npm run tauri:build

# Rust 检查
cargo check --workspace
cargo fmt --all
cargo clippy --workspace --all-targets
```

## 使用方式

1. 启动应用后会自动拉取专辑列表。
2. 左侧边栏选择专辑，主区域会展示封面横幅、简介和曲目列表。
3. 单击曲目可开始播放，底部播放器支持暂停、继续、拖动进度、上一首、下一首、乱序和循环模式切换。
4. 底部播放器可展开歌词面板和当前播放队列；队列中的曲目可直接点击切换播放。
5. 右上角设置图标可打开下载设置面板，选择输出格式、下载目录、是否生成同名 `.lrc` 歌词文件，以及下载完成 / 播放切换系统通知开关。
6. 当前播放曲目和专辑曲目行都可以直接触发单曲下载。
7. 专辑详情页横幅区域有”下载整张专辑”按钮，可一键创建整专下载任务。
8. 工具栏下载图标可打开独立下载面板，展示任务列表、进度、取消/重试按钮，以及”清理历史”功能。
9. 下载任务完成或播放切换到新曲目时，应用会触发系统通知。

## 已知限制

- 数据完全依赖塞壬唱片公开 API，若上游接口结构或资源地址变化，应用也需要同步调整
- 首次播放、拖动进度或切歌时会进行音频拉取与缓存预热，网络较慢时体感会受影响

## 后端 API

前端通过 `@tauri-apps/api/core` 的 `invoke()` 调用 Rust 后端，完整的 command / event 契约、共享类型定义和状态机规则见 [doc/BACKEND_API_PLAN.md](doc/BACKEND_API_PLAN.md)。

## 上游 HTTP API

`siren_core::ApiClient` 封装了塞壬唱片公开 REST API：

| 接口 | 说明 |
| --- | --- |
| `GET /api/albums` | 获取全部专辑列表 |
| `GET /api/album/:cid/detail` | 获取专辑详情及曲目列表 |
| `GET /api/song/:cid` | 获取单曲详情，包含 `sourceUrl` |

## 生成 Rust 文档

```bash
# 共享核心库文档
cargo doc -p siren_core --no-deps

# Tauri 后端 command 文档（包含私有 command 函数）
cargo doc -p siren-music-download --bin siren-music-download --no-deps --document-private-items
```

`siren_core` 的文档主要覆盖上游 API、音频处理和下载流程；`src-tauri` 的文档主要覆盖 Tauri command、播放器事件和前后端共享的后端数据结构。

## 项目结构

```text
.
├── Cargo.toml
├── README.md
├── UI_DESIGN.md
├── doc/
│   ├── BACKEND_API_CONTRACT.md  # 下载任务系统 API 契约（唯一事实来源）
│   ├── BACKEND_API_PRD.md       # 下载任务系统产品需求
│   └── BACKEND_API_PLAN.md      # 下载任务系统实施计划
├── src/
│   ├── App.svelte
│   ├── app.css
│   ├── main.ts
│   └── lib/
│       ├── api.ts
│       ├── cache.ts
│       ├── lazyLoad.ts
│       ├── theme.ts
│       ├── types.ts
│       ├── actions/
│       └── components/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs          # Tauri command 入口
│       ├── app_state.rs     # 应用状态组合
│       ├── audio_cache.rs   # 流式播放缓存
│       ├── theme.rs         # 封面取色
│       ├── commands/       # Tauri command 包装层
│       ├── downloads/       # 下载桥接层与事件
│       └── player/          # 播放器实现
└── crates/
    └── siren-core/
        └── src/
            ├── lib.rs
            ├── api.rs
            ├── audio.rs
            ├── downloader.rs
            └── download/    # 下载领域模型与执行
                ├── mod.rs
                ├── model.rs
                ├── planner.rs
                ├── service.rs
                ├── worker.rs
                └── error.rs
```

## 许可证

本项目基于 [MIT](./LICENSE) 许可证发布。
