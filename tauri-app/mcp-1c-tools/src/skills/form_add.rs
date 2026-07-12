use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const SUPPORTED_TYPES: &[&str] = &[
    "Document", "Catalog", "DataProcessor", "Report",
    "ExternalDataProcessor", "ExternalReport",
    "InformationRegister", "AccumulationRegister", "ChartOfAccounts",
    "ChartOfCharacteristicTypes", "ExchangePlan", "BusinessProcess", "Task",
];

const FORM_MODULE_BSL: &str = r#"#Область ОписаниеПеременных

#КонецОбласти

#Область ПрограммныйИнтерфейс

#КонецОбласти

#Область СлужебныеПроцедурыИФункции

#КонецОбласти
"#;

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

fn get_object_type_and_name(xml: &str) -> Result<(String, String)> {
    for t in SUPPORTED_TYPES {
        let marker = format!("<{} ", t);
        if let Some(pos) = xml.find(&marker) {
            // Find name from Properties/Name section
            let after = &xml[pos..];
            let name_marker = "<Name>";
            if let Some(name_pos) = after.find(name_marker) {
                let name_start = name_pos + name_marker.len();
                if let Some(name_end) = after[name_start..].find("</Name>") {
                    let name = &after[name_start..name_start + name_end];
                    return Ok((t.to_string(), name.to_string()));
                }
            }
            return Ok((t.to_string(), "Unknown".to_string()));
        }
    }
    Err(anyhow!("Не удалось определить тип объекта. Поддерживаемые типы: {}", SUPPORTED_TYPES.join(", ")))
}

fn purpose_to_default_form<'a>(purpose: &'a str, obj_type: &'a str, _obj_name: &'a str) -> &'a str {
    match purpose {
        "List" => "DefaultListForm",
        "Choice" => "DefaultChoiceForm",
        "Record" => "DefaultRecordForm",
        _ => {
            match obj_type {
                "DataProcessor" | "Report" | "ExternalDataProcessor" | "ExternalReport" => "DefaultForm",
                _ => "DefaultObjectForm",
            }
        }
    }
}

pub async fn add_form(args: Value) -> Result<String> {
    let object_path = args.get("object_path")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'object_path' обязателен"))?;

    let form_name = args.get("form_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'form_name' обязателен"))?;

    let purpose = args.get("purpose")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Object");

    let synonym = args.get("synonym")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(form_name);

    let set_default = args.get("set_default")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Resolve path
    let object_path = if object_path.contains(':') || object_path.contains("\\\\") || object_path.starts_with('/') {
        object_path.to_string()
    } else {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        PathBuf::from(cwd).join(object_path).to_string_lossy().to_string()
    };

    let obj_path = PathBuf::from(&object_path);
    if !obj_path.exists() {
        return Err(anyhow!("Файл объекта не найден: {}", object_path));
    }

    let obj_dir = obj_path.parent()
        .ok_or_else(|| anyhow!("Не удалось определить каталог объекта"))?;

    // Object data is in <object_dir>/<filename_without_ext>/
    let obj_stem = obj_path.file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Не удалось определить имя объекта из пути"))?;
    let obj_data_dir = obj_dir.join(obj_stem);

    let format_version = detect_format_version(&obj_dir.to_string_lossy());

    // Read and parse object XML
    let root_xml = fs::read_to_string(&obj_path).context("Не удалось прочитать XML объекта")?;
    let (obj_type, obj_name) = get_object_type_and_name(&root_xml)?;

    // Prepare paths
    let forms_dir = obj_data_dir.join("Forms");
    let form_meta_path = forms_dir.join(format!("{}.xml", form_name));
    let form_dir = forms_dir.join(form_name);
    let form_xml_path = form_dir.join("Form.xml");
    let form_module_path = form_dir.join("Ext").join("FormModule.bsl");

    if form_meta_path.exists() {
        return Err(anyhow!("Форма уже существует: {}", form_meta_path.display()));
    }

    // Create directories
    fs::create_dir_all(form_dir.join("Ext")).context("Не удалось создать каталог формы")?;

    // 1. Form metadata XML
    let form_uuid = uuid::Uuid::new_v4().to_string();
    let form_meta_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:app="http://v8.1c.ru/8.2/managed-application/core" xmlns:cfg="http://v8.1c.ru/8.1/data/enterprise/current-config" xmlns:cmi="http://v8.1c.ru/8.2/managed-application/cmi" xmlns:ent="http://v8.1c.ru/8.1/data/enterprise" xmlns:lf="http://v8.1c.ru/8.2/managed-application/logform" xmlns:style="http://v8.1c.ru/8.1/data/ui/style" xmlns:sys="http://v8.1c.ru/8.1/data/ui/fonts/system" xmlns:v8="http://v8.1c.ru/8.1/data/core" xmlns:v8ui="http://v8.1c.ru/8.1/data/ui" xmlns:web="http://v8.1c.ru/8.1/data/ui/colors/web" xmlns:win="http://v8.1c.ru/8.1/data/ui/colors/windows" xmlns:xen="http://v8.1c.ru/8.3/xcf/enums" xmlns:xpr="http://v8.1c.ru/8.3/xcf/predef" xmlns:xr="http://v8.1c.ru/8.3/xcf/readable" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" version="{}">
	<Form uuid="{}">
		<Properties>
			<Name>{}</Name>
			<Synonym>
				<v8:item>
					<v8:lang>ru</v8:lang>
					<v8:content>{}</v8:content>
				</v8:item>
			</Synonym>
			<Comment/>
			<FormType>ManagedForm</FormType>
			<IncludeHelpInContents>false</IncludeHelpInContents>
			<UsePurposes>{}</UsePurposes>
			<ExtendedPresentation/>
		</Properties>
	</Form>
</MetaDataObject>
"#, format_version, form_uuid, form_name, synonym, purpose
    );
    write_utf8_bom(&form_meta_path, &form_meta_xml)?;

    // 2. Form.xml structure
    let form_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Form xmlns="http://v8.1c.ru/8.1/data/ui/form" xmlns:dcscor="http://v8.1c.ru/8.1/data-composition-system/core" xmlns:dcsset="http://v8.1c.ru/8.1/data-composition-system/settings" xmlns:lf="http://v8.1c.ru/8.2/managed-application/logform" xmlns:style="http://v8.1c.ru/8.1/data/ui/style" xmlns:sys="http://v8.1c.ru/8.1/data/ui/fonts/system" xmlns:v8="http://v8.1c.ru/8.1/data/core" xmlns:v8ui="http://v8.1c.ru/8.1/data/ui" xmlns:web="http://v8.1c.ru/8.1/data/ui/colors/web" xmlns:win="http://v8.1c.ru/8.1/data/ui/colors/windows" xmlns:xr="http://v8.1c.ru/8.3/xcf/readable" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" version="2.8">
	<AutoCommandBar name="CommandBar"/>
	<Attributes>
		<Attribute name="{0}" id="1">
			<Type>
				<v8:Type>lf:FormObjectRef({1}.{0}.Object.{0})</v8:Type>
				<v8:Type>lf:FormObject({1}.{0}.Object.{0})</v8:Type>
			</Type>
			<MainAttribute>true</MainAttribute>
		</Attribute>
	</Attributes>
	<ChildObjects>
		<Command name="ПрименитьПараметрыВвода" id="1"/>
	</ChildObjects>
</Form>
"#, form_name, obj_type
    );
    write_utf8_bom(&form_xml_path, &form_xml)?;

    // 3. Form module
    write_utf8_bom(&form_module_path, FORM_MODULE_BSL)?;

    // 4. Register in root XML
    let mut modified_xml = modified_root_xml(&root_xml, form_name, purpose, &obj_type, &obj_name, set_default)?;
    fs::write(&obj_path, &modified_xml).context("Не удалось записать корневой XML")?;

    let mut out = format!("[OK] Создана форма: {} ({})\n", form_name, purpose);
    out.push_str(&format!("     Метаданные: {}\n", form_meta_path.display()));
    out.push_str(&format!("     Form.xml:   {}\n", form_xml_path.display()));
    out.push_str(&format!("     Модуль:     {}", form_module_path.display()));
    Ok(out)
}

fn modified_root_xml(xml: &str, form_name: &str, purpose: &str, obj_type: &str, _obj_name: &str, set_default: bool) -> Result<String> {
    // 1. Add <Form>Name</Form> to ChildObjects
    let result = if let Some(pos) = xml.find("<ChildObjects/>") {
        let before = &xml[..pos];
        let after = &xml[pos + 15..];
        format!("{}<ChildObjects>\n\t\t<Form>{}</Form>\n\t</ChildObjects>{}", before, form_name, after)
    } else if let Some(pos) = xml.find("</ChildObjects>") {
        let before = &xml[..pos];
        let after = &xml[pos..];
        let spaces: String = before.rsplit('\n').next().unwrap_or("").chars().take_while(|c| c.is_whitespace()).collect();
        format!("{}\t{}<Form>{}</Form>\n{}", before, spaces, form_name, after)
    } else {
        return Err(anyhow!("Не найден элемент ChildObjects"));
    };

    // 2. Optionally set as default form
    if !set_default {
        return Ok(result);
    }

    let default_form_prop = purpose_to_default_form(purpose, obj_type, form_name);
    let default_marker = format!("<{}>", default_form_prop);
    let default_close = format!("</{}>", default_form_prop);

    if let Some(pos) = result.find(&default_marker) {
        let after_open = &result[pos + default_marker.len()..];
        if let Some(close_pos) = after_open.find(&default_close) {
            let inner = &after_open[..close_pos].trim();
            if inner.is_empty() {
                // Set the value
                let ref_path = format!("{}.{}.Form.{}", obj_type, form_name, form_name);
                let before = &result[..pos + default_marker.len()];
                let after = &result[pos + default_marker.len() + close_pos + default_close.len()..];
                return Ok(format!("{}{}{}", before, ref_path, after));
            }
        }
    }

    Ok(result)
}

fn write_utf8_bom(path: &std::path::Path, content: &str) -> Result<()> {
    let bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF]
        .iter()
        .chain(content.as_bytes().iter())
        .copied()
        .collect();
    fs::write(path, &bom).with_context(|| format!("Не удалось записать {}", path.display()))
}
