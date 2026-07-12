use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn edit(args: Value) -> Result<String> {
    let path = args.get("path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'path' обязателен"))?;
    let operation = args.get("operation").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'operation' обязателен"))?;
    let value = args.get("value").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or("");

    let p = Path::new(path);
    if !p.exists() { return Err(anyhow!("Файл не найден: {}", path)); }

    let xml = fs::read_to_string(p)?;

    let result = match operation {
        "set-name" => set_element(&xml, "name", value)?,
        "add-dataset" => add_dataset(&xml, value)?,
        "add-field" => add_field(&xml, value)?,
        "set-setting" => set_setting(&xml, value)?,
        _ => return Err(anyhow!("Неизвестная операция. Допустимо: set-name, add-dataset, add-field, set-setting")),
    };

    let bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF].iter().chain(result.as_bytes().iter()).copied().collect();
    fs::write(p, &bom)?;
    Ok(format!("[OK] SKD обновлён (операция: {})", operation))
}

fn set_element(xml: &str, tag: &str, value: &str) -> Result<String> {
    let parts: Vec<&str> = value.splitn(2, '=').collect();
    if parts.len() != 2 { return Err(anyhow!("Формат: ElementName=NewValue")); }
    let el_name = parts[0].trim();
    let new_val = parts[1].trim();

    let marker = format!("<{}>", el_name);
    let end_marker = format!("</{}>", el_name);

    if let Some(start) = xml.find(&marker) {
        let after = &xml[start + marker.len()..];
        if let Some(end) = after.find(&end_marker) {
            return Ok(format!("{}{}{}", &xml[..start + marker.len()], new_val, &xml[start + marker.len() + end + end_marker.len()..]));
        }
    }
    Err(anyhow!("Элемент <{}> не найден в SKD", el_name))
}

fn add_dataset(xml: &str, value: &str) -> Result<String> {
    let parts: Vec<&str> = value.splitn(2, '=').collect();
    let ds_name = parts[0].trim();
    let ds_type = if parts.len() > 1 { parts[1].trim() } else { "Local" };

    let ds_xml = format!("\n\t\t<dataSource>\n\t\t\t<name>{}</name>\n\t\t\t<dataSourceType>{}</dataSourceType>\n\t\t</dataSource>", ds_name, ds_type);

    if let Some(pos) = xml.find("</dataSource>") {
        let before = &xml[..pos];
        let after = &xml[pos..];
        let spaces: String = before.rsplit('\n').next().unwrap_or("").chars().take_while(|c| c.is_whitespace()).collect();
        return Ok(format!("{}\t{}{}\n{}", before, spaces, ds_xml, after));
    }
    Err(anyhow!("dataSource не найден"))
}

fn add_field(xml: &str, value: &str) -> Result<String> {
    let parts: Vec<&str> = value.splitn(3, '=').collect();
    if parts.len() < 2 { return Err(anyhow!("Формат: DataSet.FieldName=Type")); }

    let ds_field = parts[0].trim();
    let field_type = parts[1].trim();
    let parts2: Vec<&str> = ds_field.splitn(2, '.').collect();
    if parts2.len() != 2 { return Err(anyhow!("Формат: DataSetName.FieldName")); }
    let ds_name = parts2[0];
    let field_name = parts2[1];

    // Find the dataset and add field
    let ds_marker = format!("<name>{}</name>", ds_name);
    if let Some(ds_pos) = xml.find(&ds_marker) {
        let after_ds = &xml[ds_pos..];
        // Find the end of the dataset's field list
        if let Some(close_pos) = after_ds.find("</DataSet>") {
            let ds_section = &after_ds[..close_pos];
            let insert_pos = ds_section.rfind("<Field").map(|p| ds_section[..p].rfind('\n').unwrap_or(0)).unwrap_or(0);
            let actual_pos = ds_pos + insert_pos;

            let before = &xml[..actual_pos];
            let after = &xml[ds_pos + close_pos..];
            let spaces: String = before.rsplit('\n').next().unwrap_or("").chars().take_while(|c| c.is_whitespace()).collect();
            let field_xml = format!("{}<Field name=\"{}\" type=\"{}\" dataType=\"String\"/>", spaces, field_name, field_type);
            return Ok(format!("{}\n{}", before, field_xml));
        }
    }
    Err(anyhow!("DataSet {} не найден", ds_name))
}

fn set_setting(xml: &str, value: &str) -> Result<String> {
    let parts: Vec<&str> = value.splitn(3, '=').collect();
    if parts.len() < 2 { return Err(anyhow!("Формат: Setting=Field=Value или Setting=Value")); }

    let setting = parts[0].trim();

    if parts.len() == 2 {
        return set_element(xml, setting, &format!("{}={}", setting, parts[1]));
    }

    // Setting.Field=Value
    let field = parts[1].trim();
    let val = parts[2].trim();

    let marker = format!("<{}>", setting);
    if let Some(s_pos) = xml.find(&marker) {
        let rest = &xml[s_pos..];
        if let Some(e_pos) = rest.find(&format!("</{}>", setting)) {
            let section = &rest[..e_pos];
            let field_marker = format!("<{}>", field);
            if let Some(f_pos) = section.find(&field_marker) {
                let after_field = &section[f_pos + field_marker.len()..];
                let field_end = after_field.find(&format!("</{}>", field)).unwrap_or(0);
                let before = &xml[..s_pos + f_pos + field_marker.len()];
                let after_full = &xml[s_pos + f_pos + field_marker.len() + field_end + field.len() + 3..];
                return Ok(format!("{}{}{}", before, val, after_full));
            }
        }
    }
    Err(anyhow!("Настройка {} не найдена", setting))
}
