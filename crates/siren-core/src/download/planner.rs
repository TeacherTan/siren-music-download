//! 下载规划器。
//!
//! 该模块负责描述下载批次在进入执行器前的规划阶段。当前实现将任务拆分逻辑
//! 保持在 `service.rs` 中，因此这里只保留最小占位类型。

/// 下载规划阶段的占位类型。
///
/// 当前下载任务的拆分与调度准备仍由 `service.rs` 直接完成；保留该类型是为了给
/// 未来更复杂的预规划逻辑预留统一入口。
pub struct DownloadPlan;

impl DownloadPlan {
    /// 返回当前规划是否为空。
    pub fn is_empty(&self) -> bool {
        true // No additional planning beyond what service.rs does
    }
}
