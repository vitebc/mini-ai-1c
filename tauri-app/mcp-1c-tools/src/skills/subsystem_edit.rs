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

    let p = Path::new(path);
    let xml_path = if p.is_dir() { p.join("Subsystem.xml") } else { p.to_path_buf() };
    if !xml_path.exists() { return Err(anyhow!("Subsystem.xml не найден: {}", xml_path.display())); }

    let xml = fs::read_to_string(&xml_path)?;

    let result = match operation {
        "add-content" => add_content(&xml, value)?,
        "remove-content" => remove_content(&xml, value)?,
        "add-child" => add_child(&xml, value)?,
        "remove-child" => remove_child(&xml, value)?,
        "set-property" => set_property(&xml, value)?,
        _ => return Err(anyhow!("Неизвестная операция: {}. Допустимо: add-content, remove-content, add-child, remove-child, set-property", operation)),
    };

    let bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF].iter().chain(result.as_bytes().iter()).copied().collect();
    fs::write(&xml_path, &bom)?;
    Ok(format!("[OK] Subsystem.xml обновлён (операция: {})", operation))
}

fn add_content(xml: &str, value: &str) -> Result<String> {
    let norm = normalize_ref(value);
    if let Some(pos) = xml.find("</Content>") {
        let before = &xml[..pos];
        let after = &xml[pos..];
        let spaces: String = before.rsplit('\n').next().unwrap_or("").chars().take_while(|c| c.is_whitespace()).collect();
        if xml.contains(&norm) {
            return Err(anyhow!("Объект {} уже есть в Content", norm));
        }
        return Ok(format!("{}\t{}<xr:Item xr:type=\"MDObjectRef\">{}</xr:Item>\n{}", before, spaces, norm, after));
    }
    if let Some(pos) = xml.find("<Content/>") {
        let before = &xml[..pos];
        let after = &xml[pos + 10..];
        return Ok(format!("{}<Content>\n\t\t<xr:Item xr:type=\"MDObjectRef\">{}</xr:Item>\n\t</Content>{}", before, norm, after));
    }
    Err(anyhow!("Не найден Content"))
}

fn remove_content(xml: &str, value: &str) -> Result<String> {
    let norm = normalize_ref(value);
    let marker = format!("<xr:Item xr:type=\"MDObjectRef\">{}</xr:Item>", norm);
    if let Some(pos) = xml.find(&marker) {
        let before = &xml[..pos];
        let after = &xml[pos + marker.len()..];
        let line_start = before.rfind('\n').map(|i| i).unwrap_or(0);
        Ok(format!("{}{}", &xml[..line_start], after))
    } else {
        Err(anyhow!("Объект {} не найден в Content", norm))
    }
}

fn add_child(xml: &str, value: &str) -> Result<String> {
    let name = value.trim();
    let marker = format!("<Subsystem>{}</Subsystem>", name);
    if xml.contains(&marker) {
        return Err(anyhow!("Дочерняя подсистема {} уже существует", name));
    }
    if let Some(pos) = xml.find("</ChildObjects>") {
        let before = &xml[..pos];
        let after = &xml[pos..];
        let spaces: String = before.rsplit('\n').next().unwrap_or("").chars().take_while(|c| c.is_whitespace()).collect();
        Ok(format!("{}\t{}<Subsystem>{}</Subsystem>\n{}", before, spaces, name, after))
    } else if let Some(pos) = xml.find("<ChildObjects/>") {
        let before = &xml[..pos];
        let after = &xml[pos + 15..];
        Ok(format!("{}<ChildObjects>\n\t\t<Subsystem>{}</Subsystem>\n\t</ChildObjects>{}", before, name, after))
    } else {
        Err(anyhow!("Не найден ChildObjects"))
    }
}

fn remove_child(xml: &str, value: &str) -> Result<String> {
    let name = value.trim();
    let marker = format!("<Subsystem>{}</Subsystem>", name);
    if let Some(pos) = xml.find(&marker) {
        let before = &xml[..pos];
        let after = &xml[pos + marker.len()..];
        let line_start = before.rfind('\n').map(|i| i).unwrap_or(0);
        Ok(format!("{}{}", &xml[..line_start], after))
    } else {
        Err(anyhow!("Дочерняя подсистема {} не найдена", name))
    }
}

fn set_property(xml: &str, value: &str) -> Result<String> {
    let parts: Vec<&str> = value.splitn(2, '=').collect();
    if parts.len() != 2 { return Err(anyhow!("Формат: PropertyName=Value")); }
    let prop = parts[0].trim();
    let val = parts[1].trim();

    // For boolean properties
    if matches!(prop, "IncludeHelpInContents" | "IncludeInCommandInterface" | "UseOneCommand") {
        let new_val = if val.eq_ignore_ascii_case("true") || val == "1" { "true" } else { "false" };
        return replace_xml_value(xml, prop, new_val);
    }

    // For Synonym/Explanation - multi-language
    if prop == "Synonym" || prop == "Explanation" {
        let new_xml = format!("<v8:item><v8:lang>ru</v8:lang><v8:content>{}</v8:content></v8:item>", val);
        return replace_xml_value(xml, prop, &new_xml);
    }

    replace_xml_value(xml, prop, val)
}

fn replace_xml_value(xml: &str, tag: &str, new_val: &str) -> Result<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = xml.find(&open) {
        let after = &xml[start + open.len()..];
        if let Some(end) = after.find(&close) {
            return Ok(format!("{}{}{}", &xml[..start + open.len()], new_val, &xml[start + open.len() + end + close.len()..]));
        }
    }
    Err(anyhow!("Свойство {} не найдено", tag))
}

fn normalize_ref(value: &str) -> String {
    let v = value.trim();
    // Handle Russian type names
    let mut s = v.to_string();
    let replacements = [
        ("Справочники.", "Catalog."), ("Документы.", "Document."), ("Обработки.", "DataProcessor."),
        ("Отчёты.", "Report."), ("РегистрыСведений.", "InformationRegister."),
        ("РегистрыНакопления.", "AccumulationRegister."), ("ПланыСчетов.", "ChartOfAccounts."),
        ("ПланыВидовХарактеристик.", "ChartOfCharacteristicTypes."), ("БизнесПроцессы.", "BusinessProcess."),
        ("Задачи.", "Task."), ("ПланыОбмена.", "ExchangePlan."), ("Перечисления.", "Enum."),
        ("Константы.", "Constant."), ("ЖурналыДокументов.", "DocumentJournal."),
    ];
    for (rus, eng) in &replacements {
        if s.starts_with(rus) { s = s.replacen(rus, eng, 1); break; }
    }
    // Remove .xml extension if present
    if s.ends_with(".xml") { s.truncate(s.len() - 4); }
    s
}
