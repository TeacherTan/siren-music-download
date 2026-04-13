# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概览

**技术栈：Rust + Tauri 2 + Vite + Svelte 5**
- 跨平台桌面应用（macOS、Windows、Linux）
- 前端：Vite + Svelte 5 + TypeScript
- 后端：Rust（Cargo workspace 结构）
- 状态：浏览和播放功能已完成，下载功能开发中

## 常用命令

```bash
# 安装前端依赖
npm install

# 开发模式（Vite dev server + Tauri 应用）
npm run tauri dev

# 仅构建前端
npm run build

# 生产构建（打包桌面应用）
npm run tauri build

# Rust 编译检查（workspace）
cargo check

# Rust 代码检查
cargo clippy

# Rust 格式化
cargo fmt
```

## 架构

### Tauri 2 标准架构

```
Cargo workspace（根 Cargo.toml）
├── src-tauri/               # Tauri 应用 crate
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   └── src/
│       ├── main.rs          # Tauri 入口 + 命令处理
│       └── player/          # 播放器模块
└── crates/
    └── siren-core/          # 共享 Rust 核心库
        ├── Cargo.toml
        └── src/
            ├── lib.rs       # 库入口
            ├── api.rs       # API 客户端
            ├── audio.rs     # 音频处理
            └── downloader.rs # 下载逻辑
```

**前端（根目录）**：
- `index.html` → `src/main.ts` → `src/App.svelte`
- `src/lib/api.ts`：Tauri 命令桥接
- `src/lib/types.ts`：TypeScript 类型定义
- `src/lib/components/`：UI 组件

### 核心库数据流

```
前端 invoke('get_albums')
  → Tauri 命令处理（src-tauri/src/main.rs）
  → ApiClient::get_albums()（crates/siren-core/src/api.rs）
  → reqwest 异步请求
  → JSON 反序列化
  → 返回 Vec<Album> 给前端
```

### 各模块职责

**共享核心库（crates/siren-core）**：

| 模块 | 职责 |
|---|---|
| `lib.rs` | 库入口，导出公共 API |
| `api.rs` | 塞壬唱片 API 的类型化 HTTP 客户端。所有响应格式为 `{"code":0,"msg":"","data":{}}` |
| `audio.rs` | 魔数字节格式检测、`save_audio()`、纯 Rust FLAC 编码（`flacenc`）、用 `metaflac` 写入 FLAC 元数据 |
| `downloader.rs` | `download_song()` / `download_album()`：串联 API 调用、字节流传输和音频保存，并提供进度回调 |

**Tauri 应用（src-tauri）**：

| 模块 | 职责 |
|---|---|
| `src/main.rs` | Tauri 应用入口，定义命令、设置窗口、处理前端调用 |
| `src/player/mod.rs` | 播放器模块入口 |
| `src/player/controller.rs` | `AudioPlayer` 控制器，管理播放状态、进度、停止 |
| `src/player/state.rs` | `PlayerState` 结构体（通过 Tauri event 同步到前端） |
| `src/player/events.rs` | `player-state-changed` / `player-progress` 事件发射 |
| `src/player/decode.rs` | 音频解码（WAV 用 hound，FLAC/MP3 用 rodio） |
| `src/player/backend/mod.rs` | `PlaybackBackend` trait，平台后端选择 |
| `src/player/backend/coreaudio.rs` | macOS CoreAudio 后端（Hog Mode、独占采样率） |
| `src/player/backend/cpal.rs` | Windows/Linux cpal 后端 |

### 前端技术

- **UI 框架**：Svelte 5 + TypeScript（ runes 状态语法 `$state`）
- **布局**：三栏布局（专辑列表 | 歌曲列表 | 下载设置）
- **样式**：亮/暗主题自适应（`prefers-color-scheme`）
- **API 调用**：`@tauri-apps/api` `invoke()` + 事件监听 `listen()`

### Tauri 命令列表

| 命令 | 说明 |
|---|---|
| `get_albums` | 获取专辑列表 |
| `get_album_detail` | 获取专辑详情 |
| `get_song_detail` | 获取单曲详情 |
| `play_song` | 播放指定曲目 |
| `stop_playback` | 停止播放 |
| `get_player_state` | 获取当前播放状态 |
| `get_default_output_dir` | 获取默认下载目录 |

### 音频处理流程

塞壬唱片 API 通过 `song.source_url` 返回 **WAV**（无损 PCM）格式音频。三种输出模式：

- **WAV**：直接写入磁盘，无需转换
- **FLAC**：使用纯 Rust `flacenc` 库编码，无需外部依赖
- **MP3**：部分曲目 API 直接返回 MP3，直接写入

FLAC 编码流程：
1. 读取 WAV 音频数据到内存
2. 使用 `flacenc::source::MemSource` 包装样本数据
3. 使用 `flacenc::encode_with_fixed_block_size()` 编码为 FLAC
4. 使用 `metaflac` 写入标题、艺术家、专辑名及封面图片元数据

WAV 格式不写入标签。

输出路径格式：`<输出目录>/<专辑名（已过滤非法字符）>/<曲目名（已过滤非法字符）>.<扩展名>`

## 实现状态

### 已完成
- ✅ Tauri 2 框架集成（标准 Cargo workspace 结构）
- ✅ 专辑列表加载和显示
- ✅ 歌曲列表显示
- ✅ 前后端通信（Tauri commands + events）
- ✅ UI 界面（三栏布局，亮/暗自适应）
- ✅ 共享核心库集成
- ✅ 在线播放（macOS CoreAudio 优先，Windows/Linux cpal）

### 待实现
- ⏳ 下载功能（调用 `download_song`）
- ⏳ 下载进度实时更新
- ⏳ 错误处理和用户提示
- ⏳ 文件选择器集成