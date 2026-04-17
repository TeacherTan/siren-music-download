# 前端开发指南

> 前端架构、开发约定与验收基线的唯一主文档。
>
> 最后更新：2026-04-17

## 1. 布局与主要组件

### 布局结构

当前主界面采用"左侧专辑导航 + 主内容区 + 底部播放器 Dock + 右侧按需滑出面板"的结构：

```text
┌──────────────┬──────────────────────────────────────────────┐
│ 专辑侧栏     │ 主内容区                                     │
│ AlbumCard ×N │ 顶部工具栏                                   │
│              │ 专辑舞台（封面大图）                         │
│              │ 专辑信息 / 操作按钮 / 曲目列表 / 内容滚动区  │
│              │                                              │
│              │ 底部播放器 Dock                              │
│              │ ├─ 传输控制 / 进度 / 乱序 / 循环 / 下载      │
│              │ └─ 歌词面板 / 播放队列面板                   │
└──────────────┴──────────────────────────────────────────────┘
                         ▲                    ▲
                         └── 设置 Sheet        └── 下载任务 Sheet
```

左侧和主内容区都使用 `OverlayScrollbars` 管理滚动，macOS 下顶部保留拖拽区域。

### 主要组件

| 组件 | 职责 |
|------|------|
| `App.svelte` | 应用入口，负责专辑加载、播放状态同步、下载状态同步、各类面板编排 |
| `AlbumSidebar.svelte` | 左侧专辑导航容器，内部渲染 `AlbumCard.svelte` |
| `TopToolbar.svelte` | 顶部工具栏，提供刷新、下载任务入口、设置入口和活动下载数量 badge |
| `AlbumWorkspace.svelte` | 主内容区容器，包裹专辑舞台、专辑信息和曲目列表滚动区 |
| `SongRow.svelte` | 曲目行；默认点击播放，进入多选模式后改为点击勾选 |
| `PlayerDock.svelte` | 底部 Dock 容器，内部承载 `AudioPlayer.svelte` |
| `AudioPlayer.svelte` | 播放器主体，包含播放控制、进度、乱序/循环、歌词/队列切换和当前歌曲下载入口 |
| `SettingsSheet.svelte` | 右侧设置面板，负责下载参数、通知偏好和缓存清理 |
| `DownloadTasksSheet.svelte` | 右侧下载任务面板，负责任务列表、进度、失败项、取消/重试和历史清理 |
| `StatusToastHost.svelte` | 顶部 toast 宿主，替代同步 `alert()` 反馈 |

## 2. 目录结构与域边界

### 目录树

```text
src/
├── App.svelte                         # 当前顶层装配与主要业务编排
├── app.css                            # 全局变量、基础样式、兼容性覆盖
└── lib/
    ├── components/
    │   ├── AlbumCard.svelte           # 专辑卡片
    │   ├── SongRow.svelte             # 曲目行
    │   ├── AudioPlayer.svelte         # 播放器展示组件
    │   └── app/                       # 壳层组件
    │       ├── TopToolbar.svelte
    │       ├── AlbumSidebar.svelte
    │       ├── AlbumWorkspace.svelte
    │       ├── PlayerDock.svelte
    │       ├── SettingsSheet.svelte
    │       ├── DownloadTasksSheet.svelte
    │       └── StatusToastHost.svelte
    ├── design/
    │   ├── tokens.ts                  # 设计 token
    │   ├── variants.ts                # 视觉变体
    │   └── motion.ts                  # 动效参数
    ├── features/
    │   ├── env/store.svelte.ts        # 只读环境状态
    │   ├── shell/store.svelte.ts      # 全局壳层状态
    │   ├── library/                   # 专辑与舞台相关逻辑
    │   ├── player/                    # 播放与歌词相关逻辑
    │   └── download/                  # 下载任务与偏好相关逻辑
    ├── api.ts                         # Tauri command bridge
    ├── cache.ts                       # 专辑/歌词/主题色缓存
    ├── theme.ts                       # 动态主题变量应用
    └── types.ts                       # 前后端共享结构的 TS 版本
```

### 五域边界

| 域 | 职责 | 状态 |
|----|------|------|
| `env` | 只读环境状态：`isMacOS`、`prefersReducedMotion`、视口信号 | 已落地 |
| `library` | 专辑列表与详情加载、切换竞态控制、封面预加载、舞台滚动状态 | 骨架待迁移 |
| `player` | 当前歌曲、播放队列、歌词加载与高亮、上一首/下一首/乱序/循环 | 骨架待迁移 |
| `download` | 任务列表与操作、下载设置与偏好、单曲/整专/多选入口、批量选择模式 | 骨架待迁移 |
| `shell` | 设置面板与下载面板开关、toast 宿主、全局页面级交互协调 | 已落地 |

### 依赖方向

推荐保持单向读依赖：

```text
env → library → player → download → shell
```

- `env` 是只读环境域，可被其他域读取
- `library` 提供内容上下文
- `player` 可依赖当前专辑上下文构建播放队列
- `download` 可依赖专辑/当前曲目上下文创建任务
- `shell` 只读取其他域的聚合结果，不反向写入业务状态

## 3. UI 系统约束

### 设计 token

核心 token 维度：`surface`、`text`、`accent`、`motion`、`density`

关键表面语义：
- `surface.window`
- `surface.sidebar`
- `surface.workspace`
- `surface.sheet`
- `surface.dock`
- `surface.flyout`
- `surface.state`

### Apple 化边界

视觉方向：`macOS 应用骨架 + Apple Music 的内容表达`

固定约束：

1. 玻璃材质只集中在 `sheet / dock / flyout`
2. 主工作区保持干净，不做整页玻璃化
3. 动态专辑色继续保留，但默认降饱和、提亮、压对比
4. 列表区优先效率，标题区优先内容层级
5. 阴影、边框、高光都保持轻量，避免厚重卡片感

### 动效规则

- 不使用 bounce 类夸张反馈
- `reduced motion` 开启时要能降级
- 页面与面板动效共用 `motion.ts` 参数，不各处自定义

### 组件分层

| 层级 | 说明 | 示例 |
|------|------|------|
| Primitive | 基础交互原语，来源于 shadcn-svelte / Bits UI | Button、Badge、Sheet、Select、Switch、Progress、Tooltip、Dialog、Skeleton、Tabs、Slider |
| App Variant | 在原语之上包一层项目视觉和状态约束 | ToolbarIconButton、AppBadge、SheetSectionHeader、DockUtilityButton |
| Composite | 面向单个业务区域的复合组件 | TopToolbar、AlbumSidebar、AlbumWorkspace、PlayerDock、SettingsSheet、DownloadTasksSheet、LyricsFlyout、PlaylistFlyout |
| Pattern | 跨多个组件复用的结构模式 | 侧栏列表模式、主工作区模式、右侧 Sheet 模式、状态反馈模式、空状态/加载状态/错误状态模式 |

### 组件生命周期

沿用 `draft → beta → stable → deprecated` 四状态。

进入 `stable` 的最低要求：
1. 已接入设计 token
2. 已定义变体边界
3. 已满足键盘可达性
4. 已写入说明
5. 已至少被两个场景复用

## 4. Svelte 5 Store 约定

> 所有领域 store 必须遵守以下约定。

### 4.1 store 形态：模块单例 + 显式生命周期

`features/*/store.svelte.ts` 导出一个单例对象，内部用 `$state` / `$derived` 暴露响应式字段，并暴露 `init()` / `dispose()` 方法：

```ts
let currentSong = $state<PlayerSong | null>(null);
let unlisteners: Array<() => void> = [];

async function init() {
  if (unlisteners.length) return; // 幂等
  const un1 = await listen('player-state-changed', handler);
  unlisteners = [un1];
}

function dispose() {
  unlisteners.forEach(fn => fn());
  unlisteners = [];
}

export const playerStore = {
  get currentSong() { return currentSong; },
  init, dispose,
};
```

### 4.2 禁止用 `$effect` 订阅 Tauri 事件

`$effect` 只能在 Svelte 组件生命周期内运行，模块作用域中无法使用。即使用 `$effect.root()` 创建独立根，销毁时机也不可控。

**结论**：Tauri 事件订阅一律走 `init()` / `dispose()` 手动管理，禁止在 store 模块顶层 `$effect` 中 `listen`。

### 4.3 初始化协调顺序

`App.svelte` 的 `onMount` 为唯一初始化协调点，按依赖顺序调用：

```text
env.init() → shell.init() → library.init() → download.init() → player.init()
```

`onDestroy` 中按反向顺序调用 `dispose()`。

> 注意：init 顺序仅保证域间 store 引用不空，不保证事件不丢。不丢事件由每个 `init()` 自身的 catch-up 规约负责。

### 4.4 catch-up 责任

每个订阅 Tauri 事件的 `init()` 必须遵循 **listen → 再 pull 一次快照** 的两段式：

```ts
async function init() {
  const un = await listen('xxx', handler);
  unlisteners = [un];
  // 订阅完成后立即拉一次当前快照，兜底可能错过的事件
  try { syncState(await getXxxSnapshot()); } catch { /* 容忍 */ }
}
```

### 4.5 HMR 安全性

Svelte 5 模块级单例 + Vite HMR 场景下，旧模块实例被替换时不会自动调用 `dispose()`。每个 store 文件末尾必须加：

```ts
if (import.meta.hot) {
  import.meta.hot.dispose(() => { dispose(); });
}
```

### 4.6 响应式粒度规约

单个 `$state` 承载复合对象（如整个 `DownloadManagerSnapshot`）会让每次进度事件触发所有消费者重新求值。

规约：
1. 高频 progress 数据单独 `$state`，与 `jobs` 结构体拆开
2. 结构变更（新增/删除 job、状态切换）走 `jobs = [...]` 重建
3. 任务级字段更新允许细粒度 patch，但需用 `new Map(old).set(...)` 触发响应

### 4.7 组件 ↔ store 消费矩阵

**规则**：容器组件 `import store` 直读；纯展示组件只走 props + events，不得 import store。

| 组件 | 类型 | 消费方式 |
|------|------|----------|
| `App.svelte` | 装配层 | 协调 init/dispose，不读业务字段 |
| `TopToolbar.svelte` | 容器 | 直读 `library`、`shell`、`download` |
| `AlbumWorkspace.svelte` | 容器 | 直读 `library`、`player`、`download` |
| `PlayerDock.svelte` | 容器 | 直读 `player`，再以 props 喂 `AudioPlayer` |
| `SettingsSheet.svelte` | 容器 | 直读 `download`、`shell` |
| `DownloadTasksSheet.svelte` | 容器 | 直读 `download`、`shell` |
| `AudioPlayer.svelte` | 展示 | props + events |
| `AlbumCard.svelte` | 展示 | props + events |
| `SongRow.svelte` | 展示 | props + events |

## 5. IPC 规则

**UI 组件禁止直接调用 `invoke` 或 `listen`**。

建立通信网关层：
- 创建领域服务文件（如 `src/lib/api.ts` 或 `features/*/service.ts`）封装 Tauri IPC
- UI 仅绑定服务层暴露的响应式状态或调用服务层暴露的方法

## 6. 交互模式

### 顶部工具栏

- 刷新当前缓存与页面数据
- 打开下载任务面板
- 打开设置面板
- 展示活动下载任务数量 badge

设置面板和下载任务面板互斥打开。

### 曲目点击行为

- 默认状态：点击 `SongRow` 立即播放该曲
- 曲目行右侧下载按钮：创建单曲下载任务
- 多选模式：点击 `SongRow` 切换选中状态，右侧下载按钮禁用

### 多选下载

入口位于专辑信息区。进入多选模式后显示：
- `多选下载 / 取消多选`
- `全选`、`清空`、`反选`
- `下载所选歌曲`
- 已选歌曲数量文案

批量下载调用 `create_download_job`，使用 `kind = selection`。

### 播放状态流

前端通过 Tauri command 拉起播放，通过 Tauri event 持续同步状态：

```text
App.svelte
  ├─ invoke('play_song')
  ├─ invoke('pause_playback')
  ├─ invoke('resume_playback')
  ├─ invoke('seek_current_playback')
  ├─ invoke('play_next' / 'play_previous')
  └─ listen('player-state-changed' / 'player-progress')
```

事件载荷统一是 `PlayerState`，包含当前曲目、播放状态、进度、队列、乱序/循环状态等。

## 7. 内容与反馈规范

### Tone

- 句子更短
- 少解释
- 不营销
- 不做安抚式废话
- 风险动作直接说清

### Titles

- 面板标题像系统功能名
- 页面标题强调内容，不堆副标题
- 不使用宣传式语气

### Buttons

- 动词优先
- 文案尽量控制在 6 个字内
- 危险动作明确说明结果

### Errors

- 先说失败对象
- 再说可恢复动作
- 技术细节不默认暴露

### Success Feedback

- 只保留结果和必要下一步
- toast 不解释实现过程

### Empty / Loading States

- 空状态只说当前没有什么，再给一句引导动作
- Loading 优先骨架，文字次之
- 不写夸张进行态语句

## 8. QA 基线

> 最近一次完整验证：2026-04-17

### Baseline Notes

- `cargo build` 在全新 worktree 中会先依赖前端 `dist/` 产物，需要先执行 `npm run build`
- `npm run check:types` 和 `npm run check:build` 在补齐依赖后可通过
- `cargo check` 仍有一个既有 warning：`src-tauri/src/notification/desktop.rs` 中的 `Manager` 未使用

### Latest Verification

通过的命令：
- `npm run check`
- `npm run check:cargo`

### Core Flows

- [ ] 首屏可加载专辑列表
- [ ] 切换专辑后详情刷新正常
- [ ] 单曲播放 / 暂停 / 恢复 / seek 正常
- [ ] 上一首 / 下一首 / 乱序 / 循环正常
- [ ] 歌词面板显示和高亮正常
- [ ] 播放列表面板显示和切歌正常
- [ ] 单曲下载正常
- [ ] 整专下载正常
- [ ] 多选下载正常
- [ ] 下载取消 / 重试 / 清理历史正常
- [ ] 设置项持久化正常

### Visual Checks

- [ ] 亮色主题正常
- [ ] 暗色主题正常
- [ ] 动态专辑主题色正常
- [ ] 右侧面板玻璃材质正常
- [ ] 底部 Dock 玻璃材质正常
- [ ] 主工作区保持干净，无大面积玻璃化
- [ ] Apple 化排版层级正常
- [ ] reduced-motion 正常

### Usage Notes

建议在以下场景重新执行清单：
1. 调整 `src/App.svelte` 中的播放、下载或专辑加载编排
2. 修改 `src/lib/components/app/` 下的壳层组件
3. 修改 `src/lib/design/` 下的 token、variant 或 motion 规则
4. 推进 `features/library`、`features/player`、`features/download` 对运行时状态的接管

## 9. 当前遗留问题

1. `src/App.svelte` 仍持有大量业务状态、Tauri command 调用和事件订阅，尚未退化为真正的装配层
2. `libraryStore`、`playerStore`、`downloadStore` 当前主要是骨架文件，尚未接管实际运行时状态
3. 播放、下载、歌词、滚动舞台等逻辑仍然大量集中在 `src/App.svelte`
4. 自动化回归保障仍偏弱，当前仍以手工 QA 为主
5. 下载历史增强、搜索/过滤和 session 持久化仍是后续切片

如果继续推进结构迁移，优先级建议：
1. 把下载域迁出 `src/App.svelte`
2. 把播放域和歌词域迁出 `src/App.svelte`
3. 把专辑加载与舞台逻辑迁入 `library` 域
4. 最后清理 `App.svelte` 中剩余的协调态和壳层 wiring

## 10. 后续优化项

1. 全面采用 Svelte 5 runes 风格的领域单例
2. 把 Tauri IPC 收敛到领域层或网关层，避免 UI 组件直接 invoke/listen
3. 对高风险壳层组件补充轻量自动化验证（Vitest 或 Playwright）
4. 明确 CSS 变量与样式系统的映射规则，减少任意值散落
5. 对 Settings / DownloadTasks / PlayerDock 持续保持统一材质和反馈约束

## 11. 相关文档

- [BACKEND_API_CONTRACT.md](BACKEND_API_CONTRACT.md)：后端类型、命令、事件的唯一契约来源
- [BACKEND_ROADMAP.md](BACKEND_ROADMAP.md)：后端未来规划
- [DECISIONS.md](DECISIONS.md)：技术选型决策记录
- [RELEASE_PROCESS.md](RELEASE_PROCESS.md)：CI 与发布流程