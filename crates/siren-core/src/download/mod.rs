//! 下载领域模型、错误与服务层公共模块。
//!
//! 该模块聚合下载错误类型、前后端共享的下载快照模型、任务规划器、状态管理服务与
//! 工作器逻辑，是下载子系统在 `siren_core` 中的统一领域入口。

pub mod error;
pub mod model;
pub mod planner;
pub mod service;
pub mod worker;
