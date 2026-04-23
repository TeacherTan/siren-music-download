use serde::{Deserialize, Serialize};

/// 搜索关键字允许的最大长度。
pub const SEARCH_LIBRARY_QUERY_MAX_LENGTH: usize = 128;
/// 默认返回条数。
pub const SEARCH_LIBRARY_DEFAULT_LIMIT: usize = 20;
/// 单次请求允许的最大返回条数。
pub const SEARCH_LIBRARY_MAX_LIMIT: usize = 50;
/// 默认分页偏移量。
pub const SEARCH_LIBRARY_DEFAULT_OFFSET: usize = 0;
/// 允许的最大分页偏移量。
pub const SEARCH_LIBRARY_MAX_OFFSET: usize = 500;

/// 库内搜索的范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibrarySearchScope {
    /// 同时搜索专辑与歌曲。
    All,
    /// 仅搜索专辑。
    Albums,
    /// 仅搜索歌曲。
    Songs,
}

/// 搜索命中的字段类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibrarySearchHitField {
    /// 命中标题字段。
    Title,
    /// 命中艺术家字段。
    Artist,
    /// 命中简介字段。
    Intro,
    /// 命中归属字段。
    Belong,
}

/// 本地搜索索引的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibraryIndexState {
    /// 索引尚未建立。
    NotReady,
    /// 索引正在构建或重建。
    Building,
    /// 索引可用但已过期，结果可能滞后于当前磁盘状态。
    Stale,
    /// 索引已准备完成，可直接用于搜索。
    Ready,
}

/// 搜索结果的实体类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchLibraryResultKind {
    /// 专辑结果。
    Album,
    /// 歌曲结果。
    Song,
}

/// 库内搜索请求参数。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchLibraryRequest {
    /// 用户输入的搜索关键字。
    pub query: String,
    /// 本次搜索限定的范围。
    pub scope: LibrarySearchScope,
    /// 本次请求希望返回的最大条数；未提供时由服务端使用默认值。
    pub limit: Option<usize>,
    /// 分页偏移量；未提供时由服务端使用默认值。
    pub offset: Option<usize>,
}

/// 单条库内搜索结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchLibraryResultItem {
    /// 当前结果对应的实体类型。
    pub kind: SearchLibraryResultKind,
    /// 所属专辑 CID。
    pub album_cid: String,
    /// 命中歌曲的 CID；专辑结果时为空。
    pub song_cid: Option<String>,
    /// 专辑标题。
    pub album_title: String,
    /// 歌曲标题；专辑结果时为空。
    pub song_title: Option<String>,
    /// 展示用艺术家行。
    pub artist_line: Option<String>,
    /// 当前结果命中的字段集合。
    pub matched_fields: Vec<LibrarySearchHitField>,
}

/// 库内搜索响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchLibraryResponse {
    /// 当前页结果项。
    pub items: Vec<SearchLibraryResultItem>,
    /// 满足条件的总结果数。
    pub total: usize,
    /// 回显查询关键字。
    pub query: String,
    /// 回显查询范围。
    pub scope: LibrarySearchScope,
    /// 返回该结果时的索引状态。
    pub index_state: LibraryIndexState,
}

impl SearchLibraryResponse {
    /// 构造一个不含结果项的搜索响应。
    ///
    /// # 示例
    ///
    /// ```
    /// use siren_core::{LibraryIndexState, LibrarySearchScope, SearchLibraryResponse};
    ///
    /// let response = SearchLibraryResponse::empty(
    ///     "ep",
    ///     LibrarySearchScope::Albums,
    ///     LibraryIndexState::Ready,
    /// );
    ///
    /// assert!(response.items.is_empty());
    /// assert_eq!(response.total, 0);
    /// assert_eq!(response.query, "ep");
    /// ```
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
