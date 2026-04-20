# 后端后续规划

> 仅面向未来的后端规划，不含已完成里程碑。
>
> 已完成的后端能力（M1~M5b）参见 CLAUDE.md"当前实现状态"章节。
>
> 共享类型、命令、事件和状态机规则以 [BACKEND_API_CONTRACT.md](BACKEND_API_CONTRACT.md) 为唯一事实来源。

## 已新增并保持的基础能力

1. **统一日志中心**：后端已具备结构化日志记录、session / persistent 双层日志文件、运行时错误安全事件和设置页日志浏览支撑。
2. **运行时错误补盲**：播放器流式下载、解码、音频输出、媒体会话和系统通知等运行时错误已纳入统一日志中心，而不再只依赖 `eprintln!`。
3. **日志偏好联动**：`AppPreferences.logLevel` 已作为 session log 退出时晋升到 persistent log 的阈值。

## 当前缺少的后端能力

1. **本地已下载文件盘点与下载标记**：当前后端无法基于当前 `outputDir` 可靠回答“某首歌/某张专辑是否已在本地存在”，前端列表也拿不到可直接消费的下载标记。
2. **下载 session 持久化**：当前下载任务状态仍是内存态，应用重启后历史和队列都会丢失。
3. **搜索 / 过滤 / 历史视图增强的后端支撑**：当前 `list_download_jobs()` 返回完整快照，现阶段足够，但如果历史量变大，后端可能需要提供摘要、筛选或分页能力。

## Phase 5：统一偏好系统

### 目标

用统一的 `AppPreferences` 模型替代分散的偏好存储，解决偏好散落在内存/localStorage/API 三个地方的问题。

### 现状痛点

| 问题 | 涉及文件 |
|------|---------|
| 偏好分散：通知偏好(内存)、歌词偏好(localStorage)、格式/路径(无持久化) | `App.svelte`、`app_state.rs` |
| 存储策略不统一：通知偏好双写(localStorage + Rust)，其他偏好各自为政 | `App.svelte` `$effect` |
| 命令粒度过细：通知偏好需 4 个命令 | `commands/preferences.rs` |
| 偏好不跨会话保留：应用重启后通知偏好丢失 | `AppState` |

### 架构设计

```
[前端]  →  get_preferences() / set_preferences(AppPreferences)
              ↓
      [AppState.preferences: Arc<Mutex<AppPreferences>>]
              ↓ (同步)
      [preferences.toml 原子写入]
```

#### AppPreferences 模型

```rust
pub(crate) struct AppPreferences {
    pub(crate) output_format: String,    // "flac" | "wav" | "mp3"
    pub(crate) output_dir: String,
    pub(crate) download_lyrics: bool,
    pub(crate) notify_on_download_complete: bool,
    pub(crate) notify_on_playback_change: bool,
    pub(crate) log_level: String,        // "debug" | "info" | "warn" | "error"
}
```

TOML 文件结构（写入 `{app_data_dir}/preferences.toml`）：

```toml
schemaVersion = 1
outputFormat = "flac"
outputDir = "/path/to/downloads"
downloadLyrics = true
notifyOnDownloadComplete = true
notifyOnPlaybackChange = true
logLevel = "error"
```

#### 备份与恢复

- `export_preferences(outputPath)`：将当前偏好写入用户指定路径，返回写入后的偏好快照
- `import_preferences(inputPath)`：从用户指定路径读取 TOML，验证通过后替换当前偏好并落盘，返回导入后的偏好
- 导入验证规则同 `set_preferences`，验证失败时返回错误且不更新状态

#### 验证规则

- `outputFormat`：必须是 `flac` | `wav` | `mp3`
- `outputDir`：路径必须存在且为目录
- 其余字段：布尔值

### 涉及文件

- `src-tauri/src/preferences.rs` — 新增偏好读写模块（TOML 解析/序列化、原子写入）
- `src-tauri/src/app_state.rs` — 添加 `AppPreferences` 结构体，持有一份 `Arc<Mutex<AppPreferences>>` 和 `preferences.rs` 句柄
- `src-tauri/src/commands/preferences.rs` — 实现 `get_preferences`、`set_preferences`、`export_preferences`、`import_preferences`
- `src-tauri/src/main.rs` — 初始化偏好持久化

### 完成定义

- `get_preferences()` 启动时从 `preferences.toml` 加载，缺失则用默认值
- `set_preferences()` 验证通过后自动落盘，验证失败返回错误不更新状态
- 前端删除所有偏好相关的 localStorage 读写代码
- 前端不再各自维护通知偏好状态，统一从后端读取
- 备份导出到用户指定路径，恢复从用户指定路径导入，均返回操作后的偏好快照
- 恢复时验证规则同 `set_preferences`，不合法的备份文件不覆盖当前状态

### 验证项

1. 修改任意偏好项并关闭应用，重启后偏好保留原值
2. 设置不合法的 `outputDir`（不存在/非目录）时返回验证错误
3. 设置不合法的 `outputFormat` 时返回验证错误
4. 偏好变更自动落盘，不阻塞命令响应
5. 导出备份到指定路径，文件内容为完整 `preferences.toml`
6. 导入有效备份后，当前偏好被覆盖
7. 导入非法备份（路径不存在、格式错误、验证失败）时，当前偏好不变，返回错误

## Phase 6：本地已下载盘点与下载标记

### 目标

基于当前 `AppPreferences.outputDir` 建立本地盘点能力，把“是否已下载”直接带到专辑列表、专辑详情和歌曲详情返回中，并在下载目录切换后自动重新检测。

### 范围

本阶段处理：

- 当前 active `outputDir` 下的本地音频文件盘点
- `Album` / `AlbumDetail` / `SongEntry` / `SongDetail` 返回值上的下载标记 enrich
- 本地盘点快照、重扫命令与进度事件
- `outputDir` 切换后的异步自动重扫
- 条件满足时的 best-effort MD5 校验

本阶段不做：

- 旧目录到新目录的自动迁移
- 多 root 并行盘点
- 自动修复损坏文件或自动重下
- 为首版引入数据库

### 关键决策

1. **独立域**：本地盘点作为独立后端域，不并入 `DownloadService`。
2. **列表直出下载标记**：`get_albums()`、`get_album_detail()`、`get_song_detail()` 直接返回 `download` 字段，不新增第二套内容命令。
3. **状态分离**：区分 `isDownloaded` 与 `downloadStatus`，避免把“本地存在”和“已校验一致”混为一谈；其中 `unverifiable` 表示“已存在但无法可信校验”，仍视为已下载。
4. **缓存失效键**：使用 `inventoryVersion` 作为前端动态缓存的统一失效键。
5. **MD5 best-effort**：只有本地最终文件与远端 checksum 语义可比时才尝试 MD5，不把 MD5 作为已下载识别前置条件。
6. **provenance 映射**：对下载时拿到的原始资源 checksum（如 `Content-MD5` / `ETag`）与处理后产物建立持久映射，用“来源一致”替代“转码后文件必须直接命中远端 MD5”。
7. **可信边界明确化**：provenance 只承接下载链路内已建链的受信来源；若最终产物被外部修改或摘要不再匹配，则映射失效。

### 主要工作

1. 在 `siren-core` / `src-tauri` 中引入本地盘点服务、扫描器与匹配逻辑。
2. 冻结 `LocalTrackDownloadStatus`、`TrackDownloadBadge`、`AlbumDownloadBadge`、`LocalInventorySnapshot` 等共享类型。
3. 为 `Album`、`AlbumDetail`、`SongEntry`、`SongDetail` 增加 `download` 字段。
4. 新增 `get_local_inventory_snapshot()`、`rescan_local_inventory()`、`cancel_local_inventory_scan()`。
5. 新增 `local-inventory-state-changed` 与 `local-inventory-scan-progress` 事件。
6. 在 `set_preferences()` 中对 `outputDir` 变化增加异步重扫触发。
7. 明确前端缓存失效策略：缓存 key 包含 `inventoryVersion` 或在盘点事件后主动失效。
8. 补充上游 checksum 能力研究，确认 MD5 的可用边界。
9. 设计并落地 provenance 映射存储：记录 `remote_checksum`、原始资源标识、处理参数摘要与最终产物摘要，为转码/写 tag 后的文件提供来源校验依据。
10. 为 `strict` 模式补齐落地语义：显式产出 `unverifiable`，而不是把“已存在但无法校验”折叠到 `detected`。
11. 明确 provenance 失效规则：文件被外部修改、覆盖或摘要漂移后，已有映射不得继续用于“来源已验证”结论。

### 涉及文件

- `crates/siren-core/src/api.rs`
- `crates/siren-core/src/local_inventory/`（新目录）
- `src-tauri/src/app_state.rs`
- `src-tauri/src/commands/library.rs`
- `src-tauri/src/commands/preferences.rs`
- `src-tauri/src/commands/local_inventory.rs`（新文件）
- `src-tauri/src/local_inventory/`（新目录）
- `src/lib/types.ts`
- `src/lib/api.ts`
- `BACKEND_API_CONTRACT.md`
- `FRONTEND_GUIDE.md`

### 完成定义

- `get_albums()` 返回的专辑列表可直接显示“已有下载内容”
- `get_album_detail()` 返回的曲目列表可直接读取 `song.download.isDownloaded`
- `get_song_detail()` 返回包含下载标记
- 修改 `outputDir` 后，无需重启应用即可触发新的盘点
- 前端在盘点后不会继续展示旧的下载标记缓存
- 不可比场景下不会错误地因为 MD5 缺失把本地文件判为未下载
- `strict` 模式下，“已存在但无法校验”的文件会稳定落到 `unverifiable`，而不是被折叠为 `detected` 或误判为未下载
- 对于转码或写 tag/cover 后的产物，若存在可信 provenance 映射，可判定其来源对应的原始资源已校验一致
- 对已建立 provenance 的产物，一旦文件被外部修改、覆盖或摘要漂移，不再继续沿用旧的“来源已验证”结论

### 验证项

1. 空目录扫描时，列表项下载标记全部为 false。
2. 单首已落盘时，对应 `SongEntry.download.isDownloaded` 为 true。
3. 专辑目录部分曲目存在时，专辑级下载聚合字段正确。
4. 切换下载目录后，新旧目录下载标记按当前 root 正确切换。
5. WAV → FLAC 场景不会错误进入“MD5 mismatch”。
6. `strict` 模式下，远端 checksum 缺失或不可比时，已确认存在的本地文件会返回 `unverifiable`，且 `isDownloaded` 仍为 true。
7. 对已建立 provenance 映射的转码产物，可在不直接比较最终文件与远端原始 MD5 的前提下判定来源一致。
8. 对已建立 provenance 映射的产物，在文件摘要变化后重新扫描，不再返回 `verified`。

## Phase 7：缓存替换方案

### 目标

用分层多策略缓存架构替代现有的简单 TTL 缓存，解决内存无限增长、磁盘无限增长、页面刷新后缓存丢失、无法按 key 失效、无命中率统计等痛点。

### 现状痛点

| 问题 | 严重度 | 涉及文件 |
|------|--------|----------|
| 前端内存缓存无上限（`Map` 无 LRU） | CRITICAL | `src/lib/cache.ts` |
| 音频缓存磁盘无限增长 | CRITICAL | `src-tauri/src/audio_cache.rs` |
| 无法按 key 失效缓存 | HIGH | 全局 |
| 页面刷新后缓存全丢失 | HIGH | `src/lib/cache.ts` |
| siren-core HTTP 层无缓存（每次重新请求） | MEDIUM | `crates/siren-core/src/api.rs` |
| 封面缓存清理为同步阻塞 | MEDIUM | `src-tauri/src/notification/cover.rs` |
| 无命中率 / 淘汰统计 | MEDIUM | 全局 |

### 架构设计

#### 分层概览

```
[前端调用] → [CacheManager (Svelte $state)]
                  ├─ albums:    TieredCache<AlbumDetail>    (内存 LRU 50 条 / IndexedDB 持久化 / 6h TTL)
                  ├─ songs:     TieredCache<SongDetail>     (内存 LRU 200 条 / IndexedDB 持久化 / 6h TTL)
                  ├─ lyrics:    TieredCache<string>         (内存 LRU 200 条 / IndexedDB 持久化 / 6h TTL)
                  ├─ themes:    TieredCache<ThemeColors>    (内存 LRU 200 条 / 无持久化 / 24h TTL)
                  └─ covers:    TieredCache<string>         (内存 LRU 100 条 / 无持久化 / 6h TTL)
                            ↓ (TieredCache = moka-ts 内存层 + idb-keyval 持久层)
[API 响应]
         ↓ (命中不走 Tauri invoke)
[Tauri Command] → [siren-core ApiClient]
                        └─ 内部 LRU(100) 缓存（无持久化）

[音频流] → [音频缓存目录 2GB 上限]
             └─ 后台线程按 mtime LRU 淘汰

[封面图] → [封面缓存目录 7 天 / 128 文件上限]
             └─ 清理任务异步化，不阻塞主流程
```

#### 前端缓存层

使用 `moka-ts`（内存 LRU）+ `idb-keyval`（IndexedDB 持久化）实现双层缓存：

- **内存层**（`moka-ts`）：`getCached` / `setCached` 优先读写内存，命中即返回，不走 Tauri invoke
- **持久层**（`idb-keyval`）：页面刷新后从 IndexedDB 恢复命中，冷启动缓存命中率 > 40%
- **淘汰策略**：LRU，按类型独立配额，避免一种数据饥饿其他类型
- **失效机制**：`invalidateByTag(tag)` 批量失效（通过专辑 tag 关联专辑详情 + 歌曲详情 + 歌词），`invalidateKey(key)` 单条失效

缓存入口按类型分离（`cacheManager.albums.set(...)`），每类独立配置上限与 TTL。

`CacheManager` 以 `class` 实现，命中统计字段用 `$state` 声明，组件直接绑定 `$cacheManager.hits.albums`。

#### siren-core HTTP 层

`ApiClient` 内部持有 `lru::LruCache`（100 条），按 `method + path + params` 哈希作为缓存键。保持 stateless，缓存策略最终由 Tauri 命令层统一决定。

#### 后端音频缓存

缓存目录总大小以增量跟踪（写入 / 删除时更新），达到 2 GB 软上限时触发后台 `walkdir` 线程，按 mtime 升序删除最旧文件直到降至上限 × 0.8。淘汰线程不阻塞下载主流程。`.pending` 标记文件不计入容量统计。

#### 后端封面缓存

现有 7 天 / 128 文件限制保留不变，`cleanup_cache` 改为派生到 `spawn_blocking` 后台任务执行，主流程不等待清理完成。

### 量化目标

| 指标 | 目标值 |
|------|--------|
| 冷启动缓存命中率 | > 40%（通过 IndexedDB 预热） |
| 前端缓存内存上限 | 100 MB |
| 音频缓存磁盘上限 | 2 GB |
| 淘汰操作延迟 | < 50ms（后台线程） |

### 关键技术选型

| 组件 | 选型 | 理由 |
|------|------|------|
| 前端内存 LRU | `moka-ts` (≥0.12) | TTL + maxCapacity + 异步 get/set，WASM 友好 |
| 前端持久化 | `idb-keyval` | IndexedDB 封装，~3 KB，可序列化的 JSON 条目 |
| 后端内存 LRU | `lru` crate | ~0 依赖，融入 api.rs 简单 |
| 后端磁盘大小追踪 | `walkdir` | 已有或新增，缓存目录增量更新 |

### 涉及文件

- `src/lib/cache.ts` — 重写为 `CacheManager` + `TieredCache`
- `src/lib/api.ts` — 集成 `CacheManager`，移除内联缓存逻辑
- `src-tauri/src/audio_cache.rs` — 增加 2GB 上限 + 后台 LRU 淘汰
- `src-tauri/src/notification/cover.rs` — 清理任务异步化
- `crates/siren-core/src/api.rs` — 内部增加 LRU 缓存
- `src-tauri/src/app_state.rs` — 初始化 siren-core 缓存

### 完成定义

- 各类型缓存命中 / 未命中 / 淘汰计数可通过 `getCacheStats()` 查询
- 页面刷新后缓存数据（除 themes / covers 外）保留
- 音频缓存目录达到 2GB 时自动触发 LRU 淘汰，不阻塞下载
- 应用启动时后台预热最近 10 条专辑缓存到内存层
- 可按 key 失效任意缓存条目
- siren-core HTTP 层对相同请求直接返回缓存响应，不发网络请求

### 验证项

1. 缓存专辑详情后刷新页面，再次访问同一专辑不走 Tauri invoke
2. `getCacheStats()` 返回各类型 hit/miss/eviction 计数
3. 音频缓存目录超过 2GB 后，播放新歌曲不触发磁盘写满错误，旧缓存自动淘汰
4. 封面缓存清理在后台执行，不阻塞通知展示
5. siren-core 对相同请求（相同 method + path + params）的第二次调用不走网络

## Phase 8（原 Phase 6）：下载 session 持久化

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

## Phase 9（原 Phase 7）：搜索 / 过滤 / 历史视图后端支撑（条件触发）

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

1. **优先实现 Phase 5（统一偏好系统）**。偏好分散存储问题已在日常使用中造成不一致体验，且改动范围可控。
2. **紧接着实现 Phase 6（本地已下载盘点与下载标记）**。它直接支撑前端列表/详情下载态，也是 `outputDir` 切换体验的关键。
3. Phase 7（缓存替换方案）需要把 `inventoryVersion` 纳入缓存失效策略。
4. Phase 8（session 持久化）与 Phase 6 涉及不同领域，可并行或交叉进行。
5. 持久化落地后，再根据真实历史规模决定是否进入 Phase 9。
6. 搜索 / 过滤 / 历史视图若在当前数据量下可由前端直接完成，则后端继续保持现状。

## 暂不纳入后端计划的事项

- 自动续传或断点续传
- 并发下载进一步扩展
- 云端同步下载历史
- 为下载历史引入数据库或外部存储