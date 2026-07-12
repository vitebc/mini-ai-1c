use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn validate(args: Value) -> Result<String> {
    let path = args.get("path").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).ok_or_else(|| anyhow!("Параметр 'path' обязателен"))?;
    let p = Path::new(path);
    if !p.exists() { return Err(anyhow!("Путь не найден: {}", path)); }

    let xml = fs::read_to_string(p).context("Не удалось прочитать XML")?;
    let mut errors: Vec<String> = Vec::new();

    if !xml.contains("<MetaDataObject") { errors.push("Отсутствует корневой элемент MetaDataObject".into()); }
    if !xml.contains("<ExternalDataProcessor") && !xml.contains("<DataSource>") { errors.push("Не найден элемент ExternalDataProcessor".into()); }
    if xml.find("<Name>").and_then(|s| xml[s..].find("</Name>")).is_none() { errors.push("Отсутствует Name".into()); }
    if xml.find("<ChildObjects").is_none() { errors.push("Отсутствует ChildObjects".into()); }

    if errors.is_empty() {
        Ok(format!("[OK] EPF валиден: {}\n     Размер: {} байт", path, xml.len()))
    } else {
        Err(anyhow!("[ОШИБКА] EPF невалиден: {}\n     {}", path, errors.join("\n     ")))
    }
}
