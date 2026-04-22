# 后端已完成阶段

> 本文档记录已经交付的后端阶段与已落地的基础能力。
>
> 未完成或未来阶段参见 [BACKEND_PENDING_PHASES.md](BACKEND_PENDING_PHASES.md)。
>
> 共享类型、命令、事件和状态机规则以 [BACKEND_API_CONTRACT.md](BACKEND_API_CONTRACT.md) 为唯一事实来源。

## 当前总览

- **Phase 1–7 已完成**
- **Phase 8 已完成**
- **Phase 9 已完成**
- **Phase 10 已完成**
- Phase 8 当前已包含：结构化本地证据、`verified` / `mismatch` / `partial` / `unverifiable` 的实际产出、下载链路 provenance 记录、下载后自动重扫、`inventoryVersion` 驱动的前端缓存失效与状态展示
- 当前待办已切换为 **Phase 11**

## 已完成阶段

### Phase 1：下载任务领域模型

- DownloadService 与下载任务领域模型
- 单曲任务化
- 基础 commands / events

### Phase 2：整专下载与进度联动

- 整专下载
- 专辑封面落盘
- 下载进度事件推送
- 前端总进度展示
- 专辑页批量下载入口
- 重复创建保护

### Phase 3：任务控制与错误建模

- 任务取消
- 任务重试
- 历史清理
- 结构化错误码与详情
- 独立下载面板 UI

### Phase 4：系统通知集成

- 下载完成通知
- 播放切换通知
- 通知权限检查
- 测试通知

### Phase 5：批量选择管理 UI

- 全选
- 清空
- 反选按钮

### Phase 6：流水线下载优化

- download / write 两阶段流水线
- 整专下载吞吐提升

### Phase 7：统一偏好系统

- `AppPreferences` 统一偏好模型
- `preferences.toml` 持久化
- 导入 / 导出偏好
- 通知偏好收敛到统一偏好系统

### Phase 8：本地已下载盘点、校验链与下载标记

- active `outputDir` 扫描
- `SongEntry` / `SongDetail.download` enrich
- 盘点快照 / 重扫 / 取消命令
- 盘点状态 / 进度事件
- `outputDir` 变化后自动重扫
- 结构化本地证据模型（相对路径 / 文件大小 / mtime / 候选 checksum / 命中规则 / 专辑目录标记 / verification state）
- `verified` / `mismatch` / `partial` / `unverifiable` 的实际状态产出
- `Album.download` 列表级保守提示字段
- 下载写盘成功后的 provenance 记录与自动 inventory 刷新
- `inventoryVersion` 驱动的专辑详情 / 歌曲详情缓存失效
- 前端专辑列表、专辑详情、曲目行的下载状态展示

### Phase 9：缓存替换方案

- 前端缓存重写为分类型分层缓存（albums / songs / lyrics / themes / covers）
- albums / songs / lyrics 支持 IndexedDB 持久化与启动预热
- 支持按 key / tag 失效，并纳入 `inventoryVersion` 驱动的失效链
- 提供前端缓存 hit / miss / eviction 统计
- `siren-core` `ApiClient` 增加 100 条 LRU 响应缓存
- 增加 `clear_response_cache` 命令，支持手动刷新时同步清理后端响应缓存
- 音频缓存增加 2GB 软上限与后台按 mtime 淘汰
- 通知封面缓存清理改为异步后台执行，不阻塞主流程

### Phase 10：下载 session 持久化

- 下载 job / task 快照与 manager 元数据落盘到版本化 JSON 文件
- 应用启动时自动加载下载历史，并恢复到内存态 `DownloadService`
- 上一 session 中处于 `queued / preparing / downloading / writing / running` 的任务统一修正为可见终态，不自动续传
- 下载任务创建、状态跃迁、重试、取消、历史清理后都会触发持久化写盘
- 历史持久化写入使用原子替换，避免中途写坏状态文件
- 增加终态历史保留策略，限制状态文件增长
- 状态文件损坏或 schema 不兼容时会记录日志并回退为空历史，不阻塞启动

## 已落地基础能力补充

- 统一日志中心
- session / persistent 双层日志
- 运行时错误安全事件
- 设置页日志 viewer
