//! mcp-1c-help — MCP-сервер справки 1С:Предприятие.
//!
//! Читает .hbk файлы напрямую (без Java/JAR), индексирует в SQLite FTS5.
//! Standalone-бинарник: JSON-RPC через stdin/stdout. Без Node.js и Tauri.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

mod db;
mod hbk;
mod html;
mod platform;
mod tools;

fn report_status(status: &str) {
    eprintln!("HELP_STATUS:{}", status);
}

fn log(msg: &str) {
    eprintln!("[1c-help] {}", msg);
}

/// Инициализирует состояние: находит платформу, открывает/проверяет БД,
/// запускает фоновую индексацию если нужно. Возвращает состояние в Arc.
fn setup() -> Arc<tools::HelpState> {
    let platform = platform::find_platform();

    let mut state = tools::HelpState {
        db_path: db::db_path(),
        platform_version: None,
        bin_path: None,
        is_indexing: std::sync::atomic::AtomicBool::new(false),
    };

    let Some(platform) = platform else {
        let custom = std::env::var("ONEC_HELP_PATH").unwrap_or_default();
        let custom = custom.trim();
        if !custom.is_empty() {
            report_status(&format!("unavailable:1C Platform not found at custom path: {}", custom));
            log(&format!(
                "Платформа 1С не найдена по указанному пути: {}. Проверьте правильность пути (должна быть папка с подпапками вида 8.x.x.x/bin/shcntx_ru.hbk).",
                custom
            ));
        } else {
            report_status("unavailable:1C Platform not found in standard paths");
            log("Платформа 1С не найдена в стандартных путях. Установите 1С:Предприятие 8.3 или укажите путь вручную (ONEC_HELP_PATH).");
        }
        return Arc::new(state);
    };

    log(&format!("Найдена платформа: {} ({})", platform.version, platform.bin_path.display()));
    state.platform_version = Some(platform.version.clone());
    state.bin_path = Some(platform.bin_path.clone());
    let state = Arc::new(state);

    let db_path = db::db_path();
    let db_exists = db_path.exists();

    // Открываем БД и определяем, нужна ли индексация
    let (needs_indexing, ready_count): (bool, Option<String>) = if db_exists {
        match db::init_database(&db_path) {
            Ok(help_db) => {
                let meta = db::get_meta(&help_db.conn);
                let version_mismatch = meta.version.as_deref() != Some(platform.version.as_str());
                let count = meta
                    .count
                    .clone()
                    .and_then(|c| c.parse::<i64>().ok())
                    .unwrap_or(0);
                if version_mismatch {
                    log(&format!(
                        "Версия изменилась: {} → {}. Требуется переиндексация.",
                        meta.version.unwrap_or_default(),
                        platform.version
                    ));
                    (true, None)
                } else if count == 0 {
                    log("База данных пустая (0 тем). Требуется переиндексация.");
                    (true, None)
                } else {
                    log(&format!("База данных готова: {} тем.", count));
                    (false, meta.count)
                }
            }
            Err(e) => {
                log(&format!("Не удалось открыть БД: {}", e));
                (true, None)
            }
        }
    } else {
        (true, None)
    };

    // Фоновая индексация
    if needs_indexing {
        match db::init_database(&db_path) {
            Ok(help_db) => {
                state.is_indexing.store(true, Ordering::SeqCst);
                report_status("indexing:0:1000:Запуск индексации...");
                let bin = platform.bin_path.clone();
                let ver = platform.version.clone();
                let state_for_thread = Arc::clone(&state);
                std::thread::spawn(move || {
                    let result = db::run_indexing(&help_db.conn, &bin, &ver, &|_| {});
                    state_for_thread.is_indexing.store(false, Ordering::SeqCst);
                    match result {
                        Ok(()) => {
                            let meta = db::get_meta(&help_db.conn);
                            let count = meta.count.unwrap_or_default();
                            log(&format!("Индексация завершена. Всего тем: {}", count));
                            report_status(&format!("ready:{}:{}", ver, count));
                        }
                        Err(e) => {
                            state_for_thread.is_indexing.store(false, Ordering::SeqCst);
                            log(&format!("Ошибка индексации: {}", e));
                            report_status("unavailable:Indexing failed");
                        }
                    }
                });
            }
            Err(e) => {
                log(&format!("Не удалось создать БД для индексации: {}", e));
                report_status("unavailable:DB init failed");
            }
        }
    } else if let Some(count) = ready_count {
        report_status(&format!("ready:{}:{}", platform.version, count));
    }

    state
}

#[tokio::main]
async fn main() {
    let state = setup();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let stdout = Arc::new(tokio::sync::Mutex::new(stdout));

    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let request: Value = match serde_json::from_str(trimmed) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("[mcp-1c-help] JSON parse error: {}", e);
                        continue;
                    }
                };
                let id = match request.get("id") {
                    Some(id) => id.clone(),
                    None => continue,
                };
                let method = request["method"].as_str().unwrap_or("").to_string();
                let params = request.get("params").cloned().unwrap_or(json!({}));

                let state = Arc::clone(&state);
                let stdout_task = Arc::clone(&stdout);

                tokio::spawn(async move {
                    let result = handle_method(&method, &params, &state);
                    let response = match result {
                        Ok(res) => json!({ "jsonrpc": "2.0", "id": id, "result": res }),
                        Err(msg) => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": { "code": -32603, "message": msg }
                        }),
                    };
                    let resp_str = serde_json::to_string(&response).unwrap_or_default();
                    let mut out = stdout_task.lock().await;
                    let _ = out.write_all(resp_str.as_bytes()).await;
                    let _ = out.write_all(b"\n").await;
                    let _ = out.flush().await;
                });
            }
            Err(e) => {
                eprintln!("[mcp-1c-help] Read error: {}", e);
                break;
            }
        }
    }
}

fn handle_method(
    method: &str,
    params: &Value,
    state: &Arc<tools::HelpState>,
) -> Result<Value, String> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "1c-help", "version": "1.0.0" }
        })),
        "tools/list" => Ok(json!({ "tools": tools::list_tools() })),
        "tools/call" => {
            let tool_name = params["name"].as_str().unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            tools::call_tool(tool_name, &arguments, state)
        }
        "ping" => Ok(json!({})),
        _ => Err(format!("Method not found: {}", method)),
    }
}
