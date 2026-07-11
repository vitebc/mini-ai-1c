use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;

pub async fn compile_meta(args: Value) -> Result<String> {
    let json_path = args.get("json_path").and_then(|v| v.as_str()).unwrap_or("");
    let out_dir = args.get("out_dir").and_then(|v| v.as_str()).unwrap_or("");

    let json_str = fs::read_to_string(json_path).context("Failed to read JSON")?;
    let _meta: Value = serde_json::from_str(&json_str).context("Invalid JSON")?;

    let meta_type = _meta.get("type").and_then(|v| v.as_str()).unwrap_or("Catalog");
    let meta_name = _meta.get("name").and_then(|v| v.as_str()).unwrap_or("NewObject");

    let obj_dir = format!("{}\\{}.{}", out_dir, meta_type, meta_name);
    fs::create_dir_all(&obj_dir).context("Failed to create object dir")?;

    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<{0} xmlns=\"http://v8.1c.ru/8.2/managed-application\">\n  <Name>{1}</Name>\n  <Synonym>\"{1}\"</Synonym>\n</{0}>\n",
        meta_type, meta_name
    );
    let xml_path = format!("{}\\{}.xml", obj_dir, meta_type);
    fs::write(&xml_path, &xml).context("Failed to write XML")?;

    Ok(format!("Metadata compiled: {} -> {} objects", json_path, meta_name))
}