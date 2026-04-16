<!-- markdownlint-disable MD024 -->
# App.svelte 拆分方案 B 实施计划

## 背景

当前前端入口 `src/App.svelte` 已承担以下多重职责：

1. 页面编排与模板渲染。
2. 专辑、播放器、下载三大业务域状态管理。
3. Tauri 事件订阅与生命周期清理。
4. 动效参数、交互动画、滚动行为控制。
5. 设置面板与下载任务面板的完整 UI 实现。

在持续迭代场景下，这种“单文件聚合”结构会持续放大改动面和回归风险，不利于多人协作。

## 当前现状（问题基线）

基于 `src/App.svelte` 当前实现：

1. 文件规模约 2666 行。
2. 包含约 57 个 `$state` 状态。
3. 包含约 22 个 `$derived` 计算状态。
4. 包含 5 个 `$effect` 副作用块。
5. 含大量跨域函数：专辑加载、播放队列、歌词、下载任务、设置、动效、滚动逻辑均在同文件。

可维护性风险集中在三个层面：

1. **认知负担高**：新增一个按钮可能需要理解多个不相邻状态与副作用。
2. **耦合度高**：下载设置、下载创建、下载面板强耦合；播放器与歌词/队列联动逻辑分散。
3. **测试与回归困难**：难以对单一业务域做隔离验证。

## 拆分原因

采用方案 (按业务域拆状态 + 视图)的核心动机：

1. **降低复杂度**：把“页面容器职责”与“业务域职责”分离。
2. **稳定演进**：后续新增能力（如下载过滤、播放模式扩展）能在域内实现，不污染入口文件。
3. **减少冲突**：多人并行开发时按 `library/player/download/shell` 领域分工，降低同文件冲突率。
4. **提高可测试性**：纯函数和领域逻辑迁出后可独立测试。
5. **提升可读性**：`App.svelte` 退化为装配层，阅读路径更线性。

## 方案设计思路

### 总体原则

1. `App.svelte` 仅保留页面装配、组件拼接和最薄的跨域协调（初始化编排）。
2. 业务状态与行为下沉到 `features/*/*.svelte.ts`（Svelte 5 rune 友好）。
3. 视图组件化拆分，组件尽量保持“展示 + 事件抛出”。
4. 先抽“纯函数”和“领域状态”，再抽“面板组件”，分阶段迁移，避免一次性重构，也避免同一组件改两次。

### 目标目录（建议）

```text
src/
├── App.svelte
├── lib/
│   ├── features/
│   │   ├── library/
│   │   │   ├── store.svelte.ts
│   │   │   ├── selectors.ts
│   │   │   └── helpers.ts
│   │   ├── player/
│   │   │   ├── store.svelte.ts
│   │   │   ├── lyrics.ts
│   │   │   └── queue.ts
│   │   ├── download/
│   │   │   ├── store.svelte.ts
│   │   │   ├── formatters.ts
│   │   │   └── guards.ts
│   │   └── shell/
│   │       ├── store.svelte.ts
│   │       └── motion.ts
│   └── components/
│       ├── TopToolbar.svelte
│       ├── AlbumWorkspace.svelte
│       ├── PlayerDock.svelte
│       ├── SettingsPanel.svelte
│       └── DownloadTasksPanel.svelte
```

### 领域边界定义

1. `library`：专辑列表、专辑详情、封面预加载、滚动舞台（含 `OverlayScrollbars` 容器与滚动驱动参数）相关状态。
2. `player`：当前歌曲、队列、歌词、自动切歌、播放器交互。
3. `download`：任务管理、任务操作、下载文案格式化、下载设置与偏好、**批量选择模式**（`selectionModeEnabled` / `selectedSongCids`）。
4. `shell`：全局 UI 开关（设置/下载面板）、系统偏好（reduced motion / macOS）与通用动效配置。

### 领域依赖层次（关键约束）

为避免跨域循环依赖，约定**单向读依赖**，下游可以读上游的 store，上游不可反向读/写下游：

```text
env  ←  library  →  player  →  download  →  shell
（任意读）  （上游）                        （下游）
```

具体含义：

1. `player` 可读 `library.selectedAlbum` 构建播放队列。
2. `download` 可读 `library.selectedAlbum`（整专下载）与 `player.currentSong`（当前曲目保护）。
3. `shell` 可读 `download.activeDownloadCount`（面板 badge）、`library.loading*`（骨架屏）。
4. **写方向严格禁止逆流**：如 `download` 不能写 `player.currentSong`，只能通过事件回调由 `App.svelte` 协调。
5. 视图组件消费 store 时，原则上只从**与自己所在域对应**或**更上游的域**读取，禁止跨层下钻。

#### `env` 只读环境域例外

`isMacOS` / `prefersReducedMotion` / `viewportHeight` 属于**全局只读环境常量**，任何域都可以读（包括 `library` 读 `env.prefersReducedMotion` 控制滚动舞台动效）。这个例外必须满足：

1. `env` 只暴露 getter，不暴露 setter。状态更新由 `env` 内部监听浏览器事件完成。
2. `env` 不读任何其他 store，保证无循环。
3. 实现位置：`features/env/store.svelte.ts`，独立于 `shell`。

`shell` 仍保留"面板开关、init 协调、全局 UI 状态"职责，不掺环境常量。

### 归属裁定（避免实现期分歧）

| 状态 / 模块 | 归属域 | 说明 |
|---|---|---|
| `contentScrollbar` / `contentEl` / `overlayScrollbarOptions` | `library` | 滚动驱动专辑封面舞台收缩，与 library 强耦合 |
| `selectionModeEnabled` / `selectedSongCids` / `creatingSelectionKey` | `download` | 选择模式的唯一动机是批量下载；`SongRow` 通过 props 接收勾选态 |
| `downloadLyrics` / `outputDir` / `format` | `download` | 下载偏好，含 localStorage 持久化 |
| `isMacOS` / `prefersReducedMotion` / `viewportHeight` | `env` | 全局只读环境常量（见上文 env 例外） |
| `lyricsOpen` / `playlistOpen` 面板开关 | `player` | 与播放器上下文强绑定，`shell` 只管全局面板（设置/下载）|
| `downloadPanelOpen` / `settingsOpen` 面板开关 | `shell` | 右侧全局面板 |
| `albumRequestSeq` / `themeRequestSeq` | `library` | 专辑切换竞态序号 |
| `lyricRequestSeq` / `playbackEndRequestSeq` | `player` | 歌词加载 / 播放结束竞态序号 |
| `taskSpeedMap`（progress 高频） | `download` | 独立 `$state`，与 `jobs` 分开（见响应式粒度规约） |
| `clearAudioCache` 操作 | `player` | 流式播放缓存，UI 在 SettingsPanel 里调用但逻辑归 player |
| `clearCache()`（前端内存缓存） | `library` | 专辑详情 / 歌词 / 主题色缓存 |

## Svelte 5 rune store 模式（关键技术约定）

> 此节需在 Phase 0 冻结，避免每个 store 实现方式分叉。

### 1. store 形态：模块单例 + 显式生命周期

`features/*/store.svelte.ts` 导出一个**单例对象**（不是 class 构造函数），内部用 `$state` / `$derived` 暴露响应式字段，并暴露 `init()` / `dispose()` 方法供根组件调用。

```ts
// 示意
let currentSong = $state<PlayerSong | null>(null);
let progress    = $state(0);

let unlisteners: Array<() => void> = [];

async function init() {
  if (unlisteners.length) return; // 幂等
  const un1 = await listen('player-state-changed', /* ... */);
  const un2 = await listen('player-progress', /* ... */);
  unlisteners = [un1, un2];
}

function dispose() {
  unlisteners.forEach(fn => fn());
  unlisteners = [];
}

export const playerStore = {
  get currentSong() { return currentSong; },
  get progress()    { return progress; },
  init, dispose,
};
```

### 2. 为什么不用 `$effect` 订阅 Tauri 事件

`$effect` 只能在 Svelte 组件生命周期内运行，模块作用域中无法使用。即使用 `$effect.root()` 创建独立根，其销毁时机也不随任何组件，不适合承载 Tauri 事件订阅（容易出现热更新后重复订阅）。

结论：**Tauri 事件订阅一律走 `init()` / `dispose()` 手动管理**，禁止在 store 模块顶层 `$effect` 中 `listen`。

### 3. 初始化协调

`App.svelte` 的 `onMount` 保留为**唯一初始化协调点**，按依赖顺序调用：

```text
env.init()          // matchMedia / resize（谁都可以读）
  ↓
shell.init()        // 面板开关默认值（不订阅 Tauri 事件）
  ↓
library.init()      // 拉 albums、outputDir 默认值 + 初始快照
  ↓
download.init()     // listen 下载事件 + 同步读 localStorage 偏好 + 拉 listDownloadJobs 快照
  ↓
player.init()       // listen player 事件 + 拉 getPlayerState 快照
```

`onDestroy` 中按**反向顺序**调用 `dispose()`。

> **注意**：init 顺序仅保证域间 store 引用不空，**不保证事件不丢**。不丢事件由每个 `init()` 自身的 catch-up 规约（见下文第 5 条）负责。

### 4. localStorage 持久化的就绪保护

`download.init()` 中**同步**读取 `DOWNLOAD_LYRICS_PREF_KEY`，**之后**才开启写回 `$effect`。具体做法：

1. store 内部维护 `prefsReady = $state(false)`。
2. 写回 localStorage 的 `$effect` 必须在消费 store 的**组件**里（如 `SettingsPanel`）挂载，并以 `if (!prefsReady) return` 作为守卫。
3. 或者在 `init()` 中同步读完后再启用写回（推荐，更简洁）。

### 5. `init()` 的 catch-up 责任

**不要**依赖"store 间 init 顺序"来保证事件不丢。Tauri 事件是否丢失取决于 **frontend `listen()` 时间 vs backend `emit()` 时间**，与域间顺序无关。

规约：每个订阅 Tauri 事件的 `init()` 必须遵循 **listen → 再 pull 一次快照** 的两段式：

```ts
async function init() {
  if (unlisteners.length) return;
  const un = await listen('xxx', handler);
  unlisteners = [un];
  // 订阅完成后立即拉一次当前快照，兜底可能错过的事件
  try {
    syncState(await getXxxSnapshot());
  } catch {
    // 快照接口不可用时容忍
  }
}
```

当前 `App.svelte` 中的 `getPlayerState()` 追赶、`listDownloadJobs()` 初始拉取都属于此模式，迁移到 store 时必须保留。

### 6. HMR 安全性

Svelte 5 模块级单例 + Vite HMR 场景下，**旧模块实例在被替换时不会自动调用 `dispose()`**，会导致僵尸监听器。每个 store 文件末尾必须加：

```ts
if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    dispose();
  });
}
```

Phase 0 模板必须包含这一行，Phase 2/3/5/6 新建 store 时不得遗漏。

### 7. 响应式粒度规约

单个 `$state` 承载复合对象（如整个 `DownloadManagerSnapshot`）会让每次进度事件触发**所有消费者**重新求值派生态。下载任务 10+ 并行时会掉帧。

规约：

1. **高频 progress 数据**（`taskSpeedMap`、`downloadedBytes`）单独 `$state`，与 `jobs` 结构体拆开。
2. **结构变更**（新增/删除 job、状态切换）走 `jobs = [...]` 重建。
3. **任务级字段更新**（bytes / status）允许细粒度 patch：`jobs[i].tasks[j] = { ...tasks[j], ...progress }`，但整 map 仍需用 `new Map(old).set(...)` 触发响应。

`downloadStore` 建议拆三个 `$state`：

```ts
let jobs = $state<DownloadJobSnapshot[]>([]);        // 结构变更
let taskSpeedMap = $state(new Map<string, number>()); // 高频但只改 map
let managerMeta = $state<Omit<DownloadManagerSnapshot, 'jobs'> | null>(null);
```

### 8. 组件 ↔ store 消费矩阵（Phase 0 必须冻结）

**规则**：容器组件 `import store` 直读；纯展示组件（如 `AudioPlayer` / `SongRow` / `AlbumCard`）只走 props + events，**不得 import store**。

| 组件 | 类型 | 消费的 store | 消费方式 |
| --- | --- | --- | --- |
| `App.svelte` | 装配层 | `env / library / player / download / shell` | 协调 `init/dispose`，不读业务字段 |
| `TopToolbar.svelte` | 容器 | `library`（刷新）、`shell`（面板开关）、`download`（badge） | 直读 |
| `AlbumWorkspace.svelte` | 容器 | `library / player / download`（勾选态） | 直读 |
| `PlayerDock.svelte` | 容器 | `player` | 直读，再以 props 喂 `AudioPlayer` |
| `SettingsPanel.svelte` | 容器 | `download / shell / player`（仅 `clearAudioCache` action） | 直读 |
| `DownloadTasksPanel.svelte` | 容器 | `download / shell` | 直读 |
| `AudioPlayer.svelte` | 展示 | — | props + events |
| `AlbumCard.svelte` | 展示 | — | props + events |
| `SongRow.svelte` | 展示 | — | props + events（含 `selectionMode` / `selected` prop） |
| `MotionSpinner` / `MotionPulseBlock` | 展示 | — | props |

#### 跨域路由动作

`SongRow` 点击派发 `on:select` 事件，由上层容器 `AlbumWorkspace` 根据 `download.selectionModeEnabled` 判断：

- 选择模式 ON → 调用 `download.toggleSelect(cid)`
- 选择模式 OFF → 调用 `player.play(cid, context)`

这条路由规则写死在 `AlbumWorkspace.svelte`，不进 `SongRow`。

## 分阶段实施计划

> 相比初版，**Phase 2 与 Phase 3/4 顺序对调**：先抽状态，再抽视图组件，避免同一组件因“先接 props、后接 store”被改两次。

## Phase 0：冻结边界与迁移契约

### 目标

明确模块边界、命名规范、事件传递方向，避免实现过程中反复改结构。

### 输出

1. 目录和文件命名冻结（含 `features/env/`）。
2. 组件输入/输出契约（props/events）冻结。
3. **跨域依赖层次图冻结**（`env` 任意读 + `library → player → download → shell` 单向链）。
4. **Svelte 5 store 模式冻结**（单例 + `init/dispose`，禁止模块顶层 `$effect` 订阅 Tauri 事件；含 HMR dispose 模板）。
5. **归属裁定表冻结**（见上文，解决 `selection*`、`OverlayScrollbars`、`clearAudioCache` 等模糊归属）。
6. `App.svelte` 初始化协调顺序冻结。
7. **`init()` catch-up 两段式规约冻结**（listen → pull snapshot）。
8. **响应式粒度规约冻结**（`jobs / taskSpeedMap / managerMeta` 三拆）。
9. **组件 ↔ store 消费矩阵冻结**（容器直读 / 展示 props+events，`SongRow` 跨域路由由 `AlbumWorkspace` 承担）。

### 完成定义

1. 团队成员可按领域并行开发，无需回头改契约。
2. 新增代码默认进入 `features/*` 或独立组件，不再回流到 `App.svelte`。

## Phase 1：抽离纯函数与工具逻辑

### 目标

先迁移低风险逻辑，降低主文件密度且保持行为不变。

### 主要工作

1. 抽队列工具：`buildPlaybackContext`、`shufflePlaybackEntries`、`resolveWrappedQueueIndex` 等 → `features/player/queue.ts`。
2. 抽歌词工具：`parseLyricText` 及相关帮助函数 → `features/player/lyrics.ts`。
3. 抽下载展示工具：`formatByteSize`、`formatSpeed`、`getTaskStatusLabel`、`hasCurrentDownloadOptions`、`buildSelectionKey` 等 → `features/download/formatters.ts` 与 `guards.ts`。
4. 抽动效工具：`fadeEnter/fadeExit/axisEnter/axisExit/motionTransition` → `features/shell/motion.ts`。

### 完成定义

1. `App.svelte` 中纯函数数量显著下降。
2. 迁移前后 UI 行为一致。
3. 新工具文件具备独立的单元测试入口（即使当前测试不入 CI，也留好结构）。

## Phase 2：抽 download 领域状态

### 目标

把任务状态、任务操作、下载设置与偏好持久化、批量选择模式迁入 `features/download/store.svelte.ts`。

### 主要工作

1. 收敛下载任务查询、创建、取消、重试、历史清理。
2. 收敛下载设置状态（`outputDir / format / downloadLyrics`）与 localStorage 持久化。
3. 收敛批量选择：`selectionModeEnabled / selectedSongCids / creatingSelectionKey` 及相关派生态。
4. Tauri 三个下载事件订阅收敛到 `download.init()`；`App.svelte` 的 `onMount` 改为调用 `download.init()`。
5. 将下载相关派生逻辑集中到 `selectors / formatters / guards`。

### 完成定义

1. 下载域不再散落在 `App.svelte` 的多个区段。
2. 下载功能可独立演进（如筛选/分组/排序）。
3. 未出现 `downloadLyrics` 初始化时序导致的偏好丢失（回归验证）。

## Phase 3：抽 player 领域状态

### 目标

把“播放队列 + 歌词 + 自动切歌 + 面板开关联动”迁入 `features/player/store.svelte.ts`。

### 主要工作

1. 聚合播放器状态和行为方法。
2. 收敛 Tauri `player-state-changed / player-progress` 订阅到 `player.init()`。
3. `lyricsOpen / playlistOpen` 面板开关与 `currentSong` 联动迁入。
4. 对外暴露最小接口，供 `PlayerDock` 与 `AudioPlayer` 消费。
5. 明确 `AudioPlayer.svelte` 的边界：保持通过 **props + events** 通信（保留其可独立复用性），不直接 import store。

### 完成定义

1. `App.svelte` 不再直接维护播放器内部细节。
2. 播放行为（上一首/下一首/乱序/循环）无功能回退。
3. `player.init()` 订阅在 `download.init()` 之后执行，确保不丢事件。

## Phase 4：拆右侧面板组件（设置 + 下载任务）

### 目标

下载/播放 store 已就位，此时把最独立的 UI 区域拆成组件，一次性接入 store，避免双重重构。

### 主要工作

1. 新建 `SettingsPanel.svelte`，直接消费 `download.store` 与 `shell.store`（面板开关）。
2. 新建 `DownloadTasksPanel.svelte`，直接消费 `download.store`。
3. `App.svelte` 仅负责渲染入口与外层 transition，不传递 store 内部状态。

### 完成定义

1. `App.svelte` 模板长度明显减少。
2. 两个面板可单独维护、单独迭代。
3. 面板组件 props 清单极简（仅必要的外部触发回调）。

## Phase 5：抽 library 领域状态

### 目标

收敛专辑加载、骨架屏、封面预加载、滚动舞台动效与 `OverlayScrollbars` 容器。

### 主要工作

1. 收敛 `albums / selectedAlbum / loadingAlbums / loadingDetail` 与异步加载时序（`albumRequestSeq / themeRequestSeq`）。
2. 收敛封面预加载（`preloadImage / preloadAlbumArtwork / setAlbumStageAspectRatio`）。
3. 收敛滚动舞台（`albumStage*` 系列状态、rAF 动画帧、`ResizeObserver` 订阅）。
4. 收敛 `OverlayScrollbars` 容器回调。
5. 新增 `AlbumWorkspace.svelte`，承接专辑列表 + 详情视图。

### 完成定义

1. `App.svelte` 中专辑相关状态清空。
2. 滚动舞台动效无回归（封面收缩、滚动吸附一致）。

## Phase 6：抽 shell 领域与收尾

### 目标

完成全局壳层状态收敛，形成稳定终态。`App.svelte` 退化为装配层。

### 主要工作

1. 收敛系统偏好（`isMacOS / prefersReducedMotion`）与窗口 resize 事件。
2. 收敛全局面板开关（设置/下载任务面板）。
3. 清理无效状态与未使用函数。
4. 新增 `TopToolbar.svelte` / `PlayerDock.svelte`（如尚未完成）。
5. 校验 `App.svelte` 只剩：布局骨架、初始化协调、顶层 `AnimatePresence`。

### 完成定义

1. `App.svelte` 不再含 `listen()` 调用，所有 Tauri 订阅在 store 的 `init()` 中。
2. 各业务域职责边界清晰且可追踪。
3. 新增功能可直接落在对应域内，不需要回流到 `App.svelte`。

## 当前方案（方案 B）优劣分析

### 优势

1. **平衡风险与收益**：比纯组件拆分更彻底，比架构重做更可控。
2. **可渐进落地**：可按 phase 独立提交，回滚成本低。
3. **可维护性提升明显**：按业务域组织，定位问题更快。
4. **协作友好**：前端工程师可按领域并行推进。
5. **兼容现有栈**：不要求额外状态管理库，延续 Svelte 5 rune 模式。

### 劣势

1. **初期重构成本中等**：需要连续多个迭代窗口完成迁移。
2. **边界设计要求高**：若契约不清，可能形成“新文件中的耦合复制”（Phase 0 契约冻结缓解此风险）。
3. **短期心智切换成本**：团队需要适应 `features` 分层与 store 单例 + `init/dispose` 模式。
4. **测试配套需跟进**：若不补充验证，跨文件迁移容易引入隐性回归。

## 与其他方案的取舍

1. 相比方案 A（仅拆视图）：
   方案 B 能解决状态耦合根因，而不是只减少模板体积。
2. 相比方案 C（架构级重构）：
   方案 B 改造范围更可控，更适合当前项目节奏与交付约束。

结论：**方案 B 是当前阶段最优解**，建议优先执行。

## 风险与控制措施

### 主要风险

1. 迁移过程中出现事件订阅重复或清理遗漏（尤其是热更新场景）。
2. 播放队列和歌词联动出现竞态回归（`albumRequestSeq / lyricRequestSeq / playbackEndRequestSeq` 等序列号机制必须完整迁移）。
3. 下载任务状态在不同组件展示不一致（多个组件消费同一 store 时）。
4. `downloadLyrics` / `outputDir` 偏好在初始化时被默认值覆盖，导致用户偏好丢失。
5. 初始化顺序错乱：player 事件在 store 未 `init` 时已到达，导致首屏状态不同步。
6. `AudioPlayer.svelte` 被改为强耦合 store，失去独立复用性。

### 控制措施

1. 每个 phase 都设置可回归清单并独立验收。
2. 先迁移纯函数，再迁移状态，再迁移视图组件，避免同时改“结构 + 行为”。
3. store `init()` 统一幂等（二次调用直接返回），防止热更新导致重复订阅。
4. 对关键路径进行冒烟验证：专辑切换、播放控制、歌词跟随、下载创建 / 取消 / 重试、偏好持久化。
5. `App.svelte` 初始化协调点保留显式调用顺序（不要拆到各组件 `onMount`）。
6. `AudioPlayer.svelte` 坚持 props + events 接口，store 消费限定在容器层（`PlayerDock.svelte`）。

## 验收标准（工程视角）

### 量化指标

1. `App.svelte` **行数 ≤ 300 行**（当前 2666，目标减少 >88%）。
2. `App.svelte` 中 `$state` 声明数 ≤ 3（仅保留极少量协调态，如初始化完成标志）。
3. `App.svelte` 中 `listen(` 出现次数 = **0**，Tauri 订阅全部归属各 store 的 `init()`。
4. 每个 `features/*/store.svelte.ts` 行数 ≤ 400。
5. `cargo check --workspace` 不受影响（前端重构不应触发 Rust 编译错误，但需保持契约）。

### 结构指标

1. `features/env/library/player/download/shell` 五个域各有明确入口文件，依赖方向遵循 `env 任意读 + library → player → download → shell`。
2. 每个 store 文件末尾都有 `import.meta.hot?.dispose(() => dispose())` 保护。
3. 所有订阅 Tauri 事件的 `init()` 遵循 "listen → pull snapshot" 两段式。

### 功能回归（必须通过冒烟验证）

1. 专辑加载与刷新正常，滚动舞台动效（封面收缩、固化、透明度）与重构前像素级一致。
2. 播放器控制（播放/暂停/上一首/下一首/seek/音量/乱序/循环模式）全部正常。
3. 歌词面板跟随播放进度高亮正确。
4. 下载任务：单曲创建、整专创建、取消、重试、清理历史全部正常。
5. 下载偏好（输出目录、格式、歌词侧车）刷新后保持正确。
6. 批量选择模式进入/退出/选择/全选/批量创建正常。
7. `prefersReducedMotion` 启用时动效正确降级。

### 可持续性指标

1. 新增"下载过滤"或"播放模式扩展"类功能时，改动可限定在单一域内，不回流到 `App.svelte`。

## 协作建议

1. 以 phase 为最小交付单位，逐步合并，避免超大 PR。
2. 每个 phase 保持“结构改造优先，行为保持不变”的提交策略。
3. 每次合并附带“迁移前后行为对照”与“已验证场景清单”。
4. Phase 间存在硬依赖（Phase 2 → Phase 4，Phase 3 → Phase 6），解除阻塞前不启动下游 phase。
5. 如 Phase 0 契约在实施中出现需要调整的点，必须先更新文档再改代码，不允许“私下调整契约”。
