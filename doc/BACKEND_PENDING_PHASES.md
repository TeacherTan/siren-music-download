# 后端待办阶段

> 仅面向未来或尚未完成的后端阶段规划。
>
> 已完成的后端能力（Phase 1~9）参见 [BACKEND_COMPLETED_PHASES.md](BACKEND_COMPLETED_PHASES.md)。
>
> 共享类型、命令、事件和状态机规则以 [BACKEND_API_CONTRACT.md](BACKEND_API_CONTRACT.md) 为唯一事实来源。

## 当前剩余的后端能力缺口

1. **Phase 10** 下载 session 持久化：当前下载任务状态仍是内存态，应用重启后历史和队列都会丢失。
2. **Phase 11** 搜索 / 过滤 / 历史视图增强的后端支撑：当前 `list_download_jobs()` 返回完整快照，现阶段足够，但如果历史量变大，后端可能需要提供摘要、筛选或分页能力。

## Phase 10：下载 session 持久化

### 目标

让下载任务历史在应用重启后仍可恢复查看，并为后续历史视图增强提供稳定数据基础。

### 范围

本阶段只处理**任务状态持久化**，不做：

- 自动恢复未完成下载
- 断点续传
- 把下载中的音频缓存或写入 payload 落盘
- 云同步或跨设备同步

### 关键决策

1. **持久化对象**：持久化 job / task 快照和必要的 manager 元数据，不持久化下载过程中的临时二进制 payload。
2. **恢复语义**：应用重启后，上一 session 中处于 `queued / preparing / downloading / writing / running` 的任务统一恢复为**可见但不可自动继续**的终态。
3. **首版不自动续传**：不在启动时自动重启未完成任务，避免用户无感知地继续下载。
4. **写盘方式**：使用版本化 JSON 文件 + 原子写入，避免中途写坏状态文件。

### 主要工作

1. 在 `src-tauri` 侧定义下载状态文件路径与存储格式。
2. 在 `AppState` 初始化时加载持久化任务状态，并完成启动期状态修正。
3. 在下载任务创建、状态变化、历史清理后触发持久化写入。
4. 为持久化文件增加 `schemaVersion`，为后续字段演进预留空间。
5. 增加历史保留策略，避免状态文件无限增长（例如仅保留最近 N 个 job 或最近 N 天历史）。
6. 约定损坏文件的降级行为：读取失败时不阻塞应用启动，记录错误并回退到空状态。

### 涉及文件

- `src-tauri/src/app_state.rs`
- `src-tauri/src/downloads/bridge.rs`
- `src-tauri/src/commands/downloads.rs`
- `crates/siren-core/src/download/service.rs`
- `crates/siren-core/src/download/model.rs`
- 如需冻结落盘格式或恢复语义，再更新 `BACKEND_API_CONTRACT.md`

### 完成定义

- 已完成 / 失败 / 取消的任务在应用重启后仍可见
- 上一 session 的未完成任务不会自动恢复执行
- 用户能对中断任务执行手动重试
- `clear_download_history()` 会同步清理持久化状态
- 状态文件损坏不会阻塞应用启动

### 验证项

1. 创建下载任务并完成后，重启应用，任务历史仍存在。
2. 下载进行到一半时退出应用，重启后该任务显示为中断后的终态，而不是继续运行。
3. 重启后点击"重试"可以重新排队并正常执行。
4. 清理历史后再次重启，已清理记录不会重新出现。
5. 人工破坏状态文件，应用仍可启动并回退到空历史。

## Phase 11：搜索 / 过滤 / 历史视图后端支撑（条件触发）

### 触发条件

只有满足以下任一条件时，才建议进入本阶段：

1. session 持久化落地后，历史记录规模明显增长；
2. 前端基于完整 `DownloadManagerSnapshot.jobs` 的筛选已出现明显性能或复杂度问题；
3. 历史视图需要分页、摘要列表、惰性详情加载，而现有完整快照已不合适。

如果以上条件都不成立，则搜索 / 过滤 / 历史视图增强应优先在前端基于现有快照实现，不急于扩展后端契约。

### 目标

在不破坏现有实时事件模型的前提下，为下载任务列表提供更适合历史浏览的查询能力。

### 设计原则

1. **保留现有实时链路**：`download-manager-state-changed`、`download-job-updated`、`download-task-progress` 不做破坏性修改。
2. **列表与详情分离**：列表接口优先返回摘要，详情继续通过 `get_download_job(jobId)` 获取。
3. **渐进增强**：只有在现有完整快照不够用时，才新增查询对象或历史摘要类型。
4. **默认兼容当前 UI**：即使未来增加筛选参数，也应保留"无参数拿全量结果"的兼容路径。

### 主要工作

1. 评估现有 `DownloadJobSnapshot` 是否足够支撑首版历史页（状态筛选、kind 筛选、标题关键字搜索、失败任务筛选）。
2. 如果现有结构不够，再在 CONTRACT 中冻结查询模型，例如 `ListDownloadJobsRequest`、`DownloadJobListItemSnapshot`。
3. 在 `DownloadService` 中增加稳定排序和查询逻辑（活跃任务优先、历史任务按 `finishedAt` 倒序）。
4. 视 UI 需要决定是否加入 terminal-only 历史查询、limit / offset 或 cursor 分页、是否默认省略 task 明细只返回聚合字段。

### 涉及文件

- `crates/siren-core/src/download/service.rs`
- `crates/siren-core/src/download/model.rs`
- `src-tauri/src/commands/downloads.rs`
- `src/lib/api.ts`
- `src/lib/types.ts`
- `BACKEND_API_CONTRACT.md`

### 完成定义

- 前端可以按状态 / 类型 / 关键字筛选历史任务，而不必每次消费完整 task 明细
- 历史视图可以只展示 job 摘要，再按需拉取详情
- 查询接口和现有事件模型职责清晰，不出现双轨状态源

## 建议执行顺序

1. **优先实现 Phase 10（下载 session 持久化）**。当前下载任务仍是纯内存态，重启后历史和队列丢失，是后续历史视图增强的主要阻塞项。
2. 持久化落地后，再根据真实历史规模决定是否进入 Phase 11。
3. 搜索 / 过滤 / 历史视图若在当前数据量下可由前端直接完成，则后端继续保持现状。

## 暂不纳入后端计划的事项

- 自动续传或断点续传
- 并发下载进一步扩展
- 云端同步下载历史
- 为下载历史引入数据库或外部存储