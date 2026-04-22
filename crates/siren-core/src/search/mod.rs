use serde::{Deserialize, Serialize};

pub const SEARCH_LIBRARY_QUERY_MAX_LENGTH: usize = 128;
pub const SEARCH_LIBRARY_DEFAULT_LIMIT: usize = 20;
pub const SEARCH_LIBRARY_MAX_LIMIT: usize = 50;
pub const SEARCH_LIBRARY_DEFAULT_OFFSET: usize = 0;
pub const SEARCH_LIBRARY_MAX_OFFSET: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibrarySearchScope {
    All,
    Albums,
    Songs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibrarySearchHitField {
    Title,
    Artist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibraryIndexState {
    NotReady,
    Building,
    Stale,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchLibraryResultKind {
    Album,
    Song,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchLibraryRequest {
    pub query: String,
    pub scope: LibrarySearchScope,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchLibraryResultItem {
    pub kind: SearchLibraryResultKind,
    pub album_cid: String,
    pub song_cid: Option<String>,
    pub album_title: String,
    pub song_title: Option<String>,
    pub artist_line: Option<String>,
    pub matched_fields: Vec<LibrarySearchHitField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchLibraryResponse {
    pub items: Vec<SearchLibraryResultItem>,
    pub total: usize,
    pub query: String,
    pub scope: LibrarySearchScope,
    pub index_state: LibraryIndexState,
}

impl SearchLibraryResponse {
    pub fn empty(
        query: impl Into<String>,
        scope: LibrarySearchScope,
        index_state: LibraryIndexState,
    ) -> Self {
        Self {
            items: Vec::new(),
            total: 0,
            query: query.into(),
            scope,
            index_state,
        }
    }
}
