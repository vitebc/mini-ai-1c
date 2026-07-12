use anyhow::{anyhow, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub async fn edit(args: Value) -> Result<String> {
    let form_path = args.get("form_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'form_path' обязателен"))?;
    let operation = args.get("operation").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'operation' обязателен"))?;
    let value = args.get("value").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or("");

    let p = Path::new(form_path);
    if !p.exists() { return Err(anyhow!("Form.xml не найден: {}", form_path)); }

    let xml = fs::read_to_string(p)?;
    let result = match operation {
        "add-element" => add_form_element(&xml, value)?,
        "remove-element" => remove_form_element(&xml, value)?,
        "move-element" => move_form_element(&xml, value)?,
        "set-property" => set_form_property(&xml, value)?,
        _ => return Err(anyhow!("Неизвестная операция. Допустимо: add-element, remove-element, move-element, set-property")),
    };

    let bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF].iter().chain(result.as_bytes().iter()).copied().collect();
    fs::write(p, &bom)?;
    Ok(format!("[OK] Form.xml обновлён (операция: {})", operation))
}

fn add_form_element(xml: &str, value: &str) -> Result<String> {
    let parts: Vec<&str> = value.splitn(3, ',').collect();
    if parts.len() < 2 { return Err(anyhow!("Формат: ElementType,Name[,Parent]")); }
    let el_type = parts[0].trim();
    let el_name = parts[1].trim();
    let el_xml = format!("<{} name=\"{}\" id=\"1\"/>", el_type, el_name);

    let target = if parts.len() > 2 {
        format!("</{}>", parts[2].trim())
    } else {
        "</ChildObjects>".to_string()
    };

    if let Some(pos) = xml.find(&target) {
        let before = &xml[..pos];
        let after = &xml[pos..];
        let spaces: String = before.rsplit('\n').next().unwrap_or("").chars().take_while(|c| c.is_whitespace()).collect();
        if target.starts_with("</") {
            return Ok(format!("{}\t{}<{} name=\"{}\" id=\"1\"/>\n{}", before, spaces, el_type, el_name, after));
        }
        return Ok(format!("{}\t{}<{} name=\"{}\" id=\"1\"/>\n{}", before, spaces, el_type, el_name, after));
    }
    Err(anyhow!("Не найден элемент для вставки"))
}

fn remove_form_element(xml: &str, value: &str) -> Result<String> {
    let name = value.trim();
    let markers = [
        format!("<InputField name=\"{}\"", name),
        format!("<Table name=\"{}\"", name),
        format!("<Group name=\"{}\"", name),
        format!("<Button name=\"{}\"", name),
        format!("<Label name=\"{}\"", name),
    ];

    for marker in &markers {
        if let Some(pos) = xml.find(marker.as_str()) {
            // Find the closing tag
            let rest = &xml[pos..];
            let close = rest.find("/>").map(|i| i + 2).or_else(|| rest.find("</").map(|i| i + rest[i..].find('>').unwrap_or(0) + 1));
            if let Some(end) = close {
                let before = &xml[..pos];
                let line_start = before.rfind('\n').map(|i| i).unwrap_or(0);
                return Ok(format!("{}{}", &xml[..line_start], &xml[pos + end..]));
            }
        }
    }
    Err(anyhow!("Элемент {} не найден", name))
}

fn move_form_element(xml: &str, value: &str) -> Result<String> {
    Err(anyhow!("Операция move-element пока не реализована"))
}

fn set_form_property(xml: &str, value: &str) -> Result<String> {
    let parts: Vec<&str> = value.splitn(3, '=').collect();
    if parts.len() < 2 { return Err(anyhow!("Формат: Element.Property=Value")); }
    let prop_path = parts[0].trim();
    let val = parts[1].trim();

    // Support ElementName.PropertyName syntax
    let path_parts: Vec<&str> = prop_path.splitn(2, '.').collect();
    if path_parts.len() == 2 {
        let el_name = path_parts[0];
        let prop = path_parts[1];
        let open = format!("<{}>", prop);
        let close = format!("</{}>", prop);

        // Find the element first, then the property
        if let Some(el_pos) = xml.find(&format!("name=\"{}\"", el_name)) {
            let after_el = &xml[el_pos..];
            if let Some(prop_start) = after_el.find(&open) {
                let actual_pos = el_pos + prop_start;
                let after_prop = &xml[actual_pos + open.len()..];
                if let Some(prop_end) = after_prop.find(&close) {
                    let result = format!("{}{}{}", &xml[..actual_pos + open.len()], val, &xml[actual_pos + open.len() + prop_end + close.len()..]);
                    return Ok(result);
                }
            }
        }
        return Err(anyhow!("Свойство {} не найдено для элемента {}", prop, el_name));
    }

    // Global property
    let open = format!("<{}>", prop_path);
    let close = format!("</{}>", prop_path);
    if let Some(start) = xml.find(&open) {
        let after = &xml[start + open.len()..];
        if let Some(end) = after.find(&close) {
            return Ok(format!("{}{}{}", &xml[..start + open.len()], val, &xml[start + open.len() + end + close.len()..]));
        }
    }
    Err(anyhow!("Свойство {} не найдено", prop_path))
}
