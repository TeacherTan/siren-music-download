use serde::{Deserialize, Serialize};

use crate::api::Album;

/// 按系列分组的专辑集合，用于首页"按系列浏览"区块。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesGroup {
    pub series: String,
    pub albums: Vec<Album>,
}

/// 收听历史条目，对应 SQLite `listening_history` 表的一行。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    pub song_cid: String,
    pub song_name: String,
    pub album_cid: String,
    pub album_name: String,
    pub cover_url: Option<String>,
    pub artists: Vec<String>,
    pub played_at: String,
}

/// 收听事件，由 `play_song` 内部自动构造并写入 SQLite。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListeningEvent {
    pub song_cid: String,
    pub song_name: String,
    pub album_cid: String,
    pub album_name: String,
    pub cover_url: Option<String>,
    pub artists: Vec<String>,
}

/// 首页状态仪表盘聚合数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomepageStatus {
    pub platform_album_count: u32,
    pub platform_song_count: u32,
    pub local_downloaded_count: u32,
    pub local_storage_bytes: u64,
    pub active_download_count: u32,
    pub completed_download_count: u32,
}
