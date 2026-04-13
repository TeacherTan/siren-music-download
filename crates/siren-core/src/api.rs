use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://monster-siren.hypergryph.com/api";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Album {
    pub cid: String,
    pub name: String,
    #[serde(rename = "coverUrl")]
    pub cover_url: String,
    #[serde(alias = "artistes")]
    pub artists: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AlbumDetail {
    pub cid: String,
    pub name: String,
    pub intro: Option<String>,
    pub belong: String,
    #[serde(rename = "coverUrl")]
    pub cover_url: String,
    #[serde(rename = "coverDeUrl")]
    pub cover_de_url: Option<String>,
    #[serde(alias = "artistes")]
    pub artists: Option<Vec<String>>,
    pub songs: Vec<SongEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SongEntry {
    pub cid: String,
    pub name: String,
    #[serde(alias = "artistes")]
    pub artists: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SongDetail {
    pub cid: String,
    pub name: String,
    #[serde(rename = "albumCid")]
    pub album_cid: String,
    #[serde(rename = "sourceUrl")]
    pub source_url: String,
    #[serde(rename = "lyricUrl")]
    pub lyric_url: Option<String>,
    #[serde(rename = "mvUrl")]
    pub mv_url: Option<String>,
    #[serde(rename = "mvCoverUrl")]
    pub mv_cover_url: Option<String>,
    pub artists: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    code: i32,
    #[allow(dead_code)]
    msg: String,
    data: T,
}

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
}

impl ApiClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (compatible; siren-music-download)")
            .build()?;
        Ok(Self { client })
    }

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

    /// Download raw bytes from a URL with progress callback (bytes downloaded so far)
    pub async fn download_bytes(
        &self,
        url: &str,
        mut on_progress: impl FnMut(u64, Option<u64>),
    ) -> Result<Vec<u8>> {
        use futures::StreamExt;

        let resp = self.client.get(url).send().await?;
        let total = resp.content_length();
        let mut stream = resp.bytes_stream();
        let mut buf = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buf.extend_from_slice(&chunk);
            on_progress(buf.len() as u64, total);
        }

        Ok(buf)
    }
}
