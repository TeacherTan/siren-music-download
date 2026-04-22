# 后端待办阶段

> 仅面向未来或尚未完成的后端阶段规划。
>
> 已完成的后端能力（Phase 1~10）参见 [BACKEND_COMPLETED_PHASES.md](BACKEND_COMPLETED_PHASES.md)。
>
> 共享类型、命令、事件和状态机规则以 [BACKEND_API_CONTRACT.md](BACKEND_API_CONTRACT.md) 为唯一事实来源。

## 当前剩余的后端能力缺口

1. **Phase 11** 搜索 / 过滤 / 历史视图增强的后端支撑：前端已基于现有 `DownloadManagerSnapshot.jobs` 落地首版搜索、状态筛选、类型筛选和活跃/历史范围筛选；若历史量继续增大，后端再评估是否提供摘要、筛选或分页能力。
2. **Phase 12** 库内搜索与过滤：当前没有面向音乐目录的搜索能力，建议按 12A / 12B / 12C 分阶段引入后端统一搜索方案，不再以前端本地全量索引作为主线。

## Phase 11：搜索 / 过滤 / 历史视图后端支撑（条件触发）

### 触发条件

只有满足以下任一条件时，才建议进入本阶段：

1. session 持久化落地后，历史记录规模明显增长；
2. 前端基于完整 `DownloadManagerSnapshot.jobs` 的筛选已出现明显性能或复杂度问题；
3. 历史视图需要分页、摘要列表、惰性详情加载，而现有完整快照已不合适。

如果以上条件都不成立，则搜索 / 过滤 / 历史视图增强应优先在前端基于现有快照实现，不急于扩展后端契约。

> 当前状态：首版前端历史视图增强已落地，基于现有完整快照支持关键字搜索、状态筛选、类型筛选和活跃/历史范围筛选；本节保留给“当完整快照已不够用时”的后端扩展。

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

## Phase 12：库内搜索与过滤

### 总体目标

采用后端主导的统一搜索方案，为 Monster Siren 数据源建立一套可扩展的库内搜索基础设施。

但 Phase 12 不再按“一次性完成完整搜索平台”推进，而是拆分为 12A / 12B / 12C 三个渐进阶段：

- **12A**：先完成可落地的元数据搜索 MVP
- **12B**：在 MVP 稳定后补强召回、排序与中文输入体验
- **12C**：最后再纳入歌词全文检索与相关索引维护

后端承担统一搜索语义，前端负责查询输入、结果展示、定位与交互编排。

### 总体设计原则

1. **后端统一语义**：搜索范围、查询语义、排序规则和结果结构由后端定义，避免前后端各自维护一套搜索逻辑。
2. **契约先行**：每一阶段都先在 `BACKEND_API_CONTRACT.md` 冻结请求 / 响应模型，再推进 Rust 实现和前端接入。
3. **列表与详情分离**：搜索结果优先返回轻量导航摘要，详情继续通过现有 `get_album_detail()` / `get_song_detail()` 获取。
4. **渐进增强**：先证明“搜索即定位 / 过滤”的核心体验，再追加拼音、歌词、preview 和更复杂的索引策略。
5. **前端保持轻量**：前端不承担完整索引构建，只消费后端搜索结果并负责选择、定位、清空与回退。
6. **嵌入式优先**：不引入额外搜索服务进程，统一采用 Rust 进程内索引方案，便于桌面端打包、启动和状态管理。
7. **数据最小化**：首版只暴露前端真正需要的导航字段，不提前冻结 `score`、歌词 preview、复杂命中解释等高耦合字段。

### 技术选型

#### 搜索索引框架

Phase 12 当前优先采用 **Tantivy** 作为 Rust 侧嵌入式搜索实现方案。

选型理由：

- 与 Rust + Tauri 2 架构天然贴合，不需要额外 sidecar 或独立搜索服务
- 比简单 substring 过滤更适合承载后端统一的查询语义和排序演进
- 比 SQLite FTS5 更适合后续做多字段权重、中文分词扩展和可重建索引生命周期
- 能支撑后续 12B / 12C 的能力演进，但不要求在 12A 一次性全部落地

当前不采用：

- **Meilisearch / Sonic**：偏服务化，桌面本地应用集成和发布成本过高
- **SQLite FTS5**：可作为轻量候选，但当前阶段不如 Tantivy 适合做后续搜索语义演进
- **前端本地倒排索引**：不再作为 Phase 12 主线

### 12A：元数据搜索 MVP

#### 目标

先完成一个后端统一语义、前端可直接接入的元数据搜索版本，验证“搜索即定位”为主、必要时驱动局部过滤视图的核心体验。

12A 只覆盖：

1. 专辑名搜索
2. 歌曲名搜索
3. 艺术家名搜索

12A 默认**不纳入**：

- `intro` / `belong` 辅助字段召回（默认后置到 12B）
- 歌词全文检索
- 歌词 preview
- 拼音作为契约级承诺
- 复杂高亮协议
- 依赖 `score` 的前端逻辑

#### 12A 前置项

在进入 12A 搜索接口与前端接入前，先补齐以下基础能力：

1. **元数据快照 store**：为搜索建立可重建的本地元数据快照，而不是只依赖 `ApiClient` 的进程内 LRU 响应缓存。
2. **rebuild policy**：明确首次构建、后台重建、失败回退、重启恢复和离线退化策略。
3. **索引状态模型**：明确 ready / building / stale / unavailable 等状态如何对前端暴露，而不是把“未就绪”折叠成普通空结果。
4. **生命周期控制**：明确后台构建的并发上限、取消 / 去重规则、部分失败处理和 last-ready index 的原子切换。

若以上前置项未明确，12A 不应直接冻结最终 `search_library` 对外行为。

#### 契约范围

修改文件：

- `doc/BACKEND_API_CONTRACT.md`
- `src/lib/types.ts`
- `src/lib/api.ts`

建议冻结的最小模型：

- `LibrarySearchScope = 'all' | 'albums' | 'songs'`
- `LibrarySearchHitField = 'title' | 'artist'`
- `SearchLibraryRequest`
- `SearchLibraryResultItem`
- `SearchLibraryResponse`

建议 command：

- `search_library(request)`

建议 `SearchLibraryRequest` 字段：

- `query: string`
- `scope: LibrarySearchScope`
- `limit?: number`
- `offset?: number`

建议 `SearchLibraryResultItem` 字段：

- `kind: 'album' | 'song'`
- `albumCid: string`
- `songCid?: string`
- `albumTitle: string`
- `songTitle?: string`
- `artistLine?: string`
- `matchedFields: LibrarySearchHitField[]`

建议 `SearchLibraryResponse` 字段：

- `items: SearchLibraryResultItem[]`
- `total: number`
- `query: string`
- `scope: LibrarySearchScope`

12A 契约约束：

- `query` 必须先做 trim 和归一化，空 query 行为要在 CONTRACT 中明确
- `limit` / `offset` 必须有服务端上限与默认值，不能形成无界查询
- 返回值只提供导航所需的最小字段，不返回歌词、preview 或内部评分细节
- `matchedFields` 仅表达用户可理解的元数据命中来源，不暴露底层 analyzer / tokenizer 细节
- 搜索请求本身不触发在线抓取，只查询当前已建立的本地索引或 last-ready 本地快照
- “索引未就绪” 不能直接折叠为普通空结果；必须通过显式状态或类型化错误与“无命中”区分

#### Rust 实现方向

修改文件：

- `crates/siren-core/src/`
- `src-tauri/src/app_state.rs`
- `src-tauri/src/commands/library.rs`
- `src-tauri/src/main.rs`

建议模块拆分：

- `crates/siren-core/src/search/mod.rs` — 模块导出
- `crates/siren-core/src/search/model.rs` — 搜索请求、结果项、命中字段、内部文档模型
- `crates/siren-core/src/search/normalizer.rs` — 大小写、空白、标点、全半角等归一化
- `crates/siren-core/src/search/index.rs` — Tantivy schema、索引构建、writer / reader 管理
- `crates/siren-core/src/search/service.rs` — 查询执行、排序、结果映射、索引重建入口

运行时宿主建议：

- 搜索服务默认挂载在 `src-tauri/src/app_state.rs` 管理的共享应用状态中
- `search_library()` 只调用统一 search service，不在 command 层直接持有 reader / writer
- 索引初始化、重建入口和生命周期状态应通过应用级状态统一管理

12A 建议依赖：

- `tantivy` — 当前优先的嵌入式搜索实现方案
- `jieba-rs`（或兼容的 Tantivy tokenizer 集成）— 中文分词实现建议；属于实现细节，不构成 12A 对外契约的一部分

说明：

- 12A 的硬前提应是接口、状态语义和生命周期先稳定
- Tantivy 是当前首选实现，不应在 12A 文档里被表述为“先有 Tantivy，后有搜索语义”

12A 索引字段建议：

- `kind` — `album` / `song`
- `album_cid`
- `song_cid`
- `album_title`
- `song_title`
- `artist`

12A 字段来源建议：

- `album_title` / `artist` 可优先来自 `get_albums()` 返回的专辑列表
- `song_title` / song 级 `artist` 通过 `get_album_detail()` 的歌曲列表补齐
- 12A 默认避免为了建索引而对每首歌额外执行 `get_song_detail()` fan-out
- `intro` / `belong` 属于 `AlbumDetail` 级字段，默认后置到 12B，再评估是否纳入索引

元数据快照与离线语义要求：

- 12A 不能只依赖 `ApiClient` 的进程内 LRU cache 作为搜索数据源
- 需要定义独立的元数据快照 store，用于重启后恢复和离线查询退化
- 需要定义专辑详情 fan-out 失败时的降级策略：是跳过、保留旧快照，还是把索引标记为 stale / partial
- 需要定义离线时搜索可消费的数据边界：仅 last-ready 快照，还是直接不可用

12A 字段策略：

- `album_title` / `song_title` / `artist` 为主检索字段
- 首版不纳入 `intro` / `belong`、歌词字段，也不为歌词建立 preview 或 snippet 逻辑

#### 排序与查询规则

12A 只冻结粗粒度排序原则，不冻结过细评分细节：

1. 标题命中优先于辅助字段命中
2. 强元数据命中优先于弱辅助字段命中
3. 同层结果必须有稳定 tie-break 规则，避免同 query 下顺序抖动

12A 需要在 CONTRACT 或测试基线中明确：

- 空 query、单字 query、超长 query 的处理方式
- `albums` / `songs` / `all` 三种 scope 的过滤行为
- `total` 是否表示分页前总量
- 相同数据快照下结果顺序必须稳定

#### 前端接入方式

修改文件：

- `src/lib/components/app/TopToolbar.svelte`
- `src/lib/components/app/AlbumSidebar.svelte`
- `src/App.svelte`

12A 前端职责：

- 提供搜索输入框与范围切换（全部 / 专辑 / 歌曲）
- 调用后端搜索 command 获取结果
- 将 album hit 映射为专辑切换
- 将 song hit 映射为所属专辑切换 + 曲目定位
- 展示最小必要的加载态、空状态、不可用状态

12A UI 原则：

- 不新增独立搜索页，优先“搜索即过滤 / 定位”体验
- 查询非空时，左侧区域进入搜索结果 / 过滤态
- 清空搜索后恢复默认专辑导航
- 搜索不打断现有播放、下载、设置与面板状态

#### 索引生命周期要求

12A 必须明确以下产品语义：

- 搜索索引由后端统一维护，前端不持有第二套全量搜索状态源
- 索引与元数据快照都是可重建的本地派生数据，不引入独立搜索服务或额外数据库作为前提
- 必须明确后台构建策略：何时首次构建、何时触发重建、是否允许后台增量刷新
- 必须明确并发控制：同一时刻允许多少构建任务、重复请求如何去重、取消如何处理
- 必须明确原子切换语义：新索引未 ready 前继续使用 last-ready index，还是整体不可用
- 必须明确部分失败语义：某些专辑 detail fan-out 失败时，索引是 stale、partial 还是 unavailable
- “索引未就绪” 必须和“无命中”区分开来，不能只通过空结果表达
- 搜索相关日志不得记录原始歌词文本，也不应在首版引入额外的内容暴露面

#### 12A 完成定义

- 专辑名 / 曲名 / 艺术家可通过统一后端接口搜索
- 前后端共享一致的 `query`、`scope`、结果结构和基础排序语义
- 前端可基于搜索结果完成专辑切换与曲目定位
- 前端不维护独立全量搜索索引
- 搜索能力与现有播放、下载、设置交互无职责冲突

#### 12A 测试基线

12A 需要把以下测试项作为明确计划内容，而不只是补充说明：

- 空 query、单字 query、超长 query 的处理方式
- `albums` / `songs` / `all` 三种 scope 的过滤行为
- `limit` / `offset` 的默认值、上限与越界行为
- `total` 是否表示分页前总量
- 相同数据快照下结果顺序必须稳定
- 标题 / 艺术家等主检索字段的基础排序语义
- 索引状态与前端语义映射：loading / empty / unavailable 不得混淆
- 重启后、离线时、以及部分 fan-out 失败时的退化行为
- last-ready index 的原子切换和后台重建回归

### 12B：召回与排序增强

#### 目标

在 12A 已验证可用的前提下，继续优化搜索质量与中文输入体验。

12B 可纳入：

- 拼音字段与拼音召回增强
- 标题前缀 / 连续子串 / token 命中的更细排序策略
- 短 query 的更细阈值控制
- 少量 n-gram 兜底召回
- 更细致的命中解释或高亮协议（如前端确有需要）

12B 约束：

- 拼音能力默认视为增强项，而不是 12A 就必须对外承诺的契约语义
- 不应为了排序精调而破坏 12A 已冻结的基础结果结构与导航语义
- 如果需要暴露更多字段，应先确认前端确实消费，而不是为了“以后可能有用”提前设计

#### 预计修改

- `crates/siren-core/src/search/`
- `src/lib/types.ts`
- `src/lib/api.ts`
- `doc/BACKEND_API_CONTRACT.md`

#### 12B 完成定义

- 在不破坏 12A 契约主干的前提下提升召回与排序质量
- 中文标题 / 艺术家搜索体验进一步稳定
- 如引入拼音，也仅作为增强能力，不让前端依赖具体内部评分实现

### 12C：歌词全文检索与索引维护

#### 目标

在元数据搜索稳定后，再把歌词检索纳入同一后端搜索框架。

#### 12C 前置项

在进入歌词搜索前，先补齐以下能力：

1. **歌词 cache owner**：明确歌词内容由谁持有、持久化到哪里、如何在重启后恢复。
2. **歌词状态模型**：明确“无歌词 / 未缓存 / 已缓存但未入索引 / 已可搜索”的状态边界。
3. **歌词刷新策略**：明确按需在线拉取后的写入、失效、重建与索引更新触发时机。
4. **歌词隐私与暴露边界**：明确 preview 截断、日志约束和前端可见字段。

若以上前置项未明确，12C 不应直接冻结最终 `lyrics` 搜索契约。

12C 可纳入：

- `LibrarySearchScope` 扩展为包含 `lyrics`
- 歌词字段索引
- 歌词命中结果的短片段 preview
- 歌词缓存刷新后的索引更新策略
- 歌词缺失 / 未缓存 / 可搜索状态表达

12C 约束：

- 歌词搜索不应改变 12A / 12B 已建立的导航模型
- preview 只返回必要短片段，不返回整首歌词全文
- 必须明确歌词未缓存与无歌词之间的状态差异
- 必须明确歌词索引更新的触发时机与用户可见语义
- 当前歌词仍是按需在线拉取，因此“歌词缓存刷新后的索引更新”不是附带细节，而是 12C 的前置能力之一

#### 12C 测试基线

12C 需要把以下测试项作为明确计划内容：

- 歌词“无歌词 / 未缓存 / 已缓存但未入索引 / 已可搜索”四态的状态表达
- `lyrics` scope 下的结果归属仍保持在 song / album 上下文内
- lyric preview 的长度、截断与非整段返回约束
- 歌词缓存刷新或索引失效后的重建 / 更新触发语义
- 歌词按需远程拉取场景下，搜索可用性与状态反馈的一致性

#### 预计修改

- `crates/siren-core/src/search/`
- `src-tauri/src/commands/library.rs`
- `src-tauri/src/main.rs`
- `src/lib/api.ts`
- `src/lib/types.ts`
- `src/App.svelte`
- `src/lib/components/app/TopToolbar.svelte`
- `src/lib/components/app/AlbumSidebar.svelte`
- `doc/BACKEND_API_CONTRACT.md`

#### 12C 完成定义

- 歌词可通过统一后端接口检索
- 歌词命中仍落在 song / album 上下文内，不新增独立歌词详情页契约
- 歌词 preview、缓存状态与索引维护语义明确
- 歌词能力与现有播放、下载、设置交互无职责冲突
## 建议执行顺序

1. **优先评估是否进入 Phase 11（搜索 / 过滤 / 历史视图后端支撑）**。Phase 10 已完成，下载历史已具备跨重启持久化基础。
2. 根据真实历史规模决定是否需要新增后端筛选、摘要或分页能力。
3. 搜索 / 过滤 / 历史视图若在当前数据量下可由前端直接完成，则后端继续保持现状。
4. **Phase 12 直接按统一后端搜索方案推进**。优先冻结契约、实现后端索引与查询能力，再接入前端搜索交互。

## 暂不纳入后端计划的事项

- 自动续传或断点续传
- 并发下载进一步扩展
- 云端同步下载历史
- 为下载历史引入数据库或外部存储