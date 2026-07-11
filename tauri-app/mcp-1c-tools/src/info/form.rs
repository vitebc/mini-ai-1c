use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;

pub async fn get_form_info(args: Value) -> Result<String> {
    let xml_path = args.get("xml_path").and_then(|v| v.as_str()).unwrap_or("");
    let xml = fs::read_to_string(xml_path).context("Failed to read Form.xml")?;

    let mut output = format!("=== Form Info: {} ===\n", xml_path);
    output.push_str("Elements:\n");
    for line in xml.lines() {
        let trimmed = line.trim();
        if trimmed.contains("Name=\"") {
            if let Some(start) = trimmed.find("Name=\"") {
                let rest = &trimmed[start + 6..];
                if let Some(end) = rest.find('\"') {
                    output.push_str(&format!("  - {}\n", &rest[..end]));
                }
            }
        }
    }
    output.push_str(&format!("\nTotal size: {} bytes", xml.len()));
    Ok(output)
}