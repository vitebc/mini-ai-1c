use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn validate(args: Value) -> Result<String> {
    let ext_path = args.get("extension_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'extension_path' обязателен"))?;
    let detailed = args.get("detailed").and_then(|v| v.as_bool()).unwrap_or(false);

    let p = Path::new(ext_path);
    let cfg_path = if p.is_dir() { p.join("Configuration.xml") } else { p.to_path_buf() };
    if !cfg_path.exists() { return Err(anyhow!("Configuration.xml не найден: {}", cfg_path.display())); }

    let xml = fs::read_to_string(&cfg_path)?;
    let cfg_dir = cfg_path.parent().unwrap_or(Path::new("."));
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut oks: Vec<String> = Vec::new();

    // 1. Root
    if xml.contains("<MetaDataObject") { oks.push("Root: MetaDataObject".into()); }
    else { errors.push("Root: отсутствует MetaDataObject".into()); }

    if xml.contains("ConfigurationExtension") || xml.contains("xsi:type=\"xr:ConfigurationExtension\"") {
        oks.push("Type: ConfigurationExtension".into());
    } else { errors.push("Type: не является расширением".into()); }

    // 2. Name
    if let Some(name) = extract_tag(&xml, "Name") {
        if !name.is_empty() { oks.push(format!("Name: {}", name)); }
        else { errors.push("Name: пустое".into()); }
    } else { errors.push("Name: не найден".into()); }

    // 3. NamePrefix
    if let Some(p) = extract_tag(&xml, "NamePrefix") {
        if !p.is_empty() { oks.push(format!("NamePrefix: {}", p)); }
        else { warnings.push("NamePrefix: пустой".into()); }
    } else { warnings.push("NamePrefix: не найден".into()); }

    // 4. Purpose
    if let Some(p) = extract_tag(&xml, "ConfigurationExtensionPurpose") {
        let valid = ["Patch", "Customization", "AddOn"];
        if valid.contains(&p.as_str()) { oks.push(format!("Purpose: {}", p)); }
        else { warnings.push(format!("Purpose: нестандартное значение '{}'", p)); }
    } else { warnings.push("Purpose: не найден".into()); }

    // 5. ObjectBelonging
    if let Some(v) = extract_tag(&xml, "ObjectBelonging") {
        if v == "Adopted" || v == "Own" { oks.push(format!("ObjectBelonging: {}", v)); }
        else { warnings.push(format!("ObjectBelonging: нестандартное '{}'", v)); }
    } else { errors.push("ObjectBelonging: не найден".into()); }

    // 6. CompatibilityMode
    if xml.contains("<CompatibilityMode>") { oks.push("CompatibilityMode: найден".into()); }

    // 7. ChildObjects
    let obj_types = ["Catalog", "Document", "DataProcessor", "Report", "InformationRegister", "Role", "CommonModule", "Enum", "Constant"];
    let mut total_objects = 0;
    for ot in &obj_types {
        let c = xml.matches(&format!("<{}>", ot)).count();
        if c > 0 { total_objects += c; }
    }
    if total_objects > 0 { oks.push(format!("ChildObjects: {} объектов", total_objects)); }
    else { warnings.push("ChildObjects: пусто".into()); }

    // 8. Language files exist
    let lang_dir = cfg_dir.join("Languages");
    if lang_dir.exists() {
        if let Ok(entries) = fs::read_dir(&lang_dir) {
            let count = entries.flatten().count();
            if count > 0 { oks.push(format!("Languages: {} файлов", count)); }
            else { errors.push("Languages: каталог пуст".into()); }
        }
    } else { errors.push("Languages: каталог не найден".into()); }

    // 9. DefaultLanguage
    if xml.contains("<DefaultLanguage>") { oks.push("DefaultLanguage: найден".into()); }
    else { warnings.push("DefaultLanguage: не найден".into()); }

    // 10. Object directories
    let type_dirs = ["Catalogs", "Documents", "DataProcessors", "Reports", "InformationRegisters", "Roles", "CommonModules", "Enums", "Constants"];
    for td in &type_dirs {
        let d = cfg_dir.join(td);
        if d.exists() { oks.push(format!("Dir: {} - {} файлов", td, d.read_dir().map(|e| e.flatten().count()).unwrap_or(0))); }
    }

    let mut out = format!("=== Validation: Extension ===\n\n");
    if detailed { for o in &oks { out.push_str(&format!("  [OK] {}\n", o)); } }
    for w in &warnings { out.push_str(&format!("  [WARN] {}\n", w)); }
    for e in &errors { out.push_str(&format!("  [ERROR] {}\n", e)); }
    out.push_str(&format!("\nИтого: {} OK, {} WARN, {} ERROR", oks.len(), warnings.len(), errors.len()));

    if errors.is_empty() { Ok(out) }
    else { Err(anyhow!("{}", out)) }
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
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
