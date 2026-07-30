use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillFile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub content: String,
}

/// Resolve the skills directory path (same logic as in mcp_client)
fn resolve_skills_dir() -> Option<PathBuf> {
    let p = crate::settings::get_settings_dir().join(".agents").join("skills");
    Some(p)
}

fn scan_skills_dir() -> Vec<SkillFile> {
    let dir = match resolve_skills_dir() {
        Some(d) => d,
        None => return vec![],
    };

    let mut skills = vec![];
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let entry_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Check if this is a flat skill (SKILL.md directly in this dir)
        let flat_skill_path = path.join("SKILL.md");
        if flat_skill_path.exists() {
            if let Some(s) = read_skill_file(&entry_name, &path, "") {
                skills.push(s);
            }
            continue;
        }

        // Otherwise, treat this as a category directory with sub-skills
        let sub_entries = match std::fs::read_dir(&path) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let category = entry_name;

        for sub in sub_entries.flatten() {
            let sub_path = sub.path();
            if !sub_path.is_dir() {
                continue;
            }
            let skill_id = sub_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let skill_path = sub_path.join("SKILL.md");
            if !skill_path.exists() {
                continue;
            }
            if let Some(s) = read_skill_file(&format!("{}/{}", category, skill_id), &sub_path, &category) {
                skills.push(s);
            }
        }
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

fn read_skill_file(id: &str, skill_dir: &PathBuf, category: &str) -> Option<SkillFile> {
    let skill_path = skill_dir.join("SKILL.md");
    let content = std::fs::read_to_string(&skill_path).ok()?;
    let (name, description, _) = parse_frontmatter(&content);
    Some(SkillFile {
        id: id.to_string(),
        name,
        description,
        category: category.to_string(),
        content,
    })
}

fn parse_frontmatter(content: &str) -> (String, String, String) {
    let mut name = String::new();
    let mut description = String::new();
    let mut category = String::new();

    if content.starts_with("---\n") {
        if let Some(end) = content.find("\n---\n") {
            let fm = &content[4..end];
            for line in fm.lines() {
                if let Some(idx) = line.find(": ") {
                    let key = line[..idx].trim();
                    let val = line[idx + 2..].trim().to_string();
                    match key {
                        "name" => name = val,
                        "description" => description = val,
                        "category" | "domain" => {
                            if category.is_empty() {
                                category = val;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    (name, description, category)
}

fn save_skill_file(id: &str, content: &str) -> Result<PathBuf, String> {
    let dir = resolve_skills_dir().ok_or("Skills directory not found")?;
    let skill_dir = dir.join(id);
    std::fs::create_dir_all(&skill_dir).map_err(|e| e.to_string())?;
    let path = skill_dir.join("SKILL.md");
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path)
}

// ─── Tauri Commands ──────────────────────────────────────────────

#[tauri::command]
pub fn list_skills() -> Vec<SkillFile> {
    scan_skills_dir()
}

#[tauri::command]
pub fn get_skill(id: String) -> Result<SkillFile, String> {
    let skills = scan_skills_dir();
    skills.into_iter().find(|s| s.id == id).ok_or_else(|| format!("Skill '{}' not found", id))
}

#[tauri::command]
pub fn save_skill(id: String, content: String) -> Result<SkillFile, String> {
    let path = save_skill_file(&id, &content)?;
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let (name, description, category) = parse_frontmatter(&raw);
    Ok(SkillFile {
        id,
        name,
        description,
        category,
        content: raw,
    })
}

#[tauri::command]
pub fn delete_skill(id: String) -> Result<(), String> {
    let dir = resolve_skills_dir().ok_or("Skills directory not found")?;
    let skill_dir = dir.join(&id);
    if skill_dir.exists() {
        std::fs::remove_dir_all(&skill_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}
