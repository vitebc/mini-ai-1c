use crate::settings::CustomPromptsSettings;

/// Пользовательские настройки промптов (override).
///
/// P0-фикс №10: эти блоки всегда добавляются В КОНЕЦ системного промпта,
/// чтобы иметь приоритет над встроенными правилами.
pub fn append_custom_prompt_settings(prompt: &mut String, custom: &CustomPromptsSettings) {
    if !custom.system_prefix.trim().is_empty() {
        prompt.push_str("\n\n=== ПОЛЬЗОВАТЕЛЬСКИЕ ГЛОБАЛЬНЫЕ НАСТРОЙКИ (OVERRIDE) ===\n");
        prompt.push_str(&custom.system_prefix);
    }

    if !custom.on_code_change.trim().is_empty() {
        prompt.push_str("\n\n=== ПОЛЬЗОВАТЕЛЬСКИЕ ИНСТРУКЦИИ ДЛЯ ИЗМЕНЕНИЯ КОДА ===\n");
        prompt.push_str(&custom.on_code_change);
    }

    if !custom.on_code_generate.trim().is_empty() {
        prompt.push_str("\n\n=== ПОЛЬЗОВАТЕЛЬСКИЕ ИНСТРУКЦИИ ДЛЯ ГЕНЕРАЦИИ КОДА ===\n");
        prompt.push_str(&custom.on_code_generate);
    }

    let active_templates: Vec<_> = custom.templates.iter().filter(|t| t.enabled).collect();

    if !active_templates.is_empty() {
        prompt.push_str("\n\n=== АКТИВНЫЕ ШАБЛОНЫ ===\n");
        for template in active_templates {
            prompt.push_str(&format!("- {}\n{}\n", template.name, template.content));
        }
    }
}
