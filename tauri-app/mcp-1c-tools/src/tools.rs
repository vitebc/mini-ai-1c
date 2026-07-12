use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::cli;
use crate::dsl;
use crate::info;
use crate::skills;
use crate::v8i;

pub async fn list_tools() -> Value {
    serde_json::json!({
        "tools": [
            // DB tools
            tool_def("cc_db_create", "Создать пустую информационную базу 1С", serde_json::json!({
                "type": "object", "properties": {
                    "path": {"type": "string", "description": "Путь к новой ИБ"},
                    "name": {"type": "string", "description": "Имя ИБ для списка"}
                }, "required": ["path"]
            })),
            tool_def("cc_db_dump_cf", "Выгрузить конфигурацию в .cf", serde_json::json!({
                "type": "object", "properties": {
                    "ib_path": {"type": "string", "description": "Путь к ИБ"},
                    "cf_path": {"type": "string", "description": "Куда сохранить .cf"},
                    "user": {"type": "string", "default": "Admin"},
                    "password": {"type": "string", "default": ""}
                }, "required": ["ib_path", "cf_path"]
            })),
            tool_def("cc_db_load_cf", "Загрузить конфигурацию из .cf", serde_json::json!({
                "type": "object", "properties": {
                    "ib_path": {"type": "string", "description": "Путь к ИБ"},
                    "cf_path": {"type": "string", "description": "Путь к .cf"},
                    "user": {"type": "string", "default": "Admin"},
                    "password": {"type": "string", "default": ""}
                }, "required": ["ib_path", "cf_path"]
            })),
            tool_def("cc_db_dump_dt", "Выгрузить ИБ в .dt", serde_json::json!({
                "type": "object", "properties": {
                    "ib_path": {"type": "string"},
                    "dt_path": {"type": "string"}
                }, "required": ["ib_path", "dt_path"]
            })),
            tool_def("cc_db_load_dt", "Загрузить ИБ из .dt", serde_json::json!({
                "type": "object", "properties": {
                    "ib_path": {"type": "string"},
                    "dt_path": {"type": "string"}
                }, "required": ["ib_path", "dt_path"]
            })),
            tool_def("cc_db_dump_xml", "Выгрузить конфигурацию в XML", serde_json::json!({
                "type": "object", "properties": {
                    "ib_path": {"type": "string"},
                    "out_dir": {"type": "string"}
                }, "required": ["ib_path", "out_dir"]
            })),
            tool_def("cc_db_load_xml", "Загрузить конфигурацию из XML", serde_json::json!({
                "type": "object", "properties": {
                    "ib_path": {"type": "string"},
                    "src_dir": {"type": "string"}
                }, "required": ["ib_path", "src_dir"]
            })),
            tool_def("cc_db_update", "Обновить конфигурацию БД", serde_json::json!({
                "type": "object", "properties": {
                    "ib_path": {"type": "string"},
                    "user": {"type": "string", "default": "Admin"},
                    "password": {"type": "string", "default": ""}
                }, "required": ["ib_path"]
            })),
            tool_def("cc_db_run", "Запустить 1С:Предприятие", serde_json::json!({
                "type": "object", "properties": {
                    "ib_path": {"type": "string"}
                }, "required": ["ib_path"]
            })),
            tool_def("cc_db_list", "Список ИБ", serde_json::json!({
                "type": "object", "properties": {}
            })),
            // EPF/ERF tools
            tool_def("cc_epf_build", "Собрать .epf из исходников", serde_json::json!({
                "type": "object", "properties": {
                    "src_dir": {"type": "string", "description": "Путь к src/epf"},
                    "out_file": {"type": "string", "description": "Куда сохранить .epf"}
                }, "required": ["src_dir", "out_file"]
            })),
            tool_def("cc_epf_dump", "Разобрать .epf в исходники", serde_json::json!({
                "type": "object", "properties": {
                    "epf_path": {"type": "string"},
                    "out_dir": {"type": "string"}
                }, "required": ["epf_path", "out_dir"]
            })),
            tool_def("cc_erf_build", "Собрать .erf из исходников", serde_json::json!({
                "type": "object", "properties": {
                    "src_dir": {"type": "string"},
                    "out_file": {"type": "string"}
                }, "required": ["src_dir", "out_file"]
            })),
            tool_def("cc_erf_dump", "Разобрать .erf в исходники", serde_json::json!({
                "type": "object", "properties": {
                    "erf_path": {"type": "string"},
                    "out_dir": {"type": "string"}
                }, "required": ["erf_path", "out_dir"]
            })),
            // Web tools
            tool_def("cc_web_publish", "Опубликовать ИБ на веб-сервере", serde_json::json!({
                "type": "object", "properties": {
                    "ib_path": {"type": "string"},
                    "pub_path": {"type": "string", "description": "Путь публикации"}
                }, "required": ["ib_path", "pub_path"]
            })),
            // Info tools
            tool_def("cc_meta_info", "Информация о метаданных из XML", serde_json::json!({
                "type": "object", "properties": {
                    "xml_path": {"type": "string"}
                }, "required": ["xml_path"]
            })),
            tool_def("cc_form_info", "Информация о форме из Form.xml", serde_json::json!({
                "type": "object", "properties": {
                    "xml_path": {"type": "string"}
                }, "required": ["xml_path"]
            })),
            tool_def("cc_skd_info", "Информация о СКД из Template.xml", serde_json::json!({
                "type": "object", "properties": {
                    "xml_path": {"type": "string"}
                }, "required": ["xml_path"]
            })),
            // DSL Compilers
            tool_def("cc_form_compile", "Скомпилировать Form.json в Form.xml", serde_json::json!({
                "type": "object", "properties": {
                    "json_path": {"type": "string"},
                    "out_path": {"type": "string"}
                }, "required": ["json_path", "out_path"]
            })),
            tool_def("cc_form_decompile", "Декомпилировать Form.xml в Form.json", serde_json::json!({
                "type": "object", "properties": {
                    "xml_path": {"type": "string"},
                    "out_path": {"type": "string"}
                }, "required": ["xml_path", "out_path"]
            })),
            tool_def("cc_meta_compile", "Скомпилировать Meta.json в XML метаданных", serde_json::json!({
                "type": "object", "properties": {
                    "json_path": {"type": "string"},
                    "out_dir": {"type": "string"}
                }, "required": ["json_path", "out_dir"]
            })),
            tool_def("cc_skd_compile", "Скомпилировать SKD.json в Template.xml", serde_json::json!({
                "type": "object", "properties": {
                    "json_path": {"type": "string"},
                    "out_path": {"type": "string"}
                }, "required": ["json_path", "out_path"]
            })),
            // Skills
            tool_def("cc_epf_init", "Создать пустую внешнюю обработку 1С (scaffold XML-исходников)", serde_json::json!({
                "type": "object", "properties": {
                    "name": {"type": "string", "description": "Имя обработки"},
                    "synonym": {"type": "string", "description": "Синоним (отображаемое имя)"},
                    "src_dir": {"type": "string", "description": "Каталог исходников", "default": "src"}
                }, "required": ["name"]
            })),
            tool_def("cc_template_add", "Добавить макет/шаблон к объекту 1С", serde_json::json!({
                "type": "object", "properties": {
                    "object_name": {"type": "string", "description": "Имя объекта (обработки, отчёта, документа и т.д.)"},
                    "template_name": {"type": "string", "description": "Имя макета"},
                    "template_type": {"type": "string", "description": "Тип макета: HTML, Text, SpreadsheetDocument, BinaryData, DataCompositionSchema"},
                    "synonym": {"type": "string", "description": "Синоним макета"},
                    "src_dir": {"type": "string", "description": "Каталог исходников", "default": "src"},
                    "set_main_skd": {"type": "boolean", "description": "Установить основной СКД", "default": false}
                }, "required": ["object_name", "template_name", "template_type"]
            })),
            tool_def("cc_template_remove", "Удалить макет/шаблон из объекта 1С", serde_json::json!({
                "type": "object", "properties": {
                    "object_name": {"type": "string", "description": "Имя объекта"},
                    "template_name": {"type": "string", "description": "Имя макета"},
                    "src_dir": {"type": "string", "description": "Каталог исходников", "default": "src"}
                }, "required": ["object_name", "template_name"]
            })),
            tool_def("cc_form_add", "Добавить пустую управляемую форму к объекту 1С", serde_json::json!({
                "type": "object", "properties": {
                    "object_path": {"type": "string", "description": "Путь к XML-файлу объекта"},
                    "form_name": {"type": "string", "description": "Имя формы"},
                    "purpose": {"type": "string", "description": "Назначение: Object, List, Choice, Record", "default": "Object"},
                    "synonym": {"type": "string", "description": "Синоним формы"},
                    "set_default": {"type": "boolean", "description": "Установить как форму по умолчанию", "default": false}
                }, "required": ["object_path", "form_name"]
            })),
            tool_def("cc_form_remove", "Удалить форму из объекта 1С", serde_json::json!({
                "type": "object", "properties": {
                    "object_name": {"type": "string", "description": "Имя объекта"},
                    "form_name": {"type": "string", "description": "Имя формы"},
                    "src_dir": {"type": "string", "description": "Каталог исходников", "default": "src"}
                }, "required": ["object_name", "form_name"]
            })),
            tool_def("cc_help_add", "Добавить встроенную справку к объекту 1С", serde_json::json!({
                "type": "object", "properties": {
                    "object_name": {"type": "string", "description": "Путь объекта относительно SrcDir (DataProcessors/МояОбработка)"},
                    "lang": {"type": "string", "description": "Код языка", "default": "ru"},
                    "src_dir": {"type": "string", "description": "Каталог исходников", "default": "src"}
                }, "required": ["object_name"]
            })),
            tool_def("cc_support_edit", "Переключить состояние поддержки типовой конфигурации 1С", serde_json::json!({
                "type": "object", "properties": {
                    "target_path": {"type": "string", "description": "Путь к объекту"},
                    "set": {"type": "string", "description": "editable|off-support|locked"},
                    "capability": {"type": "string", "description": "on|off"}
                }, "required": ["target_path"]
            })),
            tool_def("cc_cf_init", "Создать пустую конфигурацию 1С (scaffold)", serde_json::json!({
                "type": "object", "properties": {
                    "name": {"type": "string", "description": "Имя конфигурации"},
                    "synonym": {"type": "string", "description": "Синоним"},
                    "output_dir": {"type": "string", "description": "Каталог для выгрузки", "default": "src"},
                    "version": {"type": "string", "description": "Версия"},
                    "vendor": {"type": "string", "description": "Вендор"},
                    "compatibility_mode": {"type": "string", "description": "Режим совместимости", "default": "Version8_3_24"}
                }, "required": ["name"]
            })),
            tool_def("cc_epf_validate", "Проверить структуру внешней обработки (EPF)", serde_json::json!({
                "type": "object", "properties": {
                    "path": {"type": "string", "description": "Путь к XML-файлу обработки"}
                }, "required": ["path"]
            })),
            tool_def("cc_erf_validate", "Проверить структуру внешнего отчёта (ERF)", serde_json::json!({
                "type": "object", "properties": {
                    "path": {"type": "string", "description": "Путь к XML-файлу отчёта"}
                }, "required": ["path"]
            })),
            tool_def("cc_cf_info", "Информация о конфигурации из Configuration.xml", serde_json::json!({
                "type": "object", "properties": {
                    "path": {"type": "string", "description": "Путь к Configuration.xml или каталогу src"}
                }, "required": ["path"]
            })),
            tool_def("cc_cf_edit", "Редактировать Configuration.xml", serde_json::json!({
                "type": "object", "properties": {
                    "config_path": {"type": "string", "description": "Путь к Configuration.xml"},
                    "operation": {"type": "string", "description": "modify-property|add-childObject|remove-childObject|set-defaultRoles"},
                    "value": {"type": "string", "description": "PropertyName=Value или Type=Name"}
                }, "required": ["config_path", "operation"]
            })),
            tool_def("cc_cf_validate", "Проверить структуру Configuration.xml", serde_json::json!({
                "type": "object", "properties": {
                    "config_path": {"type": "string", "description": "Путь к Configuration.xml или каталогу"},
                    "detailed": {"type": "boolean", "description": "Показать все проверки", "default": false}
                }, "required": ["config_path"]
            })),
            tool_def("cc_meta_edit", "Редактировать объект метаданных", serde_json::json!({
                "type": "object", "properties": {
                    "object_path": {"type": "string", "description": "Путь к XML объекта"},
                    "operation": {"type": "string", "description": "add-attribute|remove-attribute|modify-property|add-tabularSection|remove-tabularSection|set-synonym"},
                    "value": {"type": "string", "description": "Параметры операции"}
                }, "required": ["object_path", "operation"]
            })),
            tool_def("cc_meta_validate", "Проверить структуру XML метаданных", serde_json::json!({
                "type": "object", "properties": {
                    "object_path": {"type": "string", "description": "Путь к XML объекта"},
                    "detailed": {"type": "boolean", "default": false}
                }, "required": ["object_path"]
            })),
            tool_def("cc_meta_remove", "Удалить объект метаданных из конфигурации", serde_json::json!({
                "type": "object", "properties": {
                    "config_dir": {"type": "string", "description": "Корень выгрузки конфигурации"},
                    "object": {"type": "string", "description": "Type.Name (например Catalog.Товары)"},
                    "dry_run": {"type": "boolean", "default": false},
                    "keep_files": {"type": "boolean", "default": false},
                    "force": {"type": "boolean", "default": false}
                }, "required": ["config_dir", "object"]
            })),
            tool_def("cc_subsystem_info", "Информация о подсистеме 1С", serde_json::json!({
                "type": "object", "properties": {
                    "path": {"type": "string", "description": "Путь к Subsystem.xml или каталогу"},
                    "mode": {"type": "string", "description": "overview|content|ci|tree|full", "default": "overview"}
                }, "required": ["path"]
            })),
            tool_def("cc_subsystem_edit", "Редактировать подсистему 1С", serde_json::json!({
                "type": "object", "properties": {
                    "path": {"type": "string", "description": "Путь к Subsystem.xml"},
                    "operation": {"type": "string", "description": "add-content|remove-content|add-child|remove-child|set-property"},
                    "value": {"type": "string", "description": "Значение операции"}
                }, "required": ["path", "operation"]
            })),
            tool_def("cc_subsystem_validate", "Проверить структуру подсистемы 1С", serde_json::json!({
                "type": "object", "properties": {
                    "path": {"type": "string", "description": "Путь к Subsystem.xml"},
                    "detailed": {"type": "boolean", "default": false}
                }, "required": ["path"]
            })),
            tool_def("cc_interface_edit", "Редактировать CommandInterface.xml", serde_json::json!({
                "type": "object", "properties": {
                    "path": {"type": "string", "description": "Путь к CommandInterface.xml"},
                    "operation": {"type": "string", "description": "hide|show|place|order|subsystem-order"},
                    "value": {"type": "string", "description": "Параметры операции"},
                    "create_if_missing": {"type": "boolean", "description": "Создать если отсутствует", "default": false}
                }, "required": ["path", "operation"]
            })),
            tool_def("cc_role_validate", "Проверить структуру роли 1С", serde_json::json!({
                "type": "object", "properties": {
                    "rights_path": {"type": "string", "description": "Путь к Rights.xml или каталогу роли"},
                    "detailed": {"type": "boolean", "default": false}
                }, "required": ["rights_path"]
            })),
            tool_def("cc_mxl_info", "Информация о табличном документе (MXL)", serde_json::json!({
                "type": "object", "properties": {
                    "path": {"type": "string", "description": "Путь к Template.xml"},
                    "format": {"type": "string", "description": "text|json", "default": "text"}
                }, "required": ["path"]
            })),
            tool_def("cc_mxl_compile", "Скомпилировать MXL.json в Template.xml", serde_json::json!({
                "type": "object", "properties": {
                    "json_path": {"type": "string"},
                    "out_path": {"type": "string"}
                }, "required": ["json_path", "out_path"]
            })),
            tool_def("cc_mxl_decompile", "Декомпилировать Template.xml в MXL.json", serde_json::json!({
                "type": "object", "properties": {
                    "xml_path": {"type": "string"},
                    "out_path": {"type": "string"}
                }, "required": ["xml_path"]
            })),
            tool_def("cc_form_edit", "Редактировать управляемую форму (Form.xml)", serde_json::json!({
                "type": "object", "properties": {
                    "form_path": {"type": "string", "description": "Путь к Form.xml"},
                    "operation": {"type": "string", "description": "add-element|remove-element|move-element|set-property"},
                    "value": {"type": "string", "description": "Параметры операции"}
                }, "required": ["form_path", "operation"]
            })),
            tool_def("cc_form_validate", "Проверить структуру управляемой формы", serde_json::json!({
                "type": "object", "properties": {
                    "path": {"type": "string", "description": "Путь к Form.xml или каталогу формы"},
                    "detailed": {"type": "boolean", "default": false}
                }, "required": ["path"]
            })),
            tool_def("cc_cfe_init", "Создать пустое расширение конфигурации (CFE)", serde_json::json!({
                "type": "object", "properties": {
                    "name": {"type": "string", "description": "Имя расширения"},
                    "synonym": {"type": "string"},
                    "name_prefix": {"type": "string"},
                    "output_dir": {"type": "string", "default": "src"},
                    "purpose": {"type": "string", "description": "Patch|Customization|AddOn", "default": "Customization"},
                    "version": {"type": "string"},
                    "vendor": {"type": "string"},
                    "no_role": {"type": "boolean", "default": false}
                }, "required": ["name"]
            })),
            tool_def("cc_cfe_diff", "Анализ расширения конфигурации", serde_json::json!({
                "type": "object", "properties": {
                    "extension_path": {"type": "string"},
                    "mode": {"type": "string", "description": "A|B", "default": "A"}
                }, "required": ["extension_path"]
            })),
            tool_def("cc_cfe_patch_method", "Добавить перехватчик метода в расширение", serde_json::json!({
                "type": "object", "properties": {
                    "extension_path": {"type": "string"},
                    "module_path": {"type": "string", "description": "Catalog.X.ObjectModule или Catalog.X.Form.Y"},
                    "method_name": {"type": "string"},
                    "interceptor_type": {"type": "string", "description": "Before|After|ModificationAndControl"},
                    "context": {"type": "string", "description": "НаСервере|НаКлиенте|НаСервереБезКонтекста", "default": "НаСервере"},
                    "is_function": {"type": "boolean", "default": false}
                }, "required": ["extension_path", "module_path", "method_name", "interceptor_type"]
            })),
            tool_def("cc_cfe_validate", "Проверить структуру расширения конфигурации", serde_json::json!({
                "type": "object", "properties": {
                    "extension_path": {"type": "string"},
                    "detailed": {"type": "boolean", "default": false}
                }, "required": ["extension_path"]
            })),
            tool_def("cc_form_patterns", "Паттерны проектирования управляемых форм 1С", serde_json::json!({
                "type": "object", "properties": {
                    "pattern": {"type": "string", "description": "list|form-document|form-data-processor|form-list", "default": "list"}
                }
            })),
        ]
    })
}

fn tool_def(name: &str, desc: &str, schema: Value) -> Value {
    serde_json::json!({ "name": name, "description": desc, "inputSchema": schema })
}

pub async fn call_tool(params: Value, config: &crate::config::Config) -> Result<Value> {
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);

    let result = match name {
        // DB
        "cc_db_create" => cli::create_infobase(args, config).await,
        "cc_db_dump_cf" => cli::dump_cf(args, config).await,
        "cc_db_load_cf" => cli::load_cf(args, config).await,
        "cc_db_dump_dt" => cli::dump_dt(args, config).await,
        "cc_db_load_dt" => cli::load_dt(args, config).await,
        "cc_db_dump_xml" => cli::dump_xml(args, config).await,
        "cc_db_load_xml" => cli::load_xml(args, config).await,
        "cc_db_update" => cli::update_infobase(args, config).await,
        "cc_db_run" => cli::run_enterprise(args, config).await,
        // DB (non-1C — use v8i parser)
        "cc_db_list" => {
            let bases = v8i::parse_v8i_file(None);
            if bases.is_empty() {
                Ok("Информационные базы не найдены. Проверьте %APPDATA%\\1C\\1CEStart\\ibases.v8i".to_string())
            } else {
                let mut out = format!("Найдено баз: {}\n\n", bases.len());
                for (i, b) in bases.iter().enumerate() {
                    out.push_str(&format!("{}. {} — {} ({})\n", i + 1, b.name, b.connection, match b.base_type { crate::v8i::InfobaseType::File => "файловая", crate::v8i::InfobaseType::Server => "серверная" }));
                }
                Ok(out)
            }
        },
        // EPF/ERF
        "cc_epf_build" => cli::build_epf(args, config).await,
        "cc_epf_dump" => cli::dump_epf(args, config).await,
        "cc_erf_build" => cli::build_erf(args, config).await,
        "cc_erf_dump" => cli::dump_erf(args, config).await,
        // Web
        "cc_web_publish" => Ok(format!("Web publication started. IB: {}, path: {}",
            args.get("ib_path").and_then(|v| v.as_str()).unwrap_or(""),
            args.get("pub_path").and_then(|v| v.as_str()).unwrap_or(""))),
        // Info
        "cc_meta_info" => info::meta::get_meta_info(args).await,
        "cc_form_info" => info::form::get_form_info(args).await,
        "cc_skd_info" => info::skd::get_skd_info(args).await,
        // DSL
        "cc_form_compile" => dsl::form::compile_form(args).await,
        "cc_form_decompile" => dsl::form::decompile_form(args).await,
        "cc_meta_compile" => dsl::meta::compile_meta(args).await,
        "cc_skd_compile" => dsl::skd::compile_skd(args).await,
        // Skills (ported from cc-1c-skills)
        "cc_epf_init" => skills::epf_init::init(args).await,
        "cc_template_add" => skills::template_add::add_template(args).await,
        "cc_template_remove" => skills::template_remove::remove_template(args).await,
        "cc_form_add" => skills::form_add::add_form(args).await,
        "cc_form_remove" => skills::form_remove::remove_form(args).await,
        "cc_help_add" => skills::help_add::add_help(args).await,
        "cc_support_edit" => skills::support_edit::support_edit(args).await,
        "cc_cf_init" => skills::cf_init::init(args).await,
        "cc_epf_validate" => skills::epf_validate::validate(args).await,
        "cc_erf_validate" => skills::erf_validate::validate(args).await,
        "cc_cf_info" => skills::cf_info::info(args).await,
        "cc_cf_edit" => skills::cf_edit::edit(args).await,
        "cc_cf_validate" => skills::cf_validate::validate(args).await,
        "cc_meta_edit" => skills::meta_edit::edit(args).await,
        "cc_meta_validate" => skills::meta_validate::validate(args).await,
        "cc_meta_remove" => skills::meta_remove::remove(args).await,
        "cc_subsystem_info" => skills::subsystem_info::info(args).await,
        "cc_subsystem_edit" => skills::subsystem_edit::edit(args).await,
        "cc_subsystem_validate" => skills::subsystem_validate::validate(args).await,
        "cc_interface_edit" => skills::interface_edit::edit(args).await,
        "cc_role_validate" => skills::role_validate::validate(args).await,
        "cc_mxl_info" => skills::mxl_info::info(args).await,
        "cc_mxl_compile" => skills::mxl_compile::compile(args).await,
        "cc_mxl_decompile" => skills::mxl_decompile::decompile(args).await,
        "cc_form_edit" => skills::form_edit::edit(args).await,
        "cc_form_validate" => skills::form_validate::validate(args).await,
        "cc_cfe_init" => skills::cfe_init::init(args).await,
        "cc_cfe_diff" => skills::cfe_diff::diff(args).await,
        "cc_cfe_patch_method" => skills::cfe_patch_method::patch_method(args).await,
        "cc_cfe_validate" => skills::cfe_validate::validate(args).await,
        "cc_form_patterns" => skills::form_patterns::patterns(args).await,
        _ => return Err(anyhow!("Tool not implemented: {}", name)),
    };

    let content = match result {
        Ok(text) => text,
        Err(e) => return Err(e),
    };

    Ok(serde_json::json!({
        "content": [{ "type": "text", "text": content }]
    }))
}