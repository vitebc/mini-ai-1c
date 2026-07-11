use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

static MODULE_BSL: &str = r#"#Область ОписаниеПеременных

#КонецОбласти

#Область ПрограммныйИнтерфейс

#КонецОбласти

#Область СлужебныеПроцедурыИФункции

#КонецОбласти
"#;

pub async fn init(args: Value) -> Result<String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'name' обязателен"))?;

    let synonym = args
        .get("synonym")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(name);

    let src_dir = args
        .get("src_dir")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("src");

    let u1 = uuid::Uuid::new_v4().to_string();
    let u2 = uuid::Uuid::new_v4().to_string();
    let u3 = uuid::Uuid::new_v4().to_string();
    let u4 = uuid::Uuid::new_v4().to_string();

    let root_path = PathBuf::from(src_dir).join(format!("{}.xml", name));
    let proc_dir = PathBuf::from(src_dir).join(name);
    let ext_dir = proc_dir.join("Ext");
    let module_path = ext_dir.join("ObjectModule.bsl");

    if root_path.exists() {
        return Err(anyhow!("Файл уже существует: {}", root_path.display()));
    }

    fs::create_dir_all(&ext_dir).context("Не удалось создать каталог Ext")?;

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:app="http://v8.1c.ru/8.2/managed-application/core" xmlns:cfg="http://v8.1c.ru/8.1/data/enterprise/current-config" xmlns:cmi="http://v8.1c.ru/8.2/managed-application/cmi" xmlns:ent="http://v8.1c.ru/8.1/data/enterprise" xmlns:lf="http://v8.1c.ru/8.2/managed-application/logform" xmlns:style="http://v8.1c.ru/8.1/data/ui/style" xmlns:sys="http://v8.1c.ru/8.1/data/ui/fonts/system" xmlns:v8="http://v8.1c.ru/8.1/data/core" xmlns:v8ui="http://v8.1c.ru/8.1/data/ui" xmlns:web="http://v8.1c.ru/8.1/data/ui/colors/web" xmlns:win="http://v8.1c.ru/8.1/data/ui/colors/windows" xmlns:xen="http://v8.1c.ru/8.3/xcf/enums" xmlns:xpr="http://v8.1c.ru/8.3/xcf/predef" xmlns:xr="http://v8.1c.ru/8.3/xcf/readable" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" version="2.17">
	<ExternalDataProcessor uuid="{u1}">
		<InternalInfo>
			<xr:ContainedObject>
				<xr:ClassId>c3831ec8-d8d5-4f93-8a22-f9bfae07327f</xr:ClassId>
				<xr:ObjectId>{u2}</xr:ObjectId>
			</xr:ContainedObject>
			<xr:GeneratedType name="ExternalDataProcessorObject.{name}" category="Object">
				<xr:TypeId>{u3}</xr:TypeId>
				<xr:ValueId>{u4}</xr:ValueId>
			</xr:GeneratedType>
		</InternalInfo>
		<Properties>
			<Name>{name}</Name>
			<Synonym>
				<v8:item>
					<v8:lang>ru</v8:lang>
					<v8:content>{synonym}</v8:content>
				</v8:item>
			</Synonym>
			<Comment/>
			<DefaultForm/>
			<AuxiliaryForm/>
		</Properties>
		<ChildObjects/>
	</ExternalDataProcessor>
</MetaDataObject>
"#
    );

    let xml_utf8_bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF]
        .iter()
        .chain(xml.as_bytes().iter())
        .copied()
        .collect();
    fs::write(&root_path, &xml_utf8_bom).context("Не удалось записать XML")?;

    let utf8_bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF]
        .iter()
        .chain(MODULE_BSL.as_bytes().iter())
        .copied()
        .collect();
    fs::write(&module_path, &utf8_bom).context("Не удалось записать модуль")?;

    Ok(format!(
        "[OK] Создана обработка: {}\n     Каталог: {}\n     Модуль:  {}",
        root_path.display(),
        proc_dir.display(),
        module_path.display()
    ))
}
