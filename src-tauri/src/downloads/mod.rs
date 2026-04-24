//! 下载桥接层公共模块。
//!
//! 该模块聚合下载执行循环与事件发射相关子模块，主要用于把 `siren_core` 的下载服务
//! 状态接入 Tauri 宿主层，并向前端广播下载管理器事件。

pub mod bridge;
pub mod events;
