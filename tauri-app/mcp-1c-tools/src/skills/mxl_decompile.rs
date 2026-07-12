use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn decompile(args: Value) -> Result<String> {
    let xml_path = args.get("xml_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'xml_path' обязателен"))?;
    let out_path = args.get("out_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty());

    let p = Path::new(xml_path);
    if !p.exists() { return Err(anyhow!("Файл не найден: {}", xml_path)); }

    let xml = fs::read_to_string(p)?;

    let mut json = serde_json::json!({
        "columns": [],
        "fonts": [],
        "styles": [],
        "areas": []
    });

    // Extract columns
    let mut cols: Vec<String> = Vec::new();
    for line in xml.lines() {
        let t = line.trim();
        if t.contains("<width>") && t.contains("</width>") {
            if let Some(s) = t.find("<width>") {
                if let Some(e) = t.find("</width>") {
                    cols.push(t[s+7..e].to_string());
                }
            }
        }
    }
    if !cols.is_empty() {
        json["columns"] = serde_json::json!(cols);
    }

    // Extract areas
    let mut areas: Vec<Value> = Vec::new();
    let mut current_area: Option<String> = None;
    let mut current_rows: Vec<Value> = Vec::new();
    let mut in_area = false;

    for line in xml.lines() {
        let t = line.trim();

        if t.contains("<namedItem") && t.contains("name=\"") {
            if in_area && !current_rows.is_empty() {
                areas.push(serde_json::json!({"name": current_area, "rows": current_rows}));
                current_rows = Vec::new();
            }
            if let Some(s) = t.find("name=\"") {
                let rest = &t[s + 6..];
                if let Some(end) = rest.find('\"') {
                    current_area = Some(rest[..end].to_string());
                    in_area = true;
                }
            }
            continue;
        }

        if in_area && t.contains("</namedItem>") {
            if !current_rows.is_empty() {
                areas.push(serde_json::json!({"name": current_area, "rows": current_rows}));
                current_rows = Vec::new();
            }
            in_area = false;
            continue;
        }

        if in_area && t.contains("<rowsItem>") {
            // Collect cells until </rowsItem>
            let mut cells: Vec<Value> = Vec::new();
            let mut in_row = true;

            // Simple approach - just count cells in this row
            // For MVP, use a basic approach
            current_rows.push(serde_json::json!({"cells": []}));
            continue;
        }
    }

    if in_area && !current_rows.is_empty() {
        areas.push(serde_json::json!({"name": current_area, "rows": current_rows}));
    }

    if !areas.is_empty() {
        json["areas"] = serde_json::json!(areas);
    }

    // Stats
    let row_count: usize = areas.iter().map(|a| a.get("rows").and_then(|r| r.as_array()).map(|r| r.len()).unwrap_or(0)).sum();
    let output = serde_json::to_string_pretty(&json)?;

    if let Some(out) = out_path {
        fs::write(out, &output)?;
        Ok(format!("[OK] MXL декомпилирован: {} -> {}\n     Областей: {}, строк: {}", xml_path, out, areas.len(), row_count))
    } else {
        Ok(output)
    }
}
