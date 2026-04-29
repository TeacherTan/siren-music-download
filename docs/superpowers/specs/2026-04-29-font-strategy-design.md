# 字体统一方案设计

## 背景

当前前端使用纯系统字体栈（`-apple-system, BlinkMacSystemFont, 'SF Pro Display'...`），macOS 渲染效果良好，但 Windows 侧 fallback 到 Arial + 微软雅黑，中英文视觉品质与 macOS 存在差距。作为音乐类桌面应用，希望在所有平台上提供一致的视觉体验。

## 决策

采用 **HarmonyOS Sans SC 全量打包**方案：

- 打包全部 6 个字重（Thin / Light / Regular / Medium / Bold / Black）
- 中英文统一由 HarmonyOS Sans SC 渲染，不再依赖平台系统字体
- 预留 CSS 变量扩展点，支持后续局部字体覆盖

### 选择 HarmonyOS Sans SC 的理由

- 华为开源字体，免费商用（GPL-3.0 仓库托管）
- 现代几何风格，与应用 Apple-like 视觉语言（毛玻璃、圆角、精致阴影）契合
- 中英文字形统一设计，混排协调
- 字重覆盖 Thin 到 Black，满足 UI 中 400-800 的全部需求
- 中文渲染清晰锐利，小字号下可读性好

## 字体资源

### 打包文件

| 文件                            | 字重 | 格式  |
| ------------------------------- | ---- | ----- |
| HarmonyOS-Sans-SC-Thin.woff2    | 100  | woff2 |
| HarmonyOS-Sans-SC-Light.woff2   | 300  | woff2 |
| HarmonyOS-Sans-SC-Regular.woff2 | 400  | woff2 |
| HarmonyOS-Sans-SC-Medium.woff2  | 500  | woff2 |
| HarmonyOS-Sans-SC-Bold.woff2    | 700  | woff2 |
| HarmonyOS-Sans-SC-Black.woff2   | 900  | woff2 |

预估总体积：+18-24MB（woff2 压缩后）。

### 文件位置

```
src/
  assets/
    fonts/
      HarmonyOS-Sans-SC-Thin.woff2
      HarmonyOS-Sans-SC-Light.woff2
      HarmonyOS-Sans-SC-Regular.woff2
      HarmonyOS-Sans-SC-Medium.woff2
      HarmonyOS-Sans-SC-Bold.woff2
      HarmonyOS-Sans-SC-Black.woff2
  lib/
    styles/
      fonts.css          # @font-face 声明 + CSS 变量定义
  app.css                # @import fonts.css + body 字体栈
```

Vite 构建时自动处理 woff2 文件的 content hash 和产物输出。Tauri 打包时这些文件随前端产物一起进入 bundle，不需要额外配置 `resources`。

## CSS 架构

### @font-face 声明

在 `src/lib/styles/fonts.css` 中集中定义，每个字重一条规则：

```css
@font-face {
  font-family: 'HarmonyOS Sans SC';
  font-weight: 400;
  font-style: normal;
  font-display: swap;
  src: url('../../assets/fonts/HarmonyOS-Sans-SC-Regular.woff2') format('woff2');
}
/* ...其余 5 个字重同理 */
```

使用 `font-display: swap` 确保首屏文字不被字体加载阻塞（Tauri 本地加载极快，实际几乎无闪烁）。

### CSS 变量扩展点

```css
:root {
  --font-sans: 'HarmonyOS Sans SC', sans-serif;
  --font-display: var(--font-sans);
  --font-body: var(--font-sans);
  --font-mono: ui-monospace, 'SF Mono', 'Cascadia Code', monospace;
}
```

- `--font-sans`：基础字体栈，全局默认
- `--font-display`：标题 / 大字场景，默认等于 `--font-sans`
- `--font-body`：正文场景，默认等于 `--font-sans`
- `--font-mono`：等宽字体，保留系统字体栈

后续如需局部覆盖，只需在对应组件或区域重新赋值变量：

```css
.some-special-area {
  --font-display: 'Some Other Font', var(--font-sans);
}
```

### body 字体栈

```css
body {
  font-family: var(--font-body);
}
```

中英文统一由 HarmonyOS Sans SC 渲染。唯一的 fallback `sans-serif` 仅作兜底，正常情况下不会触发。

## 字重映射

当前 CSS 中使用的字重与打包字重的对应关系：

| CSS font-weight | 打包字重                                         | 使用场景                   |
| --------------- | ------------------------------------------------ | -------------------------- |
| 400             | Regular                                          | 默认正文                   |
| 500             | Medium                                           | SongRow、AlbumCard         |
| 600             | 介于 Medium (500) 和 Bold (700) 之间，浏览器合成 | SettingsSheet、AudioPlayer |
| 700             | Bold                                             | 标题、badge、强调文本      |
| 800             | 介于 Bold (700) 和 Black (900) 之间，浏览器合成  | AlbumSidebar 单处使用      |

全量打包 6 个字重后，渲染引擎有足够的锚点做插值，600 和 800 的合成效果会比只有 3 个字重时好很多。

## 变更范围

### 新增文件

- `src/assets/fonts/` — 6 个 woff2 字体文件
- `src/lib/styles/fonts.css` — @font-face 声明与 CSS 变量

### 修改文件

- `src/app.css` — 添加 `@import './lib/styles/fonts.css'`，将 body `font-family` 改为 `var(--font-body)`

### 不变

- 各组件中的 `font-weight` 声明保持不变
- 各组件中的 `font-size` 声明保持不变
- Tauri 配置不需要修改
- 构建流程不需要修改

## 许可证

HarmonyOS Sans 由华为开源，免费商用。仓库托管于 [huawei-fonts/HarmonyOS-Sans](https://github.com/huawei-fonts/HarmonyOS-Sans)，采用 GPL-3.0 许可证。字体文件本身的使用不受 GPL 传染性约束（字体嵌入属于"使用"而非"衍生作品"），但建议在应用的关于页面或 LICENSE 文件中注明字体来源。

## 风险与缓解

| 风险                        | 影响                              | 缓解措施                                                          |
| --------------------------- | --------------------------------- | ----------------------------------------------------------------- |
| 安装包体积增长 18-24MB      | 用户下载时间增加                  | Tauri 桌面应用为一次性安装，可接受                                |
| woff2 在 WebView 中加载耗时 | 首屏文字闪烁                      | Tauri 本地文件加载极快（<10ms），实际无感知                       |
| 某些生僻字未覆盖            | 极少数字符 fallback 到 sans-serif | HarmonyOS Sans SC 覆盖 GB18030 常用集，音乐应用场景下几乎不会遇到 |
| GPL-3.0 许可证疑虑          | 法律风险                          | 字体嵌入不构成衍生作品，业界共识；在 LICENSE 中注明来源即可       |
