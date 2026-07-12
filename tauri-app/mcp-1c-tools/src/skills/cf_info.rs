use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn info(args: Value) -> Result<String> {
    let path = args.get("path").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).ok_or_else(|| anyhow!("Параметр 'path' обязателен"))?;
    let p = Path::new(path);
    let cfg_path = if p.is_dir() { p.join("Configuration.xml") } else { p.to_path_buf() };

    if !cfg_path.exists() { return Err(anyhow!("Configuration.xml не найден: {}", cfg_path.display())); }
    let xml = fs::read_to_string(&cfg_path).context("Не удалось прочитать Configuration.xml")?;

    let mut out = format!("=== Configuration Info: {} ===\n", cfg_path.display());

    if let Some(s) = xml.find("<Name>").and_then(|pos| xml[pos+6..].find("</Name>").map(|end| &xml[pos+6..pos+6+end])) {
        out.push_str(&format!("Имя: {}\n", s));
    }
    if let Some(s) = xml.find("<Synonym>").and_then(|pos| xml[pos+9..].find("</Synonym>").map(|end| &xml[pos+9..pos+9+end])) {
        out.push_str(&format!("Синоним: {}\n", s));
    }
    if let Some(s) = xml.find("<Vendor>").and_then(|pos| xml[pos+7..].find("</Vendor>").map(|end| &xml[pos+7..pos+7+end])) {
        out.push_str(&format!("Вендор: {}\n", s));
    }
    if let Some(s) = xml.find("<Version>").and_then(|pos| xml[pos+8..].find("</Version>").map(|end| &xml[pos+8..pos+8+end])) {
        out.push_str(&format!("Версия: {}\n", s));
    }
    if let Some(s) = xml.find("<CompatibilityMode>").and_then(|pos| xml[pos+18..].find("</CompatibilityMode>").map(|end| &xml[pos+18..pos+18+end])) {
        out.push_str(&format!("Режим совместимости: {}\n", s));
    }

    let lang_count = xml.matches("<Language>").count();
    out.push_str(&format!("Языки: {}\n", lang_count));

    let child_count = xml.matches("</ChildObjects>").count();
    out.push_str(&format!("Секций ChildObjects: {}\n", child_count));
    out.push_str(&format!("Размер: {} байт", xml.len()));

    Ok(out)
}
