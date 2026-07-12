use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn detect_format_version(start_dir: &str) -> String {
    let mut d = PathBuf::from(start_dir);
    loop {
        let cfg = d.join("Configuration.xml");
        if cfg.exists() {
            if let Ok(content) = fs::read_to_string(&cfg) {
                if let Some(pos) = content.find("version=\"") {
                    let rest = &content[pos + 9..];
                    if let Some(end) = rest.find('\"') {
                        return rest[..end].to_string();
                    }
                }
            }
        }
        if !d.pop() { break; }
    }
    "2.17".to_string()
}

pub async fn add_help(args: Value) -> Result<String> {
    let object_name = args.get("object_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'object_name' обязателен"))?;

    let lang = args.get("lang")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("ru");

    let src_dir = args.get("src_dir")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("src");

    let object_dir = PathBuf::from(src_dir).join(object_name);
    let ext_dir = object_dir.join("Ext");

    if !ext_dir.exists() {
        return Err(anyhow!("Каталог объекта не найден: {}. Проверьте путь ObjectName (например DataProcessors/ИмяОбработки)", ext_dir.display()));
    }

    let help_xml_path = ext_dir.join("Help.xml");
    if help_xml_path.exists() {
        return Err(anyhow!("Справка уже существует: {}", help_xml_path.display()));
    }

    let format_version = detect_format_version(src_dir);

    // 1. Help.xml
    let help_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Help xmlns="http://v8.1c.ru/8.3/xcf/extrnprops" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" version="{}">
	<Page>{}</Page>
</Help>
"#, format_version, lang
    );
    write_utf8_bom(&help_xml_path, &help_xml)?;

    // 2. Help/<lang>.html
    let help_dir = ext_dir.join("Help");
    fs::create_dir_all(&help_dir).context("Не удалось создать каталог Help")?;

    let help_html = format!(
        r#"<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.0 Transitional//EN">
<html>
<head>
    <meta http-equiv="Content-Type" content="text/html; charset=utf-8"/>
    <link rel="stylesheet" type="text/css" href="v8help://service_book/service_style"/>
</head>
<body>
    <h1>{}</h1>
    <p>Описание.</p>
</body>
</html>
"#, object_name
    );
    let help_html_path = help_dir.join(format!("{}.html", lang));
    write_utf8_bom(&help_html_path, &help_html)?;

    // 3. IncludeHelpInContents in form metadata
    let forms_dir = object_dir.join("Forms");
    if forms_dir.exists() {
        if let Ok(entries) = fs::read_dir(&forms_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "xml").unwrap_or(false) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if !content.contains("<IncludeHelpInContents>") {
                            let modified = add_include_help_to_form(&content);
                            if let Ok(modified) = modified {
                                let _ = fs::write(&path, modified);
                                eprintln!("     IncludeHelpInContents добавлен: {}", path.file_name().unwrap_or_default().to_string_lossy());
                            }
                        }
                    }
                }
            }
        }
    }

    let mut out = format!("[OK] Создана справка: {}\n", object_name);
    out.push_str(&format!("     Метаданные: {}\n", help_xml_path.display()));
    out.push_str(&format!("     Страница:   {}", help_html_path.display()));
    Ok(out)
}

fn add_include_help_to_form(xml: &str) -> Result<String> {
    let marker = "<FormType>";
    if let Some(pos) = xml.find(marker) {
        let form_type_end = pos + marker.len();
        let rest = &xml[form_type_end..];
        if let Some(end_pos) = rest.find("</FormType>") {
            let before = &xml[..form_type_end + end_pos + 11];
            let after = &xml[form_type_end + end_pos + 11..];
            return Ok(format!("{}\n\t\t\t<IncludeHelpInContents>false</IncludeHelpInContents>{}", before, after));
        }
    }
    Err(anyhow!("Не найден элемент FormType"))
}

fn write_utf8_bom(path: &std::path::Path, content: &str) -> Result<()> {
    let bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF]
        .iter()
        .chain(content.as_bytes().iter())
        .copied()
        .collect();
    fs::write(path, &bom).with_context(|| format!("Не удалось записать {}", path.display()))
}
