//! Определения MCP-инструментов и их обработчики.
//!
//! Порт mcp-skills.ts: 9 инструментов — list_skills, get_skill, get_skill_file,
//! search_skills, list_docs, get_doc, search_docs, list_rules, get_rule.

use serde_json::{json, Value};
use std::path::Path;

use crate::search::{Bm25, DocKind};
use crate::skills::{
    doc_path_from_id, get_skill_files, is_valid_skill_id, read_skill_file, rule_path_from_id,
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
            "description": "Получить полное содержимое SKILL.md + список файлов скилла. Содержимое файлов (скрипты, документы) читай через отдельный инструмент get_skill_file. ВАЖНО: вызывай этот инструмент ОДИН раз для каждого скилла и запоминай полученный контент. Если ты уже получал SKILL.md для этого скилла earlier в сессии — НЕ вызывай повторно, используй уже прочитанное.",
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
            "name": "get_skill_file",
            "description": "Прочитать содержимое конкретного файла скилла (PS1-скрипт, документация и т.д.). Сначала вызови get_skill чтобы увидеть список доступных файлов, затем вызови этот инструмент с путём.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "ID скилла (получи список доступных через list_skills)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Относительный путь к файлу внутри скилла (получи список файлов через get_skill)"
                    }
                },
                "required": ["id", "path"]
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
    format!("### {}\nID: `{}`\n{}{}{}{}", s.name, s.id, s.description, hint, tools, cat)
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

            let files = get_skill_files(skills_dir, id);
            let non_md_files: Vec<_> = files
                .iter()
                .filter(|f| *f != "SKILL.md" && !f.ends_with(".exe") && !f.ends_with(".pyc"))
                .collect();

            let mut parts = vec![
                format!("# {}", info.name),
                String::new(),
                format!("**ID:** `{}`", info.id),
                format!("**Описание:** {}", info.description),
                format!("**Директория скилла:** `{}`", skill_dir.to_string_lossy()),
                format!("**Файлов:** {}", files.len()),
                String::new(),
                "---".to_string(),
                String::new(),
                fm.body,
            ];
            if !non_md_files.is_empty() {
                parts.push(String::new());
                parts.push("---".to_string());
                parts.push("## Доступные файлы".to_string());
                parts.push(String::new());
                parts.push(format!(
                    "Для чтения содержимого файлов используй инструмент `get_skill_file` с параметрами `id` = `{}` и `path` = относительный путь.",
                    id
                ));
                parts.push(String::new());
                for f in &non_md_files {
                    parts.push(format!("- `{}`", f));
                }
            }
            parts.join("\n")
        }

        "get_skill_file" => {
            let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let file_path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() || file_path.is_empty() {
                return Err("Parameters \"id\" and \"path\" are required".to_string());
            }
            if !is_valid_skill_id(id) {
                return Err("Invalid skill id".to_string());
            }
            if file_path.contains("..")
                || file_path.starts_with('/')
                || file_path.starts_with('\\')
            {
                return Err("Invalid file path".to_string());
            }
            let content = read_skill_file(skills_dir, id, file_path)
                .ok_or_else(|| format!("File \"{}\" not found in skill \"{}\"", file_path, id))?;
            format!("### {}\n\n{}", file_path, content)
        }

        "search_skills" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let hits = match bm25 {
                Some(bm) => bm.search(query, Some(&[DocKind::Skill]), 20),
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
                Some(bm) => bm.search(query, Some(&[DocKind::Doc]), 20),
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
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(10) as usize;
            let hits = match bm25 {
                Some(bm) => bm.search(query, kinds.as_deref(), limit),
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
                        format!(
                            "### {} {} (score {:.1})\nID: `{}`\n{}{}",
                            kind_icon, h.name, h.score, h.id, h.description, cat
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

        _ => return Err(format!("Unknown tool: {}", name)),
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": text
        }]
    }))
}
