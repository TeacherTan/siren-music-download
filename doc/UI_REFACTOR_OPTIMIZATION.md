# UI 改造方案优化与补充建议

> 文档日期：2026-04-17
>
> 关联文档：[UI_REFACTOR_PLAN.md](./UI_REFACTOR_PLAN.md)

在详细审阅了《UI 改造方案》后，该方案在架构目标、技术选型和设计系统分层上已经非常完善。为了确保方案在实际落地（特别是基于 Svelte 5 和 Tauri 的上下文中）更加顺畅，降低实施风险，特提出以下优化与补充建议。

---

## 1. 深入契合 Svelte 5 范式

原方案未明确提及 Svelte 5 的新特性。既然项目基于 Svelte 5，UI 的重构和状态的拆分必须全面拥抱 Svelte 5 的 Runes 范式，这将极大地影响组件的设计方式。

### 1.1 状态驱动：从 Store 转向响应式状态 (Runes)
- **优化点**：放弃使用 Svelte 4 的 `writable` / `readable` store。在拆分 `App.svelte` 业务状态时，应使用独立的 `.ts` 或 `.svelte.ts` 文件，利用 `$state()` 和 `$derived()` 构建全局或领域响应式单例。
- **收益**：避免繁琐的 `$` 前缀订阅逻辑，状态可以在组件外部无缝传递和变更，更利于 `player`、`download`、`library` 领域的逻辑抽离。

### 1.2 组件复合：广泛使用 Snippet
- **优化点**：在设计“业务复合组件层”（如 `AlbumCard`、`PlayerDock`）时，使用 Svelte 5 的 `{#snippet}` 替代传统的 `<slot>`。
- **收益**：shadcn-svelte 的某些底层原语可以更好地与业务解耦，将局部 UI 片段作为参数传递，使 UI 代码更加紧凑和声明式。

---

## 2. 细化 Tauri 与前端通信的解耦策略

原方案提到“保留当前业务能力和后端接口”，但在重构期间，巨大的风险在于 UI 组件与 Tauri IPC 事件（`listen`）和命令（`invoke`）的强耦合。

### 2.1 隔离 Tauri IPC
- **优化建议**：在 Phase 3（结构拆分）之前，强制建立一个“通信网关层”（Gateway Layer）。
- **具体做法**：
  - UI 组件中**严禁**直接调用 `invoke` 或 `listen`。
  - 创建类似 `src/features/player/player-service.ts` 的服务，内部封装 `invoke`，并监听 Tauri 的 `player-state-changed` 等事件，然后将数据同步到 Svelte 5 的 `$state` 中。
  - UI 仅仅绑定这些 `$state`。这样在调整 UI 结构时，完全不用担心丢掉底层的事件监听。

---

## 3. 实施路径优化：引入“绞杀者模式” (Strangler Fig)

原方案的 Phase 3（拆分 3000 多行的 App.svelte）和 Phase 4（优先改造壳层和通用面板）步子仍然有些大，容易造成长时间的系统不可用。

### 3.1 调整改造顺序：从“设置面板”单点突破
- **优化建议**：在进行大规模拆分前，先拿**设置面板 (Settings Sheet)** 作为“小白鼠”进行全栈改造。
- **具体路径**：
  1. 在 `App.svelte` 不做大改的前提下，使用 shadcn-svelte 和 Tailwind 重写一个全新的 `<SettingsSheetNew>`。
  2. 验证 Tailwind 配置、设计令牌（Tokens）注入、shadcn 组件可访问性以及 Tauri 配置持久化通信在新组件中是否工作完美。
  3. 切换入口，废弃旧设置面板。
  4. 验证成功后，再全面铺开去拆分 `App.svelte` 的核心主区。
- **收益**：快速验证技术栈（Phase 1 & 2）的有效性，尽早暴露构建配置或样式冲突问题，而不影响核心的播放和下载业务逻辑。

---

## 4. Tailwind 与动态主题融合的工程细节

方案提到了“保留动态主题色”和“Tailwind 负责结构”，但在具体工程配置上需要明确规则，避免开发时混乱。

### 4.1 CSS 变量与 Tailwind Config 的映射映射
需要在 `tailwind.config.ts` 中明确定义 CSS 变量的映射，而不是在类名中使用任意值 (arbitrary values like `bg-[var(--album-color)]`)。

```typescript
// tailwind.config.ts 建议配置示例
module.exports = {
  theme: {
    extend: {
      colors: {
        // 映射shadcn的基础色
        border: "hsl(var(--border))",
        background: "hsl(var(--background))",
        // 明确映射项目的动态提取色
        album: {
          DEFAULT: "hsl(var(--album-primary))",
          muted: "hsl(var(--album-muted))",
          foreground: "hsl(var(--album-foreground))",
        }
      }
    }
  }
}
```

### 4.2 避免玻璃材质滥用
为确保落实“Apple 化风格边界”，建议在 Tailwind 配置中直接封装一个特定的 `glass` 插件或 utility class（例如 `.bg-glass-panel`），固化透明度、模糊度和轻微边框的高光参数，**禁止**开发者在模板中随意组合 `backdrop-blur-md bg-white/30`，从工程上限制材质表现。

---

## 5. 补充测试与回归保障策略 (Testing Strategy)

原方案的“验收标准”多为感官和人工验证。面对如此大规模的 UI 与结构双重重构，完全依赖人工验证极易出现回归（Regression）。

### 5.1 引入视觉/DOM 防退化测试
- **优化建议**：在重构核心组件（如 `PlayerDock`、`DownloadTasksSheet`）之前，补充轻量级的组件级自动化测试（如 Vitest + Svelte Testing Library）。
- **执行方式**：
  - 重构前，为旧组件的关键交互（如点击播放、拖动进度、展开面板）编写测试用例。
  - 替换为 shadcn/Tailwind 新组件后，确保同一套测试用例仍然能够绿灯通过。
  - 对于强视觉组件，可评估是否在 CI 中引入局部的视觉回归测试（Visual Regression Testing, 如 Playwright）。

---

## 总结优化执行 CheckList 补充：

在正式开始 Phase 1 之前，请确保：
- [ ] 确立 Svelte 5 `$state` 领域模型文件的存放规范。
- [ ] `tailwind.config.ts` 雏形已设计好包含动态色（album colors）的映射字典。
- [ ] 挑选出一个非核心的独立功能区（如设置面板或关于页面）作为新栈的首次实弹演练目标。