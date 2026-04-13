# 界面设计说明

## 组件结构

应用采用三栏布局，外加底部播放器栏：

```
┌─────────────┬─────────────────────┬──────────────┐
│   专辑列表   │      歌曲列表        │   下载设置   │
│  (sidebar)  │      (content)      │  (details)   │
│             │                     │              │
│  AlbumCard  │  SongRow × N        │  格式选择    │
│    × N      │  (可勾选、可播放)    │  目录选择    │
│             │                     │  下载按钮    │
├─────────────┴─────────────────────┴──────────────┤
│              播放器栏 (AudioPlayer)               │
│  当前曲目 · 进度条 · 停止按钮                      │
└──────────────────────────────────────────────────┘
```

**组件层级**：
- `App.svelte`：主布局容器，管理全局状态
- `AlbumCard.svelte`：专辑封面 + 名称，点击选中
- `SongRow.svelte`：曲目行（序号、勾选框、名称、艺术家、播放按钮）
- `AudioPlayer.svelte`：底部播放器栏（曲目信息、进度条、停止按钮）

## 亮暗主题自适应

应用自动适配系统的亮色/暗色模式，无需用户手动切换。

### CSS 变量系统

```css
@media (prefers-color-scheme: light) {
  :root {
    --bg-primary: #ffffff;
    --bg-secondary: #f5f5f7;
    --bg-tertiary: #e8e8ed;
    --text-primary: #1d1d1f;
    --text-secondary: #6e6e73;
    --accent: #fa2d48;
  }
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg-primary: #000000;
    --bg-secondary: #1c1c1e;
    --bg-tertiary: #2c2c2e;
    --text-primary: #ffffff;
    --text-secondary: #8e8e93;
    --accent: #fa2d48;
  }
}
```

### 平滑过渡

所有背景色和文本色都有 0.3 秒的平滑过渡动画：

```css
body {
  transition:
    background-color 0.3s ease,
    color 0.3s ease;
}
```

### 设计原则

1. **WCAG 对比度标准**：所有文本和背景组合都满足 WCAG AA 级别对比度要求
2. **层次分明**：通过背景色深浅区分不同区域（sidebar 最深、content 中等、details 最浅）
3. **一致性**：亮色和暗色模式保持相同的布局和交互
4. **无障碍**：复选框、按钮等元素都支持键盘导航

## 状态流

前端状态通过 Svelte 5 runes 管理，播放状态通过 Tauri events 从 Rust 后端同步：

```
Rust 后端（src-tauri/src/player）
  │
  │ emit('player-state-changed', PlayerState)
  │ emit('player-progress', PlayerState)
  │
  └───────────────────────────────────────▶
                                          │
前端（src/App.svelte）                     │
  listen('player-state-changed') ────────▶ currentSong, isPlaying
  listen('player-progress')      ────────▶ progress, duration
```

## 测试方法

### macOS

```bash
系统设置 → 外观 → 自动/浅色/深色
```

### Windows

```bash
设置 → 个性化 → 颜色 → 选择默认应用模式
```

应用会立即响应系统主题变化，无需重启。