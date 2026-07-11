use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;

pub async fn get_meta_info(args: Value) -> Result<String> {
    let xml_path = args.get("xml_path").and_then(|v| v.as_str()).unwrap_or("");
    let xml = fs::read_to_string(xml_path).context("Failed to read metadata XML")?;

    let mut output = format!("=== Metadata Info: {} ===\n", xml_path);
    for line in xml.lines() {
        let trimmed = line.trim();
        if let Some(tag_start) = trimmed.find('<') {
            if let Some(tag_end) = trimmed[tag_start..].find('>') {
                let tag = &trimmed[tag_start..=tag_start + tag_end];
                if !tag.starts_with("</") && !tag.starts_with("<?") {
                    output.push_str(&format!("  {}\n", tag));
                }
            }
        }
    }

    let name = xml.lines()
        .find(|l| l.contains("<Name>"))
        .and_then(|l| {
            let s = l.trim();
            let start = s.find("<Name>").map(|i| i + 6)?;
            let end = s[start..].find("</Name>").map(|i| start + i)?;
            Some(&s[start..end])
        }).unwrap_or("unknown");

    output.push_str(&format!("\nObject name: {}", name));
    output.push_str(&format!("\nFile size: {} bytes", xml.len()));
    Ok(output)
}