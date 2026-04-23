pub mod backend;
pub mod controller;
pub mod events;
pub mod media;
pub mod state;
pub mod stream;

/// 播放控制器与前后端共享的播放队列上下文类型。
pub use controller::{AudioPlayer, PlaybackContext, PlaybackQueueEntry};
/// 提供给前端与媒体会话同步使用的播放器状态快照。
pub use state::PlayerState;
