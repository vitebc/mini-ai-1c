use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub async fn init(args: Value) -> Result<String> {
    let name = args.get("name").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'name' обязателен"))?;
    let synonym = args.get("synonym").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or(name);
    let default_prefix = format!("{}_", name);
    let prefix = args.get("name_prefix").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or(&default_prefix);
    let output_dir = args.get("output_dir").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or("src");
    let purpose = args.get("purpose").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or("Customization");
    let version = args.get("version").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or("");
    let vendor = args.get("vendor").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or("");
    let compat = args.get("compatibility_mode").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or("Version8_3_24");
    let no_role = args.get("no_role").and_then(|v| v.as_bool()).unwrap_or(false);

    let out = PathBuf::from(output_dir);
    let cfg_file = out.join("Configuration.xml");
    if cfg_file.exists() { return Err(anyhow!("Configuration.xml уже существует: {}", cfg_file.display())); }

    fs::create_dir_all(out.join("Languages"))?;
    if !no_role { fs::create_dir_all(out.join("Roles"))?; }

    let u = (0..9).map(|_| uuid::Uuid::new_v4().to_string()).collect::<Vec<_>>();
    let class_ids = ["9cd510cd-abfc-11d4-9434-004095e12fc7","9fcd25a0-4822-11d4-9414-008048da11f9","e3687481-0a87-462c-a166-9f34594f9bba","9de14907-ec23-4a07-96f0-85521cb6b53b","51f2d5d8-ea4d-4064-8892-82951750031e","e68182ea-4237-4383-967f-90c1e3370bc7","fb282519-d103-4dd3-bc12-cb271d631dfc"];

    let mut internal = String::new();
    for (i, cid) in class_ids.iter().enumerate() {
        internal.push_str(&format!("\n\t\t\t<xr:ContainedObject>\n\t\t\t\t<xr:ClassId>{}</xr:ClassId>\n\t\t\t\t<xr:ObjectId>{}</xr:ObjectId>\n\t\t\t</xr:ContainedObject>", cid, u[i]));
    }

    let syn = format!("<v8:item><v8:lang>ru</v8:lang><v8:content>{}</v8:content></v8:item>", synonym);

    let cfg = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:app="http://v8.1c.ru/8.2/managed-application/core" xmlns:cfg="http://v8.1c.ru/8.1/data/enterprise/current-config" xmlns:cmi="http://v8.1c.ru/8.2/managed-application/cmi" xmlns:ent="http://v8.1c.ru/8.1/data/enterprise" xmlns:lf="http://v8.1c.ru/8.2/managed-application/logform" xmlns:style="http://v8.1c.ru/8.1/data/ui/style" xmlns:sys="http://v8.1c.ru/8.1/data/ui/fonts/system" xmlns:v8="http://v8.1c.ru/8.1/data/core" xmlns:v8ui="http://v8.1c.ru/8.1/data/ui" xmlns:web="http://v8.1c.ru/8.1/data/ui/colors/web" xmlns:win="http://v8.1c.ru/8.1/data/ui/colors/windows" xmlns:xen="http://v8.1c.ru/8.3/xcf/enums" xmlns:xpr="http://v8.1c.ru/8.3/xcf/predef" xmlns:xr="http://v8.1c.ru/8.3/xcf/readable" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" version="2.17">
	<Configuration uuid="{u7}" xsi:type="xr:ConfigurationExtension">
		<InternalInfo>{internal}</InternalInfo>
		<Properties>
			<Name>{name}</Name>
			<Synonym>{syn}</Synonym>
			<Comment/><NamePrefix>{prefix}</NamePrefix>
			<ConfigurationExtensionCompatibilityMode>{compat}</ConfigurationExtensionCompatibilityMode>
			<DefaultRunMode>ManagedApplication</DefaultRunMode>
			<UsePurposes><v8:Value xsi:type="app:ApplicationUsePurpose">PlatformApplication</v8:Value></UsePurposes>
			<ScriptVariant>Russian</ScriptVariant>
			<DefaultRoles/><Vendor>{vendor}</Vendor><Version>{version}</Version>
			<IncludeHelpInContents>false</IncludeHelpInContents>
			<ObjectBelonging>Adopted</ObjectBelonging>
			<ConfigurationExtensionPurpose>{purpose}</ConfigurationExtensionPurpose>
			<KeepMapping>false</KeepMapping>
			<CompatibilityMode>{compat}</CompatibilityMode>
		</Properties>
        <ChildObjects><Language>Русский</Language></ChildObjects>
    </Configuration>
</MetaDataObject>"#, u7 = u[7], internal = internal, name = name, syn = syn, prefix = prefix, compat = compat, vendor = vendor, version = version, purpose = purpose);

    let lang = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:app="http://v8.1c.ru/8.2/managed-application/core" xmlns:cfg="http://v8.1c.ru/8.1/data/enterprise/current-config" xmlns:cmi="http://v8.1c.ru/8.2/managed-application/cmi" xmlns:ent="http://v8.1c.ru/8.1/data/enterprise" xmlns:lf="http://v8.1c.ru/8.2/managed-application/logform" xmlns:style="http://v8.1c.ru/8.1/data/ui/style" xmlns:sys="http://v8.1c.ru/8.1/data/ui/fonts/system" xmlns:v8="http://v8.1c.ru/8.1/data/core" xmlns:v8ui="http://v8.1c.ru/8.1/data/ui" xmlns:web="http://v8.1c.ru/8.1/data/ui/colors/web" xmlns:win="http://v8.1c.ru/8.1/data/ui/colors/windows" xmlns:xen="http://v8.1c.ru/8.3/xcf/enums" xmlns:xpr="http://v8.1c.ru/8.3/xcf/predef" xmlns:xr="http://v8.1c.ru/8.3/xcf/readable" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" version="2.17">
	<Language uuid="{u8}">
		<Properties><Name>Русский</Name>
			<Synonym><v8:item><v8:lang>ru</v8:lang><v8:content>Русский</v8:content></v8:item></Synonym>
			<Comment/><LanguageCode>ru</LanguageCode>
			<ExtendedConfigurationObject>true</ExtendedConfigurationObject>
		</Properties>
	</Language>
</MetaDataObject>"#, u8 = u[8]);

    write_bom(&cfg_file, &cfg)?;
    write_bom(&out.join("Languages").join("Русский.xml"), &lang)?;

    if !no_role {
        let role_name = format!("{}ОсновнаяРоль", prefix);
        let role_uuid = &u[2];
        let role = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:app="http://v8.1c.ru/8.2/managed-application/core" xmlns:cfg="http://v8.1c.ru/8.1/data/enterprise/current-config" xmlns:cmi="http://v8.1c.ru/8.2/managed-application/cmi" xmlns:ent="http://v8.1c.ru/8.1/data/enterprise" xmlns:lf="http://v8.1c.ru/8.2/managed-application/logform" xmlns:style="http://v8.1c.ru/8.1/data/ui/style" xmlns:sys="http://v8.1c.ru/8.1/data/ui/fonts/system" xmlns:v8="http://v8.1c.ru/8.1/data/core" xmlns:v8ui="http://v8.1c.ru/8.1/data/ui" xmlns:web="http://v8.1c.ru/8.1/data/ui/colors/web" xmlns:win="http://v8.1c.ru/8.1/data/ui/colors/windows" xmlns:xen="http://v8.1c.ru/8.3/xcf/enums" xmlns:xpr="http://v8.1c.ru/8.3/xcf/predef" xmlns:xr="http://v8.1c.ru/8.3/xcf/readable" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" version="2.17">
	<Role uuid="{}"><Properties><Name>{}</Name><Comment/></Properties></Role>
</MetaDataObject>"#, role_uuid, role_name);
            write_bom(&out.join("Roles").join(format!("{}.xml", role_name)), &role)?;
        }

    let mut msg = format!("[OK] Создано расширение: {}\n     Каталог: {}\n     Configuration.xml: {}\n     Назначение: {}", name, output_dir, cfg_file.display(), purpose);
    if !version.is_empty() { msg.push_str(&format!("\n     Версия: {}", version)); }
    if !vendor.is_empty() { msg.push_str(&format!("\n     Вендор: {}", vendor)); }
    Ok(msg)
}

fn write_bom(path: &std::path::Path, content: &str) -> Result<()> {
    let bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF].iter().chain(content.as_bytes().iter()).copied().collect();
    fs::write(path, &bom).with_context(|| format!("Не удалось записать {}", path.display()))
}
