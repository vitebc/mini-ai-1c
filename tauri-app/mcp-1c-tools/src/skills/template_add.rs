use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const OBJECT_TYPE_FOLDERS: &[&str] = &[
    "Reports", "DataProcessors", "Documents", "Catalogs",
    "InformationRegisters", "AccumulationRegisters",
    "ChartsOfCharacteristicTypes", "ChartsOfAccounts", "ChartsOfCalculationTypes",
    "BusinessProcesses", "Tasks", "ExchangePlans",
];

const REPORT_LIKE_TYPES: &[&str] = &["ExternalReport", "Report"];

struct TemplateTypeInfo {
    template_type: &'static str,
    ext: &'static str,
}

fn get_type_map() -> Vec<(&'static str, TemplateTypeInfo)> {
    vec![
        ("HTML", TemplateTypeInfo { template_type: "HTMLDocument", ext: ".html" }),
        ("Text", TemplateTypeInfo { template_type: "TextDocument", ext: ".txt" }),
        ("SpreadsheetDocument", TemplateTypeInfo { template_type: "SpreadsheetDocument", ext: ".xml" }),
        ("BinaryData", TemplateTypeInfo { template_type: "BinaryData", ext: ".bin" }),
        ("DataCompositionSchema", TemplateTypeInfo { template_type: "DataCompositionSchema", ext: ".xml" }),
    ]
}

fn detect_format_version(src_dir: &str) -> String {
    let mut d = PathBuf::from(src_dir);
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
        if !d.pop() {
            break;
        }
    }
    "2.17".to_string()
}

pub async fn add_template(args: Value) -> Result<String> {
    let object_name = args.get("object_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'object_name' обязателен"))?;

    let template_name = args.get("template_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'template_name' обязателен"))?;

    let template_type_str = args.get("template_type")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'template_type' обязателен. Допустимо: HTML, Text, SpreadsheetDocument, BinaryData, DataCompositionSchema"))?;

    let synonym = args.get("synonym")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(template_name);

    let src_dir = args.get("src_dir")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("src");

    let set_main_skd = args.get("set_main_skd")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let type_info = get_type_map().into_iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(template_type_str))
        .map(|(_, info)| info)
        .ok_or_else(|| anyhow!("Неизвестный тип макета: {}. Допустимо: HTML, Text, SpreadsheetDocument, BinaryData, DataCompositionSchema", template_type_str))?;

    // Find root XML file
    let root_candidate = PathBuf::from(src_dir).join(format!("{}.xml", object_name));
    let root_xml_path = if root_candidate.exists() {
        root_candidate
    } else {
        let mut found: Vec<PathBuf> = Vec::new();
        for folder in OBJECT_TYPE_FOLDERS {
            let probe = PathBuf::from(src_dir).join(folder).join(format!("{}.xml", object_name));
            if probe.exists() {
                found.push(PathBuf::from(src_dir).join(folder));
            }
        }
        match found.len() {
            0 => return Err(anyhow!("Корневой файл объекта не найден: {}\nОжидается: <SrcDir>/<ObjectName>.xml", root_candidate.display())),
            1 => {
                let src = &found[0];
                eprintln!("[INFO] SrcDir расширен до: {}", src.display());
                src.join(format!("{}.xml", object_name))
            }
            _ => return Err(anyhow!("Объект '{}' найден в нескольких подпапках: {:?}\nУкажи SrcDir явно", object_name, found)),
        }
    };

    let object_xml_dir = root_xml_path.parent()
        .ok_or_else(|| anyhow!("Не удалось определить каталог объекта"))?;

    let templates_dir = object_xml_dir.join(object_name).join("Templates");
    let template_meta_path = templates_dir.join(format!("{}.xml", template_name));
    let template_ext_dir = templates_dir.join(template_name).join("Ext");

    if template_meta_path.exists() {
        return Err(anyhow!("Макет уже существует: {}", template_meta_path.display()));
    }

    // Create directories
    fs::create_dir_all(&template_ext_dir).context("Не удалось создать каталог макета")?;

    let format_version = detect_format_version(src_dir);

    // 1. Template metadata XML
    let template_uuid = uuid::Uuid::new_v4().to_string();
    let template_meta_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:app="http://v8.1c.ru/8.2/managed-application/core" xmlns:cfg="http://v8.1c.ru/8.1/data/enterprise/current-config" xmlns:cmi="http://v8.1c.ru/8.2/managed-application/cmi" xmlns:ent="http://v8.1c.ru/8.1/data/enterprise" xmlns:lf="http://v8.1c.ru/8.2/managed-application/logform" xmlns:style="http://v8.1c.ru/8.1/data/ui/style" xmlns:sys="http://v8.1c.ru/8.1/data/ui/fonts/system" xmlns:v8="http://v8.1c.ru/8.1/data/core" xmlns:v8ui="http://v8.1c.ru/8.1/data/ui" xmlns:web="http://v8.1c.ru/8.1/data/ui/colors/web" xmlns:win="http://v8.1c.ru/8.1/data/ui/colors/windows" xmlns:xen="http://v8.1c.ru/8.3/xcf/enums" xmlns:xpr="http://v8.1c.ru/8.3/xcf/predef" xmlns:xr="http://v8.1c.ru/8.3/xcf/readable" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" version="{}">
	<Template uuid="{}">
		<Properties>
			<Name>{}</Name>
			<Synonym>
				<v8:item>
					<v8:lang>ru</v8:lang>
					<v8:content>{}</v8:content>
				</v8:item>
			</Synonym>
			<Comment/>
			<TemplateType>{}</TemplateType>
		</Properties>
	</Template>
</MetaDataObject>
"#,
        format_version, template_uuid, template_name, synonym, type_info.template_type
    );
    write_utf8_bom(&template_meta_path, &template_meta_xml)?;

    // 2. Template content file
    let template_file_path = template_ext_dir.join(format!("Template{}", type_info.ext));
    let content = match template_type_str.to_lowercase().as_str() {
        "html" => r#"<!DOCTYPE html>
<html>
<head>
	<meta charset="UTF-8">
	<title></title>
</head>
<body>
</body>
</html>
"#.to_string(),
        "text" => String::new(),
        "spreadsheetdocument" => r#"<?xml version="1.0" encoding="UTF-8"?>
<SpreadsheetDocument xmlns="http://v8.1c.ru/spreadsheet/document" xmlns:ss="http://v8.1c.ru/spreadsheet/document" xmlns:v8="http://v8.1c.ru/8.1/data/core" xmlns:xs="http://www.w3.org/2001/XMLSchema">
</SpreadsheetDocument>
"#.to_string(),
        "datacompositionschema" => r#"<?xml version="1.0" encoding="UTF-8"?>
<DataCompositionSchema xmlns="http://v8.1c.ru/8.1/data-composition-system/schema"
		xmlns:dcscom="http://v8.1c.ru/8.1/data-composition-system/common"
		xmlns:dcscor="http://v8.1c.ru/8.1/data-composition-system/core"
		xmlns:dcsset="http://v8.1c.ru/8.1/data-composition-system/settings"
		xmlns:v8="http://v8.1c.ru/8.1/data/core"
		xmlns:v8ui="http://v8.1c.ru/8.1/data/ui"
		xmlns:xs="http://www.w3.org/2001/XMLSchema"
		xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
	<dataSource>
		<name>ИсточникДанных1</name>
		<dataSourceType>Local</dataSourceType>
	</dataSource>
</DataCompositionSchema>
"#.to_string(),
        _ => String::new(),
    };

    if template_type_str.eq_ignore_ascii_case("binarydata") {
        fs::write(&template_file_path, &[]).context("Не удалось записать файл макета")?;
    } else {
        write_utf8_bom(&template_file_path, &content)?;
    }

    // 3. Modify root XML - add Template to ChildObjects
    let root_xml_content = fs::read_to_string(&root_xml_path)
        .context("Не удалось прочитать корневой XML")?;

    // Handle self-closing <ChildObjects/> and expanded <ChildObjects>...</ChildObjects>
    let modified_xml = if let Some(pos) = root_xml_content.find("<ChildObjects/>") {
        // Self-closing → expand
        let before = &root_xml_content[..pos];
        let after = &root_xml_content[pos + 15..];
        format!("{}<ChildObjects>\n\t\t<Template>{}</Template>\n\t</ChildObjects>{}", before, template_name, after)
    } else if let Some(pos) = root_xml_content.find("</ChildObjects>") {
        let before = &root_xml_content[..pos];
        let after = &root_xml_content[pos..];
        let spaces: String = before.rsplit('\n').next().unwrap_or("").chars().take_while(|c| c.is_whitespace()).collect();
        format!("{}\t{}<Template>{}</Template>\n{}", before, spaces, template_name, after)
    } else {
        return Err(anyhow!("Не найден элемент ChildObjects в {}", root_xml_path.display()));
    };

    fs::write(&root_xml_path, &modified_xml).context("Не удалось записать корневой XML")?;

    // 4. MainDataCompositionSchema (for report-like objects)
    let mut main_dcs_updated = false;
    if template_type_str.eq_ignore_ascii_case("datacompositionschema") {
        let root_content = fs::read_to_string(&root_xml_path)?;
        // Simple check if the object is ExternalReport or Report
        for report_type in REPORT_LIKE_TYPES {
            if root_content.contains(&format!("<{}>", report_type)) {
                // Find MainDataCompositionSchema
                if let Some(dcs_pos) = root_content.find("<MainDataCompositionSchema>") {
                    let after_tag = &root_content[dcs_pos + 28..];
                    if after_tag.starts_with("</MainDataCompositionSchema>") {
                        // Empty, set it
                        let ref_path = format!("{}.{}.Template.{}", report_type, object_name, template_name);
                        let new_content = format!("{}<MainDataCompositionSchema>{}</MainDataCompositionSchema>{}",
                            &root_content[..dcs_pos],
                            ref_path,
                            &root_content[dcs_pos + 28 + 28..]
                        );
                        fs::write(&root_xml_path, &new_content)?;
                        main_dcs_updated = true;
                        eprintln!("     MainDataCompositionSchema: {}", ref_path);
                    }
                }
                break;
            }
        }
    }

    let mut out = format!("[OK] Создан макет: {} ({})\n", template_name, template_type_str);
    out.push_str(&format!("     Метаданные: {}\n", template_meta_path.display()));
    out.push_str(&format!("     Содержимое: {}", template_file_path.display()));
    if main_dcs_updated {
        out.push_str("\n     MainDataCompositionSchema обновлён");
    }
    Ok(out)
}

fn write_utf8_bom(path: &std::path::Path, content: &str) -> Result<()> {
    let bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF]
        .iter()
        .chain(content.as_bytes().iter())
        .copied()
        .collect();
    fs::write(path, &bom).with_context(|| format!("Не удалось записать {}", path.display()))
}
