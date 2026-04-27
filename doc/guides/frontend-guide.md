# 前端开发指南

> 前端架构、开发约定与验收基线的唯一主文档。
>
> 最后更新：2026-04-27

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

| 组件                           | 职责                                                                             |
| ------------------------------ | -------------------------------------------------------------------------------- |
| `App.svelte`                   | 前端根装配层，负责 controller 初始化、Tauri 事件订阅、跨域状态协调和壳层组件编排 |
| `AlbumSidebarContainer.svelte` | 左侧专辑侧栏装配容器，承接搜索输入、搜索结果、列表态与专辑选择                   |
| `TopToolbar.svelte`            | 顶部工具栏，提供刷新、下载任务入口、设置入口和活动下载数量 badge                 |
| `AlbumWorkspace.svelte`        | 主内容区布局容器                                                                 |
| `AlbumWorkspaceContent.svelte` | 专辑舞台、专辑详情、骨架屏和曲目区组合容器                                       |
| `SongRow.svelte`               | 曲目行；默认点击播放，进入多选模式后改为点击勾选                                 |
| `PlayerFlyoutStack.svelte`     | 底部播放器 Dock 与歌词 / 播放队列浮层组合容器                                    |
| `AudioPlayer.svelte`           | 播放器主体，包含播放控制、进度、乱序/循环、歌词/队列切换和当前歌曲下载入口       |
| `AppSideSheets.svelte`         | 设置面板与下载任务面板装配容器                                                   |
| `SettingsSheet.svelte`         | 右侧设置面板，负责下载参数、通知偏好、日志等级、日志浏览和缓存清理               |
| `DownloadTasksSheet.svelte`    | 右侧下载任务面板，负责任务列表、进度、失败项、取消/重试和历史清理                |
| `StatusToastHost.svelte`       | 顶部 toast 宿主，替代同步 `alert()` 反馈                                         |

## 2. 目录结构与域边界

### 目录树

```text
src/
├── App.svelte                         # 当前前端根装配层
├── main.ts                            # 前端入口
├── app.css                            # 全局变量、基础样式、兼容性覆盖
└── lib/
    ├── components/
    │   ├── AlbumCard.svelte           # 专辑卡片
    │   ├── SongRow.svelte             # 曲目行
    │   ├── AudioPlayer.svelte         # 播放器展示组件
    │   └── app/                       # 当前壳层组件
    │       ├── AlbumSidebar.svelte
    │       ├── AlbumSidebarContainer.svelte
    │       ├── AlbumStage.svelte
    │       ├── AlbumWorkspace.svelte
    │       ├── AlbumWorkspaceContent.svelte
    │       ├── AlbumDetailPanel.svelte
    │       ├── AlbumDetailSkeleton.svelte
    │       ├── PlayerDock.svelte
    │       ├── PlayerFlyoutStack.svelte
    │       ├── AppSideSheets.svelte
    │       ├── TopToolbar.svelte
    │       ├── SettingsSheet.svelte
    │       ├── DownloadTasksSheet.svelte
    │       └── StatusToastHost.svelte
    ├── design/
    │   ├── tokens.ts                  # 设计 token
    │   ├── variants.ts                # 视觉变体
    │   └── motion.ts                  # 动效参数
    ├── features/
    │   ├── env/store.svelte.ts        # 只读环境状态
    │   ├── library/                   # 专辑与搜索 controller / selector / helper
    │   ├── player/                    # 播放、歌词与队列 controller / helper
    │   ├── download/                  # 下载任务与筛选 controller / formatter / guard
    │   └── shell/                     # 全局壳层状态、设置与舞台动效 controller
    ├── api.ts                         # 主 Tauri command bridge
    ├── settingsApi.ts                 # 设置面板专用 IPC bridge
    ├── cache.ts                       # 专辑/歌词/主题色缓存
    ├── theme.ts                       # 动态主题变量应用
    └── types.ts                       # 前后端共享结构的 TS 版本
```

### 五域边界

| 域         | 职责                                                               | 当前实现形态       |
| ---------- | ------------------------------------------------------------------ | ------------------ |
| `env`      | 只读环境状态：`isMacOS`、`prefersReducedMotion`、视口信号          | store 已接管       |
| `library`  | 专辑列表 / 详情加载、库内搜索、切换竞态控制、封面预加载、舞台联动  | controller 为主    |
| `player`   | 当前歌曲、播放队列、歌词加载与高亮、上一首/下一首/乱序/循环        | controller 为主    |
| `download` | 任务列表与操作、下载设置与偏好、单曲/整专/多选入口、历史筛选       | controller 为主    |
| `shell`    | 设置面板与下载面板开关、toast 宿主、设置域调用、全局页面级交互协调 | store + controller |

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

| 层级        | 说明                                         | 示例                                                                                                                  |
| ----------- | -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Primitive   | 基础交互原语，来源于 shadcn-svelte / Bits UI | Button、Badge、Sheet、Select、Switch、Progress、Tooltip、Dialog、Skeleton、Tabs、Slider                               |
| App Variant | 在原语之上包一层项目视觉和状态约束           | ToolbarIconButton、AppBadge、SheetSectionHeader、DockUtilityButton                                                    |
| Composite   | 面向单个业务区域的复合组件                   | TopToolbar、AlbumSidebar、AlbumWorkspace、PlayerDock、SettingsSheet、DownloadTasksSheet、LyricsFlyout、PlaylistFlyout |
| Pattern     | 跨多个组件复用的结构模式                     | 侧栏列表模式、主工作区模式、右侧 Sheet 模式、状态反馈模式、空状态/加载状态/错误状态模式                               |

### 组件生命周期

沿用 `draft → beta → stable → deprecated` 四状态。

进入 `stable` 的最低要求：

1. 已接入设计 token
2. 已定义变体边界
3. 已满足键盘可达性
4. 已写入说明
5. 已至少被两个场景复用

## 4. 当前运行时架构约定

> 当前前端已进入 controller + shell composition 的过渡架构阶段。本节描述**当前实现事实**，不是早期目标形态。

### 4.1 当前运行时形态

当前运行时以 `App.svelte` 为根装配层，职责包括：

1. 创建并持有 `library / player / download / settings / albumStageMotion` controllers
2. 订阅 Tauri 事件并把事件分发给对应 controller 或壳层逻辑
3. 组合 `AlbumSidebarContainer`、`AlbumWorkspaceContent`、`PlayerFlyoutStack`、`AppSideSheets` 等壳层组件
4. 协调搜索定位、播放器队列、下载面板与设置面板之间的跨域交互

当前主真相来源：

- `src/App.svelte`
- `src/lib/features/library/controller.svelte.ts`
- `src/lib/features/player/controller.svelte.ts`
- `src/lib/features/download/controller.svelte.ts`
- `src/lib/features/shell/settings.svelte.ts`
- `src/lib/features/shell/albumStageMotion.svelte.ts`

### 4.2 controller 与 store 的分工

当前代码中并不是所有领域都由 `store.svelte.ts` 直接承载主运行时：

- `envStore`、`shellStore` 已承担稳定的全局环境 / 壳层状态职责
- `library`、`player`、`download` 当前以 controller 工厂为主，`store.svelte.ts` 更多承担过渡性或骨架性角色
- 因此前端指南中的部分 store 约定应理解为**目标方向或局部适用规则**，不能默认视为当前所有域的真实实现

### 4.3 初始化与销毁约定

当前 `App.svelte` 使用根级 `$effect` 作为运行时装配与销毁入口，而不是 `onMount` / `onDestroy`：

```text
libraryController.init()
playerController.init()
downloadController.init()
envStore.init()
shellStore.init()
↓
订阅 Tauri 事件
↓
bootstrapApp()
↓
cleanup 时 teardownAppRuntime(...)
```

这意味着：

- `App.svelte` 仍然是当前唯一的运行时协调中心
- 领域初始化顺序和事件 catch-up 逻辑目前由 controller / store 与 `App.svelte` 共同承担
- 若后续继续推进 store 接管，需要先改写当前 controller 责任边界，再更新本指南

### 4.4 Tauri 事件接入规则

当前 Tauri 事件订阅主要集中在 `App.svelte`，而不是统一下沉到所有领域 store：

- `player-state-changed`
- `player-progress`
- `download-manager-state-changed`
- `download-job-updated`
- `download-task-progress`
- `local-inventory-state-changed`
- `app-error-recorded`

因此当前约束应表述为：

- **UI 展示组件** 不得直接调用 `listen` 或 `invoke`
- **controller / shell / bridge 层** 可以承担 IPC 与事件接入
- 是否进一步下沉到 store，属于后续架构收敛议题，而不是当前既成事实

### 4.5 当前组件消费矩阵

**规则**：展示组件仍以 props + events 为主；装配层、壳层容器或具备明确职责边界的壳层面板可以直接消费 controller、store 与专用 IPC bridge。

| 组件                           | 类型     | 当前消费方式                                                             |
| ------------------------------ | -------- | ------------------------------------------------------------------------ |
| `App.svelte`                   | 装配层   | 创建 controller、订阅事件、协调壳层，不直接渲染细粒度业务 UI             |
| `AlbumSidebarContainer.svelte` | 壳层容器 | 消费搜索、列表、选择状态并向 `AlbumSidebar` / `AlbumCard` 下发 props     |
| `AlbumWorkspaceContent.svelte` | 壳层容器 | 消费专辑详情、骨架屏、选择态、下载态并组合详情与曲目区                   |
| `PlayerFlyoutStack.svelte`     | 壳层容器 | 消费播放器状态并组合 Dock、歌词和播放队列浮层                            |
| `AppSideSheets.svelte`         | 壳层容器 | 组合设置面板与下载任务面板                                               |
| `SettingsSheet.svelte`         | 壳层面板 | 直接消费 `settingsApi.ts` 与 bindable 状态，负责日志 viewer 与设置域调用 |
| `DownloadTasksSheet.svelte`    | 壳层面板 | 消费下载任务列表、筛选状态与操作回调                                     |
| `AudioPlayer.svelte`           | 展示     | props + events                                                           |
| `AlbumCard.svelte`             | 展示     | props + events                                                           |
| `SongRow.svelte`               | 展示     | props + events                                                           |

### 4.6 响应式粒度规约

单个 `$state` 承载复合对象（如整个 `DownloadManagerSnapshot`）会让每次进度事件触发所有消费者重新求值。

规约：

1. 高频 progress 数据单独 `$state`，与 `jobs` 结构体拆开
2. 结构变更（新增/删除 job、状态切换）走 `jobs = [...]` 重建
3. 任务级字段更新允许细粒度 patch，高频 `Map` 状态使用 `SvelteMap` 的 `.set()` / `.clear()` 触发响应

### 4.7 ESLint 收紧基线（2026-04）

当前前端静态规则基线已收紧到以下状态：

**基础设施：**

- 启用 type-aware linting（`projectService` + `allowDefaultProject`），为 TS / Svelte / `.svelte.ts` 三个 block 提供完整类型信息
- 拆分 browser / node globals：`src/` 下使用 `globals.browser + globals.es2025`，根目录配置文件（`eslint.config.js`、`vite.config.ts` 等）使用 `globals.node + globals.es2025`
- Svelte 配置链追加 `flat/prettier`，避免 Svelte stylistic 规则与 Prettier 冲突
- UI 组件库 `src/lib/components/ui/**` 对 `no-unnecessary-condition` 降级为 off（shadcn-svelte 生成代码的 `children?.()` / `value ?? 0` 是标准写法）

**核心规则（全局生效）：**

- `eqeqeq`：`error` — 强制严格相等
- `no-console`：`error` — 禁止 console 调试输出
- `no-var`：`error` — 禁止 var 声明
- `prefer-const`：`error`（`.svelte` / `.svelte.ts` 中由 `svelte/prefer-const` 接管，理解 `$props()` / `$state()` 语义；`.svelte.ts` 额外排除 `$state` rune 以兼容模块级 store 惯用写法）
- `no-array-constructor`：`error` — 禁止 `new Array()`
- `prefer-template`：`error` — 强制模板字符串
- `object-shorthand`：`error` — 强制对象简写
- `no-useless-rename`：`error` — 禁止无意义解构重命名
- `no-useless-computed-key`：`error` — 禁止无意义计算属性键
- `no-useless-concat`：`error` — 禁止无意义字符串拼接
- `no-lonely-if`：`error` — 禁止 else 块中孤立 if
- `prefer-arrow-callback`：`error` — 回调优先箭头函数

**TypeScript 增强（TS / Svelte / .svelte.ts）：**

- `@typescript-eslint/no-unused-vars`：`error`
- `@typescript-eslint/no-explicit-any`：`error`
- `@typescript-eslint/no-non-null-asserted-nullish-coalescing`：`error`
- `@typescript-eslint/no-useless-constructor`：`error`
- `@typescript-eslint/consistent-type-assertions`：`error`
- `@typescript-eslint/consistent-type-imports`：`error`（强制 `import type` 分离，利于 Vite tree-shaking）
- `@typescript-eslint/consistent-generic-constructors`：`error`
- `@typescript-eslint/no-inferrable-types`：`error`
- `@typescript-eslint/prefer-for-of`：`error`
- `@typescript-eslint/array-type`：`error`（统一 `T[]` 风格）

**Type-aware 规则（需要完整类型信息）：**

- `@typescript-eslint/no-floating-promises`：`error` — 禁止未处理的 Promise
- `@typescript-eslint/no-misused-promises`：`error` — 禁止在非 Promise 上下文中误用 Promise
- `@typescript-eslint/await-thenable`：`error` — 禁止 await 非 thenable 值
- `@typescript-eslint/no-unnecessary-condition`：`error` — 禁止类型上不可能的条件分支
- `@typescript-eslint/only-throw-error`：`error` — 只允许 throw Error 对象（替代 `no-throw-literal`，需要类型信息）

**Svelte 专属：**

- `svelte/prefer-const`：`error`（Svelte-aware 的 const 检查，理解 `$props` / `$state` 语义）
- `no-unused-expressions` / `@typescript-eslint/no-unused-expressions`：`error`
- `no-useless-assignment`：`error`
- `no-unsafe-finally`：`error`
- `svelte/no-unused-svelte-ignore`：`error`
- `svelte/no-useless-children-snippet`：`error`
- `svelte/prefer-svelte-reactivity`：`error`（默认启用）
- `svelte/no-at-debug-tags`：`error`
- `svelte/no-inspect`：`error`
- `svelte/button-has-type`：`error`
- `svelte/no-target-blank`：`error`
- `svelte/spaced-html-comment`：`error`
- `svelte/block-lang`：`error`（强制 `<script lang="ts">`）

这意味着：

1. 未使用的导入、局部变量与参数默认必须清理；确实需要保留的占位参数沿用 `_` 前缀约定
2. 展示层与壳层组件中的 motion helper、事件桥接与缓存调用不得再用 `any` 兜底
3. 条件判断必须使用严格相等比较，避免通过 `==` / `!=` 引入隐式类型转换
4. 前端实现不得引入 `console` 调试输出，也不得回退到 `var` 声明
5. `svelte-ignore` 只能在确有必要时保留，且应优先通过改交互或补语义来消除 suppression
6. Svelte `$effect` 中不得用裸表达式做依赖追踪；需要通过解构或显式读取到局部变量来保留依赖语义
7. Svelte TS 中持久化响应式 `Map` / `Set` 状态应使用 `SvelteMap` / `SvelteSet`；仅临时集合可用局部 suppression 标明意图
8. Svelte 文件中的 `finally` 代码块不得出现会覆盖原始控制流的返回、抛错或跳转写法
9. Svelte 组件中的临时赋值与 snippet 组合必须保持必要性，避免保留无效中间赋值或无用子 snippet 包装
10. 数组类型统一使用 `T[]` 风格，不使用 `Array<T>`
11. 所有 Svelte 文件必须声明 `<script lang="ts">`，按钮元素必须显式声明 `type` 属性
12. 不得在生产代码中保留 `@debug` 标签或 `$inspect` 调用
13. 所有 Promise 必须被显式处理（`await` / `.then()` / `void`），不得出现悬浮 Promise
14. 条件分支中不得出现类型上不可能到达的路径；async 竞态守卫等合法场景使用 `eslint-disable-next-line` 并注明原因

**已确认的合法 suppress 场景：**

- `App.svelte` 中 `$effect` 内的 `if (disposed)` 竞态守卫：`await` 后检查闭包变量是否已被外部 cleanup 置位，类型系统无法追踪跨 `await` 的变量变化
- `App.svelte` 中 Tauri `listen()` 的 async 回调：Tauri 运行时接受 async handler，但类型签名声明为 `void` 返回
- `AudioPlayer.svelte` 中 optional `$props()` 回调的 falsy 检查：svelte-eslint-parser 对未提供默认值的 optional props 推断为 always `undefined`
- `lyrics.ts` 中 optional regex capture group 的 `??` 守卫：`RegExpMatchArray` 索引访问类型为 `string`，但 optional group 运行时可为 `undefined`

与本轮收紧直接相关的典型改动包括：

- `AlbumCard.svelte` 改为原生可访问按钮语义，避免依赖 `svelte-ignore` 掩盖 a11y 问题
- 各处 Svelte motion transition helper 统一使用 `MotionTransition`，不再以 `any` 返回
- `cache.ts` / `api.ts` 通过判别联合与类型守卫收口缓存命中分支，减少调用层断言
- `selectors.ts` 修正 `coverUrl` / `coverDeUrl` 的 fallback 顺序（`coverUrl` 为 `string` 非 nullable，`coverDeUrl` 为 `string | null`，应优先取 `coverDeUrl`）
- 多处移除类型上不可能为 null/undefined 的冗余守卫（`artists || []`、`scrollTop ?? 0`、`outputFormat || state.format` 等）

## 5. IPC 规则

**UI 组件禁止直接调用 `invoke` 或 `listen`**。

建立通信网关层：

- 创建领域服务文件（如 `src/lib/api.ts` 或 `features/*/service.ts`）封装 Tauri IPC
- UI 仅绑定服务层暴露的响应式状态或调用服务层暴露的方法
- 日志 viewer 只通过 `listLogRecords()` / `getLogFileStatus()` 读取设置页所需摘要，不新增任意路径读文件能力

### 下载标记与缓存规则

- `getAlbums()` 返回轻量 `Album[]`，并携带专辑级 `download` 保守提示字段
- `getAlbumDetail()` 返回专辑级 `download` 精确聚合字段，`songs[]` 中的曲目级 `download` 仍属于动态数据
- `getSongDetail()` 返回值中的 `download` 字段属于动态数据
- `getAlbumDetail()` 与 `getSongDetail()` 的缓存 key 必须带上 `inventoryVersion`
- 不允许在缓存中长期保留脱离当前 `inventoryVersion` 的 `AlbumDetail` / `SongDetail`
- 收到 `local-inventory-state-changed` 且 `inventoryVersion` 变化后，前端应立即清理专辑详情和歌曲详情相关缓存
- 盘点完成且 `inventoryVersion` 变化后，前端应刷新当前专辑详情，并重新拉取专辑列表，让侧栏专辑 badge 与详情同步

### 日志与运行时错误反馈

- `App.svelte` 统一订阅 `app-error-recorded` 事件，把后端运行时错误的前端安全摘要接入壳层
- 默认反馈方式仍以 toast 为主；若设置面板处于打开状态，同时刷新当前日志 viewer
- `SettingsSheet.svelte` 中的日志区负责两类能力：
  1. 调整 `logLevel`
  2. 浏览 `session` / `persistent` 两类日志摘要
- 日志 viewer 通过 `listLogRecords()` 与 `getLogFileStatus()` 读取数据，不直接读取本地文件
- viewer 只消费 `LogViewerRecord` 前端安全投影，不依赖后端原始 details / cause chain / context
- `logRecords` 按时间倒序展示，`kind` 切换只影响当前查看的日志层，不改变后端记录策略
- toast、inline 错误位、下载面板和日志 viewer 可以并存，但长期追溯应以日志中心为准，不在前端维护第二套“历史错误真相”

### IPC 补充约束

- `app-error-recorded` 只用于运行时错误摘要广播，不承载完整后端日志明细
- UI 不应假设 `LogViewerRecord.details` 一定存在；当前契约下应按摘要 viewer 处理
- 需要用户即时感知的运行时错误走 toast；需要回溯时由设置页日志 viewer 提供入口

## 6. 交互模式

### 顶部工具栏

- 刷新当前缓存与页面数据
- 打开下载任务面板
- 打开设置面板
- 展示活动下载任务数量 badge

设置面板和下载任务面板互斥打开。

### 下载标记消费规则

- 专辑列表可消费 `Album.download` 作为保守提示；当前语义更接近“该专辑在当前 active root 下已发现可关联的本地内容”，不是完整性结论
- 专辑详情页优先消费 `AlbumDetail.download` 作为专辑级精确聚合结果，不从 `songs[]` 再次本地推导
- 专辑详情曲目列表直接消费 `SongEntry.download`
- 当前歌曲详情或播放器关联区直接消费 `SongDetail.download`
- 前端不得自己以下载任务历史或本地临时映射推导“是否已下载”，统一以后端内容接口返回的 `download` 字段为准
- `download.isDownloaded = true` 的最低语义是“当前 active root 下已确认存在本地文件”，不等于“已完成一致性校验”
- `download.downloadStatus = detected` 时，前端展示为“已检测到”
- `download.downloadStatus = verified` 时，前端展示为“已校验”
- `download.downloadStatus = partial` 时，前端展示为“部分下载”
- `download.downloadStatus = unverifiable` 时，前端展示为“不可校验”，但仍属于已下载
- `download.downloadStatus = mismatch` 时，前端应按异常态处理，不应继续展示为普通“已下载”
- 若需要区分“已存在 / 已校验 / 异常”，使用 `download.downloadStatus`，不在前端重复推导或再发明枚举

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

- `cargo build` 在全新 worktree 中会先依赖前端 `dist/` 产物，需要先执行 `bun run build`
- `bun run check:types` 和 `bun run check:build` 在补齐依赖后可通过
- `cargo check` 仍有一个既有 warning：`src-tauri/src/notification/desktop.rs` 中的 `Manager` 未使用

### Latest Verification

通过的命令：

- `bun run check`
- `bun run check:cargo`

### Core Flows

- [ ] 首屏可加载专辑列表
- [ ] 切换专辑后详情刷新正常
- [ ] 曲目列表下载标记正常
- [ ] 切换下载目录后下载标记可重建
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
5. 下载任务 Sheet 已支持基于现有完整快照的关键字搜索、状态筛选、类型筛选和活跃/历史范围筛选；后续若历史规模继续增长，再评估是否把查询、摘要列表和惰性详情下沉到后端

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

- [backend-api-contract.md](../reference/backend-api-contract.md)：后端类型、命令、事件的唯一契约来源
- [backend-completed-phases.md](../history/backend-completed-phases.md)：后端已完成阶段
- [backend-pending-phases.md](../history/backend-pending-phases.md)：后端待办阶段
- [decisions.md](../history/decisions.md)：技术选型决策记录
- [release-process.md](../process/release-process.md)：CI 与发布流程
