# HarmonyOS Sans SC 字体统一方案 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将前端字体从平台系统字体栈切换为打包的 HarmonyOS Sans SC（全部 6 个字重），实现跨平台中英文渲染一致性，并预留 CSS 变量扩展点。

**Architecture:** 在 `src/assets/fonts/` 放置 6 个 woff2 字体文件，在 `src/lib/styles/fonts.css` 集中声明 `@font-face` 规则和 CSS 自定义属性，`src/app.css` 通过 `@import` 引入并将 body `font-family` 改为变量引用。Vite 构建自动处理字体文件的 hash 和产物输出，无需修改 Tauri 或 Vite 配置。

**Tech Stack:** Vite (asset handling), CSS @font-face, CSS custom properties, woff2

**Spec:** `docs/superpowers/specs/2026-04-29-font-strategy-design.md`

---

## File Map

| Action | Path                                 | Responsibility                                      |
| ------ | ------------------------------------ | --------------------------------------------------- |
| Create | `src/assets/fonts/*.woff2` (6 files) | HarmonyOS Sans SC 字体资源                          |
| Create | `src/lib/styles/fonts.css`           | @font-face 声明 + CSS 变量定义                      |
| Modify | `src/app.css:1-2, 12-35, 202-206`    | 引入 fonts.css，添加字体变量，替换 body font-family |

---

### Task 1: 获取 HarmonyOS Sans SC 字体文件

**Files:**

- Create: `src/assets/fonts/HarmonyOS-Sans-SC-Thin.woff2`
- Create: `src/assets/fonts/HarmonyOS-Sans-SC-Light.woff2`
- Create: `src/assets/fonts/HarmonyOS-Sans-SC-Regular.woff2`
- Create: `src/assets/fonts/HarmonyOS-Sans-SC-Medium.woff2`
- Create: `src/assets/fonts/HarmonyOS-Sans-SC-Bold.woff2`
- Create: `src/assets/fonts/HarmonyOS-Sans-SC-Black.woff2`

- [ ] **Step 1: 下载 HarmonyOS Sans 字体包**

从华为官方 GitHub 仓库下载字体包：

```bash
cd /tmp
curl -L -o HarmonyOS-Sans.zip https://github.com/huawei-fonts/HarmonyOS-Sans/archive/refs/heads/main.zip
unzip -o HarmonyOS-Sans.zip
ls HarmonyOS-Sans-main/
```

Expected: 解压后可以看到包含 SC 字体的目录结构。

- [ ] **Step 2: 定位 SC 字体 TTF 文件**

```bash
find /tmp/HarmonyOS-Sans-main -name "*SC*" -type f | sort
```

Expected: 找到 HarmonyOS Sans SC 的 Thin / Light / Regular / Medium / Bold / Black 共 6 个 TTF 文件。

- [ ] **Step 3: 转换 TTF 为 woff2**

安装 woff2 转换工具并批量转换：

```bash
# 如果没有 woff2_compress，先安装
brew install woff2 2>/dev/null || pip install fonttools brotli

# 使用 fonttools 转换（更可靠的跨平台方案）
mkdir -p src/assets/fonts

# 对每个 TTF 文件执行转换，具体路径根据 Step 2 的输出调整
for ttf in /tmp/HarmonyOS-Sans-main/<path-to-SC>/*.ttf; do
  name=$(basename "$ttf" .ttf)
  python3 -c "
from fontTools.ttLib import TTFont
font = TTFont('$ttf')
font.flavor = 'woff2'
font.save('src/assets/fonts/${name}.woff2')
font.close()
"
done
```

如果 TTF 文件名与 spec 中的命名不一致，需要重命名为：

- `HarmonyOS-Sans-SC-Thin.woff2`
- `HarmonyOS-Sans-SC-Light.woff2`
- `HarmonyOS-Sans-SC-Regular.woff2`
- `HarmonyOS-Sans-SC-Medium.woff2`
- `HarmonyOS-Sans-SC-Bold.woff2`
- `HarmonyOS-Sans-SC-Black.woff2`

- [ ] **Step 4: 验证字体文件**

```bash
ls -lh src/assets/fonts/*.woff2
```

Expected: 6 个 woff2 文件，每个约 2-5MB。

- [ ] **Step 5: 清理临时文件**

```bash
rm -rf /tmp/HarmonyOS-Sans.zip /tmp/HarmonyOS-Sans-main
```

- [ ] **Step 6: Commit**

```bash
git add src/assets/fonts/
git commit -m "chore: 添加 HarmonyOS Sans SC 全量字体文件（6 字重 woff2）"
```

---

### Task 2: 创建 fonts.css — @font-face 声明与 CSS 变量

**Files:**

- Create: `src/lib/styles/fonts.css`

- [ ] **Step 1: 创建 fonts.css**

创建 `src/lib/styles/fonts.css`，内容如下：

```css
/* HarmonyOS Sans SC — 6 weights, woff2 only */

@font-face {
  font-family: 'HarmonyOS Sans SC';
  font-weight: 100;
  font-style: normal;
  font-display: swap;
  src: url('../../assets/fonts/HarmonyOS-Sans-SC-Thin.woff2') format('woff2');
}

@font-face {
  font-family: 'HarmonyOS Sans SC';
  font-weight: 300;
  font-style: normal;
  font-display: swap;
  src: url('../../assets/fonts/HarmonyOS-Sans-SC-Light.woff2') format('woff2');
}

@font-face {
  font-family: 'HarmonyOS Sans SC';
  font-weight: 400;
  font-style: normal;
  font-display: swap;
  src: url('../../assets/fonts/HarmonyOS-Sans-SC-Regular.woff2') format('woff2');
}

@font-face {
  font-family: 'HarmonyOS Sans SC';
  font-weight: 500;
  font-style: normal;
  font-display: swap;
  src: url('../../assets/fonts/HarmonyOS-Sans-SC-Medium.woff2') format('woff2');
}

@font-face {
  font-family: 'HarmonyOS Sans SC';
  font-weight: 700;
  font-style: normal;
  font-display: swap;
  src: url('../../assets/fonts/HarmonyOS-Sans-SC-Bold.woff2') format('woff2');
}

@font-face {
  font-family: 'HarmonyOS Sans SC';
  font-weight: 900;
  font-style: normal;
  font-display: swap;
  src: url('../../assets/fonts/HarmonyOS-Sans-SC-Black.woff2') format('woff2');
}
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/styles/fonts.css
git commit -m "feat: 添加 HarmonyOS Sans SC @font-face 声明"
```

---

### Task 3: 修改 app.css — 引入字体并替换 font-family

**Files:**

- Modify: `src/app.css:1-2` (添加 @import)
- Modify: `src/app.css:12-35` (添加 CSS 变量到 :root)
- Modify: `src/app.css:202-206` (替换 body font-family)

- [ ] **Step 1: 在 app.css 顶部添加 fonts.css 导入**

在 `src/app.css` 的第 2 行（`@import 'tw-animate-css';` 之后）插入：

```css
@import './lib/styles/fonts.css';
```

修改后文件顶部应为：

```css
@import 'tailwindcss';
@import 'tw-animate-css';
@import './lib/styles/fonts.css';

@custom-variant dark (&:is(.dark *));
```

- [ ] **Step 2: 在 :root 中添加字体 CSS 变量**

在 `src/app.css` 的 `:root` 块中，`--ease-linear: linear;` 之后（约第 36 行），添加字体变量：

```css
--font-sans: 'HarmonyOS Sans SC', sans-serif;
--font-display: var(--font-sans);
--font-body: var(--font-sans);
--font-mono: ui-monospace, 'SF Mono', 'Cascadia Code', monospace;
```

- [ ] **Step 3: 替换 body font-family**

将 `src/app.css` 中 body 的 font-family 从：

```css
body {
  font-family:
    -apple-system, BlinkMacSystemFont, 'SF Pro Display', 'SF Pro Text',
    'Helvetica Neue', Arial, sans-serif;
```

改为：

```css
body {
  font-family: var(--font-body);
```

- [ ] **Step 4: Commit**

```bash
git add src/app.css
git commit -m "feat: 切换全局字体为 HarmonyOS Sans SC，添加字体 CSS 变量扩展点"
```

---

### Task 4: 构建验证

- [ ] **Step 1: 运行前端构建**

```bash
bun run build
```

Expected: 构建成功，无错误。woff2 文件应出现在 `dist/assets/` 产物中。

- [ ] **Step 2: 验证产物中包含字体文件**

```bash
find dist/assets -name "*.woff2" | sort
```

Expected: 6 个带 content hash 的 woff2 文件。

- [ ] **Step 3: 运行完整检查**

```bash
bun run check
```

Expected: 格式、lint、类型、Svelte 检查、构建、cargo check 全部通过。

- [ ] **Step 4: 启动开发服务器并目视验证**

```bash
bun run tauri:dev
```

验证项：

- 中文文字使用 HarmonyOS Sans SC 渲染（DevTools → Computed → font-family 确认）
- 英文文字同样使用 HarmonyOS Sans SC 渲染
- 各字重（400/500/600/700/800）视觉区分正常
- 毛玻璃、圆角等 UI 元素与新字体视觉协调
- 无 FOUT（Flash of Unstyled Text）或字体加载闪烁
