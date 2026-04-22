use crate::search::snapshot::{inventory_index_dir, LibrarySearchSnapshot};
use anyhow::{Context, Result};
use siren_core::{
    LibrarySearchHitField, LibrarySearchScope, SearchLibraryRequest, SearchLibraryResultItem,
    SearchLibraryResultKind,
};
use std::cmp::Ordering;
use std::path::Path;
use tantivy::collector::{Count, TopDocs};
use tantivy::query::{BooleanQuery, Occur, Query, QueryParser, TermQuery};
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TantivyDocument, TextFieldIndexing, TextOptions, Value,
    STORED, STRING,
};
use tantivy::tokenizer::{LowerCaser, NgramTokenizer, TextAnalyzer};
use tantivy::{doc, Index, IndexReader, ReloadPolicy, Term};

const SEARCH_TOKENIZER_NAME: &str = "siren_ngram";

#[derive(Clone)]
pub(crate) struct LibrarySearchIndex {
    index: Index,
    reader: IndexReader,
    fields: LibrarySearchFields,
}

#[derive(Clone, Copy)]
struct LibrarySearchFields {
    kind: Field,
    album_cid: Field,
    song_cid: Field,
    album_title: Field,
    song_title: Field,
    artist_line: Field,
}

#[derive(Clone)]
pub(crate) struct SanitizedSearchRequest {
    pub query: String,
    pub scope: LibrarySearchScope,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug)]
struct ScoredSearchItem {
    score: f32,
    item: SearchLibraryResultItem,
}

impl LibrarySearchIndex {
    pub(crate) fn build(base_dir: &Path, snapshot: &LibrarySearchSnapshot) -> Result<Self> {
        let index_dir = inventory_index_dir(base_dir, &snapshot.inventory_version);
        if index_dir.exists() {
            std::fs::remove_dir_all(&index_dir)
                .with_context(|| format!("failed to clean {}", index_dir.display()))?;
        }
        std::fs::create_dir_all(&index_dir)
            .with_context(|| format!("failed to create {}", index_dir.display()))?;

        let (schema, fields) = build_schema();
        let index = Index::create_in_dir(&index_dir, schema)?;
        register_tokenizers(&index)?;

        let mut writer = index.writer(50_000_000)?;

        for album in &snapshot.albums {
            let _ = writer.add_document(doc!(
                fields.kind => "album",
                fields.album_cid => album.album_cid.clone(),
                fields.song_cid => "",
                fields.album_title => album.album_title.clone(),
                fields.song_title => "",
                fields.artist_line => album.artist_line.clone().unwrap_or_default(),
            ));
        }

        for song in &snapshot.songs {
            let _ = writer.add_document(doc!(
                fields.kind => "song",
                fields.album_cid => song.album_cid.clone(),
                fields.song_cid => song.song_cid.clone(),
                fields.album_title => song.album_title.clone(),
                fields.song_title => song.song_title.clone(),
                fields.artist_line => song.artist_line.clone().unwrap_or_default(),
            ));
        }

        writer.commit()?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        reader.reload()?;

        Ok(Self {
            index,
            reader,
            fields,
        })
    }

    pub(crate) fn open(base_dir: &Path, inventory_version: &str) -> Result<Self> {
        let index_dir = inventory_index_dir(base_dir, inventory_version);
        let index = Index::open_in_dir(&index_dir)
            .with_context(|| format!("failed to open {}", index_dir.display()))?;
        register_tokenizers(&index)?;
        let fields = load_fields(index.schema())?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        reader.reload()?;
        Ok(Self {
            index,
            reader,
            fields,
        })
    }

    pub(crate) fn search(
        &self,
        request: &SanitizedSearchRequest,
    ) -> Result<(Vec<SearchLibraryResultItem>, usize)> {
        let searcher = self.reader.searcher();
        let query = self.build_query(request)?;
        let fetch_limit = request
            .limit
            .saturating_add(request.offset)
            .max(request.limit);
        let (top_docs, total) =
            searcher.search(query.as_ref(), &(TopDocs::with_limit(fetch_limit), Count))?;

        let normalized_query = normalize_query_text(&request.query);
        let mut items = top_docs
            .into_iter()
            .map(|(score, address)| {
                let document = searcher.doc::<TantivyDocument>(address)?;
                Ok(ScoredSearchItem {
                    score,
                    item: self.document_to_result_item(&document, &normalized_query),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        items.sort_by(compare_scored_items);

        let paged_items = items
            .into_iter()
            .skip(request.offset)
            .take(request.limit)
            .map(|item| item.item)
            .collect::<Vec<_>>();

        Ok((paged_items, total))
    }

    fn build_query(&self, request: &SanitizedSearchRequest) -> Result<Box<dyn Query>> {
        let query_fields = vec![
            self.fields.album_title,
            self.fields.song_title,
            self.fields.artist_line,
        ];
        let mut parser = QueryParser::for_index(&self.index, query_fields);
        parser.set_field_boost(self.fields.album_title, 3.0);
        parser.set_field_boost(self.fields.song_title, 3.0);
        parser.set_field_boost(self.fields.artist_line, 1.5);

        let escaped_query = escape_query_text(&request.query);
        let parsed_query = parser.parse_query(&escaped_query)?;
        let Some(kind_value) = scope_kind_value(request.scope) else {
            return Ok(parsed_query);
        };

        let kind_filter: Box<dyn Query> = Box::new(TermQuery::new(
            Term::from_field_text(self.fields.kind, kind_value),
            IndexRecordOption::Basic,
        ));

        Ok(Box::new(BooleanQuery::new(vec![
            (Occur::Must, parsed_query),
            (Occur::Must, kind_filter),
        ])))
    }

    fn document_to_result_item(
        &self,
        document: &TantivyDocument,
        normalized_query: &str,
    ) -> SearchLibraryResultItem {
        let kind = field_text(document, self.fields.kind);
        let album_title = field_text(document, self.fields.album_title);
        let song_title = field_text(document, self.fields.song_title);
        let artist_line = field_text(document, self.fields.artist_line);

        SearchLibraryResultItem {
            kind: if kind == "song" {
                SearchLibraryResultKind::Song
            } else {
                SearchLibraryResultKind::Album
            },
            album_cid: field_text(document, self.fields.album_cid),
            song_cid: empty_to_none(field_text(document, self.fields.song_cid)),
            album_title: album_title.clone(),
            song_title: empty_to_none(song_title.clone()),
            artist_line: empty_to_none(artist_line.clone()),
            matched_fields: collect_matched_fields(
                normalized_query,
                &album_title,
                &song_title,
                &artist_line,
            ),
        }
    }
}

pub(crate) fn sanitize_search_request(
    request: SearchLibraryRequest,
    max_limit: usize,
    max_offset: usize,
) -> Result<SanitizedSearchRequest> {
    let query = request.query.trim().to_string();
    if query.is_empty() {
        anyhow::bail!("搜索关键词不能为空");
    }
    if query.chars().count() > siren_core::SEARCH_LIBRARY_QUERY_MAX_LENGTH {
        anyhow::bail!("搜索关键词长度不能超过 128 个字符");
    }

    Ok(SanitizedSearchRequest {
        query,
        scope: request.scope,
        limit: request
            .limit
            .unwrap_or(siren_core::SEARCH_LIBRARY_DEFAULT_LIMIT)
            .min(max_limit),
        offset: request
            .offset
            .unwrap_or(siren_core::SEARCH_LIBRARY_DEFAULT_OFFSET)
            .min(max_offset),
    })
}

fn build_schema() -> (Schema, LibrarySearchFields) {
    let mut builder = Schema::builder();
    let text_options = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(SEARCH_TOKENIZER_NAME)
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();

    let fields = LibrarySearchFields {
        kind: builder.add_text_field("kind", STRING | STORED),
        album_cid: builder.add_text_field("album_cid", STRING | STORED),
        song_cid: builder.add_text_field("song_cid", STRING | STORED),
        album_title: builder.add_text_field("album_title", text_options.clone()),
        song_title: builder.add_text_field("song_title", text_options.clone()),
        artist_line: builder.add_text_field("artist_line", text_options),
    };

    (builder.build(), fields)
}

fn load_fields(schema: Schema) -> Result<LibrarySearchFields> {
    Ok(LibrarySearchFields {
        kind: schema
            .get_field("kind")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        album_cid: schema
            .get_field("album_cid")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        song_cid: schema
            .get_field("song_cid")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        album_title: schema
            .get_field("album_title")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        song_title: schema
            .get_field("song_title")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
        artist_line: schema
            .get_field("artist_line")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?,
    })
}

fn register_tokenizers(index: &Index) -> Result<()> {
    let tokenizer = TextAnalyzer::builder(NgramTokenizer::new(1, 3, false)?)
        .filter(LowerCaser)
        .build();
    index
        .tokenizers()
        .register(SEARCH_TOKENIZER_NAME, tokenizer);
    Ok(())
}

fn field_text(document: &TantivyDocument, field: Field) -> String {
    document
        .get_first(field)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

fn empty_to_none(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn normalize_query_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn escape_query_text(value: &str) -> String {
    const RESERVED: [char; 16] = [
        '\\', '+', '-', '&', '|', '!', '(', ')', '{', '}', '[', ']', '^', '"', '~', ':',
    ];

    value
        .chars()
        .flat_map(|character| {
            if RESERVED.contains(&character) {
                vec!['\\', character]
            } else {
                vec![character]
            }
        })
        .collect()
}

fn contains_normalized_field(field_value: &str, normalized_query: &str) -> bool {
    if normalized_query.is_empty() {
        return false;
    }

    normalize_query_text(field_value).contains(normalized_query)
}

fn collect_matched_fields(
    normalized_query: &str,
    album_title: &str,
    song_title: &str,
    artist_line: &str,
) -> Vec<LibrarySearchHitField> {
    let mut matched_fields = Vec::new();
    if contains_normalized_field(album_title, normalized_query)
        || contains_normalized_field(song_title, normalized_query)
    {
        matched_fields.push(LibrarySearchHitField::Title);
    }
    if contains_normalized_field(artist_line, normalized_query) {
        matched_fields.push(LibrarySearchHitField::Artist);
    }
    matched_fields
}

fn compare_scored_items(left: &ScoredSearchItem, right: &ScoredSearchItem) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| compare_result_items(&left.item, &right.item))
}

fn compare_result_items(
    left: &SearchLibraryResultItem,
    right: &SearchLibraryResultItem,
) -> Ordering {
    let left_title = left.song_title.as_ref().unwrap_or(&left.album_title);
    let right_title = right.song_title.as_ref().unwrap_or(&right.album_title);
    left_title
        .cmp(right_title)
        .then_with(|| left.album_title.cmp(&right.album_title))
        .then_with(|| left.album_cid.cmp(&right.album_cid))
        .then_with(|| left.song_cid.cmp(&right.song_cid))
}

fn scope_kind_value(scope: LibrarySearchScope) -> Option<&'static str> {
    match scope {
        LibrarySearchScope::All => None,
        LibrarySearchScope::Albums => Some("album"),
        LibrarySearchScope::Songs => Some("song"),
    }
}

#[cfg(test)]
mod tests {
    use super::{sanitize_search_request, LibrarySearchIndex};
    use crate::search::snapshot::LibrarySearchSnapshot;
    use crate::search::snapshot::{LibrarySearchAlbumRecord, LibrarySearchSongRecord};
    use tempfile::tempdir;

    fn build_snapshot() -> LibrarySearchSnapshot {
        LibrarySearchSnapshot {
            root_output_dir: "/tmp/music".to_string(),
            inventory_version: "inv-1".to_string(),
            built_at: "2026-01-01T00:00:00Z".to_string(),
            albums: vec![
                LibrarySearchAlbumRecord {
                    album_cid: "album-a".to_string(),
                    album_title: "Alpha".to_string(),
                    artist_line: Some("Artist One".to_string()),
                },
                LibrarySearchAlbumRecord {
                    album_cid: "album-b".to_string(),
                    album_title: "Beta".to_string(),
                    artist_line: Some("Artist Two".to_string()),
                },
            ],
            songs: vec![
                LibrarySearchSongRecord {
                    album_cid: "album-a".to_string(),
                    song_cid: "song-a1".to_string(),
                    album_title: "Alpha".to_string(),
                    song_title: "Beacon".to_string(),
                    artist_line: Some("Artist One".to_string()),
                },
                LibrarySearchSongRecord {
                    album_cid: "album-b".to_string(),
                    song_cid: "song-b1".to_string(),
                    album_title: "Beta".to_string(),
                    song_title: "Beyond".to_string(),
                    artist_line: Some("Artist Two".to_string()),
                },
            ],
        }
    }

    #[test]
    fn rejects_empty_query() {
        let request = siren_core::SearchLibraryRequest {
            query: "   ".to_string(),
            scope: siren_core::LibrarySearchScope::All,
            limit: None,
            offset: None,
        };
        assert!(sanitize_search_request(request, 50, 500).is_err());
    }

    #[test]
    fn clamps_limit_and_offset() {
        let request = siren_core::SearchLibraryRequest {
            query: "alpha".to_string(),
            scope: siren_core::LibrarySearchScope::All,
            limit: Some(999),
            offset: Some(999),
        };
        let sanitized = sanitize_search_request(request, 50, 500).expect("sanitized");
        assert_eq!(sanitized.limit, 50);
        assert_eq!(sanitized.offset, 500);
    }

    #[test]
    fn accepts_plain_text_with_query_parser_characters() {
        let request = siren_core::SearchLibraryRequest {
            query: "artist:(alpha) \"beta\"".to_string(),
            scope: siren_core::LibrarySearchScope::All,
            limit: None,
            offset: None,
        };
        let sanitized = sanitize_search_request(request, 50, 500).expect("sanitized");
        assert_eq!(sanitized.query, "artist:(alpha) \"beta\"");
    }

    #[test]
    fn searches_by_scope_and_preserves_total_before_pagination() {
        let temp_dir = tempdir().expect("temp dir");
        let index = LibrarySearchIndex::build(temp_dir.path(), &build_snapshot()).expect("index");
        let request = sanitize_search_request(
            siren_core::SearchLibraryRequest {
                query: "be".to_string(),
                scope: siren_core::LibrarySearchScope::Songs,
                limit: Some(1),
                offset: Some(0),
            },
            50,
            500,
        )
        .expect("request");

        let (items, total) = index.search(&request).expect("search");
        assert_eq!(total, 2);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, siren_core::SearchLibraryResultKind::Song);
    }
}
