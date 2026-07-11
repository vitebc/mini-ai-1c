// Main.rs - Entry point for the MCP server
use anyhow::Result;
use clap::Parser;
use serde_json::Value;
use std::env;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

mod cli;
mod config;
mod dsl;
mod info;
mod setup;
mod skills;
mod tools;
mod v8i;

#[derive(Parser, Debug)]
#[clap(name = "mcp-1c-tools", about = "MCP server for 1C:Enterprise development tools")]
struct Args {
    /// Path to 1cv8.exe (auto-detected if not specified)
    #[arg(long)]
    v8_path: Option<String>,

    /// Path to ibcmd.exe (auto-detected if not specified)
    #[arg(long)]
    ibcmd_path: Option<String>,

    /// Workspace root path (where .v8-project.json or src/cf lives)
    #[arg(long)]
    workspace: Option<String>,
}

impl Args {
    fn resolve_env(self) -> Self {
        Self {
            v8_path: self.v8_path.or_else(|| env::var("ONEC_V8_PATH").ok()),
            ibcmd_path: self.ibcmd_path.or_else(|| env::var("ONEC_IBCMD_PATH").ok()),
            workspace: self.workspace.or_else(|| env::var("ONEC_WORKSPACE").ok()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse().resolve_env();
    let config = config::new(args.v8_path.as_deref()).await?;

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut writer = stdout;
    let writer = Arc::new(Mutex::new(writer));

    eprintln!("[mcp-1c-tools] Starting MCP server...");
    eprintln!("[mcp-1c-tools] Config: {:?}", config);

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
                        eprintln!("[mcp-1c-tools] JSON parse error: {}", e);
                        continue;
                    }
                };

let id = match request.get("id").cloned() {
                    Some(id) => id,
                    None => continue,
                };

                let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let params = request.get("params").cloned().unwrap_or(Value::Null);

                let result: Result<Value, anyhow::Error> = match method.as_str() {
                    "initialize" => initialize(),
                    "tools/list" => Ok(tools::list_tools().await),
                    "tools/call" => tools::call_tool(params, &config).await,
                    "ping" => Ok(Value::Null),
                    _ => Err(anyhow::anyhow!("Method not found: {}", method)),
                };

                let response = match result {
                    Ok(res) => serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": res
                    }),
                    Err(msg) => serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32603, "message": msg.to_string() }
                    }),
                };

                let mut w = writer.lock().await;
                let resp_str = serde_json::to_string(&response).unwrap_or_default();
                let _ = w.write_all(resp_str.as_bytes()).await;
                let _ = w.write_all(b"\n").await;
                let _ = w.flush().await;
            }
            Err(e) => {
                eprintln!("[mcp-1c-tools] Read error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn initialize() -> Result<Value> {
    Ok(serde_json::json!({
        "protocolVersion": "2024-11-05",
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "mcp-1c-tools", "version": "0.1.0" }
    }))
}