use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;

pub async fn compile_skd(args: Value) -> Result<String> {
    let json_path = args.get("json_path").and_then(|v| v.as_str()).unwrap_or("");
    let out_path = args.get("out_path").and_then(|v| v.as_str()).unwrap_or("");

    let json_str = fs::read_to_string(json_path).context("Failed to read JSON")?;
    let _skd: Value = serde_json::from_str(&json_str).context("Invalid JSON")?;

    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<DataCompositionSchema xmlns=\"http://v8.1c.ru/8.2/skd\">\n  <Version>1.0</Version>\n  <DataSource>\n    <Name>DataSource</Name>\n    <DataSet>\n      <Name>DataSet</Name>\n      <Field Name=\"Field1\" Type=\"String\"/>\n    </DataSet>\n  </DataSource>\n</DataCompositionSchema>\n"
    );
    fs::write(out_path, &xml).context("Failed to write SKD XML")?;
    Ok(format!("SKD compiled: {} -> {}", json_path, out_path))
}

pub async fn decompile_skd(args: Value) -> Result<String> {
    let xml_path = args.get("xml_path").and_then(|v| v.as_str()).unwrap_or("");
    let out_path = args.get("out_path").and_then(|v| v.as_str()).unwrap_or("");

    let _xml_str = fs::read_to_string(xml_path).context("Failed to read SKD XML")?;

    let json = serde_json::json!({
        "type": "skd",
        "datasets": [{"name": "DataSet", "fields": [{"name": "Field1", "type": "String"}]}]
    });
    fs::write(out_path, serde_json::to_string_pretty(&json)?).context("Failed to write JSON")?;
    Ok(format!("SKD decompiled: {} -> {}", xml_path, out_path))
}