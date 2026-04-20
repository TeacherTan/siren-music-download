use crate::local_inventory::TrackDownloadBadge;
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://monster-siren.hypergryph.com/api";

/// `GET /api/albums` 返回的专辑摘要。
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Album {
    /// 供详情接口使用的稳定专辑标识。
    pub cid: String,
    /// 专辑显示名称。
    pub name: String,
    /// 封面图地址。
    #[serde(rename = "coverUrl")]
    pub cover_url: String,
    /// 上游接口返回的专辑艺术家列表。
    #[serde(alias = "artistes")]
    pub artists: Vec<String>,
}

/// `GET /api/album/{cid}/detail` 返回的专辑详情。
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AlbumDetail {
    /// 稳定专辑标识。
    pub cid: String,
    /// 专辑显示名称。
    pub name: String,
    /// 可选的专辑简介文本。
    pub intro: Option<String>,
    /// 上游接口中的归属字段，例如系列名或分类名。
    pub belong: String,
    /// 默认封面图地址。
    #[serde(rename = "coverUrl")]
    pub cover_url: String,
    /// 备用封面图地址，通常是更大图或去标版本。
    #[serde(rename = "coverDeUrl")]
    pub cover_de_url: Option<String>,
    /// 上游响应中携带的专辑艺术家列表。
    #[serde(alias = "artistes")]
    pub artists: Option<Vec<String>>,
    /// 该专辑包含的歌曲列表。
    pub songs: Vec<SongEntry>,
}

/// [`AlbumDetail`] 中内嵌的歌曲摘要。
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SongEntry {
    /// 稳定歌曲标识。
    pub cid: String,
    /// 歌曲显示名称。
    pub name: String,
    /// 歌曲艺术家列表。
    #[serde(alias = "artistes")]
    pub artists: Vec<String>,
    /// 当前 active outputDir 下的本地下载标记。
    #[serde(default)]
    pub download: TrackDownloadBadge,
}

/// `GET /api/song/{cid}` 返回的歌曲详情。
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SongDetail {
    /// 稳定歌曲标识。
    pub cid: String,
    /// 歌曲显示名称。
    pub name: String,
    /// 所属专辑的标识。
    #[serde(rename = "albumCid")]
    pub album_cid: String,
    /// 用于播放和下载的音频地址。
    #[serde(rename = "sourceUrl")]
    pub source_url: String,
    /// 可选歌词文件地址。
    #[serde(rename = "lyricUrl")]
    pub lyric_url: Option<String>,
    /// 可选 MV 地址。
    #[serde(rename = "mvUrl")]
    pub mv_url: Option<String>,
    /// 可选 MV 封面地址。
    #[serde(rename = "mvCoverUrl")]
    pub mv_cover_url: Option<String>,
    /// 歌曲艺术家列表。
    pub artists: Vec<String>,
    /// 当前 active outputDir 下的本地下载标记。
    #[serde(default)]
    pub download: TrackDownloadBadge,
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    code: i32,
    #[allow(dead_code)]
    msg: String,
    data: T,
}

/// 面向 Monster Siren 公开接口的强类型 HTTP 客户端。
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
}

impl ApiClient {
    /// 创建一个带有上游服务期望 `User-Agent` 的客户端。
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (compatible; siren-music-download)")
            .build()?;
        Ok(Self { client })
    }

    /// 获取 `GET /api/albums` 返回的完整专辑列表。
    pub async fn get_albums(&self) -> Result<Vec<Album>> {
        let resp: ApiResponse<Vec<Album>> = self
            .client
            .get(format!("{BASE_URL}/albums"))
            .header("Accept", "application/json")
            .send()
            .await?
            .json()
            .await?;

        anyhow::ensure!(resp.code == 0, "API error code {}", resp.code);
        Ok(resp.data)
    }

    /// 获取单个专辑及其内嵌歌曲列表，对应
    /// `GET /api/album/{album_cid}/detail`。
    pub async fn get_album_detail(&self, album_cid: &str) -> Result<AlbumDetail> {
        let resp: ApiResponse<AlbumDetail> = self
            .client
            .get(format!("{BASE_URL}/album/{album_cid}/detail"))
            .header("Accept", "application/json")
            .send()
            .await?
            .json()
            .await?;

        anyhow::ensure!(resp.code == 0, "API error code {}", resp.code);
        Ok(resp.data)
    }

    /// 获取单首歌曲详情，对应 `GET /api/song/{cid}`。
    pub async fn get_song_detail(&self, cid: &str) -> Result<SongDetail> {
        let resp: ApiResponse<SongDetail> = self
            .client
            .get(format!("{BASE_URL}/song/{cid}"))
            .header("Accept", "application/json")
            .send()
            .await?
            .json()
            .await?;

        anyhow::ensure!(resp.code == 0, "API error code {}", resp.code);
        Ok(resp.data)
    }

    /// 从 `url` 按块流式下载原始字节。
    ///
    /// 回调会收到每个数据块、当前累计下载字节数，以及可选的总长度。
    /// 如果返回 `Ok(false)`，会提前停止读取。
    pub async fn download_stream(
        &self,
        url: &str,
        mut on_chunk: impl FnMut(&[u8], u64, Option<u64>) -> Result<bool>,
    ) -> Result<()> {
        use futures::StreamExt;

        let resp = self.client.get(url).send().await?;
        let total = resp.content_length();
        let mut stream = resp.bytes_stream();
        let mut downloaded = 0_u64;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded += chunk.len() as u64;
            if !on_chunk(&chunk, downloaded, total)? {
                break;
            }
        }

        Ok(())
    }

    /// 把 `url` 的全部内容下载到内存中。
    ///
    /// 回调会在每个数据块结束后收到当前累计下载字节数和可选总长度。
    pub async fn download_bytes(
        &self,
        url: &str,
        mut on_progress: impl FnMut(u64, Option<u64>),
    ) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.download_stream(url, |chunk, downloaded, total| {
            buf.extend_from_slice(chunk);
            on_progress(downloaded, total);
            Ok(true)
        })
        .await?;
        Ok(buf)
    }

    /// 从 `url` 下载文本内容。
    pub async fn download_text(&self, url: &str) -> Result<String> {
        let bytes = self.download_bytes(url, |_, _| {}).await?;
        Ok(String::from_utf8_lossy(&bytes)
            .trim_start_matches('\u{feff}')
            .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::ApiClient;

    #[tokio::test]
    #[ignore = "hits the live Monster Siren API"]
    async fn downloads_real_lyrics_text() {
        let client = ApiClient::new().expect("client");
        let detail = client.get_song_detail("048760").await.expect("song detail");
        let lyric_url = detail.lyric_url.expect("lyric url");
        let lyric_text = client.download_text(&lyric_url).await.expect("lyric text");

        assert!(
            !lyric_text.trim().is_empty(),
            "lyric text should not be empty"
        );
        assert!(
            lyric_text.contains("[00:"),
            "lyric text should contain timestamped LRC lines"
        );
    }
}
