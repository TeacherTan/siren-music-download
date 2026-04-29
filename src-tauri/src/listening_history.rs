use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;
use siren_core::homepage::{HistoryEntry, ListeningEvent};

const MAX_HISTORY_ROWS: u32 = 500;

/// 收听历史持久化服务。
///
/// 基于 SQLite 存储用户播放记录，提供写入、查询、去重与上限截断能力。
/// 内部持有 `Mutex<Connection>` 以保证线程安全。
pub(crate) struct ListeningHistoryService {
    conn: Mutex<Connection>,
}

impl ListeningHistoryService {
    pub(crate) fn new(db_path: &Path) -> Result<Self, String> {
        let conn =
            Connection::open(db_path).map_err(|e| format!("打开收听历史数据库失败: {e}"))?;
        let service = Self {
            conn: Mutex::new(conn),
        };
        service.initialize_schema()?;
        Ok(service)
    }

    #[cfg(test)]
    fn new_in_memory() -> Result<Self, String> {
        let conn =
            Connection::open_in_memory().map_err(|e| format!("创建内存数据库失败: {e}"))?;
        let service = Self {
            conn: Mutex::new(conn),
        };
        service.initialize_schema()?;
        Ok(service)
    }

    fn initialize_schema(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("获取数据库锁失败: {e}"))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS listening_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                song_cid TEXT NOT NULL,
                song_name TEXT NOT NULL,
                album_cid TEXT NOT NULL,
                album_name TEXT NOT NULL,
                cover_url TEXT,
                artists TEXT NOT NULL,
                played_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_listening_played_at
                ON listening_history(played_at DESC);",
        )
        .map_err(|e| format!("初始化收听历史表失败: {e}"))?;
        Ok(())
    }

    pub(crate) fn record(&self, event: &ListeningEvent) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("获取数据库锁失败: {e}"))?;

        let last_cid: Option<String> = conn
            .query_row(
                "SELECT song_cid FROM listening_history ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        if last_cid.as_deref() == Some(&event.song_cid) {
            return Ok(());
        }

        let artists_json =
            serde_json::to_string(&event.artists).map_err(|e| format!("序列化 artists 失败: {e}"))?;
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .map_err(|e| format!("格式化时间失败: {e}"))?;

        conn.execute(
            "INSERT INTO listening_history (song_cid, song_name, album_cid, album_name, cover_url, artists, played_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                event.song_cid,
                event.song_name,
                event.album_cid,
                event.album_name,
                event.cover_url,
                artists_json,
                now,
            ],
        )
        .map_err(|e| format!("写入收听历史失败: {e}"))?;

        let count: u32 = conn
            .query_row("SELECT COUNT(*) FROM listening_history", [], |row| row.get(0))
            .map_err(|e| format!("查询收听历史条数失败: {e}"))?;

        if count > MAX_HISTORY_ROWS {
            conn.execute(
                "DELETE FROM listening_history WHERE id IN (
                    SELECT id FROM listening_history ORDER BY played_at ASC LIMIT ?1
                )",
                [count - MAX_HISTORY_ROWS],
            )
            .map_err(|e| format!("截断收听历史失败: {e}"))?;
        }

        Ok(())
    }

    pub(crate) fn get_recent(&self, limit: u32) -> Result<Vec<HistoryEntry>, String> {
        let conn = self.conn.lock().map_err(|e| format!("获取数据库锁失败: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, song_cid, song_name, album_cid, album_name, cover_url, artists, played_at
                 FROM listening_history ORDER BY played_at DESC LIMIT ?1",
            )
            .map_err(|e| format!("准备查询语句失败: {e}"))?;

        let entries = stmt
            .query_map([limit], |row| {
                let artists_json: String = row.get(6)?;
                let artists: Vec<String> =
                    serde_json::from_str(&artists_json).unwrap_or_default();
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    song_cid: row.get(1)?,
                    song_name: row.get(2)?,
                    album_cid: row.get(3)?,
                    album_name: row.get(4)?,
                    cover_url: row.get(5)?,
                    artists,
                    played_at: row.get(7)?,
                })
            })
            .map_err(|e| format!("查询收听历史失败: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("读取收听历史行失败: {e}"))?;

        Ok(entries)
    }

    pub(crate) fn clear(&self) -> Result<u32, String> {
        let conn = self.conn.lock().map_err(|e| format!("获取数据库锁失败: {e}"))?;
        let deleted = conn
            .execute("DELETE FROM listening_history", [])
            .map_err(|e| format!("清除收听历史失败: {e}"))?;
        Ok(deleted as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(song_cid: &str, album_cid: &str) -> ListeningEvent {
        ListeningEvent {
            song_cid: song_cid.to_string(),
            song_name: format!("Song {song_cid}"),
            album_cid: album_cid.to_string(),
            album_name: format!("Album {album_cid}"),
            cover_url: Some("https://example.com/cover.jpg".to_string()),
            artists: vec!["Artist A".to_string()],
        }
    }

    #[test]
    fn creates_table_on_init() {
        let service = ListeningHistoryService::new_in_memory().unwrap();
        let entries = service.get_recent(10).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn records_and_retrieves_listening_event() {
        let service = ListeningHistoryService::new_in_memory().unwrap();
        service.record(&make_event("s1", "a1")).unwrap();
        let entries = service.get_recent(10).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].song_cid, "s1");
        assert_eq!(entries[0].album_cid, "a1");
        assert_eq!(entries[0].artists, vec!["Artist A"]);
    }

    #[test]
    fn deduplicates_consecutive_same_song() {
        let service = ListeningHistoryService::new_in_memory().unwrap();
        service.record(&make_event("s1", "a1")).unwrap();
        service.record(&make_event("s1", "a1")).unwrap();
        let entries = service.get_recent(10).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn allows_same_song_after_different_song() {
        let service = ListeningHistoryService::new_in_memory().unwrap();
        service.record(&make_event("s1", "a1")).unwrap();
        service.record(&make_event("s2", "a1")).unwrap();
        service.record(&make_event("s1", "a1")).unwrap();
        let entries = service.get_recent(10).unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn truncates_beyond_max_rows() {
        let service = ListeningHistoryService::new_in_memory().unwrap();
        for i in 0..(MAX_HISTORY_ROWS + 5) {
            service
                .record(&make_event(&format!("s{i}"), "a1"))
                .unwrap();
        }
        let entries = service.get_recent(MAX_HISTORY_ROWS + 10).unwrap();
        assert_eq!(entries.len() as u32, MAX_HISTORY_ROWS);
    }

    #[test]
    fn clear_removes_all_and_returns_count() {
        let service = ListeningHistoryService::new_in_memory().unwrap();
        service.record(&make_event("s1", "a1")).unwrap();
        service.record(&make_event("s2", "a1")).unwrap();
        let deleted = service.clear().unwrap();
        assert_eq!(deleted, 2);
        let entries = service.get_recent(10).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn get_recent_respects_limit() {
        let service = ListeningHistoryService::new_in_memory().unwrap();
        for i in 0..10 {
            service
                .record(&make_event(&format!("s{i}"), "a1"))
                .unwrap();
        }
        let entries = service.get_recent(3).unwrap();
        assert_eq!(entries.len(), 3);
        // 最新的在前
        assert_eq!(entries[0].song_cid, "s9");
    }
}
