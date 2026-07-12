use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn diff(args: Value) -> Result<String> {
    let ext_path = args.get("extension_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'extension_path' обязателен"))?;
    let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("A");

    let p = Path::new(ext_path);
    let cfg_path = if p.is_dir() { p.join("Configuration.xml") } else { p.to_path_buf() };
    if !cfg_path.exists() { return Err(anyhow!("Configuration.xml не найден: {}", cfg_path.display())); }

    let xml = fs::read_to_string(&cfg_path)?;
    let ext_dir = cfg_path.parent().unwrap_or(Path::new("."));

    let mut out = String::new();

    if mode == "A" {
        out.push_str(&format!("=== Extension Overview: {} ===\n\n", ext_path));

        let child_entries: Vec<&str> = xml.lines()
            .filter(|l| {
                let t = l.trim();
                (t.starts_with("<Catalog>") || t.starts_with("<Document>") || t.starts_with("<DataProcessor>") ||
                 t.starts_with("<Report>") || t.starts_with("<InformationRegister>") || t.starts_with("<Role>") ||
                 t.starts_with("<CommonModule>") || t.starts_with("<Enum>") || t.starts_with("<Constant>"))
                    && t.contains('/')
            })
            .collect();

        if child_entries.is_empty() {
            out.push_str("Объекты расширения:\n");
            for line in xml.lines() {
                let t = line.trim();
                // Simple extraction of ChildObjects entries
                if t.contains('>') && !t.contains('<') && !t.contains("xml") {
                    // Already handled by looping
                }
            }
            // Fallback: count object tags
            let obj_types = ["Catalog", "Document", "DataProcessor", "Report", "InformationRegister", "Role", "CommonModule", "Enum", "Constant", "Register", "ChartOfAccounts", "BusinessProcess", "Task", "ExchangePlan", "Subsystem", "FilterCriterion"];
            for ot in &obj_types {
                let count = xml.matches(&format!("<{}>", ot)).count();
                if count > 0 {
                    out.push_str(&format!("  {}: {} объектов\n", ot, count));
                }
            }
        }

        // BSL files
        let mut bsl_files = 0;
        if let Ok(entries) = fs::read_dir(ext_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "bsl").unwrap_or(false) {
                    bsl_files += 1;
                }
                if path.is_dir() {
                    bsl_files += count_bsl_files(&path);
                }
            }
        }
        out.push_str(&format!("\nBSL-файлов: ~{}\n", bsl_files));

        out.push_str("\n=== Summary ===\n");
        out.push_str(&format!("Расширение: {}", ext_path));
    } else {
        out.push_str("Режим B (проверка переноса) пока не реализован\n");
    }

    Ok(out)
}

fn count_bsl_files(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "bsl").unwrap_or(false) {
                count += 1;
            }
            if path.is_dir() {
                count += count_bsl_files(&path);
            }
        }
    }
    count
}
