use serde::{Deserialize, Serialize};

/// 下载服务对外暴露的结构化错误。
///
/// 适用于 Tauri command、测试或上层编排逻辑统一传递下载服务级失败原因；其中
/// `code` 面向机器判定，`message` 面向日志或直接展示。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadServiceError {
    /// 机器可读的稳定错误代码。
    pub code: &'static str,
    /// 面向调用方的错误摘要。
    pub message: String,
}

impl DownloadServiceError {
    /// 构造一个新的下载服务错误。
    ///
    /// 入参 `code` 应使用稳定、可枚举的错误标识；`message` 用于补充当前失败上下文。
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for DownloadServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for DownloadServiceError {}
