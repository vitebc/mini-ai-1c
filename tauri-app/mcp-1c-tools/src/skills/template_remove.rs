use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub async fn remove_template(args: Value) -> Result<String> {
    let object_name = args.get("object_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'object_name' обязателен"))?;

    let template_name = args.get("template_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'template_name' обязателен"))?;

    let src_dir = args.get("src_dir")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("src");

    let root_xml_path = PathBuf::from(src_dir).join(format!("{}.xml", object_name));
    if !root_xml_path.exists() {
        return Err(anyhow!("Корневой файл объекта не найден: {}", root_xml_path.display()));
    }

    let templates_dir = PathBuf::from(src_dir).join(object_name).join("Templates");
    let template_meta_path = templates_dir.join(format!("{}.xml", template_name));
    let template_dir = templates_dir.join(template_name);

    if !template_meta_path.exists() {
        return Err(anyhow!("Метаданные макета не найдены: {}", template_meta_path.display()));
    }

    let mut out = String::new();

    // Remove template directory
    if template_dir.exists() {
        fs::remove_dir_all(&template_dir).context("Не удалось удалить каталог макета")?;
        out.push_str(&format!("[OK] Удалён каталог: {}\n", template_dir.display()));
    }

    // Remove template metadata file
    fs::remove_file(&template_meta_path).context("Не удалось удалить файл метаданных макета")?;
    out.push_str(&format!("[OK] Удалён файл: {}\n", template_meta_path.display()));

    // Remove <Template>Name</Template> from root XML
    let root_content = fs::read_to_string(&root_xml_path)
        .context("Не удалось прочитать корневой XML")?;

    let modified = remove_template_from_xml(&root_content, template_name);

    fs::write(&root_xml_path, &modified).context("Не удалось записать корневой XML")?;

    out.push_str(&format!("[OK] Макет {} удалён из {}", template_name, root_xml_path.display()));
    Ok(out)
}

fn remove_template_from_xml(xml: &str, template_name: &str) -> String {
    let marker = format!("<Template>{}</Template>", template_name);

    if let Some(pos) = xml.find(&marker) {
        // Remove the template tag and any preceding whitespace
        let before = &xml[..pos];
        let after = &xml[pos + marker.len()..];

        // Clean up whitespace before the tag
        let cleaned_before = before.trim_end();
        let removed_whitespace = &before[cleaned_before.len()..];

        // Also handle the case where there's a newline before the removed whitespace
        let (final_before, remove_newline) = if removed_whitespace.contains('\n') {
            // The line containing the template tag is being removed
            // Remove the entire line including the newline before it
            let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
            if line_start > 0 {
                (&xml[..line_start - 1], true)
            } else {
                (&xml[..line_start], false)
            }
        } else {
            (cleaned_before, false)
        };

        let extra = if remove_newline { "" } else { " " };
        format!("{}{}{}", final_before, extra, after)
    } else {
        xml.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_template_from_xml() {
        let xml = r#"<ChildObjects>
			<Template>Keep</Template>
			<Template>Remove</Template>
		</ChildObjects>"#;
        let result = remove_template_from_xml(xml, "Remove");
        assert!(!result.contains("Remove"));
        assert!(result.contains("Keep"));
    }
}
