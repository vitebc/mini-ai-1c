use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;

pub async fn get_skd_info(args: Value) -> Result<String> {
    let xml_path = args.get("xml_path").and_then(|v| v.as_str()).unwrap_or("");
    let xml = fs::read_to_string(xml_path).context("Failed to read SKD XML")?;

    let mut output = format!("=== SKD Schema Info: {} ===\n", xml_path);

    let datasets: Vec<&str> = xml.lines()
        .filter(|l| l.contains("<DataSet>") || l.contains("<DataSource>"))
        .map(|l| l.trim())
        .collect();
    output.push_str(&format!("DataSources/Datasets: {}\n", datasets.len()));

    let fields: Vec<&str> = xml.lines()
        .filter(|l| l.contains("<Field"))
        .map(|l| l.trim())
        .collect();
    for f in &fields {
        output.push_str(&format!("  - {}\n", f));
    }

    let settings = xml.lines().filter(|l| l.contains("<Settings")).count();
    output.push_str(&format!("\nVariants: {}", settings));
    output.push_str(&format!("\nTotal size: {} bytes", xml.len()));
    Ok(output)
}