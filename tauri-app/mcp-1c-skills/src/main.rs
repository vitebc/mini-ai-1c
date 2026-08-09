//! mcp-1c-skills — MCP-сервер для скиллов, документации и правил 1С.
//!
//! Standalone-бинарник: читает JSON-RPC со stdin, пишет на stdout.
//! Не зависит от Node.js и Tauri. Можно подключать к opencode, Cursor,
//! Claude Desktop и любым другим MCP-клиентам.

use std::path::Path;
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

mod search;
mod skills;
mod tools;

#[tokio::main]
async fn main() {
    let skills_dir = skills::resolve_skills_dir();

    let bm25: Option<Arc<search::Bm25>> = skills_dir.as_ref().map(|d| {
        let s = skills::scan_skills(d);
        let docs = skills::scan_docs(d);
        let rules = skills::scan_rules(d);
        let index = Arc::new(search::Bm25::build(&s, &docs, &rules));
        eprintln!(
            "[mcp-skills] {} skills, {} docs, {} rules loaded from {}",
            s.len(),
            docs.len(),
            rules.len(),
            d.to_string_lossy()
        );
        index
    });

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let stdout = Arc::new(tokio::sync::Mutex::new(stdout));

    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let request: Value = match serde_json::from_str(trimmed) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("[mcp-skills] JSON parse error: {}", e);
                        continue;
                    }
                };

                // Notifications не требуют ответа
                let id = match request.get("id") {
                    Some(id) => id.clone(),
                    None => continue,
                };

                let method = request["method"].as_str().unwrap_or("").to_string();
                let params = request.get("params").cloned().unwrap_or(json!({}));

                let skills_dir = skills_dir.clone();
                let bm25 = bm25.clone();
                let stdout_task = Arc::clone(&stdout);

                tokio::spawn(async move {
                    let result = handle_method(&method, &params, skills_dir.as_deref(), bm25.as_deref());

                    let response = match result {
                        Ok(res) => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": res
                        }),
                        Err(msg) => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32603,
                                "message": msg
                            }
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
                eprintln!("[mcp-skills] Read error: {}", e);
                break;
            }
        }
    }
}

fn handle_method(
    method: &str,
    params: &Value,
    skills_dir: Option<&Path>,
    bm25: Option<&search::Bm25>,
) -> Result<Value, String> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "1c-skills", "version": "1.0.0" }
        })),
        "tools/list" => Ok(json!({ "tools": tools::list_tools() })),
        "tools/call" => {
            let tool_name = params["name"].as_str().unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            let skills_dir = skills_dir.ok_or_else(|| {
                "Каталог скиллов не найден. Укажите SKILLS_DIR или расположите .agents/skills рядом с бинарником.".to_string()
            })?;
            tools::call_tool(tool_name, &arguments, skills_dir, bm25)
        }
        "ping" => Ok(json!({})),
        _ => Err(format!("Method not found: {}", method)),
    }
}
