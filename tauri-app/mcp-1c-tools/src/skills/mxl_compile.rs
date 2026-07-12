use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn compile(args: Value) -> Result<String> {
    let json_path = args.get("json_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'json_path' обязателен"))?;
    let out_path = args.get("out_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'out_path' обязателен"))?;

    let json_str = fs::read_to_string(json_path)?;
    let json: Value = serde_json::from_str(&json_str)?;

    let cols = json.get("columns").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let areas = json.get("areas").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);

    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<SpreadsheetDocument xmlns="http://v8.1c.ru/spreadsheet/document" xmlns:ss="http://v8.1c.ru/spreadsheet/document" xmlns:v8="http://v8.1c.ru/8.1/data/core" xmlns:xs="http://www.w3.org/2001/XMLSchema">
<language><code>ru</code></language>"#);

    // Columns
    if cols > 0 {
        xml.push_str("\n<columns>");
        for i in 0..cols {
            let w = json["columns"][i].as_str().unwrap_or("5x");
            xml.push_str(&format!("\n\t<columnsItem><width>{}</width></columnsItem>", w));
        }
        xml.push_str("\n</columns>");
    }

    // Areas
    if let Some(area_list) = json["areas"].as_array() {
        for (ai, area) in area_list.iter().enumerate() {
            let default_name = format!("Area{}", ai + 1);
            let name = area.get("name").and_then(|v| v.as_str()).unwrap_or(&default_name);
            xml.push_str(&format!("\n<namedItem name=\"{}\" type=\"Rows\">", name));

            if let Some(rows) = area["rows"].as_array() {
                for row in rows {
                    xml.push_str("\n\t<rowsItem>");
                    if let Some(empty) = row.get("empty").and_then(|v| v.as_i64()) {
                        for _ in 0..empty {
                            xml.push_str("\n\t\t<empty>true</empty>");
                        }
                        xml.push_str("\n\t</rowsItem>");
                        continue;
                    }
                    if let Some(cells) = row.get("cells").and_then(|v| v.as_array()) {
                        for cell in cells {
                            xml.push_str("\n\t\t<cell>");
                            if let Some(param) = cell.get("param").and_then(|v| v.as_str()) {
                                xml.push_str(&format!("\n\t\t\t<parameter>{}</parameter>", param));
                            }
                            if let Some(text) = cell.get("text").and_then(|v| v.as_str()) {
                                xml.push_str(&format!("\n\t\t\t<text>{}</text>", text));
                            }
                            if let Some(style) = cell.get("style").and_then(|v| v.as_str()) {
                                xml.push_str(&format!("\n\t\t\t<style>{}</style>", style));
                            }
                            xml.push_str("\n\t\t</cell>");
                        }
                    }
                    xml.push_str("\n\t</rowsItem>");
                }
            }
            xml.push_str("\n</namedItem>");
        }
    }

    xml.push_str("\n</SpreadsheetDocument>");

    let bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF].iter().chain(xml.as_bytes().iter()).copied().collect();
    fs::write(out_path, &bom)?;

    Ok(format!("[OK] MXL скомпилирован: {} -> {}\n     Областей: {}, строк: {}", json_path, out_path, areas, json.pointer("/areas").and_then(|v| v.as_array()).map(|a| a.iter().map(|r| r.get("rows").and_then(|v| v.as_array()).map(|c| c.len()).unwrap_or(0)).sum::<usize>()).unwrap_or(0)))
}
