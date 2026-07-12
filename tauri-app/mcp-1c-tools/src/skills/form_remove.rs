use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub async fn remove_form(args: Value) -> Result<String> {
    let object_name = args.get("object_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'object_name' обязателен"))?;

    let form_name = args.get("form_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'form_name' обязателен"))?;

    let src_dir = args.get("src_dir")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("src");

    let root_xml_path = PathBuf::from(src_dir).join(format!("{}.xml", object_name));
    if !root_xml_path.exists() {
        return Err(anyhow!("Корневой файл объекта не найден: {}", root_xml_path.display()));
    }

    let forms_dir = PathBuf::from(src_dir).join(object_name).join("Forms");
    let form_meta_path = forms_dir.join(format!("{}.xml", form_name));
    let form_dir = forms_dir.join(form_name);

    if !form_meta_path.exists() {
        return Err(anyhow!("Метаданные формы не найдены: {}", form_meta_path.display()));
    }

    let mut out = String::new();

    if form_dir.exists() {
        fs::remove_dir_all(&form_dir).context("Не удалось удалить каталог формы")?;
        out.push_str(&format!("[OK] Удалён каталог: {}\n", form_dir.display()));
    }

    fs::remove_file(&form_meta_path).context("Не удалось удалить файл метаданных формы")?;
    out.push_str(&format!("[OK] Удалён файл: {}\n", form_meta_path.display()));

    // Remove <Form>Name</Form> from root XML
    let root_content = fs::read_to_string(&root_xml_path)
        .context("Не удалось прочитать корневой XML")?;

    let marker = format!("<Form>{}</Form>", form_name);
    let modified = if let Some(pos) = root_content.find(&marker) {
        let before = &root_content[..pos];
        let after = &root_content[pos + marker.len()..];

        // Remove the preceding whitespace/newline
        let trimmed_before = before.trim_end_matches('\n').trim_end_matches('\r').trim_end_matches(char::is_whitespace);
        let cleaned = if before.ends_with('\n') {
            let newline_pos = before.rfind('\n').unwrap();
            let line_before = &before[..newline_pos];
            format!("{}\n{}", line_before, after)
        } else {
            format!("{} {}", trimmed_before, after.trim_start())
        };
        cleaned
    } else {
        return Err(anyhow!("Не найден элемент <Form>{} в корневом XML", form_name));
    };

    // Clear any Default*Form properties that pointed to this form
    let form_ref_pattern = format!("Form.{}", form_name);
    let modified = clear_form_references(&modified, &form_ref_pattern);

    fs::write(&root_xml_path, &modified).context("Не удалось записать корневой XML")?;

    out.push_str(&format!("[OK] Форма {} удалена из {}", form_name, root_xml_path.display()));
    Ok(out)
}

fn clear_form_references(xml: &str, form_ref: &str) -> String {
    let mut result = xml.to_string();
    for prop in &["DefaultForm", "DefaultObjectForm", "DefaultListForm", "DefaultChoiceForm", "DefaultRecordForm", "AuxiliaryForm"] {
        let open = format!("<{}>", prop);
        let close = format!("</{}>", prop);
        if let Some(start) = result.find(&open) {
            let after = &result[start + open.len()..];
            if let Some(end) = after.find(&close) {
                let value = &after[..end];
                if value.contains(form_ref) {
                    let before = &result[..start + open.len()];
                    let after_full = &result[start + open.len() + end + close.len()..];
                    result = format!("{}%{}", before, after_full); // Minimal non-empty placeholder
                    // Actually just clear it
                    result = format!("{}{}{}", &result[..start + open.len()], "", &result[start + open.len() + end + close.len()..]);
                }
            }
        }
    }
    result
}
