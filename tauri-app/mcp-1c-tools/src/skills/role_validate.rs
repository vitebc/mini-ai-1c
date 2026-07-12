use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn validate(args: Value) -> Result<String> {
    let rights_path = args.get("rights_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'rights_path' обязателен"))?;
    let detailed = args.get("detailed").and_then(|v| v.as_bool()).unwrap_or(false);

    let p = Path::new(rights_path);
    let xml_path = if p.is_dir() { p.join("Ext").join("Rights.xml") } else { p.to_path_buf() };
    let xml_path = if xml_path.exists() { xml_path } else {
        if p.extension().map(|e| e == "xml").unwrap_or(false) {
            p.to_path_buf()
        } else {
            p.join("Ext").join("Rights.xml")
        }
    };

    if !xml_path.exists() { return Err(anyhow!("Rights.xml не найден: {}", xml_path.display())); }

    let xml = fs::read_to_string(&xml_path)?;
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut oks: Vec<String> = Vec::new();

    if xml.contains("<Rights ") { oks.push("Root: <Rights> с namespace".into()); }
    else { errors.push("Root: отсутствует <Rights>".into()); }

    let mut flag_count = 0;
    for flag in &["setForNewObjects", "setForAttributesByDefault", "independentRightsOfChildObjects"] {
        if let Some(v) = extract_xml_value(&xml, flag) {
            if v == "true" || v == "false" { oks.push(format!("Flag: {} = {}", flag, v)); }
            else { warnings.push(format!("Flag: {} = '{}' (ожидается true/false)", flag, v)); }
            flag_count += 1;
        }
    }
    if flag_count == 3 { oks.push("Flags: все 3 глобальных флага присутствуют".into()); }

    let obj_count = xml.matches("<object>").count();
    let right_count = xml.matches("<right>").count();
    let rls_count = xml.matches("<restrictionByCondition>").count();
    oks.push(format!("Objects: {}, Rights: {}, RLS: {}", obj_count, right_count, rls_count));

    if rls_count > 0 {
        // Check RLS conditions aren't empty
        for line in xml.lines() {
            let t = line.trim();
            if t.contains("<condition>") && (t.contains("</condition>") || t.contains("/>")) {
                let content = t.trim_start_matches("<condition>").trim_end_matches("</condition>").trim();
                if content.is_empty() || content == "/>" {
                    warnings.push("RLS: пустое условие".into());
                }
            }
        }
    }

    let tpl_count = xml.matches("<restrictionTemplate>").count();
    if tpl_count > 0 {
        oks.push(format!("Templates: {}", tpl_count));
        for line in xml.lines() {
            let t = line.trim();
            if t.contains("<name>") && t.contains("</name>") && t.contains("<restrictionTemplate>") {
                // Check template has non-empty condition
            }
        }
    }

    let mut out = format!("=== Validation: Role ===\n\n");
    if detailed { for o in &oks { out.push_str(&format!("  [OK] {}\n", o)); } }
    for w in &warnings { out.push_str(&format!("  [WARN] {}\n", w)); }
    for e in &errors { out.push_str(&format!("  [ERROR] {}\n", e)); }
    let total = oks.len() + warnings.len() + errors.len();
    out.push_str(&format!("\nРезультат: {} ошибок, {} предупреждений ({} проверок)", errors.len(), warnings.len(), total));

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
