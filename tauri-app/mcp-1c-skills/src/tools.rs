//! Определения MCP-инструментов и их обработчики.
//!
//! Порт mcp-skills.ts: инструменты — list_skills, get_skill, search_skills,
//! run_skill, list_docs, get_doc, search_docs, search, list_rules, get_rule.
//! get_skill_file удалён (путаница путей — скрипты запускаются через run_skill).

use serde_json::{json, Value};
use std::path::Path;

use crate::search::{Bm25, DocKind};
use crate::skills::{
    args_to_flat_list, doc_path_from_id, is_valid_skill_id, resolve_skill_script, rule_path_from_id,
    scan_docs, scan_rules, scan_skills, skill_dir_from_id, SkillInfo,
};

/// Возвращает список инструментов для tools/list.
pub fn list_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "list_skills",
            "description": "Получить список всех доступных скиллов (наборов знаний и инструкций). Каждый скилл — это структурированное руководство по конкретной технологии или подходу: Rust, TypeScript, дизайн UI, Tauri, MCP и т.д.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Фильтр по категории (опционально)"
                    }
                }
            }
        }),
        json!({
            "name": "get_skill",
            "description": "Получить полное содержимое SKILL.md скилла + список файлов. ВАЖНО: вызывай этот инструмент ОДИН раз для каждого скилла и запоминай полученный контент. Если ты уже получал SKILL.md для этого скилла earlier в сессии — НЕ вызывай повторно, используй уже прочитанное. Скрипты скилла (.ps1/.py) НЕ читай через файловые инструменты — запускай их через run_skill(id, args).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "ID скилла (получи список доступных через list_skills)"
                    }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "search",
            "description": "Поиск по скиллам, документации и правилам 1С. BM25-ранжирование: наиболее релевантные результаты первыми. Используй для быстрого поиска нужного скилла, правила или документа по описанию задачи. Запрос может быть многословным на русском или английском.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Поисковый запрос (например: 'создать обработку с формой и макетом')"
                    },
                    "kinds": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["skill", "doc", "rule"] },
                        "description": "Фильтр по типам. Если не указан — поиск по всем трём коллекциям."
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Фильтр по тегам скиллов (например: ['epf', 'forms', 'mxl', 'skd', 'bsp', 'db', 'query', 'template', 'workflow']). Учитывается только для kinds=['skill']."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Максимум результатов (по умолчанию 10)"
                    }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "search_skills",
            "description": "Поиск по названиям и описаниям скиллов (BM25). Вернёт список подходящих скиллов с их ID, описанием и форматом вызова.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Поисковый запрос"
                    }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "list_docs",
            "description": "Получить список всех доступных документов (документация по 1С: паттерны, соглашения, справочники). Вернёт ID, название и описание каждого документа.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Фильтр по категории (опционально)"
                    }
                }
            }
        }),
        json!({
            "name": "run_skill",
            "description": "Выполнить скрипт скилла (PowerShell .ps1 на Windows, Python .py на Linux). Сервер сам находит скрипт в каталоге скилла и запускает его с переданными аргументами. ОДИН вызов вместо run_command с ручной сборкой пути. Возвращает stdout/stderr/exit_code.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "ID скилла (получи список доступных через list_skills)"
                    },
                    "args": {
                        "type": "object",
                        "description": "Аргументы для скрипта, ключ-значение (напр. {\"-SourceFile\": \"src/Моя.xml\", \"-OutputFile\": \"build/Моя.epf\"})"
                    },
                    "timeout_ms": {
                        "type": "number",
                        "description": "Таймаут в мс (по умолчанию 60000, максимум 300000)"
                    }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "get_doc",
            "description": "Получить полное содержимое документа по его ID. Используй для чтения документации по 1С (паттерны, справочники, соглашения).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "ID документа (получи список доступных через list_docs)"
                    }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "search_docs",
            "description": "Поиск по названиям и описаниям документов (BM25). Вернёт список подходящих документов с их ID и описанием.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Поисковый запрос"
                    }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "list_rules",
            "description": "Получить список всех доступных правил кодирования 1С (стандарты, соглашения). Вернёт ID, название и описание каждого правила.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "get_rule",
            "description": "Получить полное содержимое правила кодирования по его ID. Используй для чтения стандартов и соглашений 1С.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "ID правила (получи список доступных через list_rules)"
                    }
                },
                "required": ["id"]
            }
        }),
    ]
}

fn skill_listing_md(s: &SkillInfo) -> String {
    let hint = s
        .argument_hint
        .as_deref()
        .map(|h| format!("\nВызов: `{}`", h))
        .unwrap_or_default();
    let tools = if s.allowed_tools.is_empty() {
        String::new()
    } else {
        format!("\nТребует: {}", s.allowed_tools.join(", "))
    };
    let cat = s
        .category
        .as_deref()
        .map(|c| format!("\nКатегория: {}", c))
        .unwrap_or_default();
    let tags = if s.tags.is_empty() {
        String::new()
    } else {
        format!("\nТеги: {}", s.tags.join(", "))
    };
    format!("### {}\nID: `{}`\n{}{}{}{}{}", s.name, s.id, s.description, hint, tools, tags, cat)
}

/// Обрабатывает tools/call. Возвращает содержимое ответа MCP.
pub fn call_tool(
    name: &str,
    args: &Value,
    skills_dir: &Path,
    bm25: Option<&Bm25>,
) -> Result<Value, String> {
    let text = match name {
        "list_skills" => {
            let category = args.get("category").and_then(|v| v.as_str()).unwrap_or("");
            let all = scan_skills(skills_dir);
            let filtered: Vec<_> = if category.is_empty() {
                all
            } else {
                let cl = category.to_lowercase();
                all.into_iter()
                    .filter(|s| s.category.as_deref().unwrap_or("").to_lowercase().contains(&cl))
                    .collect()
            };
            if filtered.is_empty() {
                "Скиллы не найдены.".to_string()
            } else {
                filtered.iter().map(skill_listing_md).collect::<Vec<_>>().join("\n\n")
            }
        }

        "get_skill" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                return Err("Parameter \"id\" is required".to_string());
            }
            if !is_valid_skill_id(id) {
                return Err("Invalid skill id".to_string());
            }
            let info = scan_skills(skills_dir).into_iter().find(|s| s.id == id)
                .ok_or_else(|| format!("Skill \"{}\" not found", id))?;
            let skill_dir = skill_dir_from_id(skills_dir, id)
                .ok_or_else(|| "Invalid skill path".to_string())?;
            let skill_path = skill_dir.join("SKILL.md");
            let raw = std::fs::read_to_string(&skill_path)
                .map_err(|_| format!("Skill \"{}\" not found", id))?;
            let fm = crate::skills::parse_skill_frontmatter(&raw);

            let mut parts = vec![
                format!("# {}", info.name),
                String::new(),
                format!("**ID:** `{}`", info.id),
                format!("**Описание:** {}", info.description),
            ];
            if let Some(hint) = info.argument_hint.as_deref() {
                parts.push(format!("**Вызов:** `{}`", hint));
            }
            if !info.allowed_tools.is_empty() {
                parts.push(format!("**Требует:** {}", info.allowed_tools.join(", ")));
            }
            if !info.tags.is_empty() {
                parts.push(format!("**Теги:** {}", info.tags.join(", ")));
            }
            if !info.depends_on.is_empty() {
                parts.push(format!("**Зависит от:** {}", info.depends_on.join(", ")));
            }
            parts.push(format!("**Директория скилла:** `{}`", skill_dir.to_string_lossy()));
            parts.push(String::new());
            parts.push("---".to_string());
            parts.push(String::new());
            parts.push(fm.body);
            parts.join("\n")
        }

        "search_skills" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let hits = match bm25 {
                Some(bm) => bm.search(query, Some(&[DocKind::Skill]), None, 20),
                None => Vec::new(),
            };
            if hits.is_empty() {
                format!("По запросу \"{}\" ничего не найдено.", query)
            } else {
                hits.iter()
                    .map(|h| {
                        let tags = if h.tags.is_empty() {
                            String::new()
                        } else {
                            format!("\nТеги: {}", h.tags.join(", "))
                        };
                        format!("### {}\nID: `{}`\n{}{}", h.name, h.id, h.description, tags)
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n")
            }
        }

        "list_docs" => {
            let category = args.get("category").and_then(|v| v.as_str()).unwrap_or("");
            let all = scan_docs(skills_dir);
            let filtered: Vec<_> = if category.is_empty() {
                all
            } else {
                let cl = category.to_lowercase();
                all.into_iter()
                    .filter(|d| d.category.as_deref().unwrap_or("").to_lowercase().contains(&cl))
                    .collect()
            };
            if filtered.is_empty() {
                "Документы не найдены.".to_string()
            } else {
                filtered
                    .iter()
                    .map(|d| {
                        let cat = d
                            .category
                            .as_deref()
                            .map(|c| format!("\nКатегория: {}", c))
                            .unwrap_or_default();
                        format!("### {}\nID: `{}`\n{}{}", d.name, d.id, d.description, cat)
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n")
            }
        }

        "get_doc" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                return Err("Parameter \"id\" is required".to_string());
            }
            if !is_valid_skill_id(id) {
                return Err("Invalid doc id".to_string());
            }
            let path = doc_path_from_id(skills_dir, id)
                .ok_or_else(|| format!("Document \"{}\" not found", id))?;
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read document \"{}\": {}", id, e))?;
            format!("### {}\n\n{}", id, content)
        }

        "search_docs" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let hits = match bm25 {
                Some(bm) => bm.search(query, Some(&[DocKind::Doc]), None, 20),
                None => Vec::new(),
            };
            if hits.is_empty() {
                format!("По запросу \"{}\" ничего не найдено.", query)
            } else {
                hits.iter()
                    .map(|h| format!("### {}\nID: `{}`\n{}", h.name, h.id, h.description))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            }
        }

        "search" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let kinds: Option<Vec<DocKind>> = args
                .get("kinds")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().and_then(DocKind::from_str))
                        .collect()
                });
            let tags: Option<Vec<String>> = args
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                });
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(10) as usize;
            let hits = match bm25 {
                Some(bm) => bm.search(query, kinds.as_deref(), tags.as_deref(), limit),
                None => Vec::new(),
            };
            if hits.is_empty() {
                format!("По запросу \"{}\" ничего не найдено.", query)
            } else {
                hits.iter()
                    .map(|h| {
                        let kind_icon = match h.kind {
                            DocKind::Skill => "🔧",
                            DocKind::Doc => "📄",
                            DocKind::Rule => "📏",
                        };
                        let cat = h
                            .category
                            .as_deref()
                            .map(|c| format!("\nКатегория: {}", c))
                            .unwrap_or_default();
                        let tags = if h.tags.is_empty() {
                            String::new()
                        } else {
                            format!("\nТеги: {}", h.tags.join(", "))
                        };
                        format!(
                            "### {} {} (score {:.1})\nID: `{}`\n{}{}{}",
                            kind_icon, h.name, h.score, h.id, h.description, cat, tags
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n")
            }
        }

        "list_rules" => {
            let all = scan_rules(skills_dir);
            if all.is_empty() {
                "Правила не найдены.".to_string()
            } else {
                all.iter().map(|r| format!("### {}\nID: `{}`\n{}", r.name, r.id, r.description)).collect::<Vec<_>>().join("\n\n")
            }
        }

        "get_rule" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                return Err("Parameter \"id\" is required".to_string());
            }
            if !is_valid_skill_id(id) {
                return Err("Invalid rule id".to_string());
            }
            let path = rule_path_from_id(skills_dir, id)
                .ok_or_else(|| format!("Rule \"{}\" not found", id))?;
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read rule \"{}\": {}", id, e))?;
            format!("### {}\n\n{}", id, content)
        }

        "run_skill" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                return Err("Parameter \"id\" is required".to_string());
            }
            if !is_valid_skill_id(id) {
                return Err("Invalid skill id".to_string());
            }
            let (script, runtime) = resolve_skill_script(skills_dir, id)
                .ok_or_else(|| format!("Skill \"{}\" не имеет исполняемого скрипта (scripts/*.ps1 или *.py)", id))?;
            let flat = args_to_flat_list(args.get("args").unwrap_or(&json!({})));
            let timeout_ms = args
                .get("timeout_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(60_000);

            let cwd = skill_dir_from_id(skills_dir, id)
                .ok_or_else(|| "Invalid skill path".to_string())?;

            let mut cmd_args: Vec<String> = Vec::new();
            #[cfg(windows)]
            {
                if runtime == "powershell.exe" {
                    cmd_args.push("-NoProfile".to_string());
                    cmd_args.push("-NonInteractive".to_string());
                    cmd_args.push("-File".to_string());
                    cmd_args.push(script.to_string_lossy().to_string());
                    cmd_args.extend(flat);
                } else {
                    cmd_args.push(script.to_string_lossy().to_string());
                    cmd_args.extend(flat);
                }
            }
            #[cfg(not(windows))]
            {
                if runtime == "python3" {
                    cmd_args.push(script.to_string_lossy().to_string());
                    cmd_args.extend(flat);
                } else {
                    cmd_args.push("-NoProfile".to_string());
                    cmd_args.push("-NonInteractive".to_string());
                    cmd_args.push("-File".to_string());
                    cmd_args.push(script.to_string_lossy().to_string());
                    cmd_args.extend(flat);
                }
            }

            let out = execute_command(&runtime, &cmd_args, &cwd, timeout_ms);
            format!(
                "{{ \"stdout\": {:?}, \"stderr\": {:?}, \"exit_code\": {}, \"duration_ms\": {} }}",
                out.stdout, out.stderr, out.exit_code, out.duration_ms
            )
        }

        _ => return Err(format!("Unknown tool: {}", name)),
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": text
        }]
    }))
}

/// Результат выполнения внешней команды.
struct CommandOutput {
    stdout: String,
    stderr: String,
    exit_code: i32,
    duration_ms: u64,
}

/// Выполняет внешнюю команду с таймаутом. На Windows для PowerShell форсирует
/// UTF-8 в stdout (иначе кириллица из OEM-кодовой страницы ломается).
fn execute_command(
    command: &str,
    args: &[String],
    cwd: &std::path::Path,
    timeout_ms: u64,
) -> CommandOutput {
    let timeout_ms = timeout_ms.clamp(1_000, 300_000);
    let started = std::time::Instant::now();

    tokio::task::block_in_place(|| {
        let mut cmd = build_command(command, args, cwd);
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return CommandOutput {
                    stdout: String::new(),
                    stderr: format!("Failed to spawn: {}", e),
                    exit_code: 1,
                    duration_ms: started.elapsed().as_millis() as u64,
                }
            }
        };

        let (out_rd, err_rd) = match (std::mem::take(&mut child.stdout), std::mem::take(&mut child.stderr)) {
            (Some(o), Some(e)) => (o, e),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return CommandOutput {
                    stdout: String::new(),
                    stderr: "pipe error".to_string(),
                    exit_code: 1,
                    duration_ms: started.elapsed().as_millis() as u64,
                };
            }
        };

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
            Some(st) => CommandOutput {
                stdout,
                stderr,
                exit_code: st.code().unwrap_or(1),
                duration_ms: started.elapsed().as_millis() as u64,
            },
            None => CommandOutput {
                stdout,
                stderr: format!("Command timed out after {}ms\n{}", timeout_ms, stderr),
                exit_code: 1,
                duration_ms: started.elapsed().as_millis() as u64,
            },
        }
    })
}

/// Собирает `Command`. На Windows для PowerShell форсирует UTF-8.
fn build_command(command: &str, args: &[String], cwd: &std::path::Path) -> std::process::Command {
    #[cfg(windows)]
    let is_powershell = command == "powershell.exe";

    #[cfg(windows)]
    {
        if is_powershell {
            let ps_quote = |s: &str| -> String {
                format!("'{}'", s.replace('\'', "''"))
            };
            let mut inner = ps_quote(command);
            for a in args {
                inner.push(' ');
                inner.push_str(&ps_quote(a));
            }
            let script = format!(
                "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; & {}",
                inner
            );
            let mut cmd = std::process::Command::new("powershell.exe");
            cmd.arg("-NoProfile")
                .arg("-NonInteractive")
                .arg("-Command")
                .arg(script)
                .current_dir(cwd)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            return cmd;
        }
    }

    let mut cmd = std::process::Command::new(command);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn tmp_skills_dir() -> std::path::PathBuf {
        let n = DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("mcp-skills-run-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        let skills_root = dir.join(".agents").join("skills");
        std::fs::create_dir_all(&skills_root).unwrap();
        dir
    }

    fn result_text(res: &Value) -> String {
        res["content"][0]["text"].as_str().unwrap_or("").to_string()
    }

    #[test]
    fn run_skill_executes_script_and_returns_stdout() {
        let dir = tmp_skills_dir();
        let skills_root = dir.join(".agents").join("skills");
        let skill_dir = skills_root.join("1c-hello");
        std::fs::create_dir_all(&skill_dir.join("scripts")).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "---\nname: 1c-hello\n---\n").unwrap();
        #[cfg(windows)]
        std::fs::write(
            skill_dir.join("scripts").join("hello.ps1"),
            "Write-Output 'Привет из скилла'",
        ).unwrap();
        #[cfg(unix)]
        std::fs::write(
            skill_dir.join("scripts").join("hello.py"),
            "print('Привет из скилла')",
        ).unwrap();

        let res = call_tool("run_skill", &json!({"id": "1c-hello"}), &skills_root, None).unwrap();
        let text = result_text(&res);
        assert!(
            text.contains("Привет из скилла"),
            "stdout: {:?}",
            text
        );
        assert!(text.contains("\"exit_code\": 0"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn run_skill_no_scripts_returns_error() {
        let dir = tmp_skills_dir();
        let skills_root = dir.join(".agents").join("skills");
        let skill_dir = skills_root.join("1c-emptyskill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "---\nname: 1c-emptyskill\n---\n").unwrap();

        let res = call_tool("run_skill", &json!({"id": "1c-emptyskill"}), &skills_root, None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("не имеет исполняемого скрипта"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn run_skill_missing_skill_returns_error() {
        let dir = tmp_skills_dir();
        let skills_root = dir.join(".agents").join("skills");
        let res = call_tool("run_skill", &json!({"id": "nope"}), &skills_root, None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("не имеет исполняемого скрипта"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_tools_has_no_get_skill_file_and_has_run_skill() {
        let tools = list_tools();
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(!names.contains(&"get_skill_file"), "get_skill_file должен быть удалён");
        assert!(names.contains(&"run_skill"), "run_skill должен присутствовать");
    }
}
