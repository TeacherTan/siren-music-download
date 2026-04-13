# 塞壬音乐下载器

基于 [Rust](https://www.rust-lang.org/) + [Tauri 2](https://tauri.app/) 构建的跨平台无损音乐下载工具，支持从[塞壬唱片官网](https://monster-siren.hypergryph.com/)下载全部专辑及曲目。

## 功能

- ✅ 浏览全部专辑和曲目列表
- ✅ 在线播放试听（macOS CoreAudio 优先，Windows/Linux 使用 cpal）
- ⏳ 按专辑批量下载，或勾选单曲下载（开发中）
- ✅ 支持 WAV（无损直出）、FLAC（无损转换）、MP3 输出格式
- ✅ FLAC 格式自动写入标题、艺术家、专辑名及封面图片元数据
- ⏳ 实时显示下载进度（开发中）
- ✅ 跨平台：macOS、Windows、Linux

## 截图

> _(开发中，后续补充)_

## 依赖

| 工具       | 说明                                             |
| ---------- | ------------------------------------------------ |
| Rust 1.70+ | 编译环境，通过 [rustup](https://rustup.rs/) 安装 |
| Node 18+   | 前端构建环境                                     |

**注意**：
- FLAC 编码使用纯 Rust `flacenc` 库，**无需安装 ffmpeg**
- WAV 和 MP3 格式无需任何外部依赖

## 构建与运行

```bash
# 克隆仓库
git clone https://github.com/yourname/siren-music-download
cd siren-music-download

# 安装前端依赖
npm install

# 开发模式（启动 Vite dev server + Tauri 应用）
npm run tauri dev

# 仅构建前端
npm run build

# 生产构建（打包桌面应用）
npm run tauri build
# macOS 产物位于 src-tauri/target/release/bundle/
```

## 使用方式

1. 启动后自动加载专辑列表
2. 点击左侧专辑，中间显示曲目列表
3. 点击曲目可在线试听播放
4. 勾选要下载的曲目（支持"全选 / 全不选"）
5. 右侧下拉菜单选择输出格式（WAV / FLAC / MP3）
6. 点击 **选择目录** 选择输出位置，默认为 `~/Downloads/SirenMusic/`
7. 点击 **下载选中曲目** 开始下载（功能开发中）

## 输出目录结构

```
SirenMusic/
└── <专辑名>/
    ├── <曲目名>.wav   # WAV 模式
    ├── <曲目名>.flac  # FLAC 模式（含元数据）
    └── <曲目名>.mp3   # MP3 模式（部分曲目）
```

## 项目结构

项目采用 Tauri 2 标准架构，前端和后端分离：

```
.
├── Cargo.toml                 # Cargo workspace 配置
├── package.json               # 前端 Vite + Svelte 包配置
├── index.html                 # 前端入口
├── vite.config.ts             # Vite 配置
├── src/                       # Svelte 前端源码
│   ├── main.ts                # 前端启动入口
│   ├── App.svelte             # 主应用组件
│   ├── app.css                # 全局样式（亮/暗自适应）
│   └── lib/
│       ├── api.ts             # Tauri 命令桥接
│       ├── types.ts           # TypeScript 类型定义
│       └── components/        # UI 组件
├── src-tauri/                 # Tauri 应用 crate
│   ├── Cargo.toml             # Tauri 包配置
│   ├── tauri.conf.json        # Tauri 应用配置
│   ├── capabilities/          # 权限声明
│   └── src/
│       ├── main.rs            # Tauri 入口 + 命令处理
│       └── player/            # 播放器模块（macOS/跨平台）
└── crates/
    └── siren-core/            # 共享 Rust 核心库
        ├── Cargo.toml
        └── src/
            ├── lib.rs         # 库入口
            ├── api.rs         # 塞壬唱片 API 客户端
            ├── audio.rs       # 音频处理、FLAC 编码、元数据
            └── downloader.rs  # 异步下载逻辑（含进度回调）
```

**核心库（siren_core）**：
- API 客户端（reqwest + serde）
- 音频处理（纯 Rust FLAC 编码，无需外部工具）
- 异步下载逻辑（tokio + futures）

**Tauri 应用**：
- 后端：Rust 命令处理（`get_albums`, `get_album_detail`, `get_song_detail`, `play_song`, `stop_playback`, `get_player_state`, `get_default_output_dir`）
- 前端：Svelte 5 + TypeScript，通过 `@tauri-apps/api` 与后端通信

## API 说明

塞壬唱片提供公开 REST API，无需鉴权：

| 接口                         | 说明                           |
| ---------------------------- | ------------------------------ |
| `GET /api/albums`            | 获取全部专辑列表               |
| `GET /api/album/:cid/detail` | 获取专辑详情及曲目列表         |
| `GET /api/song/:cid`         | 获取单曲详情（含 `sourceUrl`） |

`sourceUrl` 为 CDN 上的音频直链（WAV 或 MP3 格式）。

## 实现状态

- ✅ 跨平台 Tauri UI
- ✅ 专辑列表显示和选择
- ✅ 歌曲列表显示和选择
- ✅ 异步数据加载
- ✅ 响应式界面（亮/暗主题自适应）
- ✅ 在线播放（macOS CoreAudio，Windows/Linux cpal）
- ⏳ 下载功能（开发中）
- ⏳ 下载进度实时更新（开发中）
- ⏳ 错误处理和用户提示（开发中）

## 许可证

MIT