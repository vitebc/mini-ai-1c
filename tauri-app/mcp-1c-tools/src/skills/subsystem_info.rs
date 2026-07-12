use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn info(args: Value) -> Result<String> {
    let path = args.get("path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'path' обязателен"))?;
    let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("overview");

    let p = Path::new(path);
    let xml_path = if p.is_dir() { p.join("Subsystem.xml") } else { p.to_path_buf() };
    if !xml_path.exists() { return Err(anyhow!("Subsystem.xml не найден: {}", xml_path.display())); }

    let xml = fs::read_to_string(&xml_path)?;
    let mut out = format!("=== Subsystem Info: {} ===\n\n", xml_path.display());

    // Name
    let name = extract_xml_value(&xml, "Name").unwrap_or_else(|| "unknown".to_string());
    out.push_str(&format!("Имя: {}\n", name));

    // Synonym
    if let Some(syn) = extract_xml_value(&xml, "Synonym") {
        if let Some(content_pos) = syn.find("<v8:content>") {
            let rest = &syn[content_pos + 12..];
            if let Some(end) = rest.find("</v8:content>") {
                out.push_str(&format!("Синоним: {}\n", &rest[..end]));
            }
        }
    }

    // Flags
    if let Some(v) = extract_xml_value(&xml, "IncludeHelpInContents") { out.push_str(&format!("IncludeHelpInContents: {}\n", v)); }
    if let Some(v) = extract_xml_value(&xml, "IncludeInCommandInterface") { out.push_str(&format!("IncludeInCommandInterface: {}\n", v)); }
    if let Some(v) = extract_xml_value(&xml, "UseOneCommand") { out.push_str(&format!("UseOneCommand: {}\n", v)); }

    // Content count
    let content_count = xml.matches("<xr:Item>").count();
    out.push_str(&format!("\nСодержимое: {} объектов\n", content_count));

    if mode == "content" || mode == "full" {
        for line in xml.lines() {
            let t = line.trim();
            if t.contains("<xr:Item>") || t.contains("MDObjectRef") {
                out.push_str(&format!("  - {}\n", t));
            }
        }
    }

    // Child subsystems
    let child_count = xml.matches("<Subsystem>").count();
    out.push_str(&format!("\nДочерних подсистем: {}\n", child_count));

    if child_count > 0 && (mode == "tree" || mode == "full") {
        // Simple tree listing
        for line in xml.lines() {
            let t = line.trim();
            if t.starts_with("<Subsystem>") {
                let v = t.trim_start_matches("<Subsystem>").trim_end_matches("</Subsystem>").trim();
                if !v.is_empty() && !v.contains('<') {
                    out.push_str(&format!("  └─ {}\n", v));
                }
            }
        }
    }

    // CommandInterface
    let ci_path = p.parent().map(|parent| parent.join("CommandInterface.xml"))
        .unwrap_or_else(|| xml_path.parent().unwrap_or(Path::new(".")).join("CommandInterface.xml"));

    if ci_path.exists() && (mode == "ci" || mode == "full") {
        out.push_str("\n=== CommandInterface ===\n");
        let ci = fs::read_to_string(&ci_path)?;
        let visible = ci.matches("<xr:Common>true</xr:Common>").count();
        let hidden = ci.matches("<xr:Common>false</xr:Common>").count();
        out.push_str(&format!("Видимых команд: {}, скрытых: {}\n", visible, hidden));
    }

    Ok(out)
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
