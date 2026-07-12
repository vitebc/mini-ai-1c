use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn validate(args: Value) -> Result<String> {
    let path = args.get("path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'path' обязателен"))?;
    let detailed = args.get("detailed").and_then(|v| v.as_bool()).unwrap_or(false);

    let p = Path::new(path);
    let form_path = if p.is_dir() { p.join("Form.xml") } else { p.to_path_buf() };
    if !form_path.exists() { return Err(anyhow!("Form.xml не найден: {}", form_path.display())); }

    let xml = fs::read_to_string(&form_path)?;
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut oks: Vec<String> = Vec::new();

    if xml.contains("<Form ") { oks.push("Root: Form".into()); }
    else { errors.push("Root: отсутствует Form".into()); }

    if xml.contains("<Attributes>") { oks.push("Attributes: найден".into()); }
    else { warnings.push("Attributes: отсутствует".into()); }

    let attr_count = xml.matches("<Attribute ").count();
    if attr_count > 0 { oks.push(format!("Attributes: {} атрибутов", attr_count)); }

    let main_attr = xml.matches("<MainAttribute>true</MainAttribute>").count();
    if main_attr > 0 { oks.push(format!("MainAttribute: {} основных", main_attr)); }

    if xml.contains("<FormElements>") || xml.contains("<ChildObjects>") {
        let el_types = ["InputField", "Table", "Group", "Button", "Label", "CheckBox", "RadioButton", "List", "ComboBox", "DatePicker"];
        let mut count = 0;
        for el in &el_types {
            count += xml.matches(&format!("<{} ", el)).count();
        }
        if count > 0 { oks.push(format!("Элементов формы: {}", count)); }
        else { warnings.push("Элементы формы: не найдены".into()); }
    }

    if xml.contains("<AutoCommandBar") { oks.push("AutoCommandBar: найден".into()); }

    let commands = xml.matches("<Command ").count();
    if commands > 0 { oks.push(format!("Commands: {}", commands)); }

    if xml.contains("<IncludeHelpInContents>") { oks.push("IncludeHelpInContents: найден".into()); }

    let form_type = if xml.contains("<OrdinaryForm>") { "Обычная" } else if xml.contains("ManagedForm") || xml.contains("<FormType>") { "Управляемая" } else { "Не определён" };
    oks.push(format!("Тип формы: {}", form_type));

    let mut out = format!("=== Validation: {} ===\n\n", form_path.display());
    if detailed { for o in &oks { out.push_str(&format!("  [OK] {}\n", o)); } }
    for w in &warnings { out.push_str(&format!("  [WARN] {}\n", w)); }
    for e in &errors { out.push_str(&format!("  [ERROR] {}\n", e)); }
    out.push_str(&format!("\nРезультат: {} ошибок, {} предупреждений ({} проверок)", errors.len(), warnings.len(), oks.len()));

    if errors.is_empty() { Ok(out) }
    else { Err(anyhow!("{}", out)) }
}
