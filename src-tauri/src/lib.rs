//! Siren Music Download 后端库。
//!
//! 这个 library crate 暴露后端模块的最小公共边界，供 binary target 和未来的
//! integration tests 使用。
//!
//! # 设计原则
//!
//! - 只暴露 `main.rs` 启动和 Tauri command 注册所需的最小入口
//! - 内部实现细节（helper、normalization、worker pipeline）保持私有
//! - 不为测试便利扩大可见性

mod app_state;
mod audio_cache;
pub mod commands;
mod download_session;
mod downloads;
mod local_inventory;
mod local_inventory_provenance;
mod logging;
mod notification;
mod player;
mod preferences;
mod search;
mod theme;

pub use app_state::AppState;
pub use downloads::bridge::initialize as initialize_download_bridge;
pub use local_inventory::spawn_inventory_scan;
pub use logging::{LogLevel, LogPayload};
