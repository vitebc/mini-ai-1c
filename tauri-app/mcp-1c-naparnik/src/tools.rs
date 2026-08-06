//! Определения MCP-инструментов и их обработчики.
//!
//! Порт 1c-naparnik.ts: 3 инструмента — ask_1c_ai, explain_1c_syntax, check_1c_code.

use serde_json::{json, Value};
use std::sync::Arc;

use crate::naparnik;

pub struct AppState {
    pub client: reqwest::Client,
    pub token: String,
}

/// Возвращает список инструментов для tools/list.
pub fn list_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "ask_1c_ai",
            "description": "Задать вопрос ИИ-консультанту по платформе 1С, стандартам разработки и БСП",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "question": { "type": "string", "description": "Вопрос для модели 1С.ai" },
                    "programming_language": { "type": "string", "description": "Язык программирования (опционально)" },
                    "create_new_session": { "type": "boolean", "description": "Создать новую сессию для этого вопроса" }
                },
                "required": ["question"]
            }
        }),
        json!({
            "name": "explain_1c_syntax",
            "description": "Объяснить синтаксис конкретного метода, функции или встроенного объекта 1С",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "syntax_element": { "type": "string", "description": "Элемент синтаксиса или объект 1С для объяснения" },
                    "context": { "type": "string", "description": "Контекст использования" }
                },
                "required": ["syntax_element"]
            }
        }),
        json!({
            "name": "check_1c_code",
            "description": "Проверить фрагмент кода 1С на наличие логических ошибок, проблем производительности или соответствие стандартам",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "code": { "type": "string", "description": "Код 1С для проверки" },
                    "check_type": { "type": "string", "enum": ["syntax", "logic", "performance"], "description": "Тип проверки" }
                },
                "required": ["code"]
            }
        }),
    ]
}

fn ok_text(text: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": text
        }]
    })
}

fn err_text(msg: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": format!("Ошибка: {}", msg)
        }],
        "isError": true
    })
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

fn arg_bool(args: &Value, key: &str) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Обрабатывает tools/call. `state` — общий HTTP-клиент и токен.
pub async fn call_tool(name: &str, args: &Value, state: &Arc<AppState>) -> Result<Value, String> {
    match name {
        "ask_1c_ai" => {
            let question = arg_str(args, "question").ok_or("Question is required")?;
            let create_new = arg_bool(args, "create_new_session");
            match naparnik::ask(&state.client, &state.token, question, create_new).await {
                Ok(answer) => Ok(ok_text(&answer)),
                Err(e) => Ok(err_text(&e)),
            }
        }

        "explain_1c_syntax" => {
            let syntax_element = arg_str(args, "syntax_element").ok_or("syntax_element is required")?;
            let context = arg_str(args, "context").unwrap_or("");
            let question = format!(
                "Объясни синтаксис и использование: {}{}",
                syntax_element,
                if context.is_empty() {
                    String::new()
                } else {
                    format!(" в контексте: {}", context)
                }
            );
            match naparnik::ask(&state.client, &state.token, &question, false).await {
                Ok(answer) => Ok(ok_text(&answer)),
                Err(e) => Ok(err_text(&e)),
            }
        }

        "check_1c_code" => {
            let code = arg_str(args, "code").ok_or("Code is required")?;
            let check_type = arg_str(args, "check_type").unwrap_or("syntax");
            let desc = match check_type {
                "logic" => "логические ошибки и потенциальные проблемы",
                "performance" => "проблемы производительности и оптимизации",
                _ => "синтаксические ошибки",
            };
            let question = format!(
                "Проверь этот код 1С на {} и дай рекомендации:\n\n```bsl\n{}\n```",
                desc, code
            );
            match naparnik::ask(&state.client, &state.token, &question, false).await {
                Ok(answer) => Ok(ok_text(&answer)),
                Err(e) => Ok(err_text(&e)),
            }
        }

        _ => Err(format!("Unknown tool: {}", name)),
    }
}
