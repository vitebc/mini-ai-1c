//! mcp-1c-metadata — тонкий прокси к HTTP-сервису метаданных 1С.
//!
//! Порт 1c-metadata.ts: пересылает MCP-запросы на 1C-расширение (Kharin 1c_mcp)
//! через JSON-RPC 2.0 по HTTP. Standalone-бинарник, без Node.js и Tauri.

use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const DEFAULT_BASE_URL: &str = "http://localhost/base/hs/mcp";

struct Config {
    base_url: String,
    username: String,
    password: String,
    debug: bool,
}

fn load_config() -> Config {
    Config {
        base_url: std::env::var("ONEC_METADATA_URL")
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
            .trim()
            .to_string(),
        username: std::env::var("ONEC_USERNAME").unwrap_or_default(),
        password: std::env::var("ONEC_PASSWORD").unwrap_or_default(),
        debug: std::env::var("ONEC_AI_DEBUG").as_deref() == Ok("true"),
    }
}

/// Вызывает метод 1C HTTP-сервиса по JSON-RPC 2.0.
async fn call_1c(
    client: &reqwest::Client,
    config: &Config,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    let url = if config.base_url.ends_with('/') {
        format!("{}rpc", config.base_url)
    } else {
        format!("{}/rpc", config.base_url)
    };
    let request_id = fastrand_id();

    if config.debug {
        eprintln!("[1C:Native] Sending to 1C: {} {}", method, params);
    }

    let mut req = client
        .post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/json");

    if !config.username.is_empty() {
        let auth = format!(
            "Basic {}",
            base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                format!("{}:{}", config.username, config.password)
            )
        );
        req = req.header(reqwest::header::AUTHORIZATION, auth);
    }

    let body = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": request_id
    });

    let resp = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        req.json(&body).send(),
    )
    .await
    .map_err(|_| "Timeout calling 1C (5s)".to_string())?
    .map_err(|e| format!("HTTP error: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP Error {}: {}", status, text));
    }

    let json: Value = resp.json().await.map_err(|e| format!("Invalid JSON: {}", e))?;
    if let Some(err) = json.get("error") {
        let code = err.get("code").cloned().unwrap_or(json!(0));
        let message = err.get("message").cloned().unwrap_or(json!(""));
        return Err(format!("1C Error [{}]: {}", code, message));
    }

    Ok(json.get("result").cloned().unwrap_or(json!({})))
}

fn fastrand_id() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    (nanos % 1_000_000) as u32
}

/// Обрабатывает входящий MCP-запрос, проксируя на 1C.
async fn handle_request(
    method: &str,
    params: &Value,
    client: &reqwest::Client,
    config: &Config,
) -> Result<Value, String> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {}, "resources": {}, "prompts": {} },
            "serverInfo": { "name": "1c-metadata", "version": "1.0.0" }
        })),
        "tools/list" => match call_1c(client, config, "tools/list", json!({})).await {
            Ok(v) => Ok(v),
            Err(e) => {
                eprintln!("[1C:Metadata] tools/list error: {}", e);
                Ok(json!({ "tools": [] }))
            }
        },
        "tools/call" => {
            let tool_name = params["name"].as_str().unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            match call_1c(
                client,
                config,
                "tools/call",
                json!({ "name": tool_name, "arguments": arguments }),
            )
            .await
            {
                Ok(v) => Ok(v),
                Err(e) => Ok(json!({
                    "content": [{ "type": "text", "text": format!("Ошибка вызова инструмента в 1С: {}", e) }],
                    "isError": true
                })),
            }
        }
        "resources/list" => match call_1c(client, config, "resources/list", json!({})).await {
            Ok(v) => Ok(v),
            Err(e) => {
                eprintln!("[1C:Metadata] resources/list error: {}", e);
                Ok(json!({ "resources": [] }))
            }
        },
        "resources/read" => {
            let uri = params["uri"].as_str().unwrap_or("");
            match call_1c(client, config, "resources/read", json!({ "uri": uri })).await {
                Ok(v) => Ok(v),
                Err(e) => Err(format!("Ошибка чтения ресурса из 1С: {}", e)),
            }
        }
        "prompts/list" => match call_1c(client, config, "prompts/list", json!({})).await {
            Ok(v) => Ok(v),
            Err(e) => {
                eprintln!("[1C:Metadata] prompts/list error: {}", e);
                Ok(json!({ "prompts": [] }))
            }
        },
        "prompts/get" => {
            let name = params["name"].as_str().unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            match call_1c(
                client,
                config,
                "prompts/get",
                json!({ "name": name, "arguments": arguments }),
            )
            .await
            {
                Ok(v) => Ok(v),
                Err(e) => Err(format!("Ошибка получения промпта из 1С: {}", e)),
            }
        }
        "ping" => Ok(json!({})),
        _ => Err(format!("Method not found: {}", method)),
    }
}

#[tokio::main]
async fn main() {
    let config = load_config();
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[mcp-1c-metadata] Fatal: {}", e);
            std::process::exit(1);
        }
    };
    let client = Arc::new(client);
    let config = Arc::new(config);

    eprintln!("1C Metadata Proxy (Kharin-compatible) started");

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
                        eprintln!("[mcp-1c-metadata] JSON parse error: {}", e);
                        continue;
                    }
                };
                let id = match request.get("id") {
                    Some(id) => id.clone(),
                    None => continue,
                };
                let method = request["method"].as_str().unwrap_or("").to_string();
                let params = request.get("params").cloned().unwrap_or(json!({}));

                let client = Arc::clone(&client);
                let config = Arc::clone(&config);
                let stdout_task = Arc::clone(&stdout);

                tokio::spawn(async move {
                    let result = handle_request(&method, &params, &client, &config).await;
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
                eprintln!("[mcp-1c-metadata] Read error: {}", e);
                break;
            }
        }
    }
}
