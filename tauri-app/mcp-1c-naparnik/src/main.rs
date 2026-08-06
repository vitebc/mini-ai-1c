//! mcp-1c-naparnik — MCP-сервер для 1С:Напарник (code.1c.ai).
//!
//! Standalone-бинарник: читает JSON-RPC со stdin, пишет на stdout.
//! Не зависит от Node.js и Tauri. Требует переменную ONEC_AI_TOKEN.

use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

mod naparnik;
mod tools;

#[tokio::main]
async fn main() {
    let token = match std::env::var("ONEC_AI_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t,
        _ => {
            eprintln!("Error: ONEC_AI_TOKEN environment variable is required.");
            std::process::exit(1);
        }
    };

    let client = match naparnik::build_client() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[mcp-1c-naparnik] Fatal: {}", e);
            std::process::exit(1);
        }
    };

    let state = Arc::new(tools::AppState { client, token });

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
                        eprintln!("[mcp-1c-naparnik] JSON parse error: {}", e);
                        continue;
                    }
                };

                let id = match request.get("id") {
                    Some(id) => id.clone(),
                    None => continue, // notification
                };

                let method = request["method"].as_str().unwrap_or("").to_string();
                let params = request.get("params").cloned().unwrap_or(json!({}));

                let state = Arc::clone(&state);
                let stdout_task = Arc::clone(&stdout);

                tokio::spawn(async move {
                    let result = handle_method(&method, &params, &state).await;

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
                eprintln!("[mcp-1c-naparnik] Read error: {}", e);
                break;
            }
        }
    }
}

async fn handle_method(
    method: &str,
    params: &Value,
    state: &Arc<tools::AppState>,
) -> Result<Value, String> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "1c-naparnik", "version": "1.0.0" }
        })),
        "tools/list" => Ok(json!({ "tools": tools::list_tools() })),
        "tools/call" => {
            let tool_name = params["name"].as_str().unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            tools::call_tool(tool_name, &arguments, state).await
        }
        "ping" => Ok(json!({})),
        _ => Err(format!("Method not found: {}", method)),
    }
}
