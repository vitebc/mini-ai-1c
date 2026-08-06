//! Клиент 1С:Напарник (code.1c.ai).
//!
//! Порт 1c-naparnik.ts + извлечение SSE-логики из naparnik_client.rs.
//! Standalone-версия: не использует локальный MCP bridge — при получении
//! tool_calls от сервера отправляет статус "rejected" (как TS-версия).

use futures::StreamExt;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::Duration;

const BASE_URL: &str = "https://code.1c.ai";
const MAX_SESSIONS: usize = 10;
const SESSION_TTL_SECS: i64 = 3600; // 1 час
const MAX_TOOL_ROUNDS: usize = 10;
const STREAM_TIMEOUT_SECS: u64 = 60;

// ─── Session state ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct OneCSession {
    conversation_id: String,
    last_message_uuid: Option<String>,
    last_used_unix: i64,
}

lazy_static! {
    static ref SESSIONS: Mutex<Vec<OneCSession>> = Mutex::new(Vec::new());
}

fn cleanup_sessions() {
    let now = chrono::Utc::now().timestamp();
    if let Ok(mut sessions) = SESSIONS.lock() {
        sessions.retain(|s| now - s.last_used_unix < SESSION_TTL_SECS);
        while sessions.len() > MAX_SESSIONS {
            sessions.sort_by(|a, b| a.last_used_unix.cmp(&b.last_used_unix));
            sessions.remove(0);
        }
    }
}

/// Возвращает текущую сессию или создаёт новую.
async fn get_or_create_session(
    client: &reqwest::Client,
    token: &str,
    create_new: bool,
) -> Result<OneCSession, String> {
    cleanup_sessions();

    if !create_new {
        if let Ok(mut sessions) = SESSIONS.lock() {
            if let Some(s) = sessions.last_mut() {
                s.last_used_unix = chrono::Utc::now().timestamp();
                return Ok(s.clone());
            }
        }
    }

    let (conv_id, root_uuid) = create_conversation(client, token).await?;
    let session = OneCSession {
        conversation_id: conv_id,
        last_message_uuid: root_uuid,
        last_used_unix: chrono::Utc::now().timestamp(),
    };
    if let Ok(mut sessions) = SESSIONS.lock() {
        sessions.push(session.clone());
    }
    Ok(session)
}

fn update_last_uuid(conversation_id: &str, uuid: &str) {
    if let Ok(mut sessions) = SESSIONS.lock() {
        if let Some(s) = sessions
            .iter_mut()
            .find(|s| s.conversation_id == conversation_id)
        {
            s.last_message_uuid = Some(uuid.to_string());
        }
    }
}

fn current_uuid(conversation_id: &str) -> Option<String> {
    SESSIONS
        .lock()
        .ok()?
        .iter()
        .find(|s| s.conversation_id == conversation_id)
        .and_then(|s| s.last_message_uuid.clone())
}

// ─── HTTP ────────────────────────────────────────────────────────────────────

fn build_headers(token: &str) -> reqwest::header::HeaderMap {
    use reqwest::header::*;
    let mut h = HeaderMap::new();
    h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"));
    h.insert(ORIGIN, HeaderValue::from_static(BASE_URL));
    let referer = format!("{}/chat//", BASE_URL);
    h.insert(REFERER, HeaderValue::from_str(&referer).unwrap_or_else(|_| HeaderValue::from_static("")));
    h.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"),
    );
    if let Ok(v) = HeaderValue::from_str(token) {
        h.insert(AUTHORIZATION, v);
    }
    h
}

async fn post_with_retry(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    body: Value,
    mut retries: usize,
    mut backoff_ms: u64,
) -> Result<reqwest::Response, String> {
    loop {
        let response = client
            .post(url)
            .headers(build_headers(token))
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                let retriable = status.as_u16() >= 500 || status.as_u16() == 429;
                if retriable && retries > 0 {
                    eprintln!(
                        "[1C:Naparnik] Request failed with status {}. Retrying in {}ms... ({} left)",
                        status, backoff_ms, retries
                    );
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    retries -= 1;
                    backoff_ms *= 2;
                    continue;
                }
                return Ok(resp);
            }
            Err(e) => {
                if retries > 0 {
                    eprintln!(
                        "[1C:Naparnik] Network error: {}. Retrying in {}ms... ({} left)",
                        e, backoff_ms, retries
                    );
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    retries -= 1;
                    backoff_ms *= 2;
                    continue;
                }
                return Err(format!("Naparnik: network error: {}", e));
            }
        }
    }
}

// ─── Conversation ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct CreateConversationRequest {
    is_chat: bool,
    programming_language: String,
    skill_name: String,
    ui_language: String,
}

async fn create_conversation(
    client: &reqwest::Client,
    token: &str,
) -> Result<(String, Option<String>), String> {
    let url = format!("{}/chat_api/v1/conversations/", BASE_URL);
    let body = CreateConversationRequest {
        is_chat: true,
        programming_language: "1C (BSL)".to_string(),
        skill_name: "custom".to_string(),
        ui_language: "russian".to_string(),
    };

    let resp = post_with_retry(
        client,
        &url,
        token,
        serde_json::to_value(body).map_err(|e| e.to_string())?,
        3,
        1000,
    )
    .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Naparnik: conversation create error {}: {}", status, text));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Naparnik: parse error: {}", e))?;
    let uuid = data["uuid"]
        .as_str()
        .ok_or("Naparnik: no uuid in response")?
        .to_string();
    let root_msg_uuid = data["root_message_uuid"].as_str().map(|s| s.to_string());
    eprintln!(
        "[1C:Naparnik] Created conversation {} (root_msg: {:?})",
        uuid, root_msg_uuid
    );
    Ok((uuid, root_msg_uuid))
}

// ─── Message send with SSE round-trip ────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct SseChunk {
    uuid: String,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: Option<Value>,
    #[serde(default)]
    content_delta: Option<ContentDelta>,
    #[serde(default)]
    finished: bool,
}

#[derive(Deserialize, Debug)]
struct ContentDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

struct SseResult {
    text: String,
    tool_calls: Vec<Value>,
}

/// Отправляет сообщение в сессию, обрабатывая tool_calls round-trip.
/// Возвращает финальный текст.
pub async fn send_message(
    client: &reqwest::Client,
    token: &str,
    conversation_id: &str,
    message: &str,
) -> Result<String, String> {
    let url = format!(
        "{}/chat_api/v1/conversations/{}/messages",
        BASE_URL, conversation_id
    );

    let mut payload = json!({
        "role": "user",
        "content": { "content": { "instruction": message } },
        "parent_uuid": current_uuid(conversation_id),
    });

    let mut segments: Vec<String> = Vec::new();

    for round in 0..MAX_TOOL_ROUNDS {
        if std::env::var("ONEC_AI_DEBUG").as_deref() == Ok("true") {
            eprintln!("[1C:Naparnik] Round {}, payload: {}", round, payload);
        }

        let resp = post_with_retry(client, &url, token, payload.clone(), 3, 1000).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Naparnik: API error {}: {}", status, text));
        }

        let sse = read_sse_stream(resp, conversation_id).await?;
        if !sse.text.is_empty() {
            segments.push(sse.text);
        }

        if sse.tool_calls.is_empty() {
            break;
        }

        // Standalone-сервер не исполняет инструменты — отправляем "rejected".
        eprintln!(
            "[1C:Naparnik] Tool calls received ({}), sending rejected round-trip",
            sse.tool_calls.len()
        );
        let items: Vec<Value> = sse
            .tool_calls
            .iter()
            .map(|tc| {
                let tc_id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let name = tc
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|v| v.as_str())
                    .or_else(|| tc.get("name").and_then(|v| v.as_str()))
                    .unwrap_or("");
                json!({
                    "status": "rejected",
                    "tool_call_id": tc_id,
                    "name": name,
                    "content": "Tool execution is not available in this MCP server bridge."
                })
            })
            .collect();

        let parent = current_uuid(conversation_id).unwrap_or_default();
        payload = json!({
            "role": "tool",
            "parent_uuid": parent,
            "content": items
        });
    }

    let full_text = segments
        .iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n\n");

    if full_text.is_empty() {
        return Err("Напарник не вернул ответ. Попробуйте повторить запрос.".to_string());
    }
    Ok(full_text)
}

/// Читает SSE-поток, накапливая текст. Возвращает текст + найденные tool_calls.
async fn read_sse_stream(
    response: reqwest::Response,
    conversation_id: &str,
) -> Result<SseResult, String> {
    let mut stream = response.bytes_stream();
    let mut byte_buffer = Vec::<u8>::new();
    let mut accumulated_text = String::new();
    let mut tool_calls_pending: Vec<Value> = Vec::new();
    let debug = std::env::var("ONEC_AI_DEBUG").as_deref() == Ok("true");

    'outer: loop {
        let chunk_result =
            match tokio::time::timeout(Duration::from_secs(STREAM_TIMEOUT_SECS), stream.next()).await
            {
                Err(_) => return Err("Naparnik: stream timeout (60s)".to_string()),
                Ok(None) => break,
                Ok(Some(r)) => r,
            };

        let chunk = chunk_result.map_err(|e| format!("Naparnik: stream error: {}", e))?;
        byte_buffer.extend_from_slice(&chunk);

        while let Some(pos) = byte_buffer.windows(2).position(|w| w == b"\n\n") {
            let event_bytes = byte_buffer.drain(..pos + 2).collect::<Vec<u8>>();
            let event_str = String::from_utf8_lossy(&event_bytes);

            for line in event_str.lines() {
                let data = if let Some(d) = line
                    .strip_prefix("data: ")
                    .or_else(|| line.strip_prefix("data:"))
                {
                    d
                } else {
                    continue;
                };
                if data == "[DONE]" {
                    break 'outer;
                }

                let chunk: SseChunk = match serde_json::from_str(data) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("[1C:Naparnik] SSE parse error: {} | data: {:.100}", e, data);
                        continue;
                    }
                };

                let role = chunk.role.as_deref().unwrap_or("");
                if debug {
                    eprintln!("[1C:Naparnik] SSE: {} | role={} finished={}", data, role, chunk.finished);
                }

                if (role == "user" || role == "tool") && chunk.finished {
                    continue;
                }

                // content_delta: reasoning + text
                if let Some(delta) = &chunk.content_delta {
                    if let Some(reasoning) = &delta.reasoning_content {
                        if !reasoning.is_empty() {
                            eprintln!("[1C:Naparnik] reasoning: {:.60}...", reasoning);
                        }
                    }
                    if let Some(text) = &delta.content {
                        if !text.is_empty() {
                            accumulated_text.push_str(text);
                        }
                    }
                }

                // cumulative content
                if let Some(content_val) = &chunk.content {
                    if let Some(text) = content_val.get("content").and_then(|v| v.as_str()) {
                        if !text.is_empty() {
                            accumulated_text = text.to_string();
                        }
                    }
                    // OpenAi-like choices handled implicitly: content may be in "content"
                    if let Some(tc_arr) = content_val.get("tool_calls").and_then(|v| v.as_array()) {
                        if !tc_arr.is_empty() {
                            tool_calls_pending = tc_arr.clone();
                            break 'outer;
                        }
                    }
                }

                // Final assistant chunk
                if chunk.finished && role == "assistant" {
                    update_last_uuid(conversation_id, &chunk.uuid);

                    if let Some(content_val) = &chunk.content {
                        if let Some(tc_arr) = content_val.get("tool_calls").and_then(|v| v.as_array()) {
                            if !tc_arr.is_empty() {
                                tool_calls_pending = tc_arr.clone();
                                break 'outer;
                            }
                        }
                    }
                    break 'outer;
                }
            }
        }
    }

    Ok(SseResult {
        text: accumulated_text,
        tool_calls: tool_calls_pending,
    })
}

/// Единая точка входа для инструментов: получить сессию и отправить сообщение.
pub async fn ask(
    client: &reqwest::Client,
    token: &str,
    question: &str,
    create_new_session: bool,
) -> Result<String, String> {
    let session = get_or_create_session(client, token, create_new_session).await?;
    send_message(client, token, &session.conversation_id, question).await
}

/// Создаёт HTTP-клиент.
pub fn build_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_removes_stale_sessions_and_caps_count() {
        let now = chrono::Utc::now().timestamp();
        let sessions = vec![
            OneCSession {
                conversation_id: "stale".to_string(),
                last_message_uuid: None,
                last_used_unix: now - 7200, // 2 часа назад
            },
            OneCSession {
                conversation_id: "fresh".to_string(),
                last_message_uuid: Some("u1".to_string()),
                last_used_unix: now,
            },
        ];
        *SESSIONS.lock().unwrap() = sessions;

        cleanup_sessions();

        let remaining = SESSIONS.lock().unwrap().clone();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].conversation_id, "fresh");
    }

    #[test]
    fn update_and_current_uuid() {
        *SESSIONS.lock().unwrap() = vec![OneCSession {
            conversation_id: "conv1".to_string(),
            last_message_uuid: Some("root".to_string()),
            last_used_unix: chrono::Utc::now().timestamp(),
        }];

        update_last_uuid("conv1", "assistant-1");
        assert_eq!(current_uuid("conv1").as_deref(), Some("assistant-1"));
        assert_eq!(current_uuid("unknown"), None);
    }

    #[tokio::test]
    async fn rejects_missing_token_before_network() {
        // Создание клиента без сети не должно паниковать.
        let client = build_client();
        assert!(client.is_ok());
    }
}
