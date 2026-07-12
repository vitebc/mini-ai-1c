use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn edit(args: Value) -> Result<String> {
    let config_path = args.get("config_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'config_path' обязателен"))?;
    let operation = args.get("operation").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'operation' обязателен"))?;
    let value = args.get("value").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or("");

    let p = Path::new(config_path);
    let cfg_path = if p.is_dir() { p.join("Configuration.xml") } else { p.to_path_buf() };
    if !cfg_path.exists() { return Err(anyhow!("Configuration.xml не найден: {}", cfg_path.display())); }

    let xml = fs::read_to_string(&cfg_path)?;

    let result = match operation {
        "modify-property" => modify_property(&xml, value)?,
        "add-childObject" => add_child_object(&xml, value)?,
        "remove-childObject" => remove_child_object(&xml, value)?,
        "set-defaultRoles" => set_default_roles(&xml, value)?,
        _ => return Err(anyhow!("Неизвестная операция: {}. Допустимо: modify-property, add-childObject, remove-childObject, set-defaultRoles", operation)),
    };

    fs::write(&cfg_path, &result)?;
    Ok(format!("[OK] Configuration.xml обновлён (операция: {})", operation))
}

fn modify_property(xml: &str, value: &str) -> Result<String> {
    let parts: Vec<&str> = value.splitn(2, '=').collect();
    if parts.len() != 2 { return Err(anyhow!("Формат: PropertyName=NewValue")); }
    let prop = parts[0].trim();
    let val = parts[1].trim();
    let open = format!("<{}>", prop);
    let close = format!("</{}>", prop);
    if let Some(start) = xml.find(&open) {
        let after = &xml[start + open.len()..];
        if let Some(end) = after.find(&close) {
            let before = &xml[..start + open.len()];
            let after_full = &xml[start + open.len() + end + close.len()..];
            return Ok(format!("{}{}{}", before, val, after_full));
        }
    }
    Err(anyhow!("Свойство {} не найдено в Configuration.xml", prop))
}

fn add_child_object(xml: &str, value: &str) -> Result<String> {
    let parts: Vec<&str> = value.splitn(2, '=').collect();
    let child_type = parts[0].trim();
    let child_name = if parts.len() > 1 { parts[1].trim() } else { child_type };

    if let Some(pos) = xml.find("<ChildObjects/>") {
        let before = &xml[..pos];
        let after = &xml[pos + 15..];
        return Ok(format!("{}<ChildObjects>\n\t\t<{}>{}</{}>\n\t</ChildObjects>{}", before, child_type, child_name, child_type, after));
    }
    if let Some(pos) = xml.find("</ChildObjects>") {
        let before = &xml[..pos];
        let after = &xml[pos..];
        let spaces: String = before.rsplit('\n').next().unwrap_or("").chars().take_while(|c| c.is_whitespace()).collect();
        return Ok(format!("{}\t{}<{}>{}</{}>\n{}", before, spaces, child_type, child_name, child_type, after));
    }
    Err(anyhow!("Не найден ChildObjects"))
}

fn remove_child_object(xml: &str, value: &str) -> Result<String> {
    let parts: Vec<&str> = value.splitn(2, '=').collect();
    let child_type = parts[0].trim();
    let child_name = if parts.len() > 1 { parts[1].trim() } else { child_type };
    let marker = format!("<{}>{}</{}>", child_type, child_name, child_type);

    if let Some(pos) = xml.find(&marker) {
        let before = &xml[..pos];
        let after = &xml[pos + marker.len()..];
        let trimmed = before.trim_end_matches(|c: char| c.is_whitespace());
        let ws = &before[trimmed.len()..];
        if ws.contains('\n') {
            let line_start = before.rfind('\n').map(|i| i).unwrap_or(0);
            return Ok(format!("{}{}", &xml[..line_start], after));
        }
        return Ok(format!("{}{}", trimmed, after));
    }
    Err(anyhow!("Элемент <{}>{}</{}> не найден", child_type, child_name, child_type))
}

fn set_default_roles(xml: &str, value: &str) -> Result<String> {
    let open = "<DefaultRoles>";
    let close = "</DefaultRoles>";
    if let Some(start) = xml.find(open) {
        let after = &xml[start + open.len()..];
        if let Some(end) = after.find(close) {
            let before = &xml[..start + open.len()];
            let after_full = &xml[start + open.len() + end + close.len()..];
            return Ok(format!("{}\n\t\t\t<v8:Value xsi:type=\"cfg:RoleRef\">{}</v8:Value>\n\t\t{}", before.trim_end(), value, after_full));
        }
    }
    Err(anyhow!("DefaultRoles не найден"))
}
