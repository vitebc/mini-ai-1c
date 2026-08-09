//! Сканирование скиллов, документации и правил кодирования 1С.
//!
//! Порт mcp-skills.ts (TypeScript) на Rust. Логика идентична:
//! - скиллы: `.agents/skills/` (плоские или `категория/скилл/SKILL.md`);
//! - документы: `.agents/docs/**/*.md`;
//! - правила: `.agents/rules/**/*.md`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// ─── Структуры данных ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argument_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: String,
}

// ─── Резолвинг путей ──────────────────────────────────────────────

fn exists_dir(p: &Path) -> bool {
    p.is_dir()
}

/// Определяет каталог скиллов.
///
/// Приоритет:
/// 1. Переменная окружения `SKILLS_DIR`.
/// 2. Поиск `.agents/skills` относительно текущей директории и родительских.
pub fn resolve_skills_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("SKILLS_DIR") {
        let p = PathBuf::from(dir);
        if exists_dir(&p) {
            return Some(p);
        }
    }

    let cwd = std::env::current_dir().ok()?;
    for base in [
        &cwd,
        &cwd.join(".."),
        &cwd.join("..").join(".."),
    ] {
        let candidate = base.join(".agents").join("skills");
        if exists_dir(&candidate) {
            return Some(candidate);
        }
    }

    // Дополнительно: домашний каталог .config/mini-ai-1c/.agents/skills
    if let Some(home) = dirs::home_dir() {
        let candidate = home
            .join(".config")
            .join("mini-ai-1c")
            .join(".agents")
            .join("skills");
        if exists_dir(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn agents_dir(skills_dir: &Path) -> PathBuf {
    skills_dir.join("..")
}

fn docs_dir(skills_dir: &Path) -> Option<PathBuf> {
    let d = agents_dir(skills_dir).join("docs");
    if exists_dir(&d) {
        Some(d)
    } else {
        None
    }
}

fn rules_dir(skills_dir: &Path) -> Option<PathBuf> {
    let d = agents_dir(skills_dir).join("rules");
    if exists_dir(&d) {
        Some(d)
    } else {
        None
    }
}

// ─── Frontmatter ──────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct Frontmatter {
    pub metadata: serde_json::Map<String, serde_json::Value>,
    pub body: String,
}

/// Парсит YAML-подобный frontmatter `---\nkey: value\n---\nbody`.
///
/// Поддерживает `key: value`, `key: "true"/"false"`, числовые значения.
/// Не YAML-парсер — совпадает с логикой TypeScript-версии.
/// Устойчив к CRLF (Windows): `---\r\n`, `key: value\r\n` обрабатываются.
pub fn parse_skill_frontmatter(content: &str) -> Frontmatter {
    let mut metadata = serde_json::Map::new();
    let normalized = content.replace("\r\n", "\n");
    if !normalized.starts_with("---\n") {
        return Frontmatter {
            metadata,
            body: content.to_string(),
        };
    }

    if let Some(end_rel) = normalized[4..].find("\n---\n") {
        let end = 4 + end_rel;
        let fm = &normalized[4..end];
        for line in fm.split('\n') {
            if let Some(idx) = line.find(": ") {
                let key = line[..idx].trim().to_string();
                let raw = line[idx + 2..].trim();
                let val = parse_frontmatter_value(raw);
                metadata.insert(key, val);
            }
        }
        let body = normalized[end + 5..].trim().to_string();
        Frontmatter { metadata, body }
    } else {
        Frontmatter {
            metadata,
            body: content.to_string(),
        }
    }
}

fn parse_frontmatter_value(raw: &str) -> serde_json::Value {
    if raw == "true" {
        serde_json::Value::Bool(true)
    } else if raw == "false" {
        serde_json::Value::Bool(false)
    } else if let Ok(n) = raw.parse::<i64>() {
        serde_json::Value::Number(n.into())
    } else if let Ok(f) = raw.parse::<f64>() {
        serde_json::Value::Number(serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0)))
    } else {
        serde_json::Value::String(raw.to_string())
    }
}

fn metadata_str(m: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    m.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Разбивает строку-список (`tags: a, b, c`) на вектор, обрезая пробелы.
fn split_comma_list(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

// ─── Скиллы ───────────────────────────────────────────────────────

/// Проверяет валидность ID скилла (защита от path traversal).
pub fn is_valid_skill_id(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    if id.starts_with('/') || id.starts_with('\\') {
        return false;
    }
    if id.contains("..") {
        return false;
    }
    true
}

/// Сканирует все скиллы.
pub fn scan_skills(skills_dir: &Path) -> Vec<SkillInfo> {
    let mut skills = Vec::new();

    let read_skill = |category: &str, skill_dir: &Path, name: &str, out: &mut Vec<SkillInfo>| {
        let sp = skill_dir.join("SKILL.md");
        if !sp.exists() {
            return;
        }
        if let Ok(raw) = std::fs::read_to_string(&sp) {
            let fm = parse_skill_frontmatter(&raw);
            let id = if category.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", category, name)
            };
            let md_name = metadata_str(&fm.metadata, "name");
            let desc = metadata_str(&fm.metadata, "description");
            let cat = {
                let c = metadata_str(&fm.metadata, "category");
                if !c.is_empty() {
                    c
                } else {
                    metadata_str(&fm.metadata, "domain")
                }
            };
            let arg_hint = {
                let v = metadata_str(&fm.metadata, "argument-hint");
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            };
            let allowed = match fm.metadata.get("allowed-tools") {
                Some(serde_json::Value::Array(arr)) => arr
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
                _ => Vec::new(),
            };
            let tags = split_comma_list(&metadata_str(&fm.metadata, "tags"));
            let depends_on = split_comma_list(&metadata_str(&fm.metadata, "depends_on"));
            out.push(SkillInfo {
                id,
                name: if md_name.is_empty() { name.to_string() } else { md_name },
                description: desc,
                category: if category.is_empty() {
                    if cat.is_empty() {
                        None
                    } else {
                        Some(cat)
                    }
                } else {
                    Some(category.to_string())
                },
                argument_hint: arg_hint,
                allowed_tools: allowed,
                tags,
                depends_on,
            });
        }
    };

    if let Ok(entries) = std::fs::read_dir(skills_dir) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let full = entry.path();
            // Case 1: плоский скилл (SKILL.md прямо здесь)
            if full.join("SKILL.md").exists() {
                read_skill("", &full, &name, &mut skills);
                continue;
            }
            // Case 2: категория с под-скиллами
            if let Ok(subs) = std::fs::read_dir(&full) {
                for sub in subs.flatten() {
                    if !sub.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        continue;
                    }
                    let sub_name = sub.file_name().to_string_lossy().to_string();
                    read_skill(&name, &sub.path(), &sub_name, &mut skills);
                }
            }
        }
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// Возвращает каталог скилла по ID (с проверкой валидности).
pub fn skill_dir_from_id(skills_dir: &Path, id: &str) -> Option<PathBuf> {
    if !is_valid_skill_id(id) {
        return None;
    }
    let p = skills_dir.join(id);
    if p.is_dir() {
        Some(p)
    } else {
        None
    }
}

/// Список файлов скилла (относительные пути), исключая node_modules и скрытые.
pub fn get_skill_files(skills_dir: &Path, skill_id: &str) -> Vec<String> {
    let Some(sd) = skill_dir_from_id(skills_dir, skill_id) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for entry in WalkDir::new(&sd).min_depth(1) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.file_type().is_dir() {
            let n = entry.file_name().to_string_lossy().to_string();
            if n == "node_modules" || n.starts_with('.') {
                continue;
            }
            continue;
        }
        if let Ok(rel) = entry.path().strip_prefix(&sd) {
            files.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    files.sort();
    files
}

/// Читает произвольный файл скилла (с защитой от path traversal).
pub fn read_skill_file(skills_dir: &Path, skill_id: &str, file_path: &str) -> Option<String> {
    if file_path.contains("..")
        || file_path.starts_with('/')
        || file_path.starts_with('\\')
    {
        return None;
    }
    let sd = skill_dir_from_id(skills_dir, skill_id)?;
    let p = sd.join(file_path);
    if !p.exists() {
        return None;
    }
    std::fs::read_to_string(&p).ok()
}

// ─── Документы и правила ──────────────────────────────────────────

/// Первый непустой абзац для краткого описания (до 200 символов).
pub fn first_paragraph(text: &str) -> String {
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#')
            || t.starts_with("```")
            || t.starts_with("---")
            || t.starts_with('|')
        {
            continue;
        }
        if t.len() >= 2 && t.starts_with('-') && t[1..].starts_with(char::is_whitespace) {
            continue;
        }
        let clipped: String = t.chars().take(200).collect();
        return clipped;
    }
    String::new()
}

fn walk_md_files(root: &Path, out: &mut Vec<(PathBuf, String)>) {
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if entry.file_type().is_dir() {
            let n = entry.file_name().to_string_lossy().to_string();
            if n.starts_with('.') {
                continue;
            }
            continue;
        }
        if path.extension().map(|e| e.to_string_lossy().to_lowercase()) == Some("md".to_string()) {
            out.push((path.to_path_buf(), String::new()));
        }
    }
}

/// Сканирует документы `.agents/docs/**/*.md`.
pub fn scan_docs(skills_dir: &Path) -> Vec<DocInfo> {
    let mut docs = Vec::new();
    let Some(docs_root) = docs_dir(skills_dir) else {
        return docs;
    };

    let mut files = Vec::new();
    walk_md_files(&docs_root, &mut files);

    for (full, _) in files {
        if let Ok(raw) = std::fs::read_to_string(&full) {
            let rel = full.strip_prefix(&docs_root).unwrap_or(&full);
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let id = rel_str.trim_end_matches(".md").to_string();
            let name = full
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let parent = full.parent().and_then(|p| p.file_name());
            let category = if parent.is_some() && parent.unwrap() != docs_root.file_name().unwrap_or_default() {
                parent.unwrap().to_string_lossy().to_string()
            } else {
                String::new()
            };
            docs.push(DocInfo {
                id,
                name,
                description: first_paragraph(&raw),
                category: if category.is_empty() { None } else { Some(category) },
                path: full.to_string_lossy().to_string(),
            });
        }
    }

    docs.sort_by(|a, b| a.id.cmp(&b.id));
    docs
}

/// Сканирует правила `.agents/rules/**/*.md`.
pub fn scan_rules(skills_dir: &Path) -> Vec<RuleInfo> {
    let mut rules = Vec::new();
    let Some(rules_root) = rules_dir(skills_dir) else {
        return rules;
    };

    let mut files = Vec::new();
    walk_md_files(&rules_root, &mut files);

    for (full, _) in files {
        if let Ok(raw) = std::fs::read_to_string(&full) {
            let rel = full.strip_prefix(&rules_root).unwrap_or(&full);
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let id = rel_str.trim_end_matches(".md").to_string();
            let name = full
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            rules.push(RuleInfo {
                id,
                name,
                description: first_paragraph(&raw),
                path: full.to_string_lossy().to_string(),
            });
        }
    }

    rules.sort_by(|a, b| a.id.cmp(&b.id));
    rules
}

/// Находит путь документа по ID.
pub fn doc_path_from_id(skills_dir: &Path, id: &str) -> Option<String> {
    if !is_valid_skill_id(id) {
        return None;
    }
    scan_docs(skills_dir).into_iter().find(|d| d.id == id).map(|d| d.path)
}

/// Находит путь правила по ID.
pub fn rule_path_from_id(skills_dir: &Path, id: &str) -> Option<String> {
    if !is_valid_skill_id(id) {
        return None;
    }
    scan_rules(skills_dir).into_iter().find(|r| r.id == id).map(|r| r.path)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};
    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn tmp_tree() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "mcp-skills-test-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parses_frontmatter() {
        let content = "---\nname: Тест\ndescription: Описание\nenabled: true\ncount: 5\n---\n\nТело";
        let fm = parse_skill_frontmatter(content);
        assert_eq!(fm.body, "Тело");
        assert_eq!(metadata_str(&fm.metadata, "name"), "Тест");
        assert_eq!(metadata_str(&fm.metadata, "description"), "Описание");
        assert_eq!(fm.metadata.get("enabled"), Some(&serde_json::Value::Bool(true)));
        assert_eq!(fm.metadata.get("count"), Some(&serde_json::Value::Number(5.into())));
    }

    #[test]
    fn parses_crlf_frontmatter() {
        let content = "---\r\nname: Тест\r\ndescription: Описание\r\n---\r\n\r\nТело";
        let fm = parse_skill_frontmatter(content);
        assert_eq!(fm.body, "Тело");
        assert_eq!(metadata_str(&fm.metadata, "name"), "Тест");
        assert_eq!(metadata_str(&fm.metadata, "description"), "Описание");
    }

    #[test]
    fn rejects_invalid_ids() {
        assert!(!is_valid_skill_id(""));
        assert!(!is_valid_skill_id("/abs"));
        assert!(!is_valid_skill_id("\\abs"));
        assert!(!is_valid_skill_id("a/../b"));
        assert!(is_valid_skill_id("rust-engineer"));
        assert!(is_valid_skill_id("cc-1c/form-add"));
    }

    #[test]
    fn first_paragraph_skips_headers() {
        let text = "# Заголовок\n\nОбычный текст";
        assert_eq!(first_paragraph(text), "Обычный текст");
        // Список-маркер пропускается
        assert_eq!(first_paragraph("- элемент\nТекст"), "Текст");
        // Поведение соответствует TS-версии: строки внутри ```-блока НЕ пропускаются
        assert_eq!(first_paragraph("# Заголовок\n\n```bsl\nкод\n```"), "код");
    }

    #[test]
    fn scans_flat_and_categorized_skills() {
        let dir = tmp_tree();
        let skills_root = dir.join(".agents").join("skills");
        std::fs::create_dir_all(&skills_root.join("flat-skill")).unwrap();
        std::fs::write(
            skills_root.join("flat-skill").join("SKILL.md"),
            "---\nname: Flat\n---\nТело",
        )
        .unwrap();
        std::fs::create_dir_all(&skills_root.join("cat").join("sub")).unwrap();
        std::fs::write(
            skills_root.join("cat").join("sub").join("SKILL.md"),
            "---\ndescription: Описание\n---\nТело2",
        )
        .unwrap();

        let skills = scan_skills(&skills_root);
        assert_eq!(skills.len(), 2);
        let flat = skills.iter().find(|s| s.id == "flat-skill").unwrap();
        assert_eq!(flat.name, "Flat");
        let cat = skills.iter().find(|s| s.id == "cat/sub").unwrap();
        assert_eq!(cat.category.as_deref(), Some("cat"));
        assert_eq!(cat.description, "Описание");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_skill_file_and_lists_files() {
        let dir = tmp_tree();
        let skills_root = dir.join(".agents").join("skills");
        let skill_dir = skills_root.join("demo");
        std::fs::create_dir_all(&skill_dir.join("scripts")).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "---\n---\nТело").unwrap();
        std::fs::write(skill_dir.join("scripts").join("build.ps1"), "Write-Host hi").unwrap();

        let files = get_skill_files(&skills_root, "demo");
        assert!(files.contains(&"SKILL.md".to_string()));
        assert!(files.contains(&"scripts/build.ps1".to_string()));

        let content = read_skill_file(&skills_root, "demo", "scripts/build.ps1");
        assert_eq!(content.as_deref(), Some("Write-Host hi"));
        assert!(read_skill_file(&skills_root, "demo", "../evil").is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scans_docs_and_rules() {
        let dir = tmp_tree();
        let docs = dir.join(".agents").join("docs");
        std::fs::create_dir_all(&docs.join("patterns")).unwrap();
        std::fs::write(
            docs.join("patterns").join("epf-lifecycle.md"),
            "# Заголовок\n\nОписание паттерна",
        )
        .unwrap();
        let rules = dir.join(".agents").join("rules");
        std::fs::create_dir_all(&rules).unwrap();
        std::fs::write(rules.join("standards.md"), "# Правила\n\nТекст правил").unwrap();

        let skills_dir = dir.join(".agents").join("skills");
        let _ = std::fs::create_dir_all(&skills_dir);
        let docs_list = scan_docs(&skills_dir);
        assert!(docs_list.iter().any(|d| d.id == "patterns/epf-lifecycle"));

        let rules_list = scan_rules(&skills_dir);
        assert!(rules_list.iter().any(|r| r.id == "standards"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
