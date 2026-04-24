# 技术决策记录

> 记录项目关键技术选型的背景、考量与结论。
>
> 相关文档：[FRONTEND_GUIDE.md](FRONTEND_GUIDE.md)

## 决策 1：选择 `shadcn-svelte` 而非黑盒组件库

**背景**：项目需要一个 UI 组件基础层来支撑桌面应用的深度定制需求。

**考量**：

- 传统黑盒组件库（如 MUI、Ant Design）强迫页面长成统一模板风格，不适合需要独特视觉表达的桌面应用
- 项目需要快速补齐 `Sheet / Select / Switch / Progress / Tooltip / Toast` 等通用能力
- 无障碍和交互基础必须完整

**结论**：选择 `shadcn-svelte`，原因是：

1. 它不是黑盒组件库，源码直接复制到项目中，可深度定制
2. 基于 `Bits UI`，无障碍和交互基础较完整
3. 允许保留项目自己的视觉语言，不会强迫统一模板风格

## 决策 2：选择 `Tailwind CSS` 作为主样式层

**背景**：项目原有 `app.css` 超过 2000 行，样式重复较多，难以维护。

**考量**：

- 手写 CSS 在组件拆分后会让局部样式继续依赖超长全局文件
- 需要支持动态主题色和亮暗色模式
- spacing、radius、border、state 等基础样式需要收敛成稳定规则

**结论**：选择 `Tailwind CSS`，原因是：

1. 更适合把 spacing、radius、border、state 收敛成稳定规则
2. 业务组件拆分后，Tailwind 能让局部样式跟组件一起收口
3. 可以与项目级 CSS 变量结合，继续支持动态主题色和亮暗色模式

## 决策 3：设计系统 ≠ 组件库

**背景**：改造初期容易误认为"安装组件库 + 改 CSS"就是设计系统。

**结论**：设计系统不等于组件库。一个成熟设计系统至少包含：

1. 原则和最佳实践
2. foundations（color、spacing、typography、elevation、radius）
3. components
4. patterns
5. content / voice & tone
6. designers kit 或设计资产
7. source code
8. 发布、弃用、贡献和治理机制

这意味着本项目不能只做"安装 shadcn-svelte + 把旧 CSS 改成 Tailwind"，还必须补齐：设计令牌、模式层、内容规范、组件状态管理、文档和治理。

## 决策 4：先拆结构再换样式

**背景**：`App.svelte` 承担了大量业务编排，如果直接在上面堆 Tailwind class，只会让结构更加混乱。

**结论**：先解决 `App.svelte` 的职责堆积，再做分块 UI 重构。否则 Tailwind class 只会堆进旧结构里，无法真正收敛复杂度。

**执行策略**：采用"绞杀者模式"（Strangler Fig），先拿设置面板作为首次实弹演练目标，验证技术栈有效性后再全面铺开。

## 决策 5：组件迁移分类标准

**背景**：不是所有组件都适合直接用 shadcn-svelte 替换。

**结论**：采用 A/B/C 三类分类标准：

| 类别 | 策略                                | 适用条件                                                          | 示例                                                                        |
| ---- | ----------------------------------- | ----------------------------------------------------------------- | --------------------------------------------------------------------------- |
| A 类 | 直接使用 `shadcn-svelte`            | 通用交互、语义清晰、视觉差异不大、无复杂业务耦合                  | Button、Select、Switch、Sheet、Progress、Tooltip、Dialog、Skeleton          |
| B 类 | 基于 `shadcn-svelte` 包一层项目组件 | 底层交互通用、视觉状态较多、项目里重复出现                        | ToolbarIconButton、AppBadge、PanelSection、StatusToast、PlayerControlButton |
| C 类 | 保留定制组件，只消费令牌和原语      | 业务耦合强、状态密度高、布局/动画独特、需要动态主题色或复杂上下文 | AlbumStage、AlbumCard、SongRow、PlayerDock、LyricsFlyout、PlaylistFlyout    |

## 决策 6：UI 组件禁止直接调用 Tauri IPC

**背景**：UI 组件与 Tauri `invoke` / `listen` 强耦合会导致重构期间事件监听丢失、状态同步混乱。

**结论**：建立"通信网关层"（Gateway Layer），具体规则：

- UI 组件中**严禁**直接调用 `invoke` 或 `listen`
- 创建领域服务文件（如 `src/lib/features/player/player-service.ts`），内部封装 `invoke` 和事件监听
- UI 仅绑定服务层暴露的响应式状态

这样在调整 UI 结构时，不用担心丢掉底层的事件监听。
