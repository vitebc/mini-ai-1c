//! SQLite-хранилище справки: FTS5-индексация, поиск, метаданные.
//!
//! Порт логики БД из 1c-help.ts. База лежит в
//! `~/.config/mini-ai-1c/help/help.db`.

use rusqlite::{params, Connection};
use std::path::PathBuf;

pub const HBK_FILES: &[(&str, &str)] = &[
    ("shcntx_ru.hbk", "syntax"),
    ("shquery_ru.hbk", "query"),
    ("shlang_ru.hbk", "language"),
];

pub struct HelpDb {
    pub conn: Connection,
}

/// Путь к базе данных: ~/.config/mini-ai-1c/help/help.db
pub fn db_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
    let dir = home.join(".config").join("mini-ai-1c").join("help");
    if !dir.is_dir() {
        let _ = std::fs::create_dir_all(&dir);
    }
    dir.join("help.db")
}

pub fn init_database(db_path: &std::path::Path) -> Result<HelpDb, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         CREATE TABLE IF NOT EXISTS meta (
           key TEXT PRIMARY KEY,
           value TEXT
         );
         CREATE VIRTUAL TABLE IF NOT EXISTS topics USING fts5(
           topic_id, title, content, category, version,
           tokenize = \"unicode61\"
         );",
    )
    .map_err(|e| format!("Failed to init DB: {}", e))?;
    Ok(HelpDb { conn })
}

/// Метаданные об индексе.
pub struct IndexMeta {
    pub version: Option<String>,
    pub count: Option<String>,
    pub indexed_at: Option<String>,
}

pub fn get_meta(db: &Connection) -> IndexMeta {
    let get = |key: &str| -> Option<String> {
        db.query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .ok()
    };
    IndexMeta {
        version: get("indexed_version"),
        count: get("topic_count"),
        indexed_at: get("indexed_at"),
    }
}

/// Индексирует один HBK-файл в указанную категорию.
fn index_hbk(
    db: &Connection,
    hbk_path: &std::path::Path,
    category: &str,
    version: &str,
    on_progress: &dyn Fn(usize),
) -> Result<usize, String> {
    let pages = crate::hbk::parse_hbk(hbk_path);
    let mut count = 0usize;

    {
        let mut insert = db
            .prepare("INSERT INTO topics (topic_id, title, content, category, version) VALUES (?1, ?2, ?3, ?4, ?5)")
            .map_err(|e| format!("prepare: {}", e))?;

        let mut batch: Vec<(String, String, String, String, String)> = Vec::new();
        for page in &pages {
            let (title, text) = crate::html::extract_text(&page.html);
            let topic_id = format!("{}/{}/{}", version, category, page.name);
            batch.push((topic_id, title, text, category.to_string(), version.to_string()));
            count += 1;

            if batch.len() >= 100 {
                db.execute_batch("BEGIN").map_err(|e| e.to_string())?;
                for row in &batch {
                    insert
                        .execute(params![row.0, row.1, row.2, row.3, row.4])
                        .map_err(|e| format!("insert: {}", e))?;
                }
                db.execute_batch("COMMIT").map_err(|e| e.to_string())?;
                batch.clear();
                on_progress(count);
            }
        }
        if !batch.is_empty() {
            db.execute_batch("BEGIN").map_err(|e| e.to_string())?;
            for row in &batch {
                insert
                    .execute(params![row.0, row.1, row.2, row.3, row.4])
                    .map_err(|e| format!("insert: {}", e))?;
            }
            db.execute_batch("COMMIT").map_err(|e| e.to_string())?;
        }
    }

    Ok(count)
}

/// Полная индексация платформы.
pub fn run_indexing(
    db: &Connection,
    bin_path: &std::path::Path,
    version: &str,
    on_progress: &dyn Fn(usize),
) -> Result<(), String> {
    db.execute("DELETE FROM topics WHERE version = ?1", params![version])
        .map_err(|e| format!("delete: {}", e))?;

    let processed = std::cell::Cell::new(0usize);
    for (file, category) in HBK_FILES {
        let hbk_path = bin_path.join(file);
        if !hbk_path.exists() {
            continue;
        }
        eprintln!("[1c-help] Индексируется: {}", file);
        let _n = index_hbk(db, &hbk_path, category, version, &|n| {
            processed.set(processed.get() + n);
            on_progress(processed.get());
        })?;
    }

    let count: i64 = db
        .query_row(
            "SELECT COUNT(*) as c FROM topics WHERE version = ?1",
            params![version],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let now = chrono_now_iso();
    db.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
        params!["indexed_version", version],
    )
    .map_err(|e| e.to_string())?;
    db.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
        params!["topic_count", count.to_string()],
    )
    .map_err(|e| e.to_string())?;
    db.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
        params!["indexed_at", now],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn chrono_now_iso() -> String {
    // Без chrono-зависимости: приблизительное UTC из SystemTime.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, mo, d) = civil_from_days(days);
    format!("{}-{:02}-{:02}T{:02}:{:02}:{:02}.000Z", y, mo, d, h, m, s)
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Результат поиска.
pub struct SearchHit {
    pub topic_id: String,
    pub title: String,
    pub excerpt: String,
}

/// FTS5-поиск, возвращает rusqlite::Error (для fallback).
fn fts_search(
    db: &Connection,
    query: &str,
    limit: usize,
    category: Option<&str>,
) -> rusqlite::Result<Vec<SearchHit>> {
    if let Some(cat) = category {
        let mut stmt = db.prepare(
            "SELECT topic_id, title, snippet(topics, 2, '>>', '<<', '...', 30) as excerpt \
             FROM topics WHERE topics MATCH ?1 AND category = ?2 ORDER BY rank LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![query, cat, limit as i64], |row| {
            Ok(SearchHit {
                topic_id: row.get(0)?,
                title: row.get(1)?,
                excerpt: row.get(2)?,
            })
        })?;
        rows.collect()
    } else {
        let mut stmt = db.prepare(
            "SELECT topic_id, title, snippet(topics, 2, '>>', '<<', '...', 30) as excerpt \
             FROM topics WHERE topics MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![query, limit as i64], |row| {
            Ok(SearchHit {
                topic_id: row.get(0)?,
                title: row.get(1)?,
                excerpt: row.get(2)?,
            })
        })?;
        rows.collect()
    }
}

/// Полнотекстовый поиск (FTS5 с fallback на LIKE).
pub fn search(
    db: &Connection,
    query: &str,
    limit: usize,
    category: Option<&str>,
) -> Result<Vec<SearchHit>, String> {
    let limit = limit.max(1).min(50);

    match fts_search(db, query, limit, category) {
        Ok(hits) => Ok(hits),
        Err(e) => {
            eprintln!("[1c-help] FTS error: {}, falling back to LIKE", e);
            let like = format!("%{}%", query);
            let mut stmt = db
                .prepare(
                    "SELECT topic_id, title, substr(content, 1, 300) as excerpt \
                     FROM topics WHERE title LIKE ?1 OR content LIKE ?1 LIMIT ?2",
                )
                .map_err(|e| format!("LIKE prepare: {}", e))?;
            let rows = stmt
                .query_map(params![like, limit as i64], |row| {
                    Ok(SearchHit {
                        topic_id: row.get(0)?,
                        title: row.get(1)?,
                        excerpt: row.get(2)?,
                    })
                })
                .map_err(|e| format!("LIKE query: {}", e))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("LIKE collect: {}", e))
        }
    }
}

/// Получает тему по topic_id.
pub fn get_topic(db: &Connection, topic_id: &str) -> Result<Option<(String, String)>, String> {
    let mut stmt = db
        .prepare("SELECT title, content FROM topics WHERE topic_id = ?1")
        .map_err(|e| format!("prepare: {}", e))?;
    let mut rows = stmt
        .query_map(params![topic_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| format!("query: {}", e))?;
    match rows.next() {
        Some(Ok(t)) => Ok(Some(t)),
        Some(Err(e)) => Err(format!("row: {}", e)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    static DB_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_db() -> HelpDb {
        let n = DB_COUNTER.fetch_add(1, AtomicOrdering::SeqCst);
        let path = std::env::temp_dir().join(format!("mcp-help-test-{}-{}.db", std::process::id(), n));
        let _ = std::fs::remove_file(&path);
        init_database(&path).unwrap()
    }

    #[test]
    fn insert_and_search_fts() {
        let db = temp_db();
        let conn = &db.conn;
        conn.execute_batch("BEGIN").unwrap();
        for i in 0..3 {
            conn.execute(
                "INSERT INTO topics (topic_id, title, content, category, version) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    format!("8.3.0/syntax/page{}", i),
                    format!("Метод Выполнить {}", i),
                    "Выполняет команду на сервере".to_string(),
                    "syntax",
                    "8.3.0"
                ],
            )
            .unwrap();
        }
        conn.execute_batch("COMMIT").unwrap();

        let hits = search(conn, "Выполнить", 5, None).unwrap();
        assert_eq!(hits.len(), 3);
        assert!(hits[0].topic_id.starts_with("8.3.0/syntax/"));

        // LIKE fallback
        let hits = search(conn, "команду", 5, None).unwrap();
        assert!(!hits.is_empty());

        let topic = get_topic(conn, "8.3.0/syntax/page1").unwrap();
        assert!(topic.is_some());
        let (title, _) = topic.unwrap();
        assert_eq!(title, "Метод Выполнить 1");
    }

    #[test]
    fn meta_roundtrip() {
        let db = temp_db();
        let conn = &db.conn;
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            params!["indexed_version", "8.3.27.1989"],
        )
        .unwrap();
        let meta = get_meta(conn);
        assert_eq!(meta.version.as_deref(), Some("8.3.27.1989"));
    }
}
