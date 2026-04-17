# 后端 API 契约

本文档是下载任务系统的唯一契约来源，定义了前后端共享的类型、命令、事件和状态机规则。

相关文档：

- [BACKEND_ROADMAP.md](BACKEND_ROADMAP.md)：后端未来规划（Phase 6/7）
- [FRONTEND_GUIDE.md](FRONTEND_GUIDE.md)：前端架构与开发指南

## 共享类型

### Rust / TS 对齐类型清单

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
- `NotificationPreferences`
- `NotificationPermissionState`

## 类型字段定义

### `DownloadOptions`

- `outputDir: string`
- `format: OutputFormat`
- `downloadLyrics: boolean`

### `DownloadJobKind`

冻结枚举：

- `song`
- `album`
- `selection`

### `DownloadJobStatus`

冻结枚举：

- `queued`
- `running`
- `completed`
- `partiallyFailed`
- `failed`
- `cancelled`

### `DownloadTaskStatus`

冻结枚举：

- `queued`
- `preparing`
- `downloading`
- `writing`
- `completed`
- `failed`
- `cancelled`

### `DownloadErrorCode`

冻结枚举：

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

### `NotificationPreferences`

- `notifyOnDownloadComplete: boolean`
- `notifyOnPlaybackChange: boolean`

### `NotificationPermissionState`

冻结枚举：

- `granted`
- `denied`
- `prompt`
- `prompt-with-rationale`

## Commands

### 下载任务命令

冻结命令如下：

1. `create_download_job(request: CreateDownloadJobRequest) -> DownloadJobSnapshot`
2. `list_download_jobs() -> DownloadManagerSnapshot`
3. `get_download_job(jobId: string) -> DownloadJobSnapshot`
4. `cancel_download_job(jobId: string) -> DownloadJobSnapshot`
5. `cancel_download_task(jobId: string, taskId: string) -> DownloadJobSnapshot`
6. `retry_download_job(jobId: string) -> DownloadJobSnapshot`
7. `retry_download_task(jobId: string, taskId: string) -> DownloadJobSnapshot`
8. `clear_download_history() -> number`

说明：

- 不再单独冻结 `enqueue_album_download`，统一通过 `create_download_job` + `kind` / `albumCid` 表达，避免双入口重复。
- 旧 `download_song(songCid, outputDir, format, downloadLyrics) -> string` 视为兼容接口，新的实现开始后立即进入废弃状态。

### 通知偏好命令

冻结命令如下：

1. `get_notification_preferences() -> NotificationPreferences`
2. `set_notification_preferences(preferences: NotificationPreferences) -> NotificationPreferences`
3. `get_notification_permission_state() -> NotificationPermissionState`
4. `send_test_notification() -> void`

说明：

- 通知偏好存储在应用状态中，不持久化到磁盘
- 通知权限状态由 Tauri 通知插件返回，反映系统级权限授予情况
- 测试通知用于验证通知管道是否正常工作

## Events

冻结事件如下：

1. `download-manager-state-changed`，载荷为 `DownloadManagerSnapshot`
2. `download-job-updated`，载荷为 `DownloadJobSnapshot`
3. `download-task-progress`，载荷为 `DownloadTaskProgressEvent`

其中：

- `download-manager-state-changed` 负责同步整体任务列表概览
- `download-job-updated` 负责同步某个任务完整快照
- `download-task-progress` 负责同步细粒度下载进度

## 快照与事件载荷原则

为降低前端状态同步复杂度，建议和播放器一致：

- 快照事件尽量发送完整结构，而不是零散 patch
- 进度事件只在高频字段变化时发出
- 命令返回值与事件载荷的结构保持一致

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

## 文件落盘约定

冻结规则：

- 单曲下载：默认直接落盘到 `outputDir/`
- 整专下载：该任务下的所有歌曲统一落盘到 `outputDir/<sanitizedAlbumName>/`
- 整专下载时，专辑封面同步下载到 `outputDir/<sanitizedAlbumName>/cover.<ext>`
- `cover` 为固定基础名，扩展名由实际图片 MIME/内容类型决定
- 若同目录已存在同名 `cover.<ext>`，新下载应覆盖旧文件，避免生成 `cover (1)` 之类不稳定命名
- 任务完成后，`DownloadTaskSnapshot.outputPath` 应指向各歌曲的实际落盘路径；专辑封面属于 Job 级附属产物，不强制建模为单独 Task

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
