//! Settings management module for Mini AI 1C Agent
//! Persists application settings to JSON file

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

// ─── Кэш настроек ──────────────────────────────────────────────────────────────
//
// `load_settings()` читает файл и выполняет миграции при каждом вызове, а
// вызывается она на каждый LLM-запрос (см. `get_system_prompt`). Чтобы не
// читать диск на каждый запрос, результат кэшируется. Кэш инвалидируется:
// - при `save_settings()` (все записи настроек идут через неё);
// - при внешнем изменении settings.json — из файлового watcher
//   (`mcp_client::start_settings_watcher`).
static SETTINGS_CACHE: OnceLock<std::sync::Mutex<Option<AppSettings>>> = OnceLock::new();

fn settings_cache() -> &'static std::sync::Mutex<Option<AppSettings>> {
    SETTINGS_CACHE.get_or_init(|| std::sync::Mutex::new(None))
}

/// Инвалидирует кэш настроек. Вызывается из `save_settings` и файлового watcher.
pub fn invalidate_settings_cache() {
    if let Ok(mut guard) = settings_cache().lock() {
        *guard = None;
    }
}

// Helper functions for defaults
fn default_true() -> bool {
    true
}

fn default_editor_bridge_enabled_for_deser() -> bool {
    true
}

fn default_configurator_window_title_pattern() -> String {
    "Конфигуратор|1C:Enterprise".to_string()
}

fn is_default_configurator_window_title_pattern(value: &String) -> bool {
    value.trim().is_empty()
        || value == "Конфигуратор"
        || value == "Конфигуратор|Configurator"
        || value == &default_configurator_window_title_pattern()
}

fn default_addition_marker() -> String {
    "// Доработка START (Добавление) - {datetime}\n{newCode}\n// Доработка END".to_string()
}

fn default_modification_marker() -> String {
    "// Доработка START (Изменение) - {datetime}\n{newCode}\n// Доработка END".to_string()
}

fn default_deletion_marker() -> String {
    "// Доработка (Удаление) - {datetime}\n// {oldCode}".to_string()
}

fn default_max_iterations() -> Option<u32> {
    Some(7)
}

fn default_compress_strategy() -> String {
    "summarize".to_string()
}

/// Быстрые команды (Slash Commands)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    pub id: String,
    pub command: String,
    pub name: String,
    pub description: String,
    pub template: String,
    pub is_enabled: bool,
    pub is_system: bool,
}

fn default_slash_commands() -> Vec<SlashCommand> {
    vec![
        SlashCommand {
            id: "fix".to_string(),
            command: "исправить".to_string(),
            name: "Исправить".to_string(),
            description: "Исправить ошибки BSL и логические ошибки".to_string(),
            template: "Исправь ошибки в этом коде. Обрати внимание на следующие диагностики:\n{diagnostics}\n\nКод для исправления:\n```bsl\n{code}\n```".to_string(),
            is_enabled: true,
            is_system: true,
        },
        SlashCommand {
            id: "elaborate".to_string(),
            command: "доработай".to_string(),
            name: "Доработай".to_string(),
            description: "Доработать код по пользовательской задаче".to_string(),
            template: "Доработай этот код по следующей задаче: {query}\n\nТребования:\n- вноси только изменения, которые нужны для выполнения задачи;\n- сохрани стиль и совместимость с 1С;\n- если меняешь код, верни результат в формате, пригодном для сравнения и применения.\n\nКод для доработки:\n```bsl\n{code}\n```".to_string(),
            is_enabled: true,
            is_system: true,
        },
        SlashCommand {
            id: "refactor".to_string(),
            command: "рефакторинг".to_string(),
            name: "Рефакторинг".to_string(),
            description: "Улучшить структуру и читаемость кода".to_string(),
            template: "Проведи рефакторинг этого кода, улучши его структуру и читаемость, соблюдая стандарты 1С:\n```bsl\n{code}\n```".to_string(),
            is_enabled: true,
            is_system: true,
        },
        SlashCommand {
            id: "desc".to_string(),
            command: "описание".to_string(),
            name: "Описание".to_string(),
            description: "Сгенерировать описание процедуры/функции".to_string(),
            template: "Сгенерируй стандартную шапку описания для этой процедуры/функции в формате 1С (только комментарии //, без тегов <Описание>):\n```bsl\n{code}\n```".to_string(),
            is_enabled: true,
            is_system: true,
        },
        SlashCommand {
            id: "explain".to_string(),
            command: "объясни".to_string(),
            name: "Объясни".to_string(),
            description: "Подробно объяснить работу кода".to_string(),
            template: "Подробно объясни, как работает этот фрагмент кода:\n```bsl\n{code}\n```".to_string(),
            is_enabled: true,
            is_system: true,
        },
        SlashCommand {
            id: "review".to_string(),
            command: "ревью".to_string(),
            name: "Ревью".to_string(),
            description: "Провести код-ревью".to_string(),
            template: "Проведи подробное код-ревью этого фрагмента. Найди потенциальные баги, узкие места и предложи улучшения:\n```bsl\n{code}\n```".to_string(),
            is_enabled: true,
            is_system: true,
        },
        SlashCommand {
            id: "standards".to_string(),
            command: "стандарты".to_string(),
            name: "Стандарты".to_string(),
            description: "Проверить на соответствие стандартам 1С".to_string(),
            template: "Проверь этот код на соответствие официальным стандартам разработки 1С и БСП:\n```bsl\n{code}\n```".to_string(),
            is_enabled: true,
            is_system: true,
        },
        SlashCommand {
            id: "its".to_string(),
            command: "итс".to_string(),
            name: "1С:ИТС".to_string(),
            description: "Поиск информации в ИТС через Напарника".to_string(),
            template: "Используй инструменты MCP сервера \"Напарник\" (1C:Naparnik), чтобы найти ответ на мой вопрос в информационной системе 1С:ИТС. Мой вопрос: {query}".to_string(),
            is_enabled: true,
            is_system: true,
        },
        SlashCommand {
            id: "search-1c".to_string(),
            command: "найти".to_string(),
            name: "1С:Найти".to_string(),
            description: "Поиск кода в конфигурации 1С".to_string(),
            template: "Выполни поиск в конфигурации 1С по запросу: \"{query}\".\n\nИнструкции:\n1. Если запрос содержит имя процедуры или функции — используй find_symbol для точного поиска по символьному индексу.\n2. Если ищешь текст, переменную или фрагмент кода — используй search_code.\n3. Если в запросе упоминается конкретный объект (\"в модуле X\", \"в справочнике Y\") — передай scope в search_code.\n4. Для найденных символов — вызови get_symbol_context чтобы показать полный код.\nПокажи результаты с объяснением.".to_string(),
            is_enabled: true,
            is_system: true,
        },
        SlashCommand {
            id: "refs-1c".to_string(),
            command: "где".to_string(),
            name: "1С:Где используется".to_string(),
            description: "Найти все места использования символа в конфигурации".to_string(),
            template: "Найди все места использования \"{query}\" в конфигурации 1С.\nИспользуй инструмент find_references для поиска всех вхождений.\nПокажи результаты, сгруппированные по модулям, с краткой аннотацией к каждому месту использования.".to_string(),
            is_enabled: true,
            is_system: true,
        },
        SlashCommand {
            id: "struct-1c".to_string(),
            command: "объект".to_string(),
            name: "1С:Структура объекта".to_string(),
            description: "Показать структуру объекта конфигурации (реквизиты, ТЧ, формы)".to_string(),
            template: "Покажи структуру объекта конфигурации 1С: \"{query}\".\n1. Используй get_object_structure для получения реквизитов, табличных частей, форм и модулей.\n2. Если объект не найден — используй list_objects с name_filter для поиска похожих объектов.\n3. Опиши структуру понятно для разработчика.".to_string(),
            is_enabled: true,
            is_system: true,
        },
    ]
}

fn ensure_default_slash_commands(settings: &mut AppSettings) -> bool {
    let defaults = default_slash_commands();

    if settings.slash_commands.is_empty() {
        settings.slash_commands = defaults;
        return true;
    }

    let existing_ids: std::collections::HashSet<String> = settings
        .slash_commands
        .iter()
        .map(|command| command.id.clone())
        .collect();
    let missing_system_commands: Vec<SlashCommand> = defaults
        .into_iter()
        .filter(|command| command.is_system && !existing_ids.contains(&command.id))
        .collect();

    if missing_system_commands.is_empty() {
        return false;
    }

    settings.slash_commands.extend(missing_system_commands);
    true
}

/// Settings for 1C Configurator integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfiguratorSettings {
    #[serde(
        default = "default_configurator_window_title_pattern",
        skip_serializing_if = "is_default_configurator_window_title_pattern"
    )]
    pub window_title_pattern: String,
    /// Extra user-defined window title patterns (in addition to the default ones)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_window_title_patterns: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_window_hwnd: Option<isize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_window_pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_window_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_config_name: Option<String>,
    #[serde(default)]
    pub rdp_mode: bool,
    #[serde(default = "default_editor_bridge_enabled_for_deser")]
    pub editor_bridge_enabled: bool,
    #[serde(default)]
    pub editor_bridge_auto_apply: bool,
    /// Path to EditorBridge.exe, set after download or manual configuration
    #[serde(default)]
    pub editor_bridge_exe_path: String,
}

impl Default for ConfiguratorSettings {
    fn default() -> Self {
        Self {
            window_title_pattern: default_configurator_window_title_pattern(),
            extra_window_title_patterns: Vec::new(),
            selected_window_hwnd: None,
            selected_window_pid: None,
            selected_window_title: None,
            selected_config_name: None,
            rdp_mode: false,
            editor_bridge_enabled: false,
            editor_bridge_auto_apply: false,
            editor_bridge_exe_path: String::new(),
        }
    }
}

/// Settings for BSL Language Server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BSLServerSettings {
    #[serde(default)]
    pub executable_path: String,
    #[serde(default)]
    pub installed_version: String,
    #[serde(default)]
    pub workspace_path: String,
    pub jar_path: String,
    pub auto_download: bool,
    pub websocket_port: u16,
    pub java_path: String,
    pub enabled: bool,
    /// Remote WebSocket URL (e.g. ws://192.168.1.100:8025/lsp).
    /// When set, the client connects to this URL instead of spawning a local Java process.
    #[serde(default)]
    pub remote_url: String,
}

impl Default for BSLServerSettings {
    fn default() -> Self {
        Self {
            executable_path: String::new(),
            installed_version: String::new(),
            workspace_path: String::new(),
            jar_path: String::new(),
            auto_download: true,
            websocket_port: 8025,
            java_path: "java".to_string(),
            enabled: true,
            remote_url: String::new(),
        }
    }
}

impl BSLServerSettings {
    /// Returns true if configured to use a remote server (not a local Java process).
    pub fn is_remote(&self) -> bool {
        !self.remote_url.is_empty()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    System,
    Disabled,
    Custom,
}

impl Default for ProxyMode {
    fn default() -> Self {
        Self::System
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyProtocol {
    Http,
    Socks5,
}

impl Default for ProxyProtocol {
    fn default() -> Self {
        Self::Http
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxySettings {
    #[serde(default)]
    pub mode: ProxyMode,
    #[serde(default)]
    pub protocol: ProxyProtocol,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

impl Default for ProxySettings {
    fn default() -> Self {
        Self {
            mode: ProxyMode::System,
            protocol: ProxyProtocol::Http,
            host: String::new(),
            port: None,
            username: String::new(),
            password: String::new(),
        }
    }
}

impl std::fmt::Debug for ProxySettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxySettings")
            .field("mode", &self.mode)
            .field("protocol", &self.protocol)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field(
                "password",
                &if self.password.is_empty() {
                    ""
                } else {
                    "<redacted>"
                },
            )
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    Http,
    Stdio,
    Internal,
}

/// Configuration for an MCP server (HTTP or Stdio)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub transport: McpTransport,
    // HTTP specific
    pub url: Option<String>,
    pub login: Option<String>,
    pub password: Option<String>,
    pub headers: Option<std::collections::HashMap<String, String>>,
    // Stdio specific
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<std::collections::HashMap<String, String>>,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "New MCP Server".to_string(),
            enabled: false,
            transport: McpTransport::Http,
            url: Some("http://localhost/mcp".to_string()),
            login: None,
            password: None,
            headers: None,
            command: None,
            args: None,
            env: None,
        }
    }
}

/// Enterprise mode config (loaded from enterprise.json next to EXE)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnterpriseConfig {
    /// Server URL (e.g. http://server:9224)
    pub server_url: String,
    /// API token for authentication (optional)
    #[serde(default)]
    pub token: Option<String>,
    /// Auto-update enabled
    #[serde(default = "default_enterprise_true")]
    pub auto_update: bool,
}

fn default_enterprise_true() -> bool {
    true
}

/// Get the enterprise config file path (next to executable)
pub fn get_enterprise_config_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    exe_dir.join("enterprise.json")
}

/// Load enterprise config from file next to EXE
pub fn load_enterprise_config() -> Option<EnterpriseConfig> {
    let path = get_enterprise_config_path();
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).ok(),
            Err(_) => None,
        }
    } else {
        None
    }
}

/// Remote enterprise config fetched from server
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemoteEnterpriseConfig {
    /// MCP server overrides (transport → http, url → server endpoint)
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
    /// BSL Language Server remote URL
    #[serde(default)]
    pub bsl_remote_url: String,
    /// Default LLM profile ID
    #[serde(default)]
    pub active_llm_profile: String,
    /// LLM providers config
    #[serde(default)]
    pub llm: LLMGlobalSettings,
    /// Theme override
    #[serde(default)]
    pub theme: Option<String>,
    /// Extra settings as raw JSON (merged into AppSettings)
    #[serde(default)]
    pub extra_settings: Option<serde_json::Value>,
}

/// Main application settings container
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    /// Enterprise server URL (set when enterprise config was applied, empty = not applied)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub enterprise_server_applied: String,
    pub configurator: ConfiguratorSettings,
    pub bsl_server: BSLServerSettings,
    /// Directory for mcp-1c-search SQLite index files. Empty means default app data path.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub search_index_dir: String,
    #[serde(default)]
    pub proxy: ProxySettings,
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
    pub active_llm_profile: String,
    pub llm: LLMGlobalSettings,
    #[serde(default)]
    pub debug_mode: bool,
    #[serde(default)]
    pub onboarding_completed: bool,
    /// Настройки пользовательских промптов
    #[serde(default)]
    pub custom_prompts: CustomPromptsSettings,
    /// Настройки генерации кода
    #[serde(default)]
    pub code_generation: CodeGenerationSettings,
    /// Быстрые команды
    #[serde(default = "default_slash_commands")]
    pub slash_commands: Vec<SlashCommand>,

    /// Максимальное количество итераций агента
    #[serde(default = "default_max_iterations")]
    pub max_agent_iterations: Option<u32>,

    /// Тема оформления (light / dark)
    #[serde(default)]
    pub theme: Option<String>,

    /// Стратегия сжатия контекста: "" / "sliding_window" / "summarize"
    #[serde(default = "default_compress_strategy")]
    pub context_compress_strategy: String,

    /// Порог сжатия в токенах (chars/4 эвристика, default 8000).
    /// Заменяет max_context_messages — сжатие теперь по токенам, а не по числу сообщений.
    #[serde(default)]
    pub max_context_tokens: Option<u32>,

    /// Устаревшее поле — сохранено для миграции старых конфигов.
    #[serde(default)]
    pub max_context_messages: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LLMGlobalSettings {
    pub active_provider_id: String,
    pub providers: std::collections::HashMap<String, ProviderSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettings {
    pub enabled: bool,
    pub api_key: Option<String>, // TODO: Encrypt this
    pub base_url: Option<String>,
    pub active_model_id: Option<String>,
    pub models: std::collections::HashMap<String, ModelSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSettings {
    pub context_window: Option<u32>, // Override
    pub cost_in: Option<f64>,
    pub cost_out: Option<f64>,
}

/// Режим генерации кода
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CodeGenerationMode {
    /// Всегда полный код
    Full,
    /// Только изменения в формате Search/Replace
    Diff,
    /// Автовыбор по размеру модуля
    Auto,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PromptBehaviorPreset {
    Project,
    Maintenance,
    Cli,
    Planning,
}

impl Default for PromptBehaviorPreset {
    fn default() -> Self {
        Self::Project
    }
}

// LabelingStyle больше не нужен, он зашит в пресет

impl Default for CodeGenerationMode {
    fn default() -> Self {
        Self::Diff
    }
}

/// Настройки генерации кода
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenerationSettings {
    /// Режим генерации
    #[serde(default)]
    pub mode: CodeGenerationMode,

    /// Пресет поведения
    #[serde(default)]
    pub behavior_preset: PromptBehaviorPreset,

    /// Спрашивать перед длительными операциями (режим Ask)
    #[serde(default)]
    pub ask_before_action: bool,

    /// Маркировать изменения
    #[serde(default = "default_true")]
    pub mark_changes: bool,

    /// Шаблон маркера для добавления (Maintenance)
    #[serde(default = "default_addition_marker")]
    pub addition_marker_template: String,

    /// Шаблон маркера для изменения (Maintenance)
    #[serde(default = "default_modification_marker")]
    pub modification_marker_template: String,

    /// Шаблон маркера для удаления (Maintenance)
    #[serde(default = "default_deletion_marker")]
    pub deletion_marker_template: String,
}

impl Default for CodeGenerationSettings {
    fn default() -> Self {
        Self {
            mode: CodeGenerationMode::Diff,
            behavior_preset: PromptBehaviorPreset::Project,
            ask_before_action: false,
            mark_changes: true,
            addition_marker_template: default_addition_marker(),
            modification_marker_template: default_modification_marker(),
            deletion_marker_template: default_deletion_marker(),
        }
    }
}

/// Шаблон промпта
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub content: String,
    #[serde(default)]
    pub enabled: bool,
}

/// Настройки пользовательских промптов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPromptsSettings {
    /// Префикс, добавляемый к system prompt
    #[serde(default)]
    pub system_prefix: String,

    /// Инструкции при изменении кода
    #[serde(default)]
    pub on_code_change: String,

    /// Инструкции при генерации нового кода
    #[serde(default)]
    pub on_code_generate: String,

    /// Пользовательские шаблоны промптов
    #[serde(default)]
    pub templates: Vec<PromptTemplate>,
}

fn default_custom_prompt_templates() -> Vec<PromptTemplate> {
    vec![
        PromptTemplate {
            id: "bsl-standards".to_string(),
            name: "Стандарты 1С".to_string(),
            description: "Соблюдать стандарты разработки 1С и БСП".to_string(),
            content:
                "Соблюдай стандарты разработки 1С и Библиотеки стандартных подсистем (БСП)."
                    .to_string(),
            enabled: false,
        },
        PromptTemplate {
            id: "bsl-syntax".to_string(),
            name: "Синтаксис 1С".to_string(),
            description: "Контролировать синтаксис 1С".to_string(),
            content: "Контролируй синтаксис 1С. Если пользователь прислал BSL-код или ты предлагаешь BSL-код, перед финальным ответом проверь синтаксис через доступную проверку BSL/check_bsl_syntax и явно сообщи результат. Если код содержит синтаксические ошибки, не утверждай, что он корректен.".to_string(),
            enabled: false,
        },
    ]
}

impl Default for CustomPromptsSettings {
    fn default() -> Self {
        Self {
            system_prefix: String::new(),
            on_code_change: String::new(),
            on_code_generate: String::new(),
            templates: default_custom_prompt_templates(),
        }
    }
}

fn ensure_default_custom_prompt_templates(settings: &mut AppSettings) -> bool {
    let defaults = default_custom_prompt_templates();
    let existing_ids: std::collections::HashSet<String> = settings
        .custom_prompts
        .templates
        .iter()
        .map(|template| template.id.clone())
        .collect();

    let missing_templates: Vec<PromptTemplate> = defaults
        .into_iter()
        .filter(|template| !existing_ids.contains(&template.id))
        .collect();

    if missing_templates.is_empty() {
        return false;
    }

    settings.custom_prompts.templates.extend(missing_templates);
    true
}

/// Ensure all built-in MCP servers are present in settings.
/// Called on every settings load; idempotent.
pub fn ensure_builtin_mcp_servers(settings: &mut AppSettings) -> bool {
    let mut modified = false;

    // builtin-mcp-skills (enabled by default — agent should see skills)
    if !settings.mcp_servers.iter().any(|s| s.id == "builtin-mcp-skills") {
        crate::app_log!("[SETTINGS] Adding builtin-mcp-skills server");
        settings.mcp_servers.push(McpServerConfig {
            id: "builtin-mcp-skills".to_string(),
            name: "Скиллы".to_string(),
            enabled: true,
            transport: McpTransport::Stdio,
            command: Some(rust_mcp_binary_name("mcp-1c-skills")),
            args: None,
            env: None,
            ..Default::default()
        });
        modified = true;
    }

    // builtin-1c-filesystem (disabled by default — user configures sandbox)
    if !settings.mcp_servers.iter().any(|s| s.id == "builtin-1c-filesystem") {
        crate::app_log!("[SETTINGS] Adding builtin-1c-filesystem server");
        settings.mcp_servers.push(McpServerConfig {
            id: "builtin-1c-filesystem".to_string(),
            name: "Файловая система (Sandbox)".to_string(),
            enabled: false,
            transport: McpTransport::Stdio,
            command: Some(rust_mcp_binary_name("mcp-1c-filesystem")),
            args: None,
            env: Some(std::collections::HashMap::from([(
                "MINI_AI_1C_SANDBOX_PATH".to_string(),
                String::new(),
            )])),
            ..Default::default()
        });
        modified = true;
    }

    // builtin-jvv-1c (enabled by default — platform detection + database list)
    if !settings.mcp_servers.iter().any(|s| s.id == "builtin-jvv-1c") {
        crate::app_log!("[SETTINGS] Adding builtin-jvv-1c server");
        settings.mcp_servers.push(McpServerConfig {
            id: "builtin-jvv-1c".to_string(),
            name: "1С:Платформа и базы".to_string(),
            enabled: true,
            transport: McpTransport::Stdio,
            command: Some(rust_mcp_binary_name("mcp-1c-jvv")),
            args: None,
            env: None,
            ..Default::default()
        });
        modified = true;
    }

    // builtin-1c-naparnik (enabled by default — 1C:AI consultant)
    if !settings.mcp_servers.iter().any(|s| s.id == "builtin-1c-naparnik") {
        crate::app_log!("[SETTINGS] Adding builtin-1c-naparnik server");
        settings.mcp_servers.push(McpServerConfig {
            id: "builtin-1c-naparnik".to_string(),
            name: "1C:Напарник".to_string(),
            enabled: true,
            transport: McpTransport::Stdio,
            command: Some(rust_mcp_binary_name("mcp-1c-naparnik")),
            args: None,
            env: Some(std::collections::HashMap::from([(
                "ONEC_AI_TOKEN".to_string(),
                String::new(),
            )])),
            ..Default::default()
        });
        modified = true;
    }

    // builtin-1c-help (disabled by default — needs 1C platform)
    if !settings.mcp_servers.iter().any(|s| s.id == "builtin-1c-help") {
        crate::app_log!("[SETTINGS] Adding builtin-1c-help server");
        settings.mcp_servers.push(McpServerConfig {
            id: "builtin-1c-help".to_string(),
            name: "1С:Справка".to_string(),
            enabled: false,
            transport: McpTransport::Stdio,
            command: Some(rust_mcp_binary_name("mcp-1c-help")),
            args: None,
            env: Some(std::collections::HashMap::from([(
                "ONEC_HELP_PATH".to_string(),
                String::new(),
            )])),
            ..Default::default()
        });
        modified = true;
    }

    // builtin-1c-metadata (disabled by default — needs 1C HTTP service)
    if !settings.mcp_servers.iter().any(|s| s.id == "builtin-1c-metadata") {
        crate::app_log!("[SETTINGS] Adding builtin-1c-metadata server");
        settings.mcp_servers.push(McpServerConfig {
            id: "builtin-1c-metadata".to_string(),
            name: "1C:Метаданные".to_string(),
            enabled: false,
            transport: McpTransport::Stdio,
            command: Some(rust_mcp_binary_name("mcp-1c-metadata")),
            args: None,
            env: Some(std::collections::HashMap::from([
                (
                    "ONEC_METADATA_URL".to_string(),
                    "http://localhost/base/hs/mcp".to_string(),
                ),
                ("ONEC_USERNAME".to_string(), String::new()),
                ("ONEC_PASSWORD".to_string(), String::new()),
            ])),
            ..Default::default()
        });
        modified = true;
    }

    modified
}

/// Платформо-зависимое имя Rust-бинарника MCP-сервера.
pub fn rust_mcp_binary_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{}.exe", base)
    } else {
        base.to_string()
    }
}

/// Проверяет, является ли команда запуска Rust-бинарником builtin-сервера.
fn is_rust_mcp_binary_command(command: &str) -> bool {
    let cmd = command.trim().replace('\\', "/").to_lowercase();
    [
        "mcp-1c-skills",
        "mcp-1c-jvv",
        "mcp-1c-filesystem",
        "mcp-1c-naparnik",
        "mcp-1c-help",
        "mcp-1c-metadata",
        "mcp-1c-search",
    ]
    .iter()
    .any(|base| {
        cmd.ends_with(base) || cmd.ends_with(&format!("{}.exe", base))
    })
}

pub fn clear_runtime_only_settings(settings: &mut AppSettings) -> bool {
    let had_binding = settings.configurator.selected_window_hwnd.is_some()
        || settings.configurator.selected_window_pid.is_some()
        || settings.configurator.selected_window_title.is_some()
        || settings.configurator.selected_config_name.is_some();

    settings.configurator.selected_window_hwnd = None;
    settings.configurator.selected_window_pid = None;
    settings.configurator.selected_window_title = None;
    settings.configurator.selected_config_name = None;

    had_binding
}

fn is_builtin_managed_mcp_server(server_id: &str) -> bool {
    matches!(
        server_id,
        "builtin-1c-naparnik" | "builtin-1c-metadata" | "builtin-1c-help" | "builtin-mcp-skills" | "builtin-1c-filesystem" | "builtin-jvv-1c"
    )
}

fn migrate_builtin_mcp_launchers(settings: &mut AppSettings) -> bool {
    let mut modified = false;

    for server in settings.mcp_servers.iter_mut() {
        if is_builtin_managed_mcp_server(&server.id) {
            // Миграция node + .cjs → Rust-бинарник
            let binary = rust_mcp_binary_name(rust_binary_base_for(&server.id));
            let current_cmd = server.command.as_deref().unwrap_or("");
            let is_node_launcher =
                crate::mcp_client::is_stdio_node_launcher_command(current_cmd)
                    || current_cmd.contains("node_modules");
            let is_already_binary = current_cmd.ends_with(&binary);

            if is_node_launcher || !is_already_binary {
                crate::app_log!(
                    "[SETTINGS] Migrating builtin server '{}' to binary '{}' (was '{}')",
                    server.id,
                    binary,
                    current_cmd
                );
                server.command = Some(binary.clone());
                server.args = None;
                modified = true;
            }
        } else if server.id == "builtin-1c-search" {
            let current_cmd = server.command.as_deref().unwrap_or("");
            let search_bin = crate::mcp_client::search_binary_name();
            if current_cmd != search_bin && !current_cmd.ends_with(search_bin) {
                crate::app_log!(
                    "[SETTINGS] Migrating builtin-1c-search command to '{}'",
                    search_bin
                );
                server.command = Some(search_bin.to_string());
                server.args = None;
                modified = true;
            }
        } else if let Some(cmd) = &server.command {
            if cmd.contains("node_modules") {
                crate::app_log!(
                    "[DEBUG] Migrating stale command '{}' to 'npx' for MCP server '{}'",
                    cmd,
                    server.id
                );
                server.command = Some("npx".to_string());
                modified = true;
            }
        }
    }

    modified
}

/// Базовое имя бинарника для builtin-сервера.
fn rust_binary_base_for(server_id: &str) -> &'static str {
    match server_id {
        "builtin-mcp-skills" => "mcp-1c-skills",
        "builtin-1c-filesystem" => "mcp-1c-filesystem",
        "builtin-jvv-1c" => "mcp-1c-jvv",
        "builtin-1c-naparnik" => "mcp-1c-naparnik",
        "builtin-1c-help" => "mcp-1c-help",
        "builtin-1c-metadata" => "mcp-1c-metadata",
        _ => "mcp-1c-skills",
    }
}

/// Get the settings directory path
pub fn get_settings_dir() -> PathBuf {
    // Use HOME/.config/mini-ai-1c/ for cross-platform consistency
    // On Windows: C:\Users\<user>\.config\mini-ai-1c\
    // On Linux:   /home/<user>/.config/mini-ai-1c/
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("mini-ai-1c")
}

/// Get the settings file path
pub fn get_settings_file() -> PathBuf {
    get_settings_dir().join("settings.json")
}

/// Load settings from file
pub fn load_settings() -> AppSettings {
    // Возвращаем закэшированный результат, если он есть.
    if let Ok(guard) = settings_cache().lock() {
        if let Some(cached) = guard.as_ref() {
            crate::logger::set_debug_mode(cached.debug_mode);
            return cached.clone();
        }
    }

    let path = get_settings_file();
    let mut settings = if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AppSettings::default(),
        }
    } else {
        AppSettings::default()
    };

    let mut modified = false;

    if clear_runtime_only_settings(&mut settings) {
        crate::app_log!(
            "[SETTINGS] Removing transient configurator window binding from persisted settings"
        );
        modified = true;
    }

    // Migration: debug_mcp -> debug_mode
    let path = get_settings_file();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&content) {
                if let Some(old_val) = map.get("debug_mcp") {
                    if !map.contains_key("debug_mode") {
                        if let Some(b) = old_val.as_bool() {
                            crate::app_log!(
                                "[SETTINGS] Migrating 'debug_mcp' ({}) to 'debug_mode'",
                                b
                            );
                            settings.debug_mode = b;
                            modified = true;
                        }
                    }
                }
            }
        }
    }

    if migrate_builtin_mcp_launchers(&mut settings) {
        modified = true;
    }

    // Migration: upgrade old window_title_pattern to include "1C:Enterprise" for English UI
    {
        let p = &settings.configurator.window_title_pattern;
        if p == "Конфигуратор" || p == "Конфигуратор|Configurator" {
            crate::app_log!(
                "[SETTINGS] Migrating window_title_pattern '{}' to bilingual default",
                p
            );
            settings.configurator.window_title_pattern =
                default_configurator_window_title_pattern();
            modified = true;
        }
    }

    // Migration: Force 'Diff' mode over 'Full' if detected (to fix AI interaction issues)
    if settings.code_generation.mode == CodeGenerationMode::Full {
        crate::app_log!("[SETTINGS] Migrating deprecated 'Full' mode to 'Diff'");
        settings.code_generation.mode = CodeGenerationMode::Diff;
        modified = true;
    }

    // Migration: ensure default slash commands exist
    if ensure_default_slash_commands(&mut settings) {
        modified = true;
    }

    // Migration: ensure default custom prompt templates exist
    if ensure_default_custom_prompt_templates(&mut settings) {
        modified = true;
    }

    // Migration: ensure all built-in MCP servers are present
    if ensure_builtin_mcp_servers(&mut settings) {
        modified = true;
    }

    let profile_store = crate::llm_profiles::load_profiles();
    if !profile_store.active_profile_id.is_empty()
        && settings.active_llm_profile != profile_store.active_profile_id
    {
        crate::app_log!(
            "[SETTINGS] Syncing legacy active_llm_profile '{}' -> '{}'",
            settings.active_llm_profile,
            profile_store.active_profile_id
        );
        settings.active_llm_profile = profile_store.active_profile_id;
        modified = true;
    }

    // Cleanup stale enterprise config: if enterprise_server_applied is set
    // but enterprise.json no longer exists, revert to defaults
    let enterprise_missing = !get_enterprise_config_path().exists();
    let had_enterprise = !settings.enterprise_server_applied.is_empty();

    if had_enterprise && enterprise_missing {
        crate::app_log!(
            force: true,
            "[SETTINGS] Cleaning stale enterprise config (server: {}, enterprise.json not found)",
            settings.enterprise_server_applied
        );
        // Remove HTTP-transport MCP servers that were set by enterprise mode
        let enterprise_ids: Vec<String> = settings
            .mcp_servers
            .iter()
            .filter(|s| {
                s.transport == McpTransport::Http
                    && s.url.as_deref().map_or(false, |u| {
                        u.contains(&settings.enterprise_server_applied)
                    })
            })
            .map(|s| s.id.clone())
            .collect();

        settings.mcp_servers.retain(|s| !enterprise_ids.contains(&s.id));

        // Clear BSL remote URL if it points to the enterprise server
        if settings.bsl_server.remote_url.contains(&settings.enterprise_server_applied) {
            settings.bsl_server.remote_url.clear();
        }

        settings.enterprise_server_applied.clear();
        modified = true;
    }

    // Migration: if enterprise.json doesn't exist but MCP servers still have
    // HTTP transport pointing to localhost URLs (from a previous enterprise
    // session before the marker was added), clean them up too
    if enterprise_missing && !had_enterprise {
        let stale_count = settings.mcp_servers.len();
        settings.mcp_servers.retain(|s| {
            let is_stale = s.transport == McpTransport::Http
                && s.url.as_deref().map_or(false, |u| u.contains("localhost"))
                && matches!(s.id.as_str(), "builtin-1c-search" | "builtin-1c-help" | "builtin-1c-naparnik" | "builtin-1c-metadata");
            !is_stale
        });
        if settings.mcp_servers.len() != stale_count {
            crate::app_log!(force: true, "[SETTINGS] Removed {} stale enterprise MCP servers (legacy cleanup)", stale_count - settings.mcp_servers.len());
            modified = true;
        }
    }

    if modified {
        let _ = save_settings(&settings);
    }

    crate::logger::set_debug_mode(settings.debug_mode);

    if let Ok(mut guard) = settings_cache().lock() {
        *guard = Some(settings.clone());
    }

    settings
}

/// Save settings to file
pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let dir = get_settings_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let path = get_settings_file();
    let mut persisted_settings = settings.clone();
    clear_runtime_only_settings(&mut persisted_settings);
    let content = serde_json::to_string_pretty(&persisted_settings).map_err(|e| e.to_string())?;

    crate::logger::set_debug_mode(settings.debug_mode);
    let result = fs::write(path, content).map_err(|e| e.to_string());

    // Инвалидируем кэш после записи — следующий load_settings прочитает свежее значение.
    invalidate_settings_cache();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_configurator_settings_deserialize_without_binding_fields() {
        let mut json = serde_json::to_value(AppSettings::default())
            .expect("default settings should serialize to json");

        let configurator = json["configurator"]
            .as_object_mut()
            .expect("configurator section should exist");
        configurator.insert(
            "window_title_pattern".to_string(),
            serde_json::Value::String("Конфигуратор".to_string()),
        );
        configurator.insert(
            "selected_window_hwnd".to_string(),
            serde_json::Value::Number(12345.into()),
        );
        configurator.remove("selected_window_pid");
        configurator.remove("selected_window_title");
        configurator.remove("selected_config_name");

        let settings: AppSettings =
            serde_json::from_value(json).expect("legacy settings should deserialize");

        assert_eq!(settings.configurator.selected_window_hwnd, Some(12345));
        assert_eq!(settings.configurator.selected_window_pid, None);
        assert_eq!(settings.configurator.selected_window_title, None);
        assert_eq!(settings.configurator.selected_config_name, None);
    }

    #[test]
    fn legacy_configurator_settings_enable_bridge_when_flag_missing() {
        let mut json = serde_json::to_value(AppSettings::default())
            .expect("default settings should serialize to json");

        let configurator = json["configurator"]
            .as_object_mut()
            .expect("configurator section should exist");
        configurator.remove("editor_bridge_enabled");
        configurator.remove("rdp_mode");

        let settings: AppSettings =
            serde_json::from_value(json).expect("legacy settings should deserialize");

        assert!(settings.configurator.editor_bridge_enabled);
        assert!(!settings.configurator.rdp_mode);
    }

    #[test]
    fn clear_runtime_only_settings_drops_configurator_binding() {
        let mut settings = AppSettings {
            configurator: ConfiguratorSettings {
                window_title_pattern: "Конфигуратор".to_string(),
                extra_window_title_patterns: Vec::new(),
                selected_window_hwnd: Some(777),
                selected_window_pid: Some(888),
                selected_window_title: Some("Конфигуратор - DemoBase".to_string()),
                selected_config_name: Some("DemoBase".to_string()),
                rdp_mode: false,
                editor_bridge_enabled: true,
                editor_bridge_auto_apply: false,
                editor_bridge_exe_path: String::new(),
            },
            ..AppSettings::default()
        };

        assert!(clear_runtime_only_settings(&mut settings));
        assert_eq!(settings.configurator.selected_window_hwnd, None);
        assert_eq!(settings.configurator.selected_window_pid, None);
        assert_eq!(settings.configurator.selected_window_title, None);
        assert_eq!(settings.configurator.selected_config_name, None);
        assert_eq!(settings.configurator.window_title_pattern, "Конфигуратор");
    }

    #[test]
    fn configurator_runtime_binding_is_not_serialized_when_cleared() {
        let mut settings = AppSettings::default();
        settings.configurator.selected_window_hwnd = Some(777);
        settings.configurator.selected_window_pid = Some(888);
        settings.configurator.selected_window_title = Some("Конфигуратор - DemoBase".to_string());
        settings.configurator.selected_config_name = Some("DemoBase".to_string());

        clear_runtime_only_settings(&mut settings);

        let serialized = serde_json::to_string(&settings).expect("settings should serialize");

        assert!(!serialized.contains("selected_window_hwnd"));
        assert!(!serialized.contains("selected_window_pid"));
        assert!(!serialized.contains("selected_window_title"));
        assert!(!serialized.contains("selected_config_name"));
        assert!(!serialized.contains("window_title_pattern"));
    }

    #[test]
    fn legacy_settings_deserialize_search_index_dir_to_empty() {
        let mut json = serde_json::to_value(AppSettings::default())
            .expect("default settings should serialize to json");
        json.as_object_mut()
            .expect("settings should be an object")
            .remove("search_index_dir");

        let settings: AppSettings =
            serde_json::from_value(json).expect("legacy settings should deserialize");

        assert_eq!(settings.search_index_dir, "");
    }

    #[test]
    fn legacy_bsl_settings_deserialize_with_native_defaults() {
        let mut json = serde_json::to_value(AppSettings::default())
            .expect("default settings should serialize to json");
        let bsl = json["bsl_server"]
            .as_object_mut()
            .expect("bsl_server section should exist");
        bsl.remove("executable_path");
        bsl.remove("installed_version");
        bsl.remove("workspace_path");

        let settings: AppSettings =
            serde_json::from_value(json).expect("legacy BSL settings should deserialize");

        assert_eq!(settings.bsl_server.executable_path, "");
        assert_eq!(settings.bsl_server.installed_version, "");
        assert_eq!(settings.bsl_server.workspace_path, "");
    }

    #[test]
    fn default_proxy_settings_use_system_mode() {
        let proxy = ProxySettings::default();

        assert_eq!(proxy.mode, ProxyMode::System);
        assert_eq!(proxy.protocol, ProxyProtocol::Http);
        assert_eq!(proxy.host, "");
        assert_eq!(proxy.port, None);
    }

    #[test]
    fn legacy_settings_deserialize_proxy_to_default_system() {
        let mut json = serde_json::to_value(AppSettings::default())
            .expect("default settings should serialize to json");
        json.as_object_mut()
            .expect("settings should be an object")
            .remove("proxy");

        let settings: AppSettings =
            serde_json::from_value(json).expect("legacy settings should deserialize");

        assert_eq!(settings.proxy.mode, ProxyMode::System);
        assert_eq!(settings.proxy.protocol, ProxyProtocol::Http);
    }

    #[test]
    fn proxy_settings_debug_does_not_expose_password() {
        let proxy = ProxySettings {
            mode: ProxyMode::Custom,
            protocol: ProxyProtocol::Http,
            host: "proxy.corp.local".to_string(),
            port: Some(8080),
            username: "user".to_string(),
            password: "very-secret-proxy-password".to_string(),
        };

        let debug = format!("{:?}", proxy);

        assert!(debug.contains("proxy.corp.local"));
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("very-secret-proxy-password"));
    }

    #[test]
    fn builtin_mcp_migrates_node_to_rust_binary() {
        let mut settings = AppSettings {
            mcp_servers: vec![McpServerConfig {
                id: "builtin-1c-naparnik".to_string(),
                name: "1C:Naparnik".to_string(),
                enabled: false,
                transport: McpTransport::Stdio,
                url: None,
                login: None,
                password: None,
                headers: None,
                command: Some("node".to_string()),
                args: Some(vec![
                    "--yes".to_string(),
                    "tsx".to_string(),
                    "src/mcp-servers/1c-naparnik.ts".to_string(),
                ]),
                env: None,
            }],
            ..AppSettings::default()
        };

        assert!(migrate_builtin_mcp_launchers(&mut settings));
        let server = &settings.mcp_servers[0];

        let expected_binary = rust_mcp_binary_name("mcp-1c-naparnik");
        assert_eq!(server.command.as_deref(), Some(expected_binary.as_str()));
        assert_eq!(server.args, None);
    }

    #[test]
    fn rust_mcp_binary_name_is_platform_aware() {
        #[cfg(windows)]
        assert_eq!(rust_mcp_binary_name("mcp-1c-skills"), "mcp-1c-skills.exe");
        #[cfg(not(windows))]
        assert_eq!(rust_mcp_binary_name("mcp-1c-skills"), "mcp-1c-skills");
    }

    #[test]
    fn rust_mcp_binary_command_detection() {
        assert!(is_rust_mcp_binary_command("mcp-1c-skills"));
        assert!(is_rust_mcp_binary_command(r"C:\tools\mcp-1c-help.exe"));
        assert!(!is_rust_mcp_binary_command("node"));
        assert!(!is_rust_mcp_binary_command("mcp-servers/mcp-skills.cjs"));
    }

    #[test]
    fn ensure_default_slash_commands_adds_missing_system_commands() {
        let mut settings = AppSettings::default();
        settings.slash_commands.retain(|cmd| cmd.id != "elaborate");

        assert!(ensure_default_slash_commands(&mut settings));
        assert!(settings
            .slash_commands
            .iter()
            .any(|cmd| cmd.id == "elaborate"));
    }

    #[test]
    fn ensure_default_custom_prompt_templates_adds_bsl_syntax_rule() {
        let mut settings = AppSettings::default();
        settings
            .custom_prompts
            .templates
            .retain(|template| template.id != "bsl-syntax");

        assert!(ensure_default_custom_prompt_templates(&mut settings));
        assert!(settings
            .custom_prompts
            .templates
            .iter()
            .any(|template| template.id == "bsl-syntax" && !template.enabled));
    }
}
