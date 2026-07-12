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