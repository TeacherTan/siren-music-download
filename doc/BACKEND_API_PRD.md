# 后端 API 规划 PRD

## 背景

当前应用已经具备以下后端能力：

- 专辑列表、专辑详情、歌曲详情查询
- 歌词文本拉取
- 在线播放、暂停、恢复、拖动进度、上一首 / 下一首
- 播放状态事件推送
- 单曲下载
- 音频缓存清理
- 封面主题色提取

但下载能力在 Tauri API 层仍然停留在“单次命令返回最终结果”的阶段：

- `download_song(songCid, outputDir, format, downloadLyrics) -> string`

这意味着后端虽然能完成单曲下载，但 API 无法表达：

- 批量下载
- 下载进度
- 下载任务队列
- 取消 / 重试
- 部分失败
- 结构化错误状态
- 前端可持续订阅的任务状态

同时，核心 Rust 库已经具备批量下载和进度回调基础：

- `crates/siren-core/src/downloader.rs` 中已有 `DownloadProgress`
- `crates/siren-core/src/downloader.rs` 中已有 `download_album()`

问题不在下载底层，而在 Tauri 后端 API 缺少“下载任务系统”这一层抽象。

## 问题定义

当前下载链路存在三个主要问题：

1. 下载是阻塞式命令，不是可追踪任务。
2. 下载进度只能在内部回调里流动，无法作为稳定 API 暴露给前端。
3. 单曲下载、整专下载、未来多选下载没有统一的任务模型，后续扩展会重复建设。

## 产品目标

本阶段以后端 API 为中心，目标是建立统一下载任务系统，使前端可以围绕“任务”而不是“单次命令”工作。

### Goals

1. 为单曲下载、整专下载、未来多选下载建立统一任务模型。
2. 暴露稳定的下载任务查询、创建、取消、重试 API。
3. 通过事件持续推送任务状态和下载进度。
4. 为下载进度 UI、下载历史、错误展示提供稳定后端契约。
5. 尽量复用当前 `siren-core` 的下载能力，避免重写下载核心。

### Non-goals

1. 不做跨设备同步。
2. 不做后台常驻服务或系统级下载器。
3. 不做断点续传。
4. 不做复杂优先级调度和高并发下载。
5. 不在第一阶段解决所有 UI 细节。

## 当前后端 API 现状

### 已有 Tauri commands

- `get_albums`
- `get_album_detail`
- `get_song_detail`
- `get_song_lyrics`
- `extract_image_theme`
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
- `download_song`
- `clear_audio_cache`

### 已有事件

- `player-state-changed`
- `player-progress`

### 当前下载相关能力

- 单曲下载已可用
- 下载歌词 sidecar 已可用
- FLAC 元数据写入已可用
- 整专下载底层能力已存在，但未暴露为 Tauri command
- `DownloadProgress` 已可表达当前歌曲字节进度和批次位置

## 用户故事

### 用户故事 1：下载当前歌曲

用户在曲目行或播放器上点击下载按钮后：

- 应立即得到“任务已创建”的反馈
- 可以看到任务状态从排队到完成
- 下载失败时可以看到明确错误

### 用户故事 2：下载整张专辑

用户在专辑详情页点击“下载整张专辑”后：

- 应创建一个包含多首歌曲的下载任务
- 可以看到总进度和当前歌曲进度
- 某一首失败时不影响其他歌曲完成
- 最终可看到“全部完成”或“部分失败”的结果
- 下载入口应位于专辑详情页主操作区，作为专辑级主操作之一，而不是要求用户逐首点击下载
- 当整专任务已在排队或执行中时，前端应提供明确的“进行中 / 已加入队列”反馈，避免重复创建同一专辑任务
- 整专下载时，歌曲文件应按专辑维度存放到独立目录，而不是直接平铺到输出根目录
- 整专下载时，专辑封面图也应下载到该专辑目录中，并统一命名为 `cover`（扩展名按实际图片格式确定，例如 `cover.jpg` / `cover.png`）

### 用户故事 3：重试失败项

用户在任务完成后看到其中若干失败项时：

- 可以只重试失败项
- 可以重新拉起整个任务

### 用户故事 4：取消下载

用户在下载过程中可以：

- 取消整个任务
- 后续阶段可选支持取消单个子任务

## 设计原则

1. 下载后端契约应复用播放器现有模式：命令负责拉起 / 查询，事件负责持续同步。
2. 先建立统一下载任务系统，再把单曲、整专、多选作为不同入口接入。
3. 第一阶段优先保证状态清晰、可追踪、可扩展，而不是追求并发下载速度。
4. API 输出应结构化，不再只返回字符串错误。
5. 共享类型命名和序列化风格应与现有 `PlayerState` 一致，统一采用 `camelCase`。
6. `siren-core` 负责平台无关的下载领域逻辑，`src-tauri` 负责 Tauri 命令、事件桥接和应用壳层策略。

## 库职责拆分

当前库边界有两个明显问题：

1. `siren-core` 已经承担 API、音频和下载逻辑，但导出面还停留在单曲下载能力，和后续任务系统目标不一致。
2. `src-tauri/src/main.rs` 当前既承担 Tauri command 注册，又承担部分下载 / 播放编排，应用壳层职责偏重。

### 冻结后的职责边界

#### `siren-core`

保留并增强平台无关、可测试、可复用的下载领域逻辑：

- 上游 API 访问
- 音频格式检测、保存、FLAC 元数据写入
- 下载任务领域模型
- 下载任务状态机
- 下载执行 worker
- 批量任务规划与任务聚合
- 进度回调与结构化领域事件
- 重试 / 取消语义

#### `src-tauri`

保留 Tauri 应用壳层和平台策略：

- Tauri commands
- Tauri events
- `AppState` 生命周期与状态注入
- 下载领域事件到前端事件的桥接
- 默认下载目录策略
- 音频缓存目录和播放缓存管理
- 媒体会话、窗口管理、播放器集成

### 当前看起来职责不清的点

1. `crates/siren-core/src/downloader.rs` 同时承载数据模型、单曲 / 整专流程和文件写入编排，下一步应拆分为更小模块。
2. `src-tauri/src/main.rs` 中的下载 command 仍以直接 orchestration 为主，后续应收敛为薄 command wrapper。
3. `get_default_output_dir` 这类平台策略应继续留在 `src-tauri`，不要下沉到共享库。

### 推荐目标结构

#### `siren-core`

```text
crates/siren-core/src/
├── api.rs
├── audio.rs
└── download/
    ├── mod.rs
    ├── model.rs
    ├── planner.rs
    ├── worker.rs
    ├── service.rs
    └── error.rs
```

说明：

- `model.rs`：`DownloadJobSnapshot`、`DownloadTaskSnapshot`、状态枚举、请求 / 选项类型
- `planner.rs`：把 song / album / selection 请求展开为 task 列表
- `worker.rs`：执行单个 task，调用底层下载与写文件逻辑
- `service.rs`：提供统一下载服务 façade，对上暴露创建任务、查询状态、取消、重试能力
- `error.rs`：结构化错误类型与映射规则

#### `src-tauri`

```text
src-tauri/src/
├── app_state.rs
├── commands/
│   ├── mod.rs
│   ├── playback.rs
│   ├── library.rs
│   └── downloads.rs
├── downloads/
│   ├── mod.rs
│   ├── bridge.rs
│   └── events.rs
├── audio_cache.rs
├── theme.rs
└── player/
```

说明：

- `commands/downloads.rs`：Tauri 下载命令包装层
- `downloads/bridge.rs`：把 `siren-core` 下载服务与 Tauri `AppHandle`/事件系统对接
- `downloads/events.rs`：定义事件名和发射辅助函数
- `app_state.rs`：组合 `ApiClient`、播放器和下载服务

### 推荐迁移策略

1. 先抽薄 `src-tauri/src/main.rs`，把 command handler 拆到 `commands/`，不改变行为。
2. 在 `siren-core` 内把当前 `downloader.rs` 拆到 `download/` 目录，但先保留兼容 façade。
3. 先冻结下载任务类型和命令 / 事件规格，再开始实现服务层。
4. 等新任务接口稳定后，再让旧 `download_song -> string` 退化为兼容层。

## 领域模型与 API 契约

领域模型（DownloadManager / DownloadJob / DownloadTask）、共享类型字段定义、Commands、Events、状态迁移规则和冻结决策统一维护在 [BACKEND_API_CONTRACT.md](BACKEND_API_CONTRACT.md)，本文档不再重复。

以下为 PRD 层面对领域模型的补充说明：

- **DownloadManager**：后端下载任务管理器，负责管理任务队列、控制执行顺序、维护运行中与历史任务状态、向前端发出状态事件。
- **DownloadJob**：一次完整下载请求，可来源于单曲下载、整专下载或多选下载。
- **DownloadTask**：Job 内的单首歌曲下载单元。

## 验收标准

### 后端 API 层

1. 后端可以创建单曲下载任务。
2. 后端可以创建整专下载任务。
3. 后端可以查询当前任务列表和单个任务详情。
4. 后端可以发出持续进度事件。
5. 后端可以明确区分成功、失败、部分失败、取消。
6. 后端可以取消任务。
7. 后端可以重试失败任务或失败项。

### 前端契约与交互层

1. 前端无需依赖阻塞式下载命令等待最终返回路径。
2. 前端可以仅通过快照和事件完整还原下载状态。
3. 单曲下载和批量下载共用同一套类型模型。
4. 返回结构中不再只有裸字符串错误。
5. 前端提供专辑级“批量下载”入口，用户可一键创建整专下载任务。
6. 前端在专辑级批量下载触发后，应立即展示“任务已创建 / 已加入队列”的反馈。
7. 下载面板可以同时展示单曲任务和整专任务，并能清晰区分任务标题、总进度和失败项。

### 文件落盘与产物层

1. 单曲下载默认保存到用户选择的输出根目录。
2. 整专下载保存到以专辑名命名的独立目录中。
3. 整专下载完成后，对应专辑目录内存在专辑封面文件，文件基础名固定为 `cover`。
4. 专辑封面文件与该专辑歌曲文件处于同一目录，便于媒体库或播放器识别。

## 关键决策

### 决策 1：第一阶段使用串行 worker

原因：

- 当前重点是任务模型和状态管理，而不是吞吐极限
- 串行队列更容易定义取消、错误和顺序语义
- 避免并发下载导致的磁盘和网络竞争问题

### 决策 2：保留 Job / Task 两层模型

原因：

- 单曲下载和整专下载可以统一建模
- 更适合表达部分失败、重试失败项和未来多选下载

### 决策 3：复用 `siren-core` 进度回调

原因：

- 底层 `DownloadProgress` 已经存在
- 可以避免重复设计下载流程
- 风险最低，改动边界清晰

## 主要风险（已部分缓解）

1. ~~当前 Tauri API 仍大量使用 `Result<_, String>`，需要补一层结构化错误映射。~~ 已解决：M1 阶段引入了 `DownloadErrorInfo` 结构，所有下载任务走新 API 使用结构化错误。
2. ~~取消语义若定义不清，容易在写文件、写标签、写歌词阶段出现状态混乱。~~ 已解决：M3 阶段实现了 `Arc<AtomicBool>` 取消标记，在下载流、FLAC 标签、歌词写入阶段均设置了检查点。
3. ~~若继续保留旧 `download_song -> string` 作为长期主接口，会导致双轨维护。~~ 已解决：旧接口保留为兼容层，新 UI 全部走任务 API。
4. 若过早引入并发下载，状态机和事件频率会明显复杂化。——仍需保持警惕，M5 流水线优化前应充分评估。

## 里程碑建议

### M1：下载任务基础设施 ✅ 已完成

- 下载领域模型
- DownloadManager
- 新 commands / events
- 单曲任务化

### M2：整专下载接入 ✅ 已完成

- 暴露 album 级任务创建接口
- 打通任务快照和进度事件
- 前端可展示总进度
- **专辑页批量下载入口**：在专辑详情页增加”下载整张专辑”按钮，直接创建 `kind = album` 的下载任务
- **重复创建保护**：前端应在同专辑任务已存在或运行时提供去重提示或禁用态，避免连续重复提交
- **按专辑目录落盘**：整专下载时，歌曲统一写入 `outputDir/<sanitizedAlbumName>/`
- **专辑封面落盘**：整专下载时，将封面图写入同一目录并命名为 `cover.<ext>`

### M3：下载任务增强 ✅ 已完成

- 取消
- 重试
- 历史清理
- 更细致错误信息
- **独立下载面板**：下载任务列表应通过独立的 UI 入口（独立按钮或图标）触发，而非合并在设置面板内。这样下载状态始终可及，不会被其他设置项遮挡。

### M4：系统通知集成 ✅ 已完成

**目标**：接入各平台原生通知机制，在关键下载和播放事件发生时向用户推送系统通知。

**通知触发场景**：

1. **下载完成通知**
   - 单曲下载完成：通知标题为歌曲名，内容为"下载完成"
   - 整专下载完成：通知标题为专辑名，内容为"专辑下载完成（N 首歌曲）"
   - 部分失败：通知标题为专辑名，内容为"专辑下载完成（N 首成功，M 首失败）"

2. **播放切换通知**
   - 播放新歌曲时：通知标题为歌曲名，内容为艺术家名
   - 显示专辑封面作为通知图标
   - 行为与 Apple Music 保持一致

**平台支持**：

- macOS：使用 `NSUserNotificationCenter` / `UNUserNotificationCenter`
- Windows：使用 Windows Toast Notifications
- Linux：使用 `libnotify` / D-Bus Notifications

**实现要点**：

1. **Tauri 通知 API**
   - 使用 Tauri 的 `notification` 插件统一跨平台通知
   - 配置通知权限请求
   - 处理用户授权状态

2. **通知内容结构**
   ```rust
   struct NotificationPayload {
       title: String,
       body: String,
       icon: Option<PathBuf>,  // 专辑封面路径
       sound: Option<String>,   // 通知音效
   }
   ```

3. **通知触发时机**
   - 下载任务状态变为 `completed` / `partiallyFailed` 时
   - 播放器状态变为 `playing` 且 `currentSong` 变化时
   - 通知应在后台和前台都能触发

4. **用户偏好设置**
   - 允许用户在设置中开关通知
   - 分别控制下载通知和播放通知
   - 默认全部开启

5. **通知去重**
   - 避免短时间内重复通知同一首歌
   - 整专下载只在全部完成时通知一次，不逐首通知

**验收标准**：

- [x] 单曲下载完成后显示系统通知
- [x] 整专下载完成后显示汇总通知
- [x] 播放切换时显示当前歌曲通知
- [x] 通知显示专辑封面图标
- [x] 用户可在设置中控制通知开关
- [x] 通知行为与 Apple Music 一致

**相关代码入口**：

- `src-tauri/Cargo.toml` — 添加 `tauri-plugin-notification` 依赖
- `src-tauri/src/main.rs` — 注册通知插件
- `src-tauri/src/downloads/events.rs` — 下载完成时触发通知
- `src-tauri/src/player/events.rs` — 播放切换时触发通知
- `src/lib/types.ts` — 添加通知偏好设置类型
- `src/App.svelte` — 设置面板中添加通知开关

### M5：未来扩展

- 多选下载
- 更丰富任务筛选和历史视图
- 可选的 session 持久化

### M5：性能优化

#### 流水线下载（Pipeline Download）

**目标**：提升整专下载吞吐量，减少串行等待时间。

**当前瓶颈**：

- 串行执行：下载 A → 写入 A → 下载 B → 写入 B
- 网络下载和磁盘写入无法并行，导致资源利用率低

**优化方案**：

- 引入下载缓冲区和异步写入队列
- 流水线模式：下载 A 的同时写入 B
- 保持任务状态机清晰，避免并发复杂度

**实现要点**：

1. **下载阶段**：
   - 下载完成后将音频数据放入内存缓冲区
   - 立即标记任务为 `writing` 状态
   - 将写入任务提交到异步写入队列
   - 不等待写入完成，立即开始下一首下载

2. **写入队列**：
   - 单独的异步写入 worker
   - 串行处理写入任务（避免磁盘竞争）
   - 写入完成后更新任务状态为 `completed`

3. **内存管理**：
   - 限制缓冲区大小（例如最多缓存 2-3 首歌曲）
   - 当缓冲区满时，下载 worker 等待写入完成
   - 避免内存占用过高

4. **取消语义**：
   - 下载阶段取消：立即停止，丢弃缓冲区
   - 写入阶段取消：等待当前写入完成，标记为 `cancelled`

**预期收益**：

- 整专下载时间减少 30-50%（取决于网络和磁盘速度）
- 网络和磁盘资源利用率提升
- 用户体验改善，下载更快完成

**风险与缓解**：

- 内存占用增加 → 限制缓冲区大小
- 状态管理复杂度 → 保持清晰的状态流转
- 取消语义复杂 → 明确定义各阶段取消行为

## 相关代码入口

### 后端

- `src-tauri/src/main.rs` — Tauri 命令注册
- `src-tauri/src/app_state.rs` — 应用状态（player、api、download_service）
- `src-tauri/src/commands/downloads.rs` — 下载命令包装层
- `src-tauri/src/downloads/bridge.rs` — 下载执行循环 + 事件桥接
- `src-tauri/src/downloads/events.rs` — 事件名常量
- `src-tauri/src/player/state.rs` — 播放器状态
- `src-tauri/src/player/events.rs` — 播放器事件名
- `crates/siren-core/src/download/model.rs` — 领域模型（DownloadJob、DownloadTask、Snapshot 类型）
- `crates/siren-core/src/download/service.rs` — 下载服务（job CRUD + 执行态管理）
- `crates/siren-core/src/download/worker.rs` — 单任务执行逻辑
- `crates/siren-core/src/download/error.rs` — 结构化错误类型
- `crates/siren-core/src/downloader.rs` — 底层 `download_song` / `download_album`（被 worker 调用）

### 前端

- `src/lib/api.ts` — Tauri command bridge（包含新的下载 job API）
- `src/lib/types.ts` — 前后端共享类型（包含完整的 DownloadJobSnapshot 等类型）
- `src/App.svelte` — 主界面（事件订阅、下载按钮、下载面板 UI）
- `src/app.css` — 样式（包含 `.download-panel`、`.download-job-card` 等）
