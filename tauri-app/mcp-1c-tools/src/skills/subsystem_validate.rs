use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn validate(args: Value) -> Result<String> {
    let path = args.get("path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'path' обязателен"))?;
    let detailed = args.get("detailed").and_then(|v| v.as_bool()).unwrap_or(false);

    let p = Path::new(path);
    let xml_path = if p.is_dir() { p.join("Subsystem.xml") } else { p.to_path_buf() };
    if !xml_path.exists() { return Err(anyhow!("Subsystem.xml не найден: {}", xml_path.display())); }

    let xml = fs::read_to_string(&xml_path)?;
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut oks: Vec<String> = Vec::new();
    let parent_dir = xml_path.parent().unwrap_or(Path::new("."));

    if xml.contains("<MetaDataObject") { oks.push("Root: MetaDataObject".into()); }
    else { errors.push("Root: отсутствует MetaDataObject".into()); }

    if xml.contains("<Subsystem ") || xml.contains("<Subsystem>") {
        if let Some(u) = xml.find(" uuid=\"") {
            let u_end = xml[u+7..].find('\"').unwrap_or(0);
            oks.push(format!("Subsystem: uuid найден"));
        } else { warnings.push("Subsystem: uuid не найден".into()); }
    } else { errors.push("Subsystem: не найден".into()); }

    let props = ["Name", "Synonym", "Comment", "IncludeHelpInContents", "IncludeInCommandInterface", "UseOneCommand", "Explanation", "Picture", "Content"];
    for prop in &props {
        let open = format!("<{}>", prop);
        if xml.contains(&open) { oks.push(format!("Property: {}", prop)); }
        else { errors.push(format!("Property: {} отсутствует", prop)); }
    }

    if let Some(name) = extract_xml_value(&xml, "Name") {
        if name.chars().any(|c| c.is_ascii_whitespace()) { warnings.push("Name: содержит пробелы".into()); }
    }

    for bool_prop in &["IncludeHelpInContents", "IncludeInCommandInterface", "UseOneCommand"] {
        if let Some(v) = extract_xml_value(&xml, bool_prop) {
            if v != "true" && v != "false" { errors.push(format!("{}: должно быть true/false, найдено '{}'", bool_prop, v)); }
        }
    }

    let content_count = xml.matches("<xr:Item>").count();
    oks.push(format!("Content: {} элементов", content_count));

    if content_count > 0 {
        let mut names: Vec<String> = Vec::new();
        for line in xml.lines() {
            let t = line.trim();
            if t.contains("MDObjectRef") {
                if let Some(start) = t.find('>') {
                    if let Some(end) = t[start+1..].find('<') {
                        let name = &t[start+1..start+1+end];
                        if names.contains(&name.to_string()) { warnings.push(format!("Content: дубликат '{}'", name)); }
                        names.push(name.to_string());
                    }
                }
            }
        }
    }

    let child_count = xml.matches("<Subsystem>").count();
    oks.push(format!("ChildObjects: {} дочерних подсистем", child_count));

    // Check child files exist
    if child_count > 0 {
        for line in xml.lines() {
            let t = line.trim();
            if t.starts_with("<Subsystem>") && t.ends_with("</Subsystem>") {
                let name = t.trim_start_matches("<Subsystem>").trim_end_matches("</Subsystem>").trim();
                if !name.contains('<') {
                    let child_file = parent_dir.join(name).join("Subsystem.xml");
                    if !child_file.exists() { warnings.push(format!("Child: файл {} не найден", child_file.display())); }
                }
            }
        }
    }

    let mut out = format!("=== Validation: Subsystem.{} ===\n\n", extract_xml_value(&xml, "Name").unwrap_or("?".to_string()));
    if detailed { for o in &oks { out.push_str(&format!("  [OK] {}\n", o)); } }
    for w in &warnings { out.push_str(&format!("  [WARN] {}\n", w)); }
    for e in &errors { out.push_str(&format!("  [ERROR] {}\n", e)); }
    out.push_str(&format!("\nРезультат: {} ошибок, {} предупреждений ({} проверок)", errors.len(), warnings.len(), oks.len() + warnings.len() + errors.len()));

    if errors.is_empty() { Ok(out) }
    else { Err(anyhow!("{}", out)) }
}

fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = xml.find(&open) {
        let after = &xml[start + open.len()..];
        if let Some(end) = after.find(&close) {
            return Some(after[..end].to_string());
        }
    }
    None
}
