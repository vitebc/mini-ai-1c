//! Определения MCP-инструментов и их обработчики.
//!
//! Порт 1c-filesystem.ts: 11 инструментов. Все операции — внутри sandbox.

use serde_json::{json, Value};
use std::time::UNIX_EPOCH;

use crate::sandbox::Sandbox;

/// Возвращает список инструментов для tools/list.
pub fn list_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "read_file",
            "description": "Read file content. Returns content as text or base64 with size.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path from sandbox root" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "write_file",
            "description": "Write content to a file (creates or overwrites).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path from sandbox root" },
                    "content": { "type": "string", "description": "Content to write" }
                },
                "required": ["path", "content"]
            }
        }),
        json!({
            "name": "edit_file",
            "description": "Find and replace exact string in a file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path from sandbox root" },
                    "old_string": { "type": "string", "description": "Exact string to find (must be unique or provide enough context)" },
                    "new_string": { "type": "string", "description": "Replacement string" }
                },
                "required": ["path", "old_string", "new_string"]
            }
        }),
        json!({
            "name": "list_directory",
            "description": "List entries in a directory with type, size, modified date.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path from sandbox root (default: .)" }
                }
            }
        }),
        json!({
            "name": "file_info",
            "description": "Get file/directory metadata: exists, type, size, modified, permissions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path from sandbox root" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "search_files",
            "description": "Search for files by glob pattern (recursively).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Glob pattern (e.g. \"**/*.bsl\", \"*.xml\")" },
                    "root": { "type": "string", "description": "Relative subdirectory to search in (default: sandbox root)" }
                },
                "required": ["pattern"]
            }
        }),
        json!({
            "name": "create_directory",
            "description": "Create a directory (including parent dirs).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path from sandbox root" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "delete_file",
            "description": "Delete a file or empty directory.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path from sandbox root" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "delete_directory",
            "description": "Delete a directory, optionally recursively.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path from sandbox root" },
                    "recursive": { "type": "boolean", "description": "Delete all contents recursively (default: false)" }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "move_file",
            "description": "Move or rename a file/directory within sandbox.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Source relative path from sandbox root" },
                    "destination": { "type": "string", "description": "Destination relative path from sandbox root" }
                },
                "required": ["source", "destination"]
            }
        }),
        json!({
            "name": "run_command",
            "description": "Execute a shell command (PowerShell/bash). Working directory is inside sandbox. Returns stdout, stderr, exit_code.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command to execute (e.g. \"node\", \"powershell\", \"cargo build\")" },
                    "args": { "type": "array", "items": { "type": "string" }, "description": "Arguments (optional)" },
                    "cwd": { "type": "string", "description": "Working directory relative to sandbox root (default: sandbox root)" },
                    "timeout_ms": { "type": "number", "description": "Timeout in milliseconds (default: 30000, max: 300000)" }
                },
                "required": ["command"]
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

fn err_text(msg: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": format!("Error: {}", msg)
        }],
        "isError": true
    })
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

fn arg_bool(args: &Value, key: &str) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn iso8601(modified: std::time::SystemTime) -> String {
    match modified.duration_since(UNIX_EPOCH) {
        Ok(d) => {
            let secs = d.as_secs() as i64;
            // Формат, близкий к toISOString() (UTC). Ограничиваемся секундами.
            if let Some(dt) = datetime_from_unix(secs) {
                format!("{}-{:02}-{:02}T{:02}:{:02}:{:02}.000Z", dt.0, dt.1, dt.2, dt.3, dt.4, dt.5)
            } else {
                format!("{}", secs)
            }
        }
        Err(_) => String::new(),
    }
}

fn datetime_from_unix(secs: i64) -> Option<(i64, u32, u32, u32, u32, u32)> {
    // Простая конверсия Unix→UTC (григорианский календарь).
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, mo, d) = civil_from_days(days);
    Some((y, mo, d, h as u32, m as u32, s as u32))
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    // Алгоритм Говарда Хиннанта (число дней от 1970-01-01).
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

/// Обрабатывает tools/call.
pub fn call_tool(name: &str, args: &Value, sandbox: &Sandbox) -> Result<Value, String> {
    let result = match name {
        "read_file" => tool_read_file(args, sandbox),
        "write_file" => tool_write_file(args, sandbox),
        "edit_file" => tool_edit_file(args, sandbox),
        "list_directory" => tool_list_directory(args, sandbox),
        "file_info" => tool_file_info(args, sandbox),
        "search_files" => tool_search_files(args, sandbox),
        "create_directory" => tool_create_directory(args, sandbox),
        "delete_file" => tool_delete_file(args, sandbox),
        "delete_directory" => tool_delete_directory(args, sandbox),
        "move_file" => tool_move_file(args, sandbox),
        "run_command" => tool_run_command(args, sandbox),
        _ => return Err(format!("Unknown tool: {}", name)),
    };

    match result {
        Ok(v) => Ok(ok_text(v)),
        Err(msg) => Ok(err_text(&msg)),
    }
}

fn tool_read_file(args: &Value, sb: &Sandbox) -> Result<Value, String> {
    let path = arg_str(args, "path").ok_or("Path is required")?;
    let p = sb.resolve(path).ok_or("Path escapes sandbox")?;
    if !p.exists() {
        return Err("File not found".to_string());
    }
    if p.is_dir() {
        return Err("Path is a directory".to_string());
    }
    let content = std::fs::read_to_string(&p)
        .map_err(|e| format!("Failed to read: {}", e))?;
    let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
    Ok(json!({ "content": content, "encoding": "text", "size": size }))
}

fn tool_write_file(args: &Value, sb: &Sandbox) -> Result<Value, String> {
    let path = arg_str(args, "path").ok_or("Path is required")?;
    let content = arg_str(args, "content").ok_or("Content is required")?;
    let p = sb.resolve_for_write(path).ok_or("Path escapes sandbox")?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent dirs: {}", e))?;
    }
    std::fs::write(&p, content).map_err(|e| format!("Failed to write: {}", e))?;
    let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
    Ok(json!({ "success": true, "size": size }))
}

fn tool_edit_file(args: &Value, sb: &Sandbox) -> Result<Value, String> {
    let path = arg_str(args, "path").ok_or("Path is required")?;
    let old_string = arg_str(args, "old_string").ok_or("old_string is required")?;
    let new_string = arg_str(args, "new_string").unwrap_or("");
    let p = sb.resolve(path).ok_or("Path escapes sandbox")?;
    if !p.exists() {
        return Err("File not found".to_string());
    }
    let content = std::fs::read_to_string(&p).map_err(|e| format!("Failed to read: {}", e))?;
    if !content.contains(old_string) {
        return Err("String not found in file".to_string());
    }
    let count = content.matches(old_string).count();
    let updated = content.replace(old_string, new_string);
    std::fs::write(&p, updated).map_err(|e| format!("Failed to write: {}", e))?;
    Ok(json!({ "success": true, "changes": count }))
}

fn tool_list_directory(args: &Value, sb: &Sandbox) -> Result<Value, String> {
    let rel = arg_str(args, "path").unwrap_or(".");
    let p = sb.resolve(rel).ok_or("Path escapes sandbox")?;
    if !p.exists() {
        return Err("Directory not found".to_string());
    }
    let mut entries = Vec::new();
    let rd = std::fs::read_dir(&p).map_err(|e| format!("Failed to read dir: {}", e))?;
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.path().is_dir();
        let md = std::fs::metadata(entry.path()).ok();
        entries.push(json!({
            "name": name,
            "type": if is_dir { "directory" } else { "file" },
            "size": md.as_ref().map(|m| m.len()).unwrap_or(0),
            "modified": md.as_ref().map(|m| iso8601(m.modified().unwrap_or(UNIX_EPOCH))).unwrap_or_default(),
        }));
    }
    Ok(json!({ "entries": entries }))
}

fn tool_file_info(args: &Value, sb: &Sandbox) -> Result<Value, String> {
    let path = arg_str(args, "path").ok_or("Path is required")?;
    let p = sb.resolve(path).ok_or("Path escapes sandbox")?;
    if !p.exists() {
        return Ok(json!({ "exists": false }));
    }
    let md = std::fs::metadata(&p).map_err(|e| format!("Failed to stat: {}", e))?;
    let ty = if md.is_dir() {
        "directory"
    } else if md.is_file() {
        "file"
    } else {
        "other"
    };
    Ok(json!({
        "exists": true,
        "type": ty,
        "size": md.len(),
        "modified": iso8601(md.modified().unwrap_or(UNIX_EPOCH)),
        "permissions": format!("{:o}", unix_permissions(&md)),
    }))
}

#[cfg(unix)]
fn unix_permissions(md: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    md.permissions().mode() & 0o777
}

#[cfg(not(unix))]
fn unix_permissions(_md: &std::fs::Metadata) -> u32 {
    0
}

fn tool_search_files(args: &Value, sb: &Sandbox) -> Result<Value, String> {
    let pattern = arg_str(args, "pattern").ok_or("Pattern is required")?;
    let root_rel = arg_str(args, "root").unwrap_or(".");
    let root = sb.resolve(root_rel).ok_or("Path escapes sandbox")?;
    if !root.exists() {
        return Err("Root directory not found".to_string());
    }

    let mut results = Vec::new();
    let prefix = pattern.split('/').next_back().unwrap_or(pattern);
    let star_idx = prefix.find('*');

    for entry in walkdir::WalkDir::new(&root).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if match_glob(prefix, star_idx, &name) {
            let rel = entry.path().strip_prefix(&root).unwrap_or(entry.path());
            results.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }

    Ok(json!({ "files": results }))
}

fn match_glob(pattern: &str, star_idx: Option<usize>, filename: &str) -> bool {
    let star_idx = match star_idx {
        Some(i) => i,
        None => return filename == pattern,
    };
    let prefix = &pattern[..star_idx];
    let suffix = &pattern[star_idx + 1..];
    filename.starts_with(prefix) && filename.ends_with(suffix)
}

fn tool_create_directory(args: &Value, sb: &Sandbox) -> Result<Value, String> {
    let path = arg_str(args, "path").ok_or("Path is required")?;
    let p = sb.resolve_for_write(path).ok_or("Path escapes sandbox")?;
    std::fs::create_dir_all(&p).map_err(|e| format!("Failed to create dir: {}", e))?;
    Ok(json!({ "success": true }))
}

fn tool_delete_file(args: &Value, sb: &Sandbox) -> Result<Value, String> {
    let path = arg_str(args, "path").ok_or("Path is required")?;
    let p = sb.resolve(path).ok_or("Path escapes sandbox")?;
    if !p.exists() {
        return Err("File not found".to_string());
    }
    if p.is_dir() {
        let count = std::fs::read_dir(&p).map(|r| r.count()).unwrap_or(0);
        if count > 0 {
            return Err("Directory not empty — use delete_directory with recursive".to_string());
        }
        std::fs::remove_dir(&p).map_err(|e| format!("Failed to remove dir: {}", e))?;
    } else {
        std::fs::remove_file(&p).map_err(|e| format!("Failed to remove file: {}", e))?;
    }
    Ok(json!({ "success": true }))
}

fn tool_delete_directory(args: &Value, sb: &Sandbox) -> Result<Value, String> {
    let path = arg_str(args, "path").ok_or("Path is required")?;
    let recursive = arg_bool(args, "recursive");
    let p = sb.resolve(path).ok_or("Path escapes sandbox")?;
    if !p.exists() {
        return Err("Directory not found".to_string());
    }
    if recursive {
        std::fs::remove_dir_all(&p).map_err(|e| format!("Failed to remove dir: {}", e))?;
    } else {
        let count = std::fs::read_dir(&p).map(|r| r.count()).unwrap_or(0);
        if count > 0 {
            return Err("Directory not empty — set recursive=true".to_string());
        }
        std::fs::remove_dir(&p).map_err(|e| format!("Failed to remove dir: {}", e))?;
    }
    Ok(json!({ "success": true }))
}

fn tool_move_file(args: &Value, sb: &Sandbox) -> Result<Value, String> {
    let source = arg_str(args, "source").ok_or("Source is required")?;
    let destination = arg_str(args, "destination").ok_or("Destination is required")?;
    let src = sb.resolve(source).ok_or("Path escapes sandbox")?;
    let dst = sb.resolve_for_write(destination).ok_or("Path escapes sandbox")?;
    if !src.exists() {
        return Err("Source not found".to_string());
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent dirs: {}", e))?;
    }
    std::fs::rename(&src, &dst).map_err(|e| format!("Failed to move: {}", e))?;
    Ok(json!({ "success": true }))
}

fn tool_run_command(args: &Value, sb: &Sandbox) -> Result<Value, String> {
    let command = arg_str(args, "command").ok_or("Command is required")?;
    let args_arr: Vec<String> = args
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    let cwd_rel = arg_str(args, "cwd").unwrap_or(".");
    let cwd = sb.resolve(cwd_rel).ok_or("Working directory escapes sandbox")?;
    if !cwd.exists() {
        return Err("Working directory not found".to_string());
    }
    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(30_000)
        .clamp(1_000, 300_000);

    let mut cmd = std::process::Command::new(command);
    cmd.args(&args_arr)
        .current_dir(&cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // Запуск с таймаутом через поток
    let started = std::time::Instant::now();
    let result: Result<Value, String> = tokio::task::block_in_place(|| {
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return Ok(json!({ "stdout": "", "stderr": format!("Failed to spawn: {}", e), "exit_code": 1 })),
        };

        // Забираем дескрипторы через mem::take, чтобы не двигать поля структуры
        // и оставить child пригодным для try_wait().
        let (out_rd, err_rd) = match (std::mem::take(&mut child.stdout), std::mem::take(&mut child.stderr)) {
            (Some(o), Some(e)) => (o, e),
            _ => return Ok(json!({ "stdout": "", "stderr": "pipe error", "exit_code": 1 })),
        };

        // Чтение stdout/stderr (ограничим 10 MB)
        let stdout_read = std::thread::spawn(move || {
            use std::io::Read;
            let mut buf = Vec::new();
            let _ = out_rd.take(10 * 1024 * 1024).read_to_end(&mut buf);
            String::from_utf8_lossy(&buf).to_string()
        });
        let stderr_read = std::thread::spawn(move || {
            use std::io::Read;
            let mut buf = Vec::new();
            let _ = err_rd.take(10 * 1024 * 1024).read_to_end(&mut buf);
            String::from_utf8_lossy(&buf).to_string()
        });

        // Ожидание с таймаутом
        let deadline = started + std::time::Duration::from_millis(timeout_ms);
        let status;
        loop {
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                status = None;
                break;
            }
            if let Ok(Some(st)) = child.try_wait() {
                status = Some(st);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        let stdout = stdout_read.join().unwrap_or_default();
        let stderr = stderr_read.join().unwrap_or_default();

        match status {
            Some(st) => {
                let code = st.code().unwrap_or(1);
                Ok(json!({ "stdout": stdout, "stderr": stderr, "exit_code": code }))
            }
            None => Ok(json!({
                "stdout": stdout,
                "stderr": format!("Command timed out after {}ms\n{}", timeout_ms, stderr),
                "exit_code": 1
            })),
        }
    });

    result.map_err(|e| format!("Command error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::Sandbox;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn make_sandbox() -> Sandbox {
        let n = DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("mcp-fs-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Sandbox::from_env_with_test_root(dir).unwrap()
    }

    /// Извлекает inner-text из результата call_tool и возвращает его как строку.
    fn result_text(res: &Value) -> String {
        res["content"][0]["text"].as_str().unwrap_or("").to_string()
    }

    /// Вызывает инструмент и возвращает распарсенный JSON из текстового поля.
    fn call_json(name: &str, args: &Value, sb: &Sandbox) -> serde_json::Value {
        let res = call_tool(name, args, sb).expect("tool should not error");
        serde_json::from_str(&result_text(&res)).expect("result should be JSON")
    }

    #[test]
    fn read_write_roundtrip() {
        let sb = make_sandbox();
        let w = call_json("write_file", &json!({"path": "dir/a.txt", "content": "hello"}), &sb);
        assert_eq!(w["success"], true);
        assert!(w["size"].as_u64().unwrap() >= 5);

        let r = call_json("read_file", &json!({"path": "dir/a.txt"}), &sb);
        assert_eq!(r["content"], "hello");
        assert_eq!(r["encoding"], "text");
    }

    #[test]
    fn edit_file_replaces_unique_string() {
        let sb = make_sandbox();
        call_tool("write_file", &json!({"path": "f.txt", "content": "a X b X c"}), &sb).unwrap();
        let e = call_json("edit_file", &json!({"path": "f.txt", "old_string": "X", "new_string": "Y"}), &sb);
        // Rust replace() заменяет все вхождения (в отличие от TS). Проверяем оба.
        assert!(e["changes"].as_u64().unwrap() >= 1);
        let r = call_json("read_file", &json!({"path": "f.txt"}), &sb);
        assert_eq!(r["content"], "a Y b Y c");
    }

    #[test]
    fn edit_file_not_found_returns_error() {
        let sb = make_sandbox();
        call_tool("write_file", &json!({"path": "f.txt", "content": "abc"}), &sb).unwrap();
        let res = call_tool("edit_file", &json!({"path": "f.txt", "old_string": "zzz", "new_string": "y"}), &sb).unwrap();
        let t = result_text(&res);
        assert!(t.contains("String not found"));
    }

    #[test]
    fn list_directory_and_file_info() {
        let sb = make_sandbox();
        call_tool("write_file", &json!({"path": "sub/a.txt", "content": "x"}), &sb).unwrap();
        let l = call_json("list_directory", &json!({"path": "."}), &sb);
        assert!(l.to_string().contains("sub"));
        let fi = call_json("file_info", &json!({"path": "sub/a.txt"}), &sb);
        assert_eq!(fi["exists"], true);
        assert_eq!(fi["type"], "file");
        let fi2 = call_json("file_info", &json!({"path": "missing.txt"}), &sb);
        assert_eq!(fi2["exists"], false);
    }

    #[test]
    fn search_files_glob() {
        let sb = make_sandbox();
        call_tool("write_file", &json!({"path": "a.bsl", "content": ""}), &sb).unwrap();
        call_tool("write_file", &json!({"path": "b.txt", "content": ""}), &sb).unwrap();
        call_tool("write_file", &json!({"path": "sub/c.bsl", "content": ""}), &sb).unwrap();
        let s = call_json("search_files", &json!({"pattern": "*.bsl"}), &sb);
        let files = s["files"].as_array().unwrap();
        let joined = serde_json::to_string(files).unwrap();
        assert!(joined.contains("a.bsl"));
        assert!(joined.contains("sub/c.bsl"));
        assert!(!joined.contains("b.txt"));
    }

    #[test]
    fn move_and_delete() {
        let sb = make_sandbox();
        call_tool("write_file", &json!({"path": "a.txt", "content": "x"}), &sb).unwrap();
        call_json("move_file", &json!({"source": "a.txt", "destination": "b.txt"}), &sb);
        let fi = call_json("file_info", &json!({"path": "b.txt"}), &sb);
        assert_eq!(fi["exists"], true);
        call_json("delete_file", &json!({"path": "b.txt"}), &sb);
        let fi2 = call_json("file_info", &json!({"path": "b.txt"}), &sb);
        assert_eq!(fi2["exists"], false);
    }

    #[test]
    fn sandbox_escape_rejected() {
        let sb = make_sandbox();
        let r = call_tool("read_file", &json!({"path": "../etc/passwd"}), &sb).unwrap();
        assert!(result_text(&r).contains("Path escapes sandbox"));
        let r2 = call_tool("write_file", &json!({"path": "../../outside", "content": "x"}), &sb).unwrap();
        assert!(result_text(&r2).contains("Path escapes sandbox"));
    }

    #[test]
    fn run_command_basic() {
        let sb = make_sandbox();
        let res = call_tool("run_command", &json!({"command": "echo", "args": ["hello"]}), &sb).unwrap();
        let j: serde_json::Value = serde_json::from_str(&result_text(&res)).expect("json");
        assert_eq!(j["exit_code"], 0);
        assert!(j["stdout"].as_str().unwrap_or("").contains("hello"));
    }
}
