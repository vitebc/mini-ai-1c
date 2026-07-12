use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

static TYPE_DIR_MAP: &[(&str, &str)] = &[
    ("Catalog","Catalogs"),("Document","Documents"),("DataProcessor","DataProcessors"),
    ("Report","Reports"),("InformationRegister","InformationRegisters"),
    ("AccumulationRegister","AccumulationRegisters"),("ChartOfAccounts","ChartsOfAccounts"),
    ("ChartOfCharacteristicTypes","ChartsOfCharacteristicTypes"),("BusinessProcess","BusinessProcesses"),
    ("Task","Tasks"),("ExchangePlan","ExchangePlans"),("Enum","Enums"),
    ("ExternalDataProcessor","DataProcessors"),("ExternalReport","Reports"),
    ("Constant","Constants"),("CommonModule","CommonModules"),
    ("Role","Roles"),("Subsystem","Subsystems"),
];

pub async fn patch_method(args: Value) -> Result<String> {
    let ext_path = args.get("extension_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'extension_path' обязателен"))?;
    let module_path = args.get("module_path").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'module_path' обязателен"))?;
    let method_name = args.get("method_name").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'method_name' обязателен"))?;
    let interceptor_type = args.get("interceptor_type").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'interceptor_type' обязателен"))?;
    let context = args.get("context").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or("НаСервере");
    let is_function = args.get("is_function").and_then(|v| v.as_bool()).unwrap_or(false);

    let p = Path::new(ext_path);
    let cfg_path = if p.is_dir() { p.join("Configuration.xml") } else { p.to_path_buf() };
    if !cfg_path.exists() { return Err(anyhow!("Configuration.xml не найден: {}", cfg_path.display())); }

    // Read prefix
    let xml = fs::read_to_string(&cfg_path)?;
    let prefix = if let Some(s) = xml.find("<NamePrefix>") {
        let after = &xml[s + 12..];
        let end = after.find("</NamePrefix>").unwrap_or(0);
        after[..end].to_string()
    } else { String::new() };

    // Parse module path: "Catalog.X.ObjectModule" or "Catalog.X.Form.Y"
    let parts: Vec<&str> = module_path.splitn(4, '.').collect();
    if parts.len() < 3 { return Err(anyhow!("Формат module_path: Type.Name.Module или Type.Name.Form.FormName")); }

    let obj_type = parts[0];
    let obj_name = parts[1];

    // Resolve BSL file path
    let dir_name = TYPE_DIR_MAP.iter().find(|(t, _)| *t == obj_type).map(|(_, d)| *d)
        .ok_or_else(|| anyhow!("Неизвестный тип: {}", obj_type))?;

    let bsl_path = if parts.len() >= 4 && parts[2] == "Form" {
        PathBuf::from(ext_path).join(dir_name).join(obj_name).join("Forms").join(parts[3]).join("Ext").join("FormModule.bsl")
    } else {
        let module_suffix = match parts[2] {
            "ObjectModule" => "ObjectModule.bsl",
            "ManagerModule" => "ManagerModule.bsl",
            "RecordModule" => "RecordModule.bsl",
            "ValueModule" => "ValueModule.bsl",
            "Module" => "Module.bsl",
            _ => return Err(anyhow!("Неизвестный тип модуля: {}", parts[2])),
        };
        PathBuf::from(ext_path).join(dir_name).join(obj_name).join("Ext").join(module_suffix)
    };

    // Generate interceptor code
    let decorator = match interceptor_type {
        "Before" => "&Перед",
        "After" => "&После",
        "ModificationAndControl" => "&ИзменениеИКонтроль",
        _ => return Err(anyhow!("Неизвестный тип перехватчика: {}. Допустимо: Before, After, ModificationAndControl", interceptor_type)),
    };

    let proc_name = format!("{}{}", prefix, method_name);
    let return_stmt = if is_function { "\tВозврат Неопределено" } else { "" };

    let code = format!(
        "\n{decorator}(\"{method_name}\")\n{context} {func_or_proc} {proc_name}()\n{return_stmt}\n{end}
",
        decorator = decorator,
        method_name = method_name,
        context = context,
        func_or_proc = if is_function { "Функция" } else { "Процедура" },
        proc_name = proc_name,
        return_stmt = return_stmt,
        end = if is_function { "КонецФункции" } else { "КонецПроцедуры" }
    );

    // Write to file
    let parent = bsl_path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent).context("Не удалось создать каталог модуля")?;

    let existing = if bsl_path.exists() {
        fs::read_to_string(&bsl_path)?
    } else { String::new() };

    let new_content = if existing.is_empty() { code.trim_start().to_string() } else { format!("{}\n{}", existing.trim_end(), code) };

    let bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF].iter().chain(new_content.as_bytes().iter()).copied().collect();
    fs::write(&bsl_path, &bom)?;

    let msg = if existing.is_empty() {
        format!("[OK] Создан файл модуля: {}", bsl_path.display())
    } else {
        format!("[OK] Добавлен перехватчик в существующий файл: {}", bsl_path.display())
    };

    Ok(format!("{}\n     Декоратор: {}\n     Процедура: {}\n     Контекст: {}", msg, decorator, proc_name, context))
}
