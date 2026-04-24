<!-- markdownlint-disable -->

<div align="center">

# 塞壬音乐下载器

<div>
  <img alt="platform" src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows-4c8bf5">
  <img alt="license" src="https://img.shields.io/github/license/Anselyuki/siren-music-download">
  <img alt="stars" src="https://img.shields.io/github/stars/Anselyuki/siren-music-download?style=social">
</div>

面向 [塞壬唱片](https://monster-siren.hypergryph.com/) 的桌面音乐播放器与下载器。  
把专辑浏览、在线播放、整专下载、歌词和下载管理整合进同一个桌面应用里。

[下载发布版](https://github.com/Anselyuki/siren-music-download/releases) | [功能亮点](#功能亮点) | [使用方式](#使用方式) | [本地开发](#本地开发) | [开发文档](./doc/FRONTEND_GUIDE.md) | [后端契约](./doc/BACKEND_API_CONTRACT.md) | [Release 流程](./doc/RELEASE_PROCESS.md)

</div>

<!-- markdownlint-restore -->

## 下载与安装

- 推荐直接从 [GitHub Releases](https://github.com/Anselyuki/siren-music-download/releases) 下载对应系统的发布文件。
- 应用目前面向 `macOS` 和 `Windows` 提供桌面端体验。
- Release 文件名中，`macos_intel` 对应 Intel Mac，`macos_apple_silicon` 对应 Apple Silicon Mac。
- Windows 发布版为依赖系统 `WebView2` 运行时的精简 `.exe`，不会额外提供安装型打包。
- 首次启动需要联网拉取专辑、歌词和音频资源。

## 功能亮点

- 专辑浏览与在线播放：启动后即可加载专辑列表，点选曲目直接播放。
- 单曲与整专下载：既可以下载当前歌曲，也可以一键创建整张专辑的下载任务。
- 完整播放器控制：支持暂停、继续、拖动进度、上一首、下一首、乱序和循环模式切换。
- 歌词与播放队列：底部播放器可展开歌词面板和当前播放列表。
- 独立下载面板：可以查看任务进度，并支持关键字搜索、状态/类型筛选、活跃/历史范围筛选、取消、重试和清理历史。
- 下载结果更完整：支持 `WAV`、`FLAC`、`MP3`，可选保存同名 `.lrc` 歌词，`FLAC` 会写入封面和基础元数据。
- 更贴近桌面应用体验：支持系统通知，并会根据专辑封面生成动态主题色。
- 本地下载标记：专辑列表、专辑详情和歌曲详情会基于当前下载目录显示已下载状态，并区分“已检测到 / 已校验 / 部分下载 / 不可校验 / 校验异常”。
- 日志与诊断：设置面板可查看本次运行日志与持久化日志，并通过日志等级控制退出时的持久化阈值。

## 使用方式

1. 启动应用后等待专辑列表加载完成。
2. 在左侧选择专辑，在主区域查看曲目和简介。
3. 点击曲目即可开始播放，也可以在曲目行直接触发下载。
4. 在专辑页使用下载入口创建整专下载任务。
5. 通过右上角设置面板调整下载目录、输出格式、歌词保存、通知开关和持久化日志等级。
6. 如需排查运行时问题，可在设置面板的“日志与诊断”区域查看本次运行日志或持久化日志。
7. 下载完成或切换下载目录后，应用会自动重扫当前下载目录并刷新本地下载标记。
8. 通过下载任务面板查看进度，并按需搜索、筛选、取消、重试或清理历史记录。

## 本地开发

### 环境要求

- Rust
- Bun 1.3+（唯一 JS 包管理器）

### 常用命令

仓库前端依赖统一使用 `Bun` 管理，`bun.lock` 是唯一 JS 锁文件，需要提交到仓库。

```bash
# 安装依赖与启动开发
bun install
bun run tauri:dev
```

```bash
# 格式化与检查
bun run format              # 格式化前端代码与 Markdown 文档
bun run format:check        # 检查格式是否符合规范
bun run lint                # 运行前端 ESLint 与 Rust fmt 检查
bun run check               # 运行格式、lint、类型、前端构建与 Rust workspace 检查
cargo fmt --all             # 格式化 Rust 代码
cargo test --workspace      # 运行 Rust 工作区测试
```

```bash
# 构建
bun run build
bun run tauri:build
cargo check --workspace
cargo test --workspace
```

文档相关命令（按需执行）：

```bash
cargo doc -p siren_core --no-deps
cargo doc -p siren-music-download --lib --no-deps --document-private-items
cargo doc -p siren-music-download --bin siren-music-download --no-deps --document-private-items
```

### 代码规范

- **前端代码与 Markdown 文档**：使用 Prettier 统一格式化
- **前端静态规则检查**：使用 ESLint
- **Rust 代码格式化**：使用 `cargo fmt --all`

开发相关文档：

- [前端开发指南](./doc/FRONTEND_GUIDE.md)
- [后端 API 契约](./doc/BACKEND_API_CONTRACT.md)
- [后端已完成阶段](./doc/BACKEND_COMPLETED_PHASES.md)
- [后端待办阶段](./doc/BACKEND_PENDING_PHASES.md)
- [Release 流程](./doc/RELEASE_PROCESS.md)

发布约定与版本标记方式见 [Release 流程](./doc/RELEASE_PROCESS.md)。

## 说明

- 项目依赖塞壬唱片公开接口与公开资源；若上游接口或资源地址变化，应用也需要同步调整。
- 本项目为桌面端体验整合与学习项目，与塞壬唱片或鹰角网络无官方隶属关系。
- 如果你在使用中遇到问题或有改进建议，欢迎提交 [Issue](https://github.com/Anselyuki/siren-music-download/issues) 或 Pull Request。

## 许可证

本项目基于 [MIT](./LICENSE) 许可证开源。
