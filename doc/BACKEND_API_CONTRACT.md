# 后端 API 契约

本文档是后端类型、命令、事件和状态机规则的唯一契约来源。

相关文档：

- [BACKEND_ROADMAP.md](BACKEND_ROADMAP.md)：后端未来规划（Phase 5~9）
- [FRONTEND_GUIDE.md](FRONTEND_GUIDE.md)：前端架构与开发指南

## 共享类型

### Rust / TS 对齐类型清单

- `OutputFormat`
- `DownloadOptions`
- `DownloadJobKind`
- `DownloadJobStatus`
- `DownloadTaskStatus`
- `DownloadErrorCode`
- `DownloadErrorInfo`
- `DownloadTaskSnapshot`
- `DownloadJobSnapshot`
- `DownloadManagerSnapshot`
- `CreateDownloadJobRequest`
- `DownloadTaskProgressEvent`
- `LocalTrackDownloadStatus`
- `TrackDownloadBadge`
- `LocalInventoryStatus`
- `VerificationMode`
- `LocalInventorySnapshot`
- `LocalInventoryScanProgressEvent`
- `Album`
- `SongEntry`
- `SongDetail`
- `AlbumDetail`
- `AppPreferences`
- `NotificationPermissionState`
- `LogLevel`
- `LogFileKind`
- `LogViewerQuery`
- `LogViewerRecord`
- `LogViewerPage`
- `LogFileStatus`
- `AppErrorEvent`

## 类型字段定义

### `OutputFormat`

枚举值：

- `flac`
- `wav`
- `mp3`

### `DownloadOptions`

- `outputDir: string`
- `format: OutputFormat`
- `downloadLyrics: boolean`

### `DownloadJobKind`

枚举值：

- `song`
- `album`
- `selection`

### `DownloadJobStatus`

枚举值：

- `queued`
- `running`
- `completed`
- `partiallyFailed`
- `failed`
- `cancelled`

### `DownloadTaskStatus`

枚举值：

- `queued`
- `preparing`
- `downloading`
- `writing`
- `completed`
- `failed`
- `cancelled`

### `DownloadErrorCode`

枚举值：

- `network`
- `api`
- `io`
- `decode`
- `tagging`
- `lyrics`
- `cancelled`
- `invalidRequest`
- `internal`

### `DownloadErrorInfo`

- `code: DownloadErrorCode`
- `message: string`
- `retryable: boolean`
- `details: string | null`

### `DownloadTaskSnapshot`

- `id: string`
- `jobId: string`
- `songCid: string`
- `songName: string`
- `artists: string[]`
- `albumCid: string`
- `albumName: string`
- `status: DownloadTaskStatus`
- `bytesDone: number`
- `bytesTotal: number | null`
- `outputPath: string | null`
- `error: DownloadErrorInfo | null`
- `attempt: number`
- `songIndex: number`
- `songCount: number`

### `DownloadJobSnapshot`

- `id: string`
- `kind: DownloadJobKind`
- `status: DownloadJobStatus`
- `createdAt: string`
- `startedAt: string | null`
- `finishedAt: string | null`
- `options: DownloadOptions`
- `title: string`
- `taskCount: number`
- `completedTaskCount: number`
- `failedTaskCount: number`
- `cancelledTaskCount: number`
- `tasks: DownloadTaskSnapshot[]`
- `error: DownloadErrorInfo | null`

### `DownloadManagerSnapshot`

- `jobs: DownloadJobSnapshot[]`
- `activeJobId: string | null`
- `queuedJobIds: string[]`

### `CreateDownloadJobRequest`

- `kind: DownloadJobKind`
- `songCids: string[]`
- `albumCid: string | null`
- `options: DownloadOptions`

### `DownloadTaskProgressEvent`

- `jobId: string`
- `taskId: string`
- `status: DownloadTaskStatus`
- `bytesDone: number`
- `bytesTotal: number | null`
- `songIndex: number`
- `songCount: number`

### `LocalTrackDownloadStatus`

枚举值：

- `missing`
- `detected`
- `verified`
- `mismatch`
- `partial`
- `unverifiable`
- `unknown`

语义：

- `missing`：当前 active `outputDir` 下未发现可接受的本地文件
- `detected`：已确认本地文件存在，并可作为“已下载”消费，但本次未进入校验或无需校验
- `verified`：已通过直接可比的 checksum 校验，或通过可信 provenance 映射确认来源一致
- `mismatch`：已进入可比校验，且校验结果明确不一致
- `partial`：只发现了部分预期产物，尚不足以视为完整一致，但仍可提示用户本地已有残留内容
- `unverifiable`：已确认本地文件存在，但在 `strict` 模式下因远端 checksum 缺失、不可比、来源链条不完整或校验前提不足，无法给出一致性结论
- `unknown`：发现了候选文件或线索，但尚不足以确认“该曲目已在本地存在”

前端布尔下载标记映射规则：

- `isDownloaded = true` 对应 `detected | verified | partial | unverifiable`
- `isDownloaded = false` 对应 `missing | mismatch | unknown`

### `TrackDownloadBadge`

- `isDownloaded: boolean`
- `downloadStatus: LocalTrackDownloadStatus`
- `inventoryVersion: string`

字段职责：

- 前端直接消费 `isDownloaded`
- 更细粒度语义由 `downloadStatus` 表达
- `inventoryVersion` 用于动态缓存失效

### `LocalInventoryStatus`

枚举值：

- `idle`
- `scanning`
- `completed`
- `failed`

### `VerificationMode`

枚举值：

- `none`
- `whenAvailable`
- `strict`

语义：

- `none`：只做本地盘点与匹配，不做 MD5 或 provenance 校验
- `whenAvailable`：仅在本地文件与远端 checksum 语义可比，或存在可验证的 provenance 映射时尝试校验；否则保留“已检测到但未校验”语义
- `strict`：在 `whenAvailable` 基础上，显式把“已存在但无法校验”标记为 `unverifiable`，而不是折叠为 `detected`

### `LocalInventorySnapshot`

- `rootOutputDir: string`
- `status: LocalInventoryStatus`
- `inventoryVersion: string`
- `startedAt: string | null`
- `finishedAt: string | null`
- `scannedFileCount: number`
- `matchedTrackCount: number`
- `verifiedTrackCount: number`
- `lastError: string | null`

### `LocalInventoryScanProgressEvent`

- `rootOutputDir: string`
- `inventoryVersion: string`
- `filesScanned: number`
- `matchedTrackCount: number`
- `verifiedTrackCount: number`
- `currentPath: string | null`

### `Album`

- `cid: string`
- `name: string`
- `coverUrl: string`
- `artists: string[]`

### `SongEntry`

- `cid: string`
- `name: string`
- `artists: string[]`
- `download: TrackDownloadBadge`

### `SongDetail`

- `cid: string`
- `name: string`
- `albumCid: string`
- `sourceUrl: string`
- `lyricUrl: string | null`
- `mvUrl: string | null`
- `mvCoverUrl: string | null`
- `artists: string[]`
- `download: TrackDownloadBadge`

### `AlbumDetail`

- `cid: string`
- `name: string`
- `intro: string | null`
- `belong: string`
- `coverUrl: string`
- `coverDeUrl: string | null`
- `artists: string[] | null`
- `songs: SongEntry[]`

### `AppPreferences`

应用偏好持久化到 `{app_data_dir}/preferences.toml`。

- `outputFormat: OutputFormat`
- `outputDir: string`
- `downloadLyrics: boolean`
- `notifyOnDownloadComplete: boolean`
- `notifyOnPlaybackChange: boolean`
- `logLevel: LogLevel`

### `NotificationPermissionState`

枚举值：

- `granted`
- `denied`
- `prompt`
- `prompt-with-rationale`

### `LogLevel`

枚举值：

- `debug`
- `info`
- `warn`
- `error`

语义：

- 用作持久化日志晋升阈值
- 应用退出时，session log 中 `level >= logLevel` 的记录写入 persistent log

### `LogFileKind`

枚举值：

- `session`：本次运行日志，应用干净退出时删除
- `persistent`：持久化日志，累积保留

### `LogViewerQuery`

- `kind: LogFileKind`
- `level?: LogLevel | null`
- `domain?: string | null`
- `search?: string | null`
- `limit?: number | null`
- `offset?: number | null`

### `LogViewerRecord`

前端安全投影，不暴露后端敏感细节。

- `id: string`
- `ts: string`
- `level: LogLevel`
- `domain: string`
- `code: string`
- `message: string`
- `details: null`（固定为 null，不暴露后端 details）

### `LogViewerPage`

- `records: LogViewerRecord[]`
- `total: number`
- `kind: LogFileKind`

### `LogFileStatus`

- `hasSessionLog: boolean`
- `hasPersistentLog: boolean`

### `AppErrorEvent`

前端安全运行时错误事件，仅在后端标记 `frontend_event()` 时发出。

- `id: string`
- `ts: string`
- `level: LogLevel`（仅 `warn` 或 `error`）
- `domain: string`
- `code: string`
- `message: string`

### `NotificationPermissionState`

枚举值（已存在，保持不变）：

- `granted`
- `denied`
- `prompt`
- `prompt-with-rationale`

### 权限分级与路径边界

后端命令按权限面分为三类：

1. **只读命令**：只读取远端内容或内存态，不接收本地文件系统路径参数。
   - 例如：`get_albums`、`get_album_detail`、`get_song_detail`、`get_song_lyrics`、`get_player_state`
2. **受限文件命令**：允许读写本地文件系统，但只应作用于当前受信工作目录或应用数据目录。
   - 例如：下载落盘、本地盘点、偏好持久化
3. **高风险文件命令**：涉及用户指定导入/导出路径，必须被视为显式授权操作。
   - 例如：`export_preferences`、`import_preferences`

路径边界规则：

- `AppPreferences.outputDir` 是下载与本地盘点的唯一 active root
- 本地盘点只允许扫描当前 active `outputDir`，不得隐式扩展到其他目录
- 下载任务默认只允许写入当前 active `outputDir` 及其派生专辑子目录
- 应用内部状态文件只允许写入 `{app_data_dir}`
- session log 只允许写入 `{app_cache_dir}/logs/`
- persistent log 只允许写入 `{app_data_dir}/logs/`
- 日志浏览命令只允许读取应用自有的 session / persistent 日志文件，不开放任意路径浏览
- 导入/导出命令虽然接收用户提供路径，但在实现上应视为高风险能力，不得被普通业务命令复用为通用文件读写通道
- 若前端不需要消费绝对路径，则命令返回值、事件和错误信息不应暴露绝对本地路径

前端可见性规则：

- 事件与命令返回优先暴露业务语义，不暴露超出 UI 所需的本地文件系统细节
- 进度事件中的路径字段应优先使用相对路径、标识符或可省略字段，而不是绝对路径
- 错误字符串应避免把本地绝对路径直接回传到前端
- 日志浏览返回 `LogViewerRecord` 前端安全投影，而不是后端完整日志记录
- `LogViewerRecord.details` 固定为 `null`，前端 viewer 不直接暴露后端原始 details / cause chain / context
- `app-error-recorded` 只承载前端安全字段，不作为后端完整日志明细通道

实现约束：

- 新增 Tauri command 时，必须先归类到上述三类之一，再决定是否允许接收路径参数
- 默认不把“任意路径读写”设计成通用能力暴露给前端
- 若命令需要用户指定路径，契约和前端交互都应明确这是一次显式授权，而不是普通后台行为

### 内容命令

命令如下：

1. `get_albums() -> Album[]`
2. `get_album_detail(albumCid: string) -> AlbumDetail`
3. `get_song_detail(cid: string) -> SongDetail`
4. `get_song_lyrics(cid: string) -> string | null`
5. `extract_image_theme(imageUrl: string) -> ThemePalette`
6. `get_image_data_url(imageUrl: string) -> string`
7. `get_default_output_dir() -> string`

返回约束：

- `get_albums` 是轻量列表接口，不触发 per-album 详情 fan-out
- `get_albums` 不返回专辑级下载 badge
- `get_album_detail` 不承载专辑级聚合下载 badge
- `get_song_detail` 的返回值必须包含 `download` 字段
- `SongEntry.download` 继续由 `get_album_detail` 内联返回

### 本地盘点命令

命令如下：

1. `get_local_inventory_snapshot() -> LocalInventorySnapshot`
2. `rescan_local_inventory(verificationMode?: VerificationMode) -> LocalInventorySnapshot`
3. `cancel_local_inventory_scan() -> LocalInventorySnapshot`

扫描行为：

- `set_preferences()` 中 `outputDir` 变更后，异步触发新的本地盘点
- `set_preferences()` 不阻塞等待盘点完成
- 同一时刻仅允许一个 active root；新扫描请求可以覆盖旧扫描请求
- 盘点命令不得把当前 active root 之外的目录作为隐式扫描目标
- 盘点事件与快照若无 UI 必要，不应暴露绝对本地路径

### 下载任务命令

命令如下：

1. `create_download_job(request: CreateDownloadJobRequest) -> DownloadJobSnapshot`
2. `list_download_jobs() -> DownloadManagerSnapshot`
3. `get_download_job(jobId: string) -> DownloadJobSnapshot`
4. `cancel_download_job(jobId: string) -> DownloadJobSnapshot`
5. `cancel_download_task(jobId: string, taskId: string) -> DownloadJobSnapshot`
6. `retry_download_job(jobId: string) -> DownloadJobSnapshot`
7. `retry_download_task(jobId: string, taskId: string) -> DownloadJobSnapshot`
8. `clear_download_history() -> number`

建模约束：

- `create_download_job` 通过 `kind` / `albumCid` 表达整专下载
- 下载写入根目录应受当前 active `outputDir` 约束，不应退化为前端可随意指定的通用文件写入能力

### 日志命令

1. `list_log_records(query: LogViewerQuery) -> LogViewerPage`
2. `get_log_file_status() -> LogFileStatus`

行为说明：

- `list_log_records` 只面向设置页日志 viewer，读取应用自有日志文件，不接收任意路径参数
- `kind = session` 读取本次运行日志；`kind = persistent` 读取持久化日志
- 返回值是前端安全投影，供 UI 浏览摘要，不等价于后端完整日志记录
- `search` 只匹配前端可见字段（message / code / domain），不匹配隐藏 details
- `limit` / `offset` 用于 viewer 分页或分批加载

返回约束：

- `records` 按时间倒序返回
- `LogViewerRecord.details` 固定为 `null`
- 不通过 viewer 命令向前端暴露后端 `details`、`context`、`causeChain` 或绝对路径
- `get_log_file_status` 仅返回文件存在性摘要，不返回路径

### 偏好命令

1. `get_preferences() -> AppPreferences`
2. `set_preferences(preferences: AppPreferences) -> AppPreferences`
3. `get_notification_permission_state() -> NotificationPermissionState`
4. `send_test_notification() -> void`

行为说明：

- 通知开关字段属于 `AppPreferences`，不再单独暴露 `NotificationPreferences`
- 通知权限状态由 Tauri 通知插件返回，反映系统级权限授予情况
- 测试通知用于验证通知管道是否正常工作
- `logLevel` 属于 `AppPreferences`，用于控制 session log 退出时晋升到 persistent log 的最低等级

验证规则：

- `outputFormat`：必须是 `flac` | `wav` | `mp3` 之一
- `outputDir`：路径必须存在且为目录
- `downloadLyrics`：布尔值
- `notifyOnDownloadComplete`：布尔值
- `notifyOnPlaybackChange`：布尔值
- `logLevel`：必须是 `debug` | `info` | `warn` | `error` 之一

验证失败时返回错误字符串，命令不更新状态。

存储规则：

- 偏好通过版本化 TOML 文件持久化到 `{app_data_dir}/preferences.toml`
- `{app_data_dir}` 路径由 Tauri 运行时根据 `tauri.conf.json` 中的 `identifier` 决定
  - macOS：`~/Library/Application Support/{identifier}/`
  - Windows：`%APPDATA%/{identifier}/`
  - Linux：`~/.local/share/{identifier}/`
- 文件顶层包含 `schemaVersion: integer` 字段，初始值为 `1`
- 应用启动时自动加载，缺失或损坏时使用默认值初始化并写入磁盘
- 设置变更时同步落盘（阻塞式原子写入）

### 偏好备份命令

1. `export_preferences(outputPath: string) -> AppPreferences`
2. `import_preferences(inputPath: string) -> AppPreferences`

导入导出规则：

- `export_preferences` 和 `import_preferences` 都由用户指定路径，并返回操作后的偏好快照
- `import_preferences` 导入 TOML 后执行与 `set_preferences` 相同的验证规则；验证失败时返回错误且不更新状态
- 两者属于高风险文件命令：语义上视为“用户显式授权的一次性读/写”，不是普通业务命令可复用的通用文件系统接口
- 若前端不需要绝对路径回显，则返回值和错误信息不应额外暴露本地绝对路径

## Events

事件如下：

1. `download-manager-state-changed`，载荷为 `DownloadManagerSnapshot`
2. `download-job-updated`，载荷为 `DownloadJobSnapshot`
3. `download-task-progress`，载荷为 `DownloadTaskProgressEvent`
4. `local-inventory-state-changed`，载荷为 `LocalInventorySnapshot`
5. `local-inventory-scan-progress`，载荷为 `LocalInventoryScanProgressEvent`
6. `app-error-recorded`，载荷为 `AppErrorEvent`

事件职责：

- `download-manager-state-changed` 同步任务列表概览
- `download-job-updated` 同步任务完整快照
- `download-task-progress` 同步细粒度下载进度
- `local-inventory-state-changed` 同步盘点状态与 `inventoryVersion`
- `local-inventory-scan-progress` 同步盘点进度
- `app-error-recorded` 向前端推送运行时错误的安全摘要，用于 toast 或日志 viewer 刷新

## 快照与事件载荷原则

为降低前端状态同步复杂度，保持与播放器一致：

- 快照事件尽量发送完整结构，而不是零散 patch
- 进度事件只在高频字段变化时发出
- 命令返回值与事件载荷的结构保持一致
- 运行时错误的实时通知只发送前端安全摘要，不把后端完整日志记录当作事件载荷直接广播
- 日志文件是运行时错误的长期追溯真相源；toast / inline / panel 只是不同投影，而不是独立真相源

## 状态迁移

### Job 状态流

- `queued -> running -> completed`
- `queued -> running -> failed`
- `queued -> running -> partiallyFailed`
- `queued -> cancelled`
- `running -> cancelled`

规则：

- `partiallyFailed` 表示至少一个 task 成功，且至少一个 task 失败或取消。
- `failed` 表示没有任何 task 成功，且至少一个 task 失败。
- Job 终态由 task 终态聚合推导，不能由上层随意写入。

### Task 状态流

- `queued -> preparing -> downloading -> writing -> completed`
- `queued | preparing | downloading | writing -> failed`
- `queued | preparing | downloading -> cancelled`

## 文件落盘与本地盘点规则

### 文件落盘规则

规则：

- 单曲下载：默认直接落盘到 `outputDir/`
- 整专下载：该任务下的所有歌曲统一落盘到 `outputDir/<sanitizedAlbumName>/`
- 整专下载时，专辑封面同步下载到 `outputDir/<sanitizedAlbumName>/cover.<ext>`
- `cover` 为固定基础名，扩展名由实际图片 MIME/内容类型决定
- 若同目录已存在同名 `cover.<ext>`，新下载应覆盖旧文件，避免生成 `cover (1)` 之类不稳定命名
- 任务完成后，`DownloadTaskSnapshot.outputPath` 应指向各歌曲的实际落盘路径；专辑封面属于 Job 级附属产物，不强制建模为单独 Task

### 本地盘点规则

规则：

- 本地盘点只扫描当前 `AppPreferences.outputDir`
- 曲目级下载标记进入 `SongEntry` 与 `SongDetail` 返回值
- `get_albums` 与 `get_album_detail` 不再内联专辑级 badge
- `download.isDownloaded` 是前端直接消费字段
- `download.downloadStatus` 表达更细粒度的语义状态
- `isDownloaded = true` 的最低语义是“当前 active root 下已确认存在本地文件”，不是“已完成远端一致性校验”
- `mismatch` 表示“本地存在但校验明确不一致”，属于异常态，不应被前端展示为“已下载”
- `unverifiable` 表示“本地存在但当前无法完成可信校验”，仍属于“已下载”
- `unknown` 只用于“存在候选线索但尚不足以确认本地已下载”的场景，不用于表达“已存在但无法校验”
- 本地盘点优先依据确定性路径规则、扩展名和目录结构进行匹配，必要时才使用 metadata 辅助确认
- 不要求对整个目录做全量 MD5 计算
- `inventoryVersion` 在每次成功完成盘点后变化，用作动态缓存失效键
- root 切换后，旧结果不再作为当前前端列表的真相来源

### MD5 校验规则

规则：

- MD5 校验是可选能力，不是“已下载识别”的前置条件
- 只有当本地最终文件与远端 checksum 指向的产物语义可比时，才允许进入直接 checksum 校验
- `whenAvailable` 模式下，只对可比候选或可验证 provenance 候选做校验
- 若远端 checksum 缺失、不可比或语义不明确，默认状态停留在 `detected`；仅 `strict` 模式下升级为 `unverifiable`
- 不允许因为“无法做 MD5”而否定“本地已下载”的事实
- WAV → FLAC 转码、以及 FLAC 写入 tag / cover 后，通常不能直接与原始源文件 MD5 对比
- 对于转码、重封装或写入 tag / cover 后的产物，允许通过 provenance 映射承接原始资源校验结果，而不是要求最终文件 MD5 直接等于远端原始 checksum
- provenance 映射至少应记录：远端 checksum、远端资源标识（如 `sourceUrl` / `ETag` / `Content-Length`）、处理参数摘要、最终产物摘要
- 只有当 provenance 映射链条完整且可验证时，才允许给出“来源已验证”的结论；否则按 `detected` 或 `unverifiable` 处理
- provenance 映射用于证明“该最终文件来源于某个已校验的原始资源”，不表示“该最终文件字节级等同于远端原始资源”
- 若最终产物摘要与映射记录不一致，或文件 mtime/size/摘要表明文件在映射建立后被外部修改、覆盖或替换，则该 provenance 映射必须失效
- 文件重命名或目录迁移本身不自动使 provenance 失效；只要最终产物摘要仍匹配，映射仍可复用
- provenance 只能用于承接下载链路内已记录的受信来源，不可对扫描时首次发现、且缺少可信建链信息的历史遗留文件反向臆造映射

## 冻结决策

1. 第一阶段使用单实例、内存态、单 worker 串行执行。
2. 命令统一返回完整快照，不返回 patch。
3. 快照事件发送完整对象，高频进度单独用 `download-task-progress`。
4. Job / Task ID 均使用不透明 `string`。
5. 时间字段统一使用 ISO-8601 UTC 字符串。
6. 重试不会生成新的逻辑任务，而是在原有 task / job 上增加 `attempt`。
7. 新 API 契约中不再暴露裸字符串错误，统一使用 `DownloadErrorInfo`。
8. 取消语义为 best-effort，不对残留临时文件清理做对外承诺。
9. 整专下载的文件组织方式冻结为"按专辑目录存储"，不采用输出根目录平铺。
10. 整专下载时专辑封面作为 Job 级附属产物写入专辑目录，固定基础名为 `cover`。
11. `AppPreferences` 是唯一偏好数据源，`OutputFormat` 枚举同步到前端共享类型。
12. 偏好持久化使用手写 TOML 文件，不依赖外部插件。
13. 偏好备份/恢复由用户指定文件路径，后端仅执行读写操作，不管理默认路径。
14. 本地盘点为独立域，不并入 `DownloadService`。
15. 内容命令返回 enriched 结构，不新增第二套“带下载态”的内容命令。
16. 前端统一读取 `download.isDownloaded` 作为列表和详情下载标记。
17. `inventoryVersion` 是下载标记动态缓存的统一失效键。
18. `outputDir` 改变后，后端异步自动重扫当前 active root。
19. MD5 只做 best-effort，不作为能力成功与否的前置条件。
20. Tauri command 默认遵循最小权限原则：只读、受限文件、高风险文件三类必须显式区分。
21. 非 UI 必需时，不向前端暴露绝对本地路径。
