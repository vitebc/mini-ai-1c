use super::models::{ApiMessage, ToolInfo};
use crate::commands::skills::skill_summary_md;
use crate::llm_profiles::LLMProvider;
use crate::settings::{load_settings, CustomPromptsSettings, PromptBehaviorPreset};

pub mod cli;
pub mod custom;
pub mod diff;
pub mod maintenance;
pub mod planning;
pub mod project;
pub mod search;
pub mod shared;

pub use diff::DIFF_FORMAT_INSTRUCTIONS;
use cli::{CLI_IDENTITY, CLI_MARKING};
use custom::append_custom_prompt_settings;
use maintenance::{
    maintenance_isolation_rules, MAINTENANCE_IDENTITY, MAINTENANCE_ISOLATION_RULE,
    MAINTENANCE_STRICT_COMPLIANCE,
};
use planning::{PLANNING_IDENTITY, PLANNING_READONLY_RULES};
use project::{PROJECT_IDENTITY, PROJECT_MARKING};
use search::SEARCH_GUIDE;
use shared::{
    DOC_FORMAT_BLOCK, LANGUAGE_BLOCK, QUESTION_ACTION_NO_CODE, QUESTION_ACTION_WITH_CODE,
    SKILLS_SCRIPTS_BLOCK, TABS_BLOCK,
};

/// Число последних сообщений, которые учитываются при детекции кода в контексте.
///
/// P1: `has_code_context` раньше сканировал ВСЮ историю — один кусок кода в
/// начале чата навсегда включал SEARCH/REPLACE-режим, даже для вопросов.
/// Теперь смотрим только последние сообщения (скользящее окно).
pub const CODE_CONTEXT_LOOKBACK: usize = 4;

/// Проверяет наличие BSL-кода в НЕДАВНИХ сообщениях диалога.
///
/// Смотрит только последние `CODE_CONTEXT_LOOKBACK` сообщений, чтобы
/// устаревший код из начала чата не держал SEARCH/REPLACE-режим включённым.
pub fn has_code_context(messages: &[ApiMessage]) -> bool {
    for msg in messages.iter().rev().take(CODE_CONTEXT_LOOKBACK) {
        if let Some(content) = &msg.content {
            if content.contains("```bsl") || content.contains("```1c") {
                return true;
            }
            let bsl_markers = [
                "КонецФункции",
                "КонецПроцедуры",
                "КонецЕсли",
                "Функция ",
                "Процедура ",
            ];
            let count = bsl_markers.iter().filter(|&&m| content.contains(m)).count();
            if count >= 2 {
                return true;
            }
        }
    }
    false
}

/// Возвращает true для локальных провайдеров (Ollama, LMStudio), которым нужен компактный промпт.
pub fn is_local_provider(provider: Option<&LLMProvider>) -> bool {
    matches!(
        provider,
        Some(LLMProvider::Ollama) | Some(LLMProvider::LMStudio)
    )
}

/// Компактный системный промпт для локальных моделей (Ollama/LMStudio).
pub fn get_lightweight_system_prompt(
    available_tools: &[ToolInfo],
    messages: &[ApiMessage],
) -> String {
    let settings = load_settings();
    build_lightweight_system_prompt_with_custom_prompts(
        available_tools,
        messages,
        &settings.custom_prompts,
    )
}

fn build_lightweight_system_prompt_with_custom_prompts(
    available_tools: &[ToolInfo],
    messages: &[ApiMessage],
    custom_prompts: &CustomPromptsSettings,
) -> String {
    let preset = load_settings().code_generation.behavior_preset;
    let has_code = has_code_context(messages);

    let identity = match preset {
        PromptBehaviorPreset::Project => PROJECT_IDENTITY,
        PromptBehaviorPreset::Maintenance => MAINTENANCE_IDENTITY,
        PromptBehaviorPreset::Cli => CLI_IDENTITY,
        PromptBehaviorPreset::Planning => PLANNING_IDENTITY,
    };

    let diff_section = if has_code {
        r#"
При изменении кода используй ТОЛЬКО xml-формат diff:
<diff>
  <search>[точный фрагмент оригинала]</search>
  <replace>[новый вариант]</replace>
</diff>
При создании кода с нуля — используй блок ```bsl.
Не переписывай весь файл — изменяй только запрошенные строки."#
    } else {
        "\nПри создании нового кода используй блок ```bsl."
    };

    let mut prompt = format!(
        r#"{identity}

{language}
{question}
{skills}
{diff_section}"#,
        identity = identity,
        language = "Отвечай ТОЛЬКО на русском языке.",
        question = if matches!(preset, PromptBehaviorPreset::Project | PromptBehaviorPreset::Maintenance) {
            if has_code {
                "Выполняй запросы точно и без лишних изменений. Не задавай уточняющих вопросов — выполняй задачу сразу."
            } else {
                "Выполняй запросы точно и без лишних изменений."
            }
        } else {
            "Выполняй запросы точно и без лишних изменений."
        },
        skills = "Отступы в коде BSL — табуляция (ASCII 0x09), не пробелы. Описания функций — только комментарии //.",
        diff_section = diff_section,
    );

    // Добавляем краткое перечисление доступных инструментов (без подробной матрицы)
    if !available_tools.is_empty() {
        prompt.push_str("\n\nДоступные инструменты:\n");
        for info in available_tools {
            let name = &info.tool.function.name;
            let desc = &info.tool.function.description;
            let short_desc = desc.lines().next().unwrap_or(desc);
            prompt.push_str(&format!("- `{name}`: {short_desc}\n"));
        }
    }

    append_custom_prompt_settings(&mut prompt, custom_prompts);

    prompt
}

/// Маркеры поискового намерения в запросе пользователя.
///
/// P1: тяжёлый search-блок (~20 строк) добавляется в промпт только когда
/// в недавних сообщениях есть поисковое намерение («найди», «где используется»,
/// «impact») ИЛИ контекст кода отсутствует (исследовательский чат). На простых
/// правках («добавь комментарий») лишние инструкции поиска не нужны.
pub const SEARCH_INTENT_MARKERS: &[&str] = &[
    "найди",
    "найти",
    "найдите",
    "поиск",
    "поищи",
    "где используется",
    "где используется?",
    "impact",
    "влияние изменений",
    "кто вызывает",
    "что делает",
    "как работает",
    "какая функция",
    "есть ли функция",
    "есть ли метод",
    "структура объекта",
    "найди функцию",
    "semantic_find",
    "find_references",
];

/// Определяет, есть ли в НЕДАВНИХ сообщениях поисковое намерение.
pub fn has_search_intent(messages: &[ApiMessage]) -> bool {
    for msg in messages.iter().rev().take(CODE_CONTEXT_LOOKBACK) {
        if let Some(content) = &msg.content {
            let lower = content.to_lowercase();
            if SEARCH_INTENT_MARKERS
                .iter()
                .any(|m| lower.contains(m))
            {
                return true;
            }
        }
    }
    false
}

/// Get dynamic system prompt based on available tools.
pub fn get_system_prompt(available_tools: &[ToolInfo], messages: &[ApiMessage]) -> String {
    let settings = load_settings();
    let custom = &settings.custom_prompts;
    let code_gen = &settings.code_generation;

    let mut prompt = String::new();
    let has_code = has_code_context(messages);

    // --- Идентичность (одна роль из preset, без дубля) ---
    match code_gen.behavior_preset {
        PromptBehaviorPreset::Project => prompt.push_str(PROJECT_IDENTITY),
        PromptBehaviorPreset::Maintenance => prompt.push_str(MAINTENANCE_IDENTITY),
        PromptBehaviorPreset::Cli => prompt.push_str(CLI_IDENTITY),
        PromptBehaviorPreset::Planning => prompt.push_str(PLANNING_IDENTITY),
    }
    prompt.push_str("\n\n");

    // --- Общие блоки ---
    prompt.push_str(LANGUAGE_BLOCK);
    prompt.push_str("\n");
    prompt.push_str(TABS_BLOCK);
    prompt.push_str("\n");
    prompt.push_str(DOC_FORMAT_BLOCK);
    prompt.push_str("\n");
    prompt.push_str(SKILLS_SCRIPTS_BLOCK);
    prompt.push_str("\n");

    // --- Каталог доступных скиллов (компактно, 1 строка на скилл) ---
    let skill_summary = skill_summary_md();
    if !skill_summary.is_empty() {
        prompt.push_str(&skill_summary);
        prompt.push_str("\n");
    }

    // --- Режим «вопрос/действие» (только Project и Maintenance) ---
    if matches!(
        code_gen.behavior_preset,
        PromptBehaviorPreset::Project | PromptBehaviorPreset::Maintenance
    ) {
        prompt.push_str(if has_code {
            QUESTION_ACTION_WITH_CODE
        } else {
            QUESTION_ACTION_NO_CODE
        });
        prompt.push_str("\n");
    }

    // --- Формат правок (когда есть контекст кода, кроме Planning — read-only) ---
    if has_code && code_gen.behavior_preset != PromptBehaviorPreset::Planning {
        prompt.push_str(DIFF_FORMAT_INSTRUCTIONS);
        prompt.push_str("\n");
    }

    // --- STRICT COMPLIANCE и изоляция — только для Maintenance (P0 №5) ---
    if code_gen.behavior_preset == PromptBehaviorPreset::Maintenance {
        prompt.push_str(MAINTENANCE_STRICT_COMPLIANCE);
        prompt.push_str("\n");
        prompt.push_str(MAINTENANCE_ISOLATION_RULE);
        prompt.push_str("\n");
    }

    // --- Правила маркировки изменений ---
    if code_gen.mark_changes || code_gen.behavior_preset == PromptBehaviorPreset::Maintenance {
        match code_gen.behavior_preset {
            PromptBehaviorPreset::Maintenance => {
                let now = chrono::Local::now();
                let date_str = now.format("%Y-%m-%d").to_string();
                let datetime_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

                let addition_marker = code_gen
                    .addition_marker_template
                    .replace("{datetime}", &datetime_str)
                    .replace("{date}", &date_str);
                let modification_marker = code_gen
                    .modification_marker_template
                    .replace("{datetime}", &datetime_str)
                    .replace("{date}", &date_str);
                let deletion_marker = code_gen
                    .deletion_marker_template
                    .replace("{datetime}", &datetime_str)
                    .replace("{date}", &date_str);

                prompt.push_str(&maintenance_isolation_rules(
                    &addition_marker,
                    &modification_marker,
                    &deletion_marker,
                ));
                prompt.push_str("\n");
            }
            PromptBehaviorPreset::Project => {
                prompt.push_str(PROJECT_MARKING);
                prompt.push_str("\n");
            }
            PromptBehaviorPreset::Cli => {
                prompt.push_str(CLI_MARKING);
                prompt.push_str("\n");
            }
            PromptBehaviorPreset::Planning => {}
        }
    }

    // --- Правила read-only для Planning ---
    if code_gen.behavior_preset == PromptBehaviorPreset::Planning {
        prompt.push_str(PLANNING_READONLY_RULES);
        prompt.push_str("\n");
    }

    // --- Инструменты ---
    if !available_tools.is_empty() {
        prompt.push_str("\n\nВАЖНО: Тебе доступны следующие специализированные инструменты MCP:\n");
        for info in available_tools {
            let tool = &info.tool;
            let desc = if tool.function.description.is_empty() {
                "(описание отсутствует)"
            } else {
                &tool.function.description
            };
            prompt.push_str(&format!(
                "- `{}` (сервер: {}): {}\n",
                tool.function.name, info.server_id, desc
            ));
        }

        append_tool_usage_rules(&mut prompt, available_tools);

        let has_search = available_tools
            .iter()
            .any(|t| t.server_id == "builtin-1c-search");
        // P1: search-блок инжектится только при поисковом намерении или в
        // исследовательском чате без контекста кода. Для Planning — всегда
        // (режим по своей сути исследовательский).
        let is_planning = code_gen.behavior_preset == PromptBehaviorPreset::Planning;
        let research_chat = !has_code;
        if has_search && (is_planning || has_search_intent(messages) || research_chat) {
            prompt.push_str("\n");
            prompt.push_str(SEARCH_GUIDE);
        }
    }

    // --- Пользовательские override в конце (P0 №10) ---
    append_custom_prompt_settings(&mut prompt, custom);

    prompt
}

/// Правила использования конкретных инструментов (check_bsl, ask_1c_ai, справка, метаданные).
fn append_tool_usage_rules(prompt: &mut String, available_tools: &[ToolInfo]) {
    prompt.push_str("\nКРИТИЧЕСКИЕ ПРАВИЛА ИСПОЛЬЗОВАНИЯ ИНСТРУМЕНТОВ:\n");

    if available_tools
        .iter()
        .any(|t| t.tool.function.name == "check_bsl_syntax")
    {
        prompt.push_str(
            "1. `check_bsl_syntax` (сервер bsl-ls): Используй для анализа и самопроверки.\n",
        );
        prompt.push_str("\n");
        prompt
            .push_str("   РЕЖИМ А — Самопроверка (ИИ проверяет свои собственные изменения):\n");
        prompt.push_str(
            "   - Зона ответственности: ТОЛЬКО строки, которые ты сам добавил или изменил.\n",
        );
        prompt.push_str(
            "   - ЗАПРЕТ: не трогай ошибки в окружающем Legacy-коде, даже в той же функции.\n",
        );
        prompt.push_str(
            "   - 'Cognitive Complexity', 'Magic Number' в старом коде — ИГНОРИРУЙ.\n",
        );
        prompt.push_str("   - Исправляй ТОЛЬКО критические синтаксические ошибки (забытая скобка и т.п.).\n");
        prompt.push_str("\n");
        prompt.push_str("   РЕЖИМ Б — Выполнение явного запроса пользователя:\n");
        prompt.push_str("   - Если пользователь ЯВНО просит исправить ошибки, добавить описание, устранить предупреждения — ВЫПОЛНЯЙ.\n");
        prompt.push_str("   - Примеры явных запросов: 'исправь ошибки bsl', 'добавь описание параметров', 'устрани предупреждения'.\n");
        prompt.push_str("   - ОБЯЗАТЕЛЬНО: перед внесением исправлений СНАЧАЛА вызови `check_bsl_syntax` для получения актуального анализа кода.\n");
        prompt.push_str("   - В этом режиме исправляй ВСЕ указанные пользователем проблемы, включая Legacy-код.\n");
        prompt.push_str("   - НЕ отказывайся со ссылкой на правила Legacy — пользователь осознанно просит изменения.\n");
        prompt.push_str("   - ИСКЛЮЧЕНИЕ — `=== SELECTIVE BSL FIX SCOPE ===`: если пользователь прислал этот маркер, он явно ограничил объём исправления выбранным subset диагностик.\n");
        prompt.push_str("   - При `=== SELECTIVE BSL FIX SCOPE ===` НЕ вызывай `check_bsl_syntax` до внесения правок и исправляй только явно перечисленные выбранные диагностики.\n");
        prompt.push_str("   - При `=== SELECTIVE BSL FIX SCOPE ===` после правок `check_bsl_syntax` допустим только для самопроверки изменённых строк.\n");
    }

    if available_tools
        .iter()
        .any(|t| t.tool.function.name == "ask_1c_ai")
    {
        prompt.push_str("2. `ask_1c_ai` (сервер \"Напарник\" / 1C:Naparnik): Это инструмент для поиска в информационной системе 1С:ИТС.\n");
        prompt.push_str("   - При команде /итс или запросе про ИТС — ВСЕГДА вызывай `ask_1c_ai` напрямую, не раздумывая.\n");
        prompt.push_str("   - Также используй для консультаций по стандартам 1С и БСП.\n");
    }

    if available_tools
        .iter()
        .any(|t| t.server_id == "builtin-1c-help")
    {
        prompt.push_str(r#"
3. `1С:Справка` (сервер builtin-1c-help): ЭТАЛОН СИНТАКСИСА И ОБЪЕКТНОЙ МОДЕЛИ.
   - Используй `search_1c_help` и `get_1c_help_topic` как ГЛАВНЫЙ источник правды при написании кода.
   - КРИТИЧЕСКОЕ ПРАВИЛО: Если ты не уверен на 100% в названии метода, порядке или типе параметров — ты ОБЯЗАН вызвать поиск по справке.
   - ЗАПРЕТ НА ГАЛЛЮЦИНАЦИИ: Категорически запрещено выдумывать синтаксис 1С, методы или свойства, которых нет в официальной документации.
   - Отличие от BSL-чеков: Справка используется ДО написания кода для верификации знаний, а `check_bsl_syntax` — ПОСЛЕ для поиска локальных ошибок.
"#);
    }

    if available_tools
        .iter()
        .any(|t| t.tool.function.name.contains("metadata"))
    {
        prompt.push_str("4. Инструменты метаданных: при неуверенности в структуре незнакомого объекта проверяй его через метаданные перед обращением к полям через точку, чтобы избежать ошибок 'Поле объекта не обнаружено'.\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::models::{Tool, ToolFunction};
    use crate::settings::{CustomPromptsSettings, PromptTemplate};
    use serde_json::json;

    fn make_user_message(content: &str) -> ApiMessage {
        ApiMessage {
            role: "user".to_string(),
            content: Some(content.to_string()),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    fn make_check_bsl_tool() -> ToolInfo {
        ToolInfo {
            tool: Tool {
                r#type: "function".to_string(),
                function: ToolFunction {
                    name: "check_bsl_syntax".to_string(),
                    description: "Проверить BSL-код".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "code": {
                                "type": "string"
                            }
                        },
                        "required": ["code"]
                    }),
                },
            },
            server_id: "bsl-ls".to_string(),
        }
    }

    fn make_search_tool() -> ToolInfo {
        ToolInfo {
            tool: Tool {
                r#type: "function".to_string(),
                function: ToolFunction {
                    name: "semantic_find".to_string(),
                    description: "Найти функцию по описанию".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string"
                            }
                        },
                        "required": ["query"]
                    }),
                },
            },
            server_id: "builtin-1c-search".to_string(),
        }
    }

    fn make_custom_prompts_with_templates(templates: Vec<PromptTemplate>) -> CustomPromptsSettings {
        CustomPromptsSettings {
            system_prefix: String::new(),
            on_code_change: String::new(),
            on_code_generate: String::new(),
            templates,
        }
    }

    #[test]
    fn lightweight_system_prompt_includes_enabled_custom_templates() {
        let custom = make_custom_prompts_with_templates(vec![PromptTemplate {
            id: "issue-160-rule".to_string(),
            name: "Issue 160 Rule".to_string(),
            description: "Regression marker".to_string(),
            content: "ISSUE160_CHECK_BSL_AFTER_EACH_ANSWER".to_string(),
            enabled: true,
        }]);

        let prompt = build_lightweight_system_prompt_with_custom_prompts(
            &[],
            &[make_user_message("Напиши функцию")],
            &custom,
        );

        assert!(prompt.contains("ISSUE160_CHECK_BSL_AFTER_EACH_ANSWER"));
        assert!(prompt.contains("Issue 160 Rule"));
    }

    #[test]
    fn lightweight_system_prompt_skips_disabled_custom_templates() {
        let custom = make_custom_prompts_with_templates(vec![PromptTemplate {
            id: "disabled-rule".to_string(),
            name: "Disabled Rule".to_string(),
            description: "Should stay out".to_string(),
            content: "ISSUE160_DISABLED_RULE_SHOULD_NOT_APPEAR".to_string(),
            enabled: false,
        }]);

        let prompt = build_lightweight_system_prompt_with_custom_prompts(
            &[],
            &[make_user_message("Напиши функцию")],
            &custom,
        );

        assert!(!prompt.contains("ISSUE160_DISABLED_RULE_SHOULD_NOT_APPEAR"));
        assert!(!prompt.contains("Disabled Rule"));
    }

    #[test]
    fn system_prompt_describes_strict_rule_for_selective_fix_scope() {
        let prompt =
            get_system_prompt(&[make_check_bsl_tool()], &[make_user_message("/исправить")]);

        assert!(prompt.contains("=== SELECTIVE BSL FIX SCOPE ==="));
        assert!(prompt.contains("НЕ вызывай `check_bsl_syntax` до внесения правок"));
        assert!(prompt.contains("исправляй только явно перечисленные выбранные диагностики"));
    }

    #[test]
    fn lightweight_prompt_is_shorter_than_full_prompt() {
        let tools = vec![make_check_bsl_tool()];
        let msgs = vec![make_user_message("напиши функцию")];

        let full = get_system_prompt(&tools, &msgs);
        let light = get_lightweight_system_prompt(&tools, &msgs);

        assert!(
            light.len() < full.len() / 2,
            "lightweight ({} chars) should be < half of full ({} chars)",
            light.len(),
            full.len(),
        );
    }

    #[test]
    fn is_local_provider_matches_ollama_and_lmstudio() {
        use crate::llm_profiles::LLMProvider;
        assert!(is_local_provider(Some(&LLMProvider::Ollama)));
        assert!(is_local_provider(Some(&LLMProvider::LMStudio)));
        assert!(!is_local_provider(Some(&LLMProvider::OpenAI)));
        assert!(!is_local_provider(Some(&LLMProvider::Anthropic)));
        assert!(!is_local_provider(None));
    }

    #[test]
    fn maintenance_preset_contains_strict_compliance_and_identity() {
        // Проверяем блоки напрямую через константы, чтобы не зависеть от файла настроек.
        let prompt = MAINTENANCE_STRICT_COMPLIANCE.to_string()
            + "\n"
            + MAINTENANCE_ISOLATION_RULE
            + "\n"
            + MAINTENANCE_IDENTITY;
        assert!(prompt.contains("НИКОГДА не проводи рефакторинг"));
        assert!(prompt.contains("ЗАПРЕЩАЕТСЯ любой самопроизвольный рефакторинг"));
        assert!(prompt.contains("изолировать комментариями"));
    }

    #[test]
    fn project_preset_has_no_strict_compliance() {
        assert!(!PROJECT_IDENTITY.contains("НИКОГДА не проводи рефакторинг"));
        assert!(!PROJECT_IDENTITY.contains("ЗАПРЕЩАЕТСЯ любой самопроизвольный рефакторинг"));
    }

    #[test]
    fn planning_preset_is_read_only() {
        assert!(PLANNING_IDENTITY.contains("НЕ вноси изменения в код"));
        assert!(PLANNING_READONLY_RULES.contains("READ-ONLY"));
        assert!(PLANNING_READONLY_RULES.contains("план"));
    }

    #[test]
    fn diff_instructions_are_in_russian_with_example() {
        assert!(DIFF_FORMAT_INSTRUCTIONS.contains("<diff>"));
        assert!(DIFF_FORMAT_INSTRUCTIONS.contains("ПРИМЕР"));
        assert!(DIFF_FORMAT_INSTRUCTIONS.contains("НЕ выдумывай"));
        // P0 №2: формат на русском, без "Modiffy"
        assert!(!DIFF_FORMAT_INSTRUCTIONS.contains("Modiffy"));
    }

    #[test]
    fn search_guide_is_compact() {
        let lines = SEARCH_GUIDE.lines().count();
        assert!(
            lines <= 25,
            "search guide должен быть компактным (~20 строк), сейчас {lines}"
        );
        assert!(SEARCH_GUIDE.contains("semantic_find"));
        assert!(SEARCH_GUIDE.contains("smart_find"));
        assert!(SEARCH_GUIDE.contains("Справочники.Контрагенты"));
    }

    #[test]
    fn language_block_hardcodes_russian() {
        assert!(LANGUAGE_BLOCK.contains("русском"));
        // P0 №1: нет противоречивых {target_lang}
        assert!(!LANGUAGE_BLOCK.contains("{target_lang}"));
        assert!(!LANGUAGE_BLOCK.contains("ALWAYS respond"));
    }

    #[test]
    fn tabs_block_uses_ascii_not_backslash_t() {
        assert!(TABS_BLOCK.contains("0x09"));
        // P0 №8: в raw string не должно быть "буквального \\t" как инструкции
        assert!(!TABS_BLOCK.contains("\\t"));
    }

    #[test]
    fn custom_override_appears_at_the_end() {
        let mut custom = make_custom_prompts_with_templates(vec![PromptTemplate {
            id: "final-rule".to_string(),
            name: "Final Rule".to_string(),
            description: "Should be at end".to_string(),
            content: "FINAL_OVERRIDE_MARKER".to_string(),
            enabled: true,
        }]);
        custom.system_prefix = "GLOBAL_PREFIX_MARKER".to_string();

        let mut prompt = String::new();
        prompt.push_str("=== СТАРТ ===\n");
        append_custom_prompt_settings(&mut prompt, &custom);

        assert!(prompt.ends_with("FINAL_OVERRIDE_MARKER\n"));
        assert!(prompt.find("GLOBAL_PREFIX_MARKER").unwrap() < prompt.find("FINAL_OVERRIDE_MARKER").unwrap());
    }

    #[test]
    fn system_prompt_includes_search_guide_when_search_tools_available() {
        let prompt = get_system_prompt(
            &[make_search_tool()],
            &[make_user_message("найди функцию, которая рассчитывает НДС")],
        );
        assert!(prompt.contains("=== ПОИСК ПО КОНФИГУРАЦИИ (builtin-1c-search) ==="));
    }

    #[test]
    fn search_guide_omitted_for_simple_edit_without_search_intent() {
        // Контекст кода есть, но поискового намерения нет → search-блок не нужен.
        let msgs = vec![make_user_message(
            "Исправь функцию:\n```bsl\nФункция Тест()\n\tКонецФункции\n```",
        )];
        assert!(has_code_context(&msgs), "тест должен иметь контекст кода");
        assert!(!has_search_intent(&msgs), "тест не должен иметь search-intent");

        let prompt = get_system_prompt(&[make_search_tool()], &msgs);
        assert!(
            !prompt.contains("=== ПОИСК ПО КОНФИГУРАЦИИ (builtin-1c-search) ==="),
            "search-блок не должен инжектиться при простой правке"
        );
    }

    #[test]
    fn search_guide_injected_for_research_chat_without_code() {
        // Нет контекста кода → исследовательский чат → search-блок присутствует.
        let msgs = vec![make_user_message("Расскажи про учёт НДС в типовой")];
        assert!(!has_code_context(&msgs));
        assert!(!has_search_intent(&msgs));

        let prompt = get_system_prompt(&[make_search_tool()], &msgs);
        assert!(prompt.contains("=== ПОИСК ПО КОНФИГУРАЦИИ (builtin-1c-search) ==="));
    }

    #[test]
    fn has_code_context_uses_recent_messages_lookback() {
        // Код есть только в старом сообщении (раньше окна) → контекст кода отсутствует.
        let mut msgs = Vec::new();
        for _ in 0..(CODE_CONTEXT_LOOKBACK + 1) {
            msgs.push(make_user_message("просто текст без кода"));
        }
        msgs.push(make_user_message(
            "Функция Старая()\nКонецФункции\nКонецПроцедуры",
        ));
        // Старое сообщение — в начале списка (вне окна последних 4).
        let mut ordered = msgs.clone();
        ordered.reverse();
        assert!(!has_code_context(&ordered));
    }

    #[test]
    fn has_code_context_detects_recent_code() {
        let msgs = vec![
            make_user_message("вопрос без кода"),
            make_user_message("```bsl\nПроцедура Тест()\nКонецПроцедуры\n```"),
        ];
        assert!(has_code_context(&msgs));
    }

    /// Интеграционный тест с реальным Ollama + qwen2.5-coder:14b.
    ///
    /// Запустить:
    ///   OLLAMA_HOST=http://localhost:11434 cargo test -p mini-ai-1c -- ollama --nocapture --ignored
    ///
    /// Тест пропускается автоматически если Ollama недоступна или модель не загружена.
    #[tokio::test]
    #[ignore = "requires Ollama running with qwen2.5-coder:14b; run with --ignored"]
    async fn ollama_qwen_coder_14b_answers_not_rephrases() {
        let host =
            std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("reqwest client");

        // --- 1. Проверяем доступность Ollama ---
        let tags_url = format!("{host}/api/tags");
        let tags_resp = client.get(&tags_url).send().await;
        let tags_resp = match tags_resp {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[SKIP] Ollama не доступна по {host}: {e}");
                return;
            }
        };

        let tags_json: serde_json::Value = tags_resp
            .json()
            .await
            .expect("Ollama /api/tags returned invalid JSON");

        // --- 2. Проверяем что модель загружена ---
        let model_name = "qwen2.5-coder:14b";
        let models = tags_json["models"].as_array().cloned().unwrap_or_default();
        let model_available = models.iter().any(|m| {
            m["name"]
                .as_str()
                .unwrap_or("")
                .starts_with("qwen2.5-coder:14b")
                || m["model"]
                    .as_str()
                    .unwrap_or("")
                    .starts_with("qwen2.5-coder:14b")
        });
        if !model_available {
            eprintln!(
                "[SKIP] Модель {model_name} не найдена в Ollama. Доступные: {:?}",
                models
                    .iter()
                    .map(|m| m["name"].as_str().unwrap_or(""))
                    .collect::<Vec<_>>()
            );
            return;
        }

        // --- 3. Формируем лёгкий промпт ---
        let user_msg_content = "Напиши простую BSL-функцию ФункцияПример() без параметров, которая возвращает строку \"Привет, 1С!\".";
        let user_msg = make_user_message(user_msg_content);
        let tools: Vec<ToolInfo> = vec![];
        let system_content = get_lightweight_system_prompt(&tools, &[user_msg.clone()]);

        eprintln!(
            "[INFO] Лёгкий промпт ({} chars):\n{}",
            system_content.len(),
            system_content
        );

        // --- 4. Отправляем запрос ---
        let payload = serde_json::json!({
            "model": model_name,
            "messages": [
                { "role": "system", "content": system_content },
                { "role": "user",   "content": user_msg_content }
            ],
            "stream": false,
            "options": {
                "temperature": 0.1,
                "num_predict": 512
            }
        });

        let chat_url = format!("{host}/api/chat");
        let resp = client
            .post(&chat_url)
            .json(&payload)
            .send()
            .await
            .expect("Chat request failed");

        let status = resp.status();
        assert!(status.is_success(), "Ollama вернула статус {status}");

        let body: serde_json::Value = resp.json().await.expect("Response is not valid JSON");
        let answer = body["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        eprintln!("[INFO] Ответ модели:\n{answer}");

        // --- 5. Проверяем что ответ содержит BSL-код, а не перефразирование ---
        let lower = answer.to_lowercase();
        let has_code = answer.contains("Функция")
            || answer.contains("функция")
            || answer.contains("Процедура")
            || answer.contains("процедура")
            || answer.contains("```bsl")
            || answer.contains("КонецФункции")
            || answer.contains("Возврат");

        // Индикатор «перефразирования»: ответ — только вопрос без кода
        let first_line = answer.lines().next().unwrap_or("").trim();
        let is_only_question = first_line.ends_with('?') && !has_code;

        assert!(
            !is_only_question,
            "Модель перефразировала вопрос вместо ответа. Первая строка: «{first_line}»"
        );

        assert!(has_code, "Ответ не содержит BSL-кода. Ответ: «{answer}»");

        // Дополнительная проверка: промпт не содержит огромную матрицу инструментов
        assert!(
            !system_content.contains("МАТРИЦА ВЫБОРА ИНСТРУМЕНТА"),
            "Лёгкий промпт не должен содержать матрицу инструментов"
        );
        assert!(
            !system_content.contains("DIFF_FORMAT_INSTRUCTIONS"),
            "Лёгкий промпт не должен содержать полные DIFF инструкции"
        );

        eprintln!("[PASS] qwen2.5-coder:14b ответила кодом, не перефразировала вопрос.");
        let _ = lower; // suppress unused warning
    }
}
