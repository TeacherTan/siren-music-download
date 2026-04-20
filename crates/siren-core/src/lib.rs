//! 塞壬音乐下载器共享核心库。
//!
//! 这个 crate 提供 Tauri 桌面端复用的、与平台无关的后端能力：
//!
//! - [`api`]：对 Monster Siren 公开 HTTP 接口的强类型访问。
//! - [`audio`]：音频格式识别、文件写入和 FLAC 标签处理工具。
//! - [`downloader`]：带进度回调的高层下载编排逻辑。
//!
//! # 生成 rustdoc
//!
//! ```bash
//! cargo doc -p siren_core --no-deps
//! ```

pub mod api;
pub mod audio;
pub mod download;
pub mod downloader;
pub mod local_inventory;

// 重新导出公共 API，便于上层直接使用
pub use api::{Album, AlbumDetail, ApiClient, SongDetail, SongEntry};
pub use audio::{save_audio, tag_flac, AudioFormat, OutputFormat};
pub use download::model::{
    CreateDownloadJobRequest, DownloadErrorCode, DownloadErrorInfo, DownloadJobKind,
    DownloadJobSnapshot, DownloadJobStatus, DownloadManagerSnapshot, DownloadOptions,
    DownloadTaskProgressEvent, DownloadTaskSnapshot, DownloadTaskStatus,
};
pub use download::service::DownloadService;
pub use downloader::{
    album_cover_exists, album_output_dir, download_album_cover, download_song,
    download_song_phase1, write_album_cover_bytes, write_payload_to_disk, DownloadProgress,
    MetaOverride, OwnedFlacMetadata, WritePayload,
};
pub use local_inventory::{
    aggregate_album_badge, badge_for_detected_file, badge_for_status, candidate_relative_paths,
    empty_album_badge, has_detected_track, missing_track_badge, AlbumDownloadBadge,
    LocalInventoryScanProgressEvent, LocalInventorySnapshot, LocalInventoryStatus,
    LocalTrackDownloadStatus, TrackDownloadBadge, VerificationMode,
};
