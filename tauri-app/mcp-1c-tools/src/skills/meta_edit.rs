use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn edit(args: Value) -> Result<String> {
    let object_path = args.get("object_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'object_path' обязателен"))?;
    let operation = args.get("operation").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'operation' обязателен"))?;
    let value = args.get("value").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or("");

    let p = Path::new(object_path);
    let obj_path = if p.is_dir() {
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("object");
        p.join(format!("{}.xml", name))
    } else { p.to_path_buf() };

    if !obj_path.exists() { return Err(anyhow!("Файл объекта не найден: {}", obj_path.display())); }

    let xml = fs::read_to_string(&obj_path)?;
    let result = match operation {
        "add-attribute" => add_or_remove_attribute(&xml, value, true)?,
        "remove-attribute" => add_or_remove_attribute(&xml, value, false)?,
        "modify-property" => modify_property(&xml, value)?,
        "add-tabularSection" => add_tabular_section(&xml, value)?,
        "remove-tabularSection" => remove_tabular_section(&xml, value)?,
        "set-synonym" => set_synonym(&xml, value)?,
        _ => return Err(anyhow!("Неизвестная операция. Допустимо: add-attribute, remove-attribute, modify-property, add-tabularSection, remove-tabularSection, set-synonym")),
    };

    let bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF].iter().chain(result.as_bytes().iter()).copied().collect();
    fs::write(&obj_path, &bom)?;
    Ok(format!("[OK] Объект {} обновлён (операция: {})", obj_path.display(), operation))
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
            return Ok(format!("{}{}{}", &xml[..start + open.len()], val, &xml[start + open.len() + end + close.len()..]));
        }
    }
    Err(anyhow!("Свойство {} не найдено", prop))
}

fn add_or_remove_attribute(xml: &str, value: &str, add: bool) -> Result<String> {
    let parts: Vec<&str> = value.splitn(3, '=').collect();
    if parts.len() < 2 { return Err(anyhow!("Формат: AttributeName=Type или AttributeName=Type=NewName")); }
    let attr_name = parts[0].trim();
    let attr_type = parts[1].trim();
    let new_name = if parts.len() > 2 { parts[2].trim() } else { attr_name };

    if add {
        // Find ChildObjects or Attributes section and insert
        let attr_xml = format!("\n\t\t<Attribute name=\"{}\">\n\t\t\t<Type>\n\t\t\t\t<v8:Type>{}</v8:Type>\n\t\t\t</Type>\n\t\t</Attribute>", attr_name, attr_type);
        if let Some(pos) = xml.find("</Attributes>") {
            let before = &xml[..pos];
            let after = &xml[pos..];
            let spaces: String = before.rsplit('\n').next().unwrap_or("").chars().take_while(|c| c.is_whitespace()).collect();
            return Ok(format!("{}\t{}<Attribute name=\"{}\">\n\t\t\t<Type>\n\t\t\t\t<v8:Type>{}</v8:Type>\n\t\t\t</Type>\n\t\t</Attribute>\n{}", before, spaces, attr_name, attr_type, after));
        }
        return Err(anyhow!("Не найден раздел Attributes"));
    } else {
        // Remove attribute
        let marker = format!("<Attribute name=\"{}\"", attr_name);
        if let Some(pos) = xml.find(&marker) {
            // Find the closing </Attribute>
            if let Some(end) = xml[pos..].find("</Attribute>") {
                let attr_end = pos + end + 12;
                // Clean up preceding whitespace
                let before = &xml[..pos];
                let trimmed = before.trim_end_matches(|c: char| c.is_whitespace());
                let line_start = before.rfind('\n').map(|i| i).unwrap_or(0);
                return Ok(format!("{}{}", &xml[..line_start], &xml[attr_end..]));
            }
        }
        return Err(anyhow!("Атрибут {} не найден", attr_name));
    }
}

fn add_tabular_section(xml: &str, value: &str) -> Result<String> {
    let parts: Vec<&str> = value.splitn(2, '=').collect();
    let ts_name = parts[0].trim();
    let ts_synonym = if parts.len() > 1 { parts[1].trim() } else { ts_name };

    let ts_xml = format!("\n\t\t<Table name=\"{}\">\n\t\t\t<Name>{}</Name>\n\t\t\t<Synonym>\n\t\t\t\t<v8:item>\n\t\t\t\t\t<v8:lang>ru</v8:lang>\n\t\t\t\t\t<v8:content>{}</v8:content>\n\t\t\t\t</v8:item>\n\t\t\t</Synonym>\n\t\t</Table>", ts_name, ts_name, ts_synonym);

    if let Some(pos) = xml.find("</TabularSections>") {
        let before = &xml[..pos];
        let after = &xml[pos..];
        let spaces: String = before.rsplit('\n').next().unwrap_or("").chars().take_while(|c| c.is_whitespace()).collect();
        return Ok(format!("{}\t{}<Table name=\"{}\">\n\t\t\t<Name>{}</Name>\n\t\t\t<Synonym>\n\t\t\t\t<v8:item>\n\t\t\t\t\t<v8:lang>ru</v8:lang>\n\t\t\t\t\t<v8:content>{}</v8:content>\n\t\t\t\t</v8:item>\n\t\t\t</Synonym>\n\t\t</Table>\n{}", before, spaces, ts_name, ts_name, ts_synonym, after));
    }
    if let Some(pos) = xml.find("<TabularSections/>") {
        let before = &xml[..pos];
        let after = &xml[pos + 19..];
        return Ok(format!("{}<TabularSections>\n\t\t<Table name=\"{}\">\n\t\t\t<Name>{}</Name>\n\t\t\t<Synonym>\n\t\t\t\t<v8:item>\n\t\t\t\t\t<v8:lang>ru</v8:lang>\n\t\t\t\t\t<v8:content>{}</v8:content>\n\t\t\t\t</v8:item>\n\t\t\t</Synonym>\n\t\t</Table>\n\t</TabularSections>{}", before, ts_name, ts_name, ts_synonym, after));
    }
    Err(anyhow!("Не найден TabularSections"))
}

fn remove_tabular_section(xml: &str, value: &str) -> Result<String> {
    let name = value.trim();
    let marker = format!("<Table name=\"{}\"", name);
    if let Some(pos) = xml.find(&marker) {
        if let Some(end) = xml[pos..].find("</Table>") {
            let section_end = pos + end + 8;
            let before = &xml[..pos];
            let line_start = before.rfind('\n').map(|i| i).unwrap_or(0);
            return Ok(format!("{}{}", &xml[..line_start], &xml[section_end..]));
        }
    }
    Err(anyhow!("Табличная часть {} не найдена", name))
}

fn set_synonym(xml: &str, value: &str) -> Result<String> {
    let parts: Vec<&str> = value.splitn(2, '=').collect();
    let lang = if parts.len() > 1 { parts[0].trim() } else { "ru" };
    let content = if parts.len() > 1 { parts[1].trim() } else { parts[0].trim() };

    if let Some(pos) = xml.find("<Synonym>") {
        let after = &xml[pos + 9..];
        if let Some(end) = after.find("</Synonym>") {
            let before = &xml[..pos + 9];
            let after_full = &xml[pos + 9 + end + 9..];
            let new_syn = format!("<v8:item><v8:lang>{}</v8:lang><v8:content>{}</v8:content></v8:item>\n\t\t", lang, content);
            return Ok(format!("{}{}{}", before, new_syn, after_full));
        }
    }
    Err(anyhow!("Synonym не найден"))
}
