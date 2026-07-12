use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn validate(args: Value) -> Result<String> {
    let path = args.get("object_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'object_path' обязателен"))?;
    let detailed = args.get("detailed").and_then(|v| v.as_bool()).unwrap_or(false);
    let max_errors = args.get("max_errors").and_then(|v| v.as_i64()).unwrap_or(30) as usize;

    let p = Path::new(path);
    let obj_path = if p.is_dir() {
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("object");
        p.join(format!("{}.xml", name))
    } else { p.to_path_buf() };

    if !obj_path.exists() { return Err(anyhow!("Файл объекта не найден: {}", obj_path.display())); }

    let xml = fs::read_to_string(&obj_path)?;
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut oks: Vec<String> = Vec::new();

    // 1. MetaDataObject root
    if xml.contains("<MetaDataObject") { oks.push("Root: MetaDataObject".into()); }
    else { errors.push("Root: отсутствует MetaDataObject".into()); }

    // 2. Detect object type
    let object_types = ["Catalog", "Document", "DataProcessor", "Report", "InformationRegister", "AccumulationRegister", "ChartOfAccounts", "ChartOfCharacteristicTypes", "BusinessProcess", "Task", "ExchangePlan", "Enum", "ExternalDataProcessor", "ExternalReport", "Constant", "DocumentJournal", "FilterCriterion", "ScheduledJob", "Sequence", "SettingsStorage", "CalculationRegister", "AccountingRegister"];
    let mut obj_type = "";
    for t in &object_types {
        if xml.contains(&format!("<{} ", t)) {
            obj_type = t;
            break;
        }
    }
    if obj_type.is_empty() { errors.push("Type: не удалось определить тип объекта".into()); }
    else { oks.push(format!("Type: {}", obj_type)); }

    // 3. Name
    if let Some(s) = xml.find("<Name>").and_then(|pos| xml[pos+6..].find("</Name>").map(|e| &xml[pos+6..pos+6+e])) {
        let name = s.trim();
        if name.is_empty() { errors.push("Name: пустое имя".into()); }
        else if name.len() > 80 { warnings.push(format!("Name: длинное имя ({} символов)", name.len())); }
        else { oks.push(format!("Name: {}", name)); }
    } else { errors.push("Name: не найден".into()); }

    // 4. Synonym
    if xml.contains("<Synonym>") { oks.push("Synonym: найден".into()); }

    // 5. InternalInfo
    if xml.contains("<InternalInfo>") { oks.push("InternalInfo: найден".into()); }
    else { warnings.push("InternalInfo: отсутствует".into()); }

    // 6. ChildObjects
    if xml.contains("<ChildObjects") { oks.push("ChildObjects: найден".into()); }

    // 7. Attributes
    let attr_count = xml.matches("<Attribute ").count();
    if attr_count > 0 { oks.push(format!("Атрибутов/реквизитов: {}", attr_count)); }

    // 8. Forms
    let form_count = xml.matches("<Form>").count();
    if form_count > 0 { oks.push(format!("Форм: {}", form_count)); }

    // 9. Templates
    let tmpl_count = xml.matches("<Template>").count();
    if tmpl_count > 0 { oks.push(format!("Макетов: {}", tmpl_count)); }

    // 10. TabularSections
    let ts_count = xml.matches("<Table ").count();
    if ts_count > 0 { oks.push(format!("Табличных частей: {}", ts_count)); }

    let error_count = errors.len();
    let warning_count = warnings.len();

    let mut out = format!("=== Validation Report: {} ===\n\n", obj_path.display());
    if detailed { for o in &oks { out.push_str(&format!("  [OK] {}\n", o)); } }
    for w in &warnings { out.push_str(&format!("  [WARN] {}\n", w)); }
    for e in &errors { out.push_str(&format!("  [ERROR] {}\n", e)); }
    out.push_str(&format!("\nИтого: {} OK, {} WARN, {} ERROR", oks.len(), warning_count, error_count));

    if error_count > 0 { return Err(anyhow!("{}", out)); }
    Ok(out)
}
