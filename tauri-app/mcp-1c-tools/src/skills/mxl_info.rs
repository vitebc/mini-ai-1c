use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn info(args: Value) -> Result<String> {
    let path = args.get("path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'path' обязателен"))?;
    let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("text");

    let p = Path::new(path);
    if !p.exists() { return Err(anyhow!("Файл не найден: {}", path)); }

    let xml = fs::read_to_string(p)?;

    // Basic stats
    let rows = xml.matches("<rowsItem>").count();
    let columns = xml.matches("<columnsItem>").count();
    let areas = xml.matches("<namedItem>").count();
    let fonts = xml.matches("<font>").count();
    let lines = xml.matches("<line>").count();
    let formats = xml.matches("<format>").count();
    let merges = xml.matches("<merge>").count();

    // Named areas
    let mut area_names: Vec<String> = Vec::new();
    for line in xml.lines() {
        let t = line.trim();
        if t.contains("<namedItem") && t.contains("name=\"") {
            if let Some(s) = t.find("name=\"") {
                let rest = &t[s + 6..];
                if let Some(end) = rest.find('\"') {
                    area_names.push(rest[..end].to_string());
                }
            }
        }
    }

    if format == "json" {
        let result = serde_json::json!({
            "rows": rows, "columns": columns, "areas": areas,
            "fonts": fonts, "lines": lines, "formats": formats, "merges": merges,
            "area_names": area_names
        });
        return Ok(serde_json::to_string_pretty(&result)?);
    }

    let mut out = format!("=== MXL Info: {} ===\n\n", p.display());
    out.push_str(&format!("Строк: {}\n", rows));
    out.push_str(&format!("Колонок: {}\n", columns));
    out.push_str(&format!("Областей: {}\n", areas));
    out.push_str(&format!("Шрифтов: {}\n", fonts));
    out.push_str(&format!("Линий: {}\n", lines));
    out.push_str(&format!("Форматов: {}\n", formats));
    out.push_str(&format!("Объединений: {}\n", merges));

    if !area_names.is_empty() {
        out.push_str("\nОбласти:\n");
        for name in &area_names {
            out.push_str(&format!("  - {}\n", name));
        }
    }

    // Parameters in cells
    let params = xml.matches("<parameter>").count();
    let texts = xml.matches("<text>").count();
    if params > 0 || texts > 0 {
        out.push_str(&format!("\nПараметров: {}\n", params));
        out.push_str(&format!("Текстов: {}\n", texts));
    }

    Ok(out)
}
