use crate::local_inventory::{AlbumDownloadBadge, TrackDownloadBadge};
use anyhow::Result;
use lru::LruCache;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

const DEFAULT_BASE_URL: &str = "https://monster-siren.hypergryph.com/api";
const DEFAULT_CACHE_CAPACITY: usize = 100;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Album {
    pub cid: String,
    pub name: String,
    #[serde(rename = "coverUrl")]
    pub cover_url: String,
    #[serde(alias = "artistes")]
    pub artists: Vec<String>,
    #[serde(default)]
    pub download: AlbumDownloadBadge,
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
    #[serde(default)]
    pub download: AlbumDownloadBadge,
    pub songs: Vec<SongEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SongEntry {
    pub cid: String,
    pub name: String,
    #[serde(alias = "artistes")]
    pub artists: Vec<String>,
    #[serde(default)]
    pub download: TrackDownloadBadge,
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

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    response_cache: Arc<Mutex<LruCache<String, Vec<u8>>>>,
}

impl ApiClient {
    pub fn new() -> Result<Self> {
        Self::new_with_config(DEFAULT_BASE_URL.to_string(), DEFAULT_CACHE_CAPACITY)
    }

    fn new_with_config(base_url: String, capacity: usize) -> Result<Self> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (compatible; siren-music-download)")
            .build()?;
        let capacity = NonZeroUsize::new(capacity).expect("cache capacity must be non-zero");
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            response_cache: Arc::new(Mutex::new(LruCache::new(capacity))),
        })
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    fn cache_key(method: &str, resource: &str) -> String {
        format!("{:x}", md5::compute(format!("{method}:{resource}")))
    }

    fn read_cached_bytes(&self, cache_key: &str) -> Option<Vec<u8>> {
        self.response_cache
            .lock()
            .ok()
            .and_then(|mut cache| cache.get(cache_key).cloned())
    }

    fn write_cached_bytes(&self, cache_key: String, bytes: &[u8]) {
        if let Ok(mut cache) = self.response_cache.lock() {
            cache.put(cache_key, bytes.to_vec());
        }
    }

    async fn fetch_response_bytes(&self, url: &str, accept_json: bool) -> Result<Vec<u8>> {
        let mut request = self.client.get(url);
        if accept_json {
            request = request.header("Accept", "application/json");
        }

        let response = request.send().await?.error_for_status()?;
        Ok(response.bytes().await?.to_vec())
    }

    async fn fetch_streamed_bytes(
        &self,
        url: &str,
        mut on_progress: impl FnMut(u64, Option<u64>),
    ) -> Result<Vec<u8>> {
        use futures::StreamExt;

        let response = self.client.get(url).send().await?.error_for_status()?;
        let total = response.content_length();
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::new();
        let mut downloaded = 0_u64;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded += chunk.len() as u64;
            bytes.extend_from_slice(&chunk);
            on_progress(downloaded, total);
        }

        Ok(bytes)
    }

    fn decode_api_response<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
        let response: ApiResponse<T> = serde_json::from_slice(bytes)?;
        anyhow::ensure!(response.code == 0, "API error code {}", response.code);
        Ok(response.data)
    }

    async fn get_cached_api_data<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let cache_key = Self::cache_key("GET", path);
        if let Some(bytes) = self.read_cached_bytes(&cache_key) {
            return Self::decode_api_response(&bytes);
        }

        let bytes = self.fetch_response_bytes(&self.api_url(path), true).await?;
        let data = Self::decode_api_response(&bytes)?;
        self.write_cached_bytes(cache_key, &bytes);
        Ok(data)
    }

    pub fn clear_response_cache(&self) {
        if let Ok(mut cache) = self.response_cache.lock() {
            cache.clear();
        }
    }

    pub async fn get_albums(&self) -> Result<Vec<Album>> {
        self.get_cached_api_data("albums").await
    }

    pub async fn get_album_detail(&self, album_cid: &str) -> Result<AlbumDetail> {
        self.get_cached_api_data(&format!("album/{album_cid}/detail"))
            .await
    }

    pub async fn get_song_detail(&self, cid: &str) -> Result<SongDetail> {
        self.get_cached_api_data(&format!("song/{cid}")).await
    }

    pub async fn download_stream(
        &self,
        url: &str,
        mut on_chunk: impl FnMut(&[u8], u64, Option<u64>) -> Result<bool>,
    ) -> Result<()> {
        use futures::StreamExt;

        let resp = self.client.get(url).send().await?.error_for_status()?;
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

    pub async fn download_bytes(
        &self,
        url: &str,
        mut on_progress: impl FnMut(u64, Option<u64>),
    ) -> Result<Vec<u8>> {
        let cache_key = Self::cache_key("GET", url);
        if let Some(bytes) = self.read_cached_bytes(&cache_key) {
            on_progress(bytes.len() as u64, Some(bytes.len() as u64));
            return Ok(bytes);
        }

        let bytes = self.fetch_streamed_bytes(url, on_progress).await?;
        self.write_cached_bytes(cache_key, &bytes);
        Ok(bytes)
    }

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
    use httpmock::prelude::*;

    impl ApiClient {
        fn new_for_test(base_url: String, capacity: usize) -> anyhow::Result<Self> {
            Self::new_with_config(base_url, capacity)
        }
    }

    fn album_detail_body(cid: &str, name: &str) -> String {
        format!(
            r#"{{"code":0,"msg":"ok","data":{{"cid":"{cid}","name":"{name}","intro":null,"belong":"EP","coverUrl":"https://example.com/{cid}.jpg","coverDeUrl":null,"artistes":["Test Artist"],"songs":[]}}}}"#
        )
    }

    #[tokio::test]
    async fn returns_cached_album_detail_without_second_network_call() {
        let server = MockServer::start();
        let album_mock = server.mock(|when, then| {
            when.method(GET).path("/api/album/alpha/detail");
            then.status(200)
                .header("content-type", "application/json")
                .body(album_detail_body("alpha", "Alpha"));
        });

        let client =
            ApiClient::new_for_test(format!("{}/api", server.base_url()), 100).expect("client");

        let first = client.get_album_detail("alpha").await.expect("first call");
        let second = client.get_album_detail("alpha").await.expect("second call");

        assert_eq!(first.cid, "alpha");
        assert_eq!(second.name, "Alpha");
        album_mock.assert_hits(1);
    }

    #[tokio::test]
    async fn does_not_cache_failed_upstream_response() {
        let server = MockServer::start();
        let failure_mock = server.mock(|when, then| {
            when.method(GET).path("/api/album/beta/detail");
            then.status(500);
        });

        let client =
            ApiClient::new_for_test(format!("{}/api", server.base_url()), 100).expect("client");

        assert!(client.get_album_detail("beta").await.is_err());
        assert!(client.get_album_detail("beta").await.is_err());
        failure_mock.assert_hits(2);
    }

    #[tokio::test]
    async fn evicts_least_recently_used_response_when_capacity_is_exceeded() {
        let server = MockServer::start();
        let alpha_mock = server.mock(|when, then| {
            when.method(GET).path("/api/album/alpha/detail");
            then.status(200)
                .header("content-type", "application/json")
                .body(album_detail_body("alpha", "Alpha"));
        });
        let beta_mock = server.mock(|when, then| {
            when.method(GET).path("/api/album/beta/detail");
            then.status(200)
                .header("content-type", "application/json")
                .body(album_detail_body("beta", "Beta"));
        });

        let client =
            ApiClient::new_for_test(format!("{}/api", server.base_url()), 1).expect("client");

        client.get_album_detail("alpha").await.expect("alpha first");
        client.get_album_detail("beta").await.expect("beta first");
        client
            .get_album_detail("alpha")
            .await
            .expect("alpha second");

        alpha_mock.assert_hits(2);
        beta_mock.assert_hits(1);
    }

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
