use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn validate(args: Value) -> Result<String> {
    let path = args.get("config_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'config_path' обязателен"))?;
    let detailed = args.get("detailed").and_then(|v| v.as_bool()).unwrap_or(false);
    let max_errors = args.get("max_errors").and_then(|v| v.as_i64()).unwrap_or(30) as usize;

    let p = Path::new(path);
    let cfg_path = if p.is_dir() { p.join("Configuration.xml") } else { p.to_path_buf() };
    if !cfg_path.exists() { return Err(anyhow!("Configuration.xml не найден: {}", cfg_path.display())); }

    let xml = fs::read_to_string(&cfg_path)?;
    let cfg_dir = cfg_path.parent().unwrap_or(Path::new("."));
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut oks: Vec<String> = Vec::new();

    // 1. Root element
    if xml.contains("<MetaDataObject") { oks.push("Root: MetaDataObject".into()); }
    else { errors.push("Root: отсутствует MetaDataObject".into()); }
    if xml.contains("<Configuration ") { oks.push("Configuration: найден".into()); }
    else { errors.push("Configuration: не найден".into()); }

    // 2. Name
    if let Some(s) = xml.find("<Name>").and_then(|pos| xml[pos+6..].find("</Name>").map(|e| &xml[pos+6..pos+6+e])) {
        let name = s.trim();
        if name.is_empty() { errors.push("Name: пустое имя".into()); }
        else if name.len() > 100 { warnings.push(format!("Name: длинное имя ({} символов)", name.len())); }
        else { oks.push(format!("Name: {}", name)); }
    } else { errors.push("Name: не найден".into()); }

    // 3. Synonym
    if xml.contains("<Synonym>") { oks.push("Synonym: найден".into()); }
    else { warnings.push("Synonym: отсутствует".into()); }

    // 4. DefaultLanguage
    if let Some(s) = xml.find("<DefaultLanguage>").and_then(|pos| xml[pos+17..].find("</DefaultLanguage>").map(|e| &xml[pos+17..pos+17+e])) {
        let lang = s.trim();
        oks.push(format!("DefaultLanguage: {}", lang));
        // Check language file exists
        let lang_file = cfg_dir.join("Languages").join(format!("{}.xml", lang.split('.').last().unwrap_or(lang)));
        if !lang_file.exists() { errors.push(format!("Language: файл {}/Languages/{}.xml не найден", cfg_dir.display(), lang.split('.').last().unwrap_or(lang))); }
    } else { errors.push("DefaultLanguage: не найден".into()); }

    // 5. ChildObjects
    if xml.contains("<ChildObjects") {
        let count = xml.matches("<Language>").count();
        oks.push(format!("ChildObjects: {} языков", count));
    } else { errors.push("ChildObjects: не найден".into()); }

    // 6. Vendor/Version
    if xml.contains("<Vendor>") { oks.push("Vendor: найден".into()); }
    if xml.contains("<Version>") { oks.push("Version: найден".into()); }

    // 7. CompatibilityMode
    if xml.contains("<CompatibilityMode>") { oks.push("CompatibilityMode: найден".into()); }

    // 8. Object directories exist
    let type_dirs = ["Catalogs", "Documents", "Reports", "DataProcessors", "InformationRegisters", "AccumulationRegisters", "ChartsOfAccounts", "ChartsOfCharacteristicTypes", "BusinessProcesses", "Tasks", "ExchangePlans"];
    let mut found_objects = 0;
    for dir in &type_dirs {
        let d = cfg_dir.join(dir);
        if d.exists() {
            if let Ok(entries) = fs::read_dir(&d) {
                found_objects += entries.flatten().filter(|e| e.path().extension().map(|x| x == "xml").unwrap_or(false)).count();
            }
        }
    }
    oks.push(format!("Объектов метаданных: ~{}", found_objects));

    let error_count = errors.len();
    let warning_count = warnings.len();

    let mut out = format!("=== Validation Report: {} ===\n\n", cfg_path.display());
    if detailed {
        for o in &oks { out.push_str(&format!("  [OK] {}\n", o)); }
    }
    for w in &warnings { out.push_str(&format!("  [WARN] {}\n", w)); }
    for e in &errors { out.push_str(&format!("  [ERROR] {}\n", e)); }
    out.push_str(&format!("\nИтого: {} OK, {} WARN, {} ERROR", oks.len(), warning_count, error_count));

    if error_count > 0 {
        return Err(anyhow!("{}", out));
    }
    Ok(out)
}
