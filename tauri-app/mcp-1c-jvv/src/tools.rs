//! Определения MCP-инструментов и их обработчики.
//!
//! Порт jvv-1c.ts: 3 инструмента — list_infobases, find_platform, get_1c_environment.
//! Ответы возвращаются как JSON, сериализованный в текстовое поле content.

use serde_json::{json, Value};

use crate::platform::find_platform;
use crate::v8i::{find_v8i_path, parse_v8i_file};

/// Возвращает список инструментов для tools/list.
pub fn list_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "list_infobases",
            "description": "Список информационных баз 1С из ibases.v8i. Возвращает все зарегистрированные базы: имя, строку соединения, тип (файловая/серверная), ID и папку.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "v8i_path": {
                        "type": "string",
                        "description": "Путь к ibases.v8i (по умолчанию — стандартные расположения %APPDATA%\\1C\\1CEStart)"
                    }
                }
            }
        }),
        json!({
            "name": "find_platform",
            "description": "Поиск установленных версий 1С:Предприятие (1cv8.exe). Сканирует Program Files и Program Files (x86). Возвращает список версий по убыванию с путями к 1cv8.exe и ibcmd.exe.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "get_1c_environment",
            "description": "Комбинированная информация: установленные платформы + список баз 1С + путь к ibases.v8i. Один вызов — вся картина.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
    ]
}

fn ok_text(data: Value) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".to_string())
        }]
    })
}

/// Обрабатывает tools/call.
pub fn call_tool(name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "list_infobases" => {
            let v8i_path = args.get("v8i_path").and_then(|v| v.as_str());
            let bases = parse_v8i_file(v8i_path);
            Ok(ok_text(json!({
                "count": bases.len(),
                "v8i_path": v8i_path.map(|s| s.to_string()).or_else(find_v8i_path).unwrap_or_default(),
                "bases": bases.iter().map(|b| json!({
                    "name": b.name,
                    "connection": b.connection,
                    "type": b.r#type,
                    "id": b.id,
                    "folder": b.folder,
                })).collect::<Vec<_>>(),
            })))
        }

        "find_platform" => {
            let platforms = find_platform();
            Ok(ok_text(json!({
                "count": platforms.len(),
                "latest": platforms.first().cloned(),
                "platforms": platforms.iter().map(|p| json!({
                    "version": p.version,
                    "bin_path": p.bin_path,
                    "exe_path": p.exe_path,
                    "ibcmd_path": p.ibcmd_path,
                    "cestart_path": p.cestart_path,
                })).collect::<Vec<_>>(),
            })))
        }

        "get_1c_environment" => {
            let platforms = find_platform();
            let infobases = parse_v8i_file(None);
            let v8i_path = find_v8i_path();
            Ok(ok_text(json!({
                "platforms": {
                    "count": platforms.len(),
                    "latest_version": platforms.first().map(|p| p.version.clone()).unwrap_or_default(),
                    "items": platforms.iter().map(|p| json!({
                        "version": p.version,
                        "exe_path": p.exe_path,
                        "ibcmd_path": p.ibcmd_path,
                    })).collect::<Vec<_>>(),
                },
                "infobases": {
                    "count": infobases.len(),
                    "v8i_path": v8i_path,
                    "items": infobases.iter().map(|b| json!({
                        "name": b.name,
                        "connection": b.connection,
                        "type": b.r#type,
                    })).collect::<Vec<_>>(),
                }
            })))
        }

        _ => Err(format!("Unknown tool: {}", name)),
    }
}
