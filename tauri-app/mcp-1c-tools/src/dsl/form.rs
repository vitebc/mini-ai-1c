use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn compile_form(args: Value) -> Result<String> {
    let json_path = args.get("json_path").and_then(|v| v.as_str()).unwrap_or("");
    let out_path = args.get("out_path").and_then(|v| v.as_str()).unwrap_or("");

    let json_str = fs::read_to_string(json_path).context("Failed to read JSON file")?;
    let _json: Value = serde_json::from_str(&json_str).context("Invalid JSON")?;

    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<Form xmlns=\"http://v8.1c.ru/8.2/uicustom\">\n");
    xml.push_str("  <Version>1.0</Version>\n");
    xml.push_str("  <FormElements>\n");
    if let Some(elements) = _json.get("elements").and_then(|v| v.as_array()) {
        for el in elements {
            let name = el.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
            xml.push_str(&format!("    <FormElement Name=\"{}\"/>\n", name));
        }
    }
    xml.push_str("  </FormElements>\n");
    xml.push_str("</Form>\n");

    fs::write(out_path, &xml).context("Failed to write XML")?;
    Ok(format!("Form compiled: {} -> {}", json_path, out_path))
}

pub async fn decompile_form(args: Value) -> Result<String> {
    let xml_path = args.get("xml_path").and_then(|v| v.as_str()).unwrap_or("");
    let out_path = args.get("out_path").and_then(|v| v.as_str()).unwrap_or("");

    let xml_str = fs::read_to_string(xml_path).context("Failed to read XML")?;

    let mut json = serde_json::json!({
        "type": "form",
        "elements": []
    });
    if let Some(elements) = json.get_mut("elements").and_then(|v| v.as_array_mut()) {
        elements.push(serde_json::json!({"name": "Form", "title": "Form from XML"}));
    }

    fs::write(out_path, serde_json::to_string_pretty(&json)?).context("Failed to write JSON")?;
    Ok(format!("Form decompiled: {} -> {}", xml_path, out_path))
}