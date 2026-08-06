//! Определения MCP-инструментов и их обработчики.
//!
//! Порт 1c-help.ts: search_1c_help, get_1c_help_topic, list_1c_help_versions, reindex_1c_help.

use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::db;

pub struct HelpState {
    pub db_path: PathBuf,
    pub platform_version: Option<String>,
    pub bin_path: Option<PathBuf>,
    pub is_indexing: AtomicBool,
}

/// Возвращает список инструментов для tools/list.
pub fn list_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "search_1c_help",
            "description": "Полнотекстовый поиск по официальной справке платформы 1С:Предприятие 8.3. Ищет по всем разделам: встроенный язык, объектная модель, язык запросов. Используй для поиска методов, свойств, операторов, функций встроенного языка.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Поисковый запрос (название метода, объекта, функции или описание задачи)" },
                    "limit": { "type": "number", "description": "Максимальное количество результатов (по умолчанию 5)" },
                    "category": { "type": "string", "enum": ["syntax", "query", "language", "all"], "description": "Раздел справки: syntax — объектная модель, query — язык запросов, language — встроенный язык" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "get_1c_help_topic",
            "description": "Получить полное содержимое темы из справки 1С по её идентификатору. Используй topic_id из результатов search_1c_help.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic_id": { "type": "string", "description": "Идентификатор темы из результатов поиска" }
                },
                "required": ["topic_id"]
            }
        }),
        json!({
            "name": "list_1c_help_versions",
            "description": "Получить список проиндексированных версий платформы 1С и статистику.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        }),
        json!({
            "name": "reindex_1c_help",
            "description": "Принудительно пересоздать индекс справки 1С:Предприятие. Используй если база данных справки пустая или устаревшая.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        }),
    ]
}

fn ok_text(text: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": text }] })
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

fn arg_u64(args: &Value, key: &str) -> u64 {
    args.get(key).and_then(|v| v.as_u64()).unwrap_or(5)
}

/// Открывает БД (создаёт схему если нужно).
pub fn open_db(state: &HelpState) -> Result<db::HelpDb, String> {
    db::init_database(&state.db_path)
}

/// Обрабатывает tools/call. Принимает `&Arc<HelpState>`, чтобы фоновый
/// reindex-поток мог клонировать состояние.
pub fn call_tool(name: &str, args: &Value, state: &std::sync::Arc<HelpState>) -> Result<Value, String> {
    match name {
        "search_1c_help" => {
            let query = arg_str(args, "query").unwrap_or("").trim().to_string();
            if query.is_empty() {
                return Ok(ok_text("Ошибка: укажите поисковый запрос."));
            }
            let limit = arg_u64(args, "limit") as usize;
            let category = arg_str(args, "category").unwrap_or("all");
            let category = if category == "all" { None } else { Some(category) };

            let db = open_db(state)?;
            let hits = db::search(&db.conn, &query, limit, category)
                .map_err(|e| format!("Search error: {}", e))?;

            if hits.is_empty() {
                return Ok(ok_text(&format!(
                    "По запросу \"{}\" ничего не найдено в справке 1С.",
                    query
                )));
            }

            let body: Vec<String> = hits
                .iter()
                .enumerate()
                .map(|(i, h)| {
                    format!(
                        "**{}. {}**\nID: `{}`\n{}\n",
                        i + 1,
                        h.title,
                        h.topic_id,
                        h.excerpt
                    )
                })
                .collect();
            Ok(ok_text(&format!(
                "## Результаты поиска по справке 1С: \"{}\"\n\n{}",
                query,
                body.join("\n---\n")
            )))
        }

        "get_1c_help_topic" => {
            let topic_id = arg_str(args, "topic_id").unwrap_or("").trim().to_string();
            if topic_id.is_empty() {
                return Ok(ok_text("Ошибка: укажите topic_id."));
            }
            let db = open_db(state)?;
            match db::get_topic(&db.conn, &topic_id)? {
                Some((title, content)) => Ok(ok_text(&format!("# {}\n\n{}", title, content))),
                None => Ok(ok_text(&format!("Тема \"{}\" не найдена.", topic_id))),
            }
        }

        "list_1c_help_versions" => {
            let db = open_db(state)?;
            let meta = db::get_meta(&db.conn);
            match meta.version {
                Some(v) => Ok(ok_text(&format!(
                    "## 1С:Справка — Статус\n\n✅ Готово\n- Версия платформы: **{}**\n- Тем в базе: **{}**\n- Дата индексации: {}",
                    v,
                    meta.count.unwrap_or_default(),
                    meta.indexed_at.unwrap_or_default()
                ))),
                None => Ok(ok_text("⚠️ База данных не содержит проиндексированных версий.")),
            }
        }

        "reindex_1c_help" => {
            if state.is_indexing.load(Ordering::SeqCst) {
                return Ok(ok_text("⏳ Индексация уже выполняется. Подождите завершения."));
            }
            let bin_path = state.bin_path.clone();
            let version = state.platform_version.clone();
            let (Some(bin), Some(ver)) = (bin_path, version) else {
                return Ok(ok_text("⚠️ Платформа 1С не найдена. Переиндексация невозможна."));
            };

            let db = open_db(state)?;
            let _ = db.conn.execute("DELETE FROM meta", []);
            let _ = db.conn.execute("DELETE FROM topics", []);

            state.is_indexing.store(true, Ordering::SeqCst);
            let state_for_thread = std::sync::Arc::clone(state);
            std::thread::spawn(move || {
                let result = db::run_indexing(&db.conn, &bin, &ver, &|_| {});
                state_for_thread.is_indexing.store(false, Ordering::SeqCst);
                match result {
                    Ok(()) => {
                        let meta = db::get_meta(&db.conn);
                        let count = meta.count.clone().unwrap_or_default();
                        eprintln!("[1c-help] Переиндексация завершена. Тем: {}", count);
                        eprintln!("HELP_STATUS:ready:{}:{}", ver, count);
                    }
                    Err(e) => {
                        eprintln!("[1c-help] Ошибка переиндексации: {}", e);
                        eprintln!("HELP_STATUS:unavailable:Reindex failed");
                    }
                }
            });

            Ok(ok_text("🔄 Переиндексация запущена. Займёт 1-3 минуты."))
        }

        _ => Err(format!("Unknown tool: {}", name)),
    }
}
