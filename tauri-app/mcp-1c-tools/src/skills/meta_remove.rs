use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const TYPE_DIR_MAP: &[(&str, &str)] = &[
    ("Catalog", "Catalogs"), ("Document", "Documents"), ("DataProcessor", "DataProcessors"),
    ("Report", "Reports"), ("InformationRegister", "InformationRegisters"),
    ("AccumulationRegister", "AccumulationRegisters"), ("ChartOfAccounts", "ChartsOfAccounts"),
    ("ChartOfCharacteristicTypes", "ChartsOfCharacteristicTypes"),
    ("BusinessProcess", "BusinessProcesses"), ("Task", "Tasks"),
    ("ExchangePlan", "ExchangePlans"), ("Enum", "Enums"),
    ("ExternalDataProcessor", "DataProcessors"), ("ExternalReport", "Reports"),
    ("Constant", "Constants"), ("DocumentJournal", "DocumentJournals"),
    ("FilterCriterion", "FilterCriteria"), ("ScheduledJob", "ScheduledJobs"),
    ("Sequence", "Sequences"), ("SettingsStorage", "SettingsStorages"),
    ("CalculationRegister", "CalculationRegisters"), ("AccountingRegister", "AccountingRegisters"),
];

pub async fn remove(args: Value) -> Result<String> {
    let config_dir = args.get("config_dir").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'config_dir' обязателен"))?;
    let object = args.get("object").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'object' обязателен"))?;
    let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);
    let keep_files = args.get("keep_files").and_then(|v| v.as_bool()).unwrap_or(false);
    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

    let cfg = Path::new(config_dir);
    if !cfg.join("Configuration.xml").exists() {
        return Err(anyhow!("Configuration.xml не найден в {}", config_dir));
    }

    // Parse Object spec (e.g. "Catalog.Товары")
    let parts: Vec<&str> = object.splitn(2, '.').collect();
    if parts.len() != 2 { return Err(anyhow!("Формат: Type.Name (например Catalog.Товары)")); }
    let obj_type = parts[0];
    let obj_name = parts[1];

    let dir_name = TYPE_DIR_MAP.iter().find(|(t, _)| *t == obj_type).map(|(_, d)| *d)
        .ok_or_else(|| anyhow!("Неизвестный тип объекта: {}", obj_type))?;

    let obj_file = cfg.join(dir_name).join(format!("{}.xml", obj_name));
    let obj_dir = cfg.join(dir_name).join(obj_name);

    if !obj_file.exists() && !obj_dir.exists() {
        return Err(anyhow!("Объект {} не найден в {}/{}", object, config_dir, dir_name));
    }

    let mut out = String::new();
    let mut modified = false;

    // 1. Remove from Configuration.xml ChildObjects
    let cfg_xml_path = cfg.join("Configuration.xml");
    let cfg_xml = fs::read_to_string(&cfg_xml_path)?;
    let marker = format!("<{}>{}</{}>", obj_type, obj_name, obj_type);

    if cfg_xml.contains(&marker) {
        let mut new_cfg = String::new();
        if let Some(pos) = cfg_xml.find(&marker) {
            let before = &cfg_xml[..pos];
            let after = &cfg_xml[pos + marker.len()..];
            let line_start = before.rfind('\n').map(|i| i).unwrap_or(0);
            new_cfg = format!("{}{}", &cfg_xml[..line_start], after);
        }
        if !dry_run {
            let bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF].iter().chain(new_cfg.as_bytes().iter()).copied().collect();
            fs::write(&cfg_xml_path, &bom)?;
        }
        out.push_str(&format!("[OK] Удалён из Configuration.xml ChildObjects\n"));
        modified = true;
    } else {
        out.push_str("[WARN] Объект не найден в ChildObjects Configuration.xml\n");
    }

    // 2. Remove from subsystems (scan all subsystem XMLs)
    let subsystems_dir = cfg.join("Subsystems");
    if subsystems_dir.exists() {
        scan_and_remove_from_dir(&subsystems_dir, obj_type, obj_name, dry_run, &mut out)?;
    }

    // 3. Delete object files
    if !keep_files && !dry_run {
        if obj_file.exists() {
            fs::remove_file(&obj_file)?;
            out.push_str(&format!("[OK] Удалён файл: {}\n", obj_file.display()));
        }
        if obj_dir.exists() {
            fs::remove_dir_all(&obj_dir)?;
            out.push_str(&format!("[OK] Удалён каталог: {}\n", obj_dir.display()));
        }
    }

    if !modified && !force {
        return Err(anyhow!("Объект не найден. Используйте --force для принудительного удаления файлов"));
    }

    out.push_str(&format!("\n[OK] Объект {} успешно удалён", object));
    Ok(out)
}

fn scan_and_remove_from_dir(dir: &Path, obj_type: &str, obj_name: &str, dry_run: bool, out: &mut String) -> Result<()> {
    if !dir.is_dir() { return Ok(()); }
    for entry in fs::read_dir(dir).context("Ошибка чтения каталога")? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "xml").unwrap_or(false) {
            let content = fs::read_to_string(&path)?;
            let ref_marker = format!("<Content>{}.{}</Content>", obj_type, obj_name);
            if content.contains(&ref_marker) {
                let new_content = content.replace(&ref_marker, "");
                if !dry_run {
                    fs::write(&path, &new_content)?;
                }
                out.push_str(&format!("[OK] Удалена ссылка из: {}\n", path.display()));
            }
        }
        if path.is_dir() {
            scan_and_remove_from_dir(&path, obj_type, obj_name, dry_run, out)?;
        }
    }
    Ok(())
}
