//! mcp-1c-filesystem — MCP-сервер для файловых операций в sandbox.
//!
//! Standalone-бинарник: читает JSON-RPC со stdin, пишет на stdout.
//! Не зависит от Node.js и Tauri. Подключается к opencode, Cursor и др.

use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

mod sandbox;
mod tools;

fn emit_status(status: &str) {
    eprintln!("FS_STATUS:{}", status);
}

fn report_status(sandbox: &Option<sandbox::Sandbox>) {
    match sandbox {
        None => emit_status("unavailable"),
        Some(_sb) => emit_status("ready"),
    }
}

#[tokio::main]
async fn main() {
    let sandbox = sandbox::Sandbox::from_env();
    report_status(&sandbox);

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
                        eprintln!("[mcp-1c-filesystem] JSON parse error: {}", e);
                        continue;
                    }
                };

                let id = match request.get("id") {
                    Some(id) => id.clone(),
                    None => continue, // notification
                };

                let method = request["method"].as_str().unwrap_or("").to_string();
                let params = request.get("params").cloned().unwrap_or(json!({}));

                let sandbox = sandbox.clone();
                let stdout_task = Arc::clone(&stdout);

                tokio::spawn(async move {
                    let result = handle_method(&method, &params, sandbox.as_ref());

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
                eprintln!("[mcp-1c-filesystem] Read error: {}", e);
                break;
            }
        }
    }
}

fn handle_method(method: &str, params: &Value, sandbox: Option<&sandbox::Sandbox>) -> Result<Value, String> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "1c-filesystem", "version": "1.0.0" }
        })),
        "tools/list" => Ok(json!({ "tools": tools::list_tools() })),
        "tools/call" => {
            let tool_name = params["name"].as_str().unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            let sb = sandbox.ok_or_else(|| {
                "Sandbox не настроен (MINI_AI_1C_SANDBOX_PATH не задан или каталог не существует)".to_string()
            })?;
            tools::call_tool(tool_name, &arguments, sb)
        }
        "ping" => Ok(json!({})),
        _ => Err(format!("Method not found: {}", method)),
    }
}
