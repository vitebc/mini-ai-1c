use anyhow::{anyhow, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn edit(args: Value) -> Result<String> {
    let path = args.get("path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'path' обязателен"))?;
    let operation = args.get("operation").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'operation' обязателен"))?;
    let value = args.get("value").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or("");
    let create_if_missing = args.get("create_if_missing").and_then(|v| v.as_bool()).unwrap_or(false);

    let p = Path::new(path);
    let ci_path: std::path::PathBuf = if p.is_dir() { p.join("CommandInterface.xml") } else { p.to_path_buf() };

    if !ci_path.exists() {
        if create_if_missing {
            let default = r#"<?xml version="1.0" encoding="UTF-8"?>
<CommandInterface xmlns="http://v8.1c.ru/8.2/managed-application/core" xmlns:xr="http://v8.1c.ru/8.3/xcf/readable">
	<CommandsVisibility/>
	<CommandsPlacement/>
	<CommandsOrder/>
	<SubsystemsOrder/>
	<GroupsOrder/>
</CommandInterface>"#;
            let bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF].iter().chain(default.as_bytes().iter()).copied().collect();
            fs::write(&ci_path, &bom)?;
        } else {
            return Err(anyhow!("CommandInterface.xml не найден: {}", ci_path.display()));
        }
    }

    let xml = fs::read_to_string(&ci_path)?;
    let result = match operation {
        "hide" => set_visibility(&xml, value, false)?,
        "show" => set_visibility(&xml, value, true)?,
        "place" => place_command(&xml, value)?,
        "order" => set_order(&xml, value)?,
        "subsystem-order" => set_subsystem_order(&xml, value)?,
        _ => return Err(anyhow!("Неизвестная операция: {}", operation)),
    };

    let bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF].iter().chain(result.as_bytes().iter()).copied().collect();
    fs::write(&ci_path, &bom)?;
    Ok(format!("[OK] CommandInterface.xml обновлён (операция: {})", operation))
}

fn set_visibility(xml: &str, value: &str, visible: bool) -> Result<String> {
    let cmd_name = normalize_cmd(value);
    let marker = format!("<Command name=\"{}\">", cmd_name);
    let vis = if visible { "true" } else { "false" };

    if let Some(pos) = xml.find(&marker) {
        let rest = &xml[pos..];
        if let Some(vis_end) = rest.find("</Command>") {
            let cmd_section = &rest[..vis_end + 10];
            if cmd_section.contains("<Visibility>") {
                // Replace existing visibility
                let new_cmd = if let Some(vpos) = cmd_section.find("<xr:Common>") {
                    let before = &xml[..pos + vpos + 11];
                    let after = &xml[pos + vpos + 11 + cmd_section[vpos..].find("</xr:Common>").unwrap_or(13) + 13..];
                    format!("{}{}", before, after)
                } else {
                    xml.to_string()
                };
                return set_visibility_if_not_found(&new_cmd, value, visible);
            } else {
                // Add visibility before </Command>
                if let Some(end) = rest.find("</Command>") {
                    let before = &xml[..pos + end];
                    let after = &xml[pos + end..];
                    return Ok(format!("{}\n\t\t<Visibility>\n\t\t\t<xr:Common>{}</xr:Common>\n\t\t</Visibility>{}", before, vis, after));
                }
            }
        }
    }

    // Command not found, add it
    let new_cmd = format!("<Command name=\"{}\">\n\t\t<Visibility>\n\t\t\t<xr:Common>{}</xr:Common>\n\t\t</Visibility>\n\t</Command>", cmd_name, vis);

    if let Some(pos) = xml.find("</CommandsVisibility>") {
        let before = &xml[..pos];
        let after = &xml[pos..];
        let spaces: String = before.rsplit('\n').next().unwrap_or("").chars().take_while(|c| c.is_whitespace()).collect();
        return Ok(format!("{}\t{}<Command name=\"{}\" id=\"1\">\n\t\t\t<Visibility>\n\t\t\t\t<xr:Common>{}</xr:Common>\n\t\t\t</Visibility>\n\t\t</Command>\n{}", before, spaces, cmd_name, vis, after));
    }

    Err(anyhow!("Не найден CommandsVisibility"))
}

fn set_visibility_if_not_found(xml: &str, value: &str, _visible: bool) -> Result<String> {
    Err(anyhow!("Не удалось обновить видимость для {}", value))
}

fn place_command(xml: &str, value: &str) -> Result<String> {
    let parts: Vec<&str> = value.splitn(3, '=').collect();
    if parts.len() < 2 { return Err(anyhow!("Формат: CommandName=Group[=Placement]")); }
    let cmd = normalize_cmd(parts[0]);
    let group = parts[1].trim();
    let placement = if parts.len() > 2 { parts[2].trim() } else { "" };

    if let Some(pos) = xml.find("</CommandsPlacement>") {
        let before = &xml[..pos];
        let after = &xml[pos..];
        let spaces: String = before.rsplit('\n').next().unwrap_or("").chars().take_while(|c| c.is_whitespace()).collect();
        let entry = if placement.is_empty() {
            format!("{}<Command name=\"{}\"><CommandGroup>{}</CommandGroup></Command>\n", before, cmd, group)
        } else {
            format!("{}<Command name=\"{}\"><CommandGroup>{}</CommandGroup><Placement>{}</Placement></Command>\n", before, cmd, group, placement)
        };
        let after_parts: Vec<&str> = after.splitn(2, '\n').collect();
        Ok(format!("{}{}\n{}", entry, spaces, after_parts.get(1).unwrap_or(&"")))
    } else {
        Err(anyhow!("Не найден CommandsPlacement"))
    }
}

fn set_order(xml: &str, value: &str) -> Result<String> {
    let parts: Vec<&str> = value.splitn(2, '=').collect();
    if parts.len() != 2 { return Err(anyhow!("Формат: Group=OrderedList")); }
    let group = parts[0].trim();
    let items: Vec<&str> = parts[1].split(',').map(|s| s.trim()).collect();

    let mut new_order = format!("<Group name=\"{}\">", group);
    for item in &items {
        let cmd = normalize_cmd(item);
        new_order.push_str(&format!("<Command>{}</Command>", cmd));
    }
    new_order.push_str(&format!("</Group>", ));

    if let Some(pos) = xml.find("<CommandsOrder>") {
        let before = &xml[..pos + 15];
        let after_full = &xml[pos..];
        if let Some(end) = after_full.find("</CommandsOrder>") {
            let after = &xml[pos + end..];
            return Ok(format!("{}{}{}", before, new_order, after));
        }
    }
    Err(anyhow!("Не найден CommandsOrder"))
}

fn set_subsystem_order(xml: &str, value: &str) -> Result<String> {
    let items: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
    let mut new_order = String::new();
    for item in &items {
        new_order.push_str(&format!("<Subsystem>{}</Subsystem>", item));
    }

    if let Some(pos) = xml.find("<SubsystemsOrder>") {
        let before = &xml[..pos + 17];
        if let Some(end) = xml[pos..].find("</SubsystemsOrder>") {
            let after = &xml[pos + end..];
            return Ok(format!("{}{}{}", before, new_order, after));
        }
    }
    Err(anyhow!("Не найден SubsystemsOrder"))
}

fn normalize_cmd(value: &str) -> String {
    let v = value.trim();
    let mut s = v.to_string();
    let reps = [
        ("Справочник.", "Catalog."), ("Документ.", "Document."), ("Обработка.", "DataProcessor."),
        ("Отчёт.", "Report."),
    ];
    for (rus, eng) in &reps {
        if s.starts_with(rus) { s = s.replacen(rus, eng, 1); break; }
    }
    s
}
