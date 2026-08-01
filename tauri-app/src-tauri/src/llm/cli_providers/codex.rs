//! OpenAI Codex CLI provider — OAuth2+PKCE browser-redirect flow
//!
//! Auth flow:
//!   1. `auth_start` — generates PKCE, starts local callback server on port 1455,
//!      returns browser auth URL
//!   2. User opens URL in browser, authorises → browser redirects to localhost:1455/auth/callback
//!   3. `auth_poll` — checks if callback received; if so, exchanges code for tokens
//!   4. Frontend calls `cli_save_token` to persist tokens

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use lazy_static::lazy_static;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use super::{CliAuthInitResponse, CliAuthStatus, CliStatus, CliUsageWindow};

// ─── Constants ─────────────────────────────────────────────────────────────

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const CODEX_USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const CODEX_MODELS_URL: &str = "https://chatgpt.com/backend-api/codex/models";
const CODEX_USER_AGENT: &str = "codex_cli_rs/0.114.0 (Windows NT 10.0; x86_64) WindowsTerminal";
const REDIRECT_PORT: u16 = 1455;
const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const SCOPE: &str = "openid profile email offline_access";
const FIVE_HOUR_WINDOW_MINUTES: u32 = 5 * 60;
const WEEKLY_WINDOW_MINUTES: u32 = 7 * 24 * 60;
const FIVE_HOUR_WINDOW_SECONDS: i64 = (FIVE_HOUR_WINDOW_MINUTES as i64) * 60;
const WEEKLY_WINDOW_SECONDS: i64 = (WEEKLY_WINDOW_MINUTES as i64) * 60;

// ─── Callback State ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum CallbackResult {
    Pending,
    Success { code: String, state: Option<String> },
    Error(String),
}

lazy_static! {
    static ref CALLBACK: Mutex<CallbackResult> = Mutex::new(CallbackResult::Pending);
    static ref CALLBACK_SERVER: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>> =
        tokio::sync::Mutex::new(None);
    static ref TOKEN_REFRESH_LOCKS: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>> =
        Mutex::new(HashMap::new());
    static ref TOKEN_GENERATIONS: Mutex<HashMap<String, u64>> = Mutex::new(HashMap::new());
}

fn reset_callback() {
    if let Ok(mut cb) = CALLBACK.lock() {
        *cb = CallbackResult::Pending;
    }
}

fn set_callback(result: CallbackResult) {
    if let Ok(mut cb) = CALLBACK.lock() {
        *cb = result;
    }
}

fn read_callback() -> CallbackResult {
    CALLBACK
        .lock()
        .map(|cb| cb.clone())
        .unwrap_or(CallbackResult::Error("Lock error".to_string()))
}

fn token_refresh_lock(profile_id: &str) -> Arc<tokio::sync::Mutex<()>> {
    let mut locks = TOKEN_REFRESH_LOCKS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    locks
        .entry(profile_id.to_string())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

fn token_generations() -> std::sync::MutexGuard<'static, HashMap<String, u64>> {
    TOKEN_GENERATIONS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn token_generation(generations: &HashMap<String, u64>, profile_id: &str) -> u64 {
    generations.get(profile_id).copied().unwrap_or_default()
}

fn token_generation_matches(
    generations: &HashMap<String, u64>,
    profile_id: &str,
    expected_generation: u64,
) -> bool {
    token_generation(generations, profile_id) == expected_generation
}

fn advance_token_generation(generations: &mut HashMap<String, u64>, profile_id: &str) -> u64 {
    let generation = generations.entry(profile_id.to_string()).or_default();
    *generation = generation.wrapping_add(1);
    *generation
}

fn refresh_attempt_matches_current(
    current_access_token: &str,
    current_refresh_token: Option<&str>,
    attempted_access_token: &str,
    attempted_refresh_token: &str,
) -> bool {
    current_access_token == attempted_access_token
        && current_refresh_token == Some(attempted_refresh_token)
}

fn select_refresh_token<'a>(
    rotated_refresh_token: Option<&'a str>,
    existing_refresh_token: &'a str,
) -> &'a str {
    rotated_refresh_token.unwrap_or(existing_refresh_token)
}

// ─── PKCE Helpers ───────────────────────────────────────────────────────────

fn generate_code_verifier() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn generate_code_challenge(code_verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

fn random_state() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn safe_error_detail(body: &str) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(body).ok()?;

    parsed
        .get("error_description")
        .and_then(|value| value.as_str())
        .or_else(|| parsed.get("message").and_then(|value| value.as_str()))
        .or_else(|| parsed.get("error").and_then(|value| value.as_str()))
        .or_else(|| {
            parsed
                .get("error")
                .and_then(|value| value.get("message"))
                .and_then(|value| value.as_str())
        })
        .map(|value| value.replace(['\r', '\n'], " "))
}

fn safe_response_summary(status: reqwest::StatusCode, body: &str) -> String {
    format!("status={} body_len={}", status.as_u16(), body.len())
}

fn json_value_as_f32(value: Option<&serde_json::Value>) -> Option<f32> {
    match value? {
        serde_json::Value::Number(number) => number.as_f64().map(|value| value as f32),
        serde_json::Value::String(text) => text.trim().parse::<f32>().ok(),
        _ => None,
    }
}

fn json_value_as_i64(value: Option<&serde_json::Value>) -> Option<i64> {
    match value? {
        serde_json::Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_u64().and_then(|value| i64::try_from(value).ok())),
        serde_json::Value::String(text) => text.trim().parse::<i64>().ok(),
        _ => None,
    }
}

type UsageSnapshot = (Vec<CliUsageWindow>, Option<String>);

fn resolve_live_usage_result(result: Result<Option<UsageSnapshot>, String>) -> UsageSnapshot {
    match result {
        Ok(Some(snapshot)) => snapshot,
        Ok(None) => (Vec::new(), None),
        Err(error) => {
            crate::app_log!(
                force: true,
                "[Codex] Live usage unavailable; no cross-profile session fallback used: {}",
                error
            );
            (Vec::new(), None)
        }
    }
}

// ─── Callback HTTP Server ───────────────────────────────────────────────────

/// Starts a one-shot local HTTP server on port 1455 to receive the OAuth callback.
/// Stores the received code (or error) in CALLBACK global state.
async fn start_callback_server() -> Result<(), String> {
    let mut server = CALLBACK_SERVER.lock().await;
    if let Some(previous) = server.take() {
        previous.abort();
        let _ = previous.await;
    }
    reset_callback();

    let listener = TcpListener::bind(format!("127.0.0.1:{}", REDIRECT_PORT))
        .await
        .map_err(|error| {
            crate::app_log!(force: true, "[Codex] Failed to bind callback server on port {}: {}", REDIRECT_PORT, error);
            format!(
                "Не удалось запустить сервер авторизации (порт {} занят): {}",
                REDIRECT_PORT, error
            )
        })?;

    crate::app_log!(force: true, "[Codex] Callback server listening on port {}", REDIRECT_PORT);

    *server = Some(tokio::spawn(async move {
        match listener.accept().await {
            Ok((mut stream, _addr)) => {
                let mut reader = BufReader::new(&mut stream);
                let mut request_line = String::new();
                let _ = reader.read_line(&mut request_line).await;

                crate::app_log!(force: true, "[Codex] OAuth callback request received");

                // Parse: GET /auth/callback?code=...&state=... HTTP/1.1
                let callback_url = request_line.split_whitespace().nth(1).and_then(|path| {
                    let full = format!("http://localhost{}", path);
                    url::Url::parse(&full).ok()
                });
                let code = callback_url.as_ref().and_then(|u| {
                    u.query_pairs()
                        .find(|(k, _)| k == "code")
                        .map(|(_, v)| v.to_string())
                });
                let state = callback_url.as_ref().and_then(|u| {
                    u.query_pairs()
                        .find(|(k, _)| k == "state")
                        .map(|(_, v)| v.to_string())
                });

                let response_html = if code.is_some() {
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
                    <html><head><meta charset=\"utf-8\"></head><body style=\"font-family:sans-serif;text-align:center;padding:40px\">\
                    <h2>&#10003; Авторизация успешна!</h2>\
                    <p>Вернитесь в приложение Mini AI 1C.</p>\
                    <script>setTimeout(()=>window.close(),2000);</script>\
                    </body></html>"
                } else {
                    "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
                    <html><head><meta charset=\"utf-8\"></head><body style=\"font-family:sans-serif;text-align:center;padding:40px\">\
                    <h2>&#10007; Ошибка авторизации</h2>\
                    <p>Код авторизации не получен. Попробуйте снова.</p>\
                    </body></html>"
                };

                let _ = stream.write_all(response_html.as_bytes()).await;
                let _ = stream.flush().await;

                match code {
                    Some(c) => {
                        crate::app_log!(force: true, "[Codex] Auth code received (len={})", c.len());
                        set_callback(CallbackResult::Success { code: c, state });
                    }
                    None => {
                        set_callback(CallbackResult::Error(
                            "No authorization code in callback".to_string(),
                        ));
                    }
                }
            }
            Err(e) => {
                crate::app_log!(force: true, "[Codex] Callback server accept error: {}", e);
                set_callback(CallbackResult::Error(format!(
                    "Ошибка сервера авторизации: {}",
                    e
                )));
            }
        }
    }));

    Ok(())
}

// ─── Token exchange ─────────────────────────────────────────────────────────

async fn exchange_code(code: &str, code_verifier: &str) -> Result<CliAuthStatus, String> {
    let client = crate::http_client::http_client_builder()?
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let params = [
        ("client_id", CLIENT_ID),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
        ("code_verifier", code_verifier),
    ];

    let resp = client
        .post(TOKEN_URL)
        .form(&params)
        .header("Accept", "application/json")
        .header(
            "User-Agent",
            "codex_cli_rs/0.114.0 (Windows NT 10.0; x86_64)",
        )
        .send()
        .await
        .map_err(|e| format!("Ошибка сети при обмене кода: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    crate::app_log!(
        force: true,
        "[Codex] Token exchange response: {}",
        safe_response_summary(status, &body)
    );

    if !status.is_success() {
        return Ok(CliAuthStatus::Error(format!(
            "Ошибка получения токена ({}): {}",
            status.as_u16(),
            safe_error_detail(&body).unwrap_or_else(|| "подробности скрыты".to_string())
        )));
    }

    let data: CodexTokenResponse =
        serde_json::from_str(&body).map_err(|e| format!("Ошибка разбора ответа токена: {}", e))?;

    let expires_at = Utc::now() + Duration::seconds(data.expires_in.unwrap_or(3600) as i64);

    // Extract account_id from id_token for ChatGPT-Account-Id header
    crate::app_log!(force: true, "[Codex] id_token present: {}", data.id_token.is_some());
    let account_id = data
        .id_token
        .as_deref()
        .and_then(extract_account_id_from_id_token);

    Ok(CliAuthStatus::Authorized {
        access_token: data.access_token,
        refresh_token: data.refresh_token,
        expires_at: expires_at.timestamp() as u64,
        resource_url: account_id, // repurposed: stores ChatGPT account_id
    })
}

// ─── Provider ──────────────────────────────────────────────────────────────

pub struct CodexCliProvider;

// ─── Rate Limit Types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CodexSessionRateLimit {
    used_percent: f32,
    window_minutes: u32,
    resets_at: Option<i64>,
    resets_in_seconds: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CodexSessionRateLimits {
    primary: Option<CodexSessionRateLimit>,
    secondary: Option<CodexSessionRateLimit>,
    plan_type: Option<String>,
}

impl CodexCliProvider {
    // ── Auth ─────────────────────────────────────────────────────────────────

    pub async fn auth_start() -> Result<CliAuthInitResponse, String> {
        let code_verifier = generate_code_verifier();
        let code_challenge = generate_code_challenge(&code_verifier);
        let state = random_state();

        crate::app_log!(force: true, "[Codex] auth_start: PKCE challenge ready, starting callback server...");

        // Start callback server before returning URL
        start_callback_server().await?;

        // Build browser auth URL
        let params: Vec<(&str, &str)> = vec![
            ("client_id", CLIENT_ID),
            ("response_type", "code"),
            ("redirect_uri", REDIRECT_URI),
            ("scope", SCOPE),
            ("code_challenge", &code_challenge),
            ("code_challenge_method", "S256"),
            ("state", &state),
            ("codex_cli_simplified_flow", "true"),
            ("id_token_add_organizations", "true"),
            ("originator", "codex_cli_rs"),
        ];

        let query = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let auth_url = format!("{}?{}", AUTH_URL, query);

        crate::app_log!(force: true, "[Codex] auth_start: auth URL ready");

        Ok(CliAuthInitResponse {
            device_code: state,       // repurposed as session identifier
            user_code: String::new(), // not used in browser redirect flow
            verification_url: auth_url,
            expires_in: 300,
            poll_interval: 2,
            code_verifier: Some(code_verifier),
        })
    }

    pub async fn auth_poll(
        device_code: &str,
        code_verifier: Option<&str>,
    ) -> Result<CliAuthStatus, String> {
        match read_callback() {
            CallbackResult::Pending => Ok(CliAuthStatus::Pending),
            CallbackResult::Error(e) => Ok(CliAuthStatus::Error(e)),
            CallbackResult::Success { code, state } => {
                let verifier = code_verifier.unwrap_or("");
                if verifier.is_empty() {
                    return Ok(CliAuthStatus::Error("PKCE verifier missing".to_string()));
                }
                if state.as_deref() != Some(device_code) {
                    crate::app_log!(
                        force: true,
                        "[Codex] auth_poll: callback state mismatch (has_state={})",
                        state.is_some()
                    );
                    return Ok(CliAuthStatus::Error(
                        "Состояние OAuth-сессии не совпало. Повторите вход через браузер."
                            .to_string(),
                    ));
                }
                crate::app_log!(force: true, "[Codex] auth_poll: exchanging code for token...");
                exchange_code(&code, verifier).await
            }
        }
    }

    // ── Token storage (file-based AES-GCM, avoids Windows Credential Manager size limit) ──

    fn token_file_path(profile_id: &str) -> std::path::PathBuf {
        crate::settings::get_settings_dir().join(format!("codex-token-{}.dat", profile_id))
    }

    fn build_usage_window_identity(window_minutes: u32) -> (String, String) {
        match window_minutes {
            FIVE_HOUR_WINDOW_MINUTES => ("5h".to_string(), "5ч".to_string()),
            WEEKLY_WINDOW_MINUTES => ("weekly".to_string(), "7д".to_string()),
            other => (format!("{}m", other), format!("{}м", other)),
        }
    }

    fn resolve_rate_limit_reset_at(
        event_timestamp: Option<&DateTime<Utc>>,
        rate_limit: &CodexSessionRateLimit,
    ) -> Option<String> {
        rate_limit
            .resets_at
            .and_then(|timestamp| {
                DateTime::<Utc>::from_timestamp(timestamp, 0).map(|dt| dt.to_rfc3339())
            })
            .or_else(|| {
                rate_limit.resets_in_seconds.and_then(|seconds| {
                    event_timestamp
                        .cloned()
                        .map(|timestamp| timestamp + Duration::seconds(seconds))
                        .map(|dt| dt.to_rfc3339())
                })
            })
    }

    fn build_usage_window(
        event_timestamp: Option<&DateTime<Utc>>,
        rate_limit: &CodexSessionRateLimit,
    ) -> CliUsageWindow {
        let used_percent = rate_limit.used_percent.clamp(0.0, 100.0);
        let remaining_percent = (100.0 - used_percent).clamp(0.0, 100.0);
        let (key, label) = Self::build_usage_window_identity(rate_limit.window_minutes);

        CliUsageWindow {
            key,
            label,
            used_percent,
            remaining_percent,
            window_minutes: rate_limit.window_minutes,
            resets_at: Self::resolve_rate_limit_reset_at(event_timestamp, rate_limit),
        }
    }

    fn resolve_api_rate_limit_reset_at(rate_limit: &CodexApiUsageWindow) -> Option<String> {
        json_value_as_i64(rate_limit.reset_at.as_ref())
            .and_then(|timestamp| DateTime::<Utc>::from_timestamp(timestamp, 0))
            .map(|dt| dt.to_rfc3339())
            .or_else(|| {
                json_value_as_i64(rate_limit.reset_after_seconds.as_ref())
                    .map(|seconds| Utc::now() + Duration::seconds(seconds))
                    .map(|dt| dt.to_rfc3339())
            })
    }

    fn build_usage_window_from_api(
        rate_limit: &CodexApiRateLimit,
        window: &CodexApiUsageWindow,
    ) -> Option<CliUsageWindow> {
        let used_percent = json_value_as_f32(window.used_percent.as_ref())
            .or_else(|| {
                if rate_limit.limit_reached.unwrap_or(false)
                    || matches!(rate_limit.allowed, Some(false))
                {
                    Some(100.0)
                } else {
                    None
                }
            })?
            .clamp(0.0, 100.0);

        let window_seconds = json_value_as_i64(window.limit_window_seconds.as_ref())?;
        if window_seconds <= 0 {
            return None;
        }
        let window_minutes = u32::try_from(window_seconds / 60).ok()?;

        let (key, label) = Self::build_usage_window_identity(window_minutes);
        Some(CliUsageWindow {
            key,
            label,
            used_percent,
            remaining_percent: (100.0 - used_percent).clamp(0.0, 100.0),
            window_minutes,
            resets_at: Self::resolve_api_rate_limit_reset_at(window),
        })
    }

    fn classify_api_usage_windows(
        rate_limit: &CodexApiRateLimit,
    ) -> (Option<&CodexApiUsageWindow>, Option<&CodexApiUsageWindow>) {
        let primary = rate_limit.primary_window.as_ref();
        let secondary = rate_limit.secondary_window.as_ref();

        let mut five_hour_window = [primary, secondary].into_iter().flatten().find(|window| {
            json_value_as_i64(window.limit_window_seconds.as_ref())
                == Some(FIVE_HOUR_WINDOW_SECONDS)
        });
        let mut weekly_window = [primary, secondary].into_iter().flatten().find(|window| {
            json_value_as_i64(window.limit_window_seconds.as_ref()) == Some(WEEKLY_WINDOW_SECONDS)
        });

        if five_hour_window.is_none() {
            five_hour_window = primary.or(secondary);
        }
        if weekly_window.is_none() {
            weekly_window = secondary
                .filter(|window| {
                    five_hour_window.map_or(true, |chosen| !std::ptr::eq(*window, chosen))
                })
                .or_else(|| {
                    primary.filter(|window| {
                        five_hour_window.map_or(true, |chosen| !std::ptr::eq(*window, chosen))
                    })
                });
        }

        (five_hour_window, weekly_window)
    }

    fn extract_usage_snapshot_from_api_payload(
        payload: CodexApiUsagePayload,
    ) -> Option<(Vec<CliUsageWindow>, Option<String>)> {
        let rate_limit = payload.rate_limit?;
        let mut windows = [
            rate_limit.primary_window.as_ref(),
            rate_limit.secondary_window.as_ref(),
        ]
        .into_iter()
        .flatten()
        .filter_map(|window| Self::build_usage_window_from_api(&rate_limit, window))
        .collect::<Vec<_>>();

        if windows.is_empty() {
            return None;
        }

        windows.sort_by_key(|window| window.window_minutes);
        windows.dedup_by_key(|window| window.window_minutes);
        Some((windows, payload.plan_type))
    }

    async fn send_usage_request(
        access_token: &str,
        account_id: &str,
    ) -> Result<(reqwest::StatusCode, String), String> {
        let client = crate::http_client::http_client_builder()?
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        let response = client
            .get(CODEX_USAGE_URL)
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Chatgpt-Account-Id", account_id)
            .header("User-Agent", CODEX_USER_AGENT)
            .send()
            .await
            .map_err(|e| format!("Ошибка сети при получении лимитов Codex: {}", e))?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Ok((status, body))
    }

    async fn fetch_usage_snapshot_from_api(
        profile_id: &str,
    ) -> Result<Option<(Vec<CliUsageWindow>, Option<String>)>, String> {
        let token_info = Self::get_token(profile_id)?;
        let (mut access_token, refresh_token, _expires_at, mut account_id) = match token_info {
            Some(token_info) => token_info,
            None => return Ok(None),
        };

        let account_id_value = account_id.clone().ok_or_else(|| {
            "Отсутствует ChatGPT account id для запроса лимитов Codex".to_string()
        })?;

        let (mut status, mut body) =
            Self::send_usage_request(&access_token, &account_id_value).await?;
        crate::app_log!(
            force: true,
            "[Codex] Usage response: {}",
            safe_response_summary(status, &body)
        );

        if status.as_u16() == 401 {
            if let Some(refresh_token) = refresh_token.as_deref() {
                crate::app_log!(
                    force: true,
                    "[Codex] Usage API returned 401, attempting token refresh for profile {}",
                    profile_id
                );
                Self::refresh_access_token(profile_id, &access_token, refresh_token).await?;
                if let Some((new_access_token, _, _, new_account_id)) = Self::get_token(profile_id)?
                {
                    access_token = new_access_token;
                    account_id = new_account_id.or(account_id);
                    let refreshed_account_id = account_id.ok_or_else(|| {
                        "Отсутствует ChatGPT account id после обновления токена Codex".to_string()
                    })?;
                    let (retried_status, retried_body) =
                        Self::send_usage_request(&access_token, &refreshed_account_id).await?;
                    status = retried_status;
                    body = retried_body;
                    crate::app_log!(
                        force: true,
                        "[Codex] Usage response after refresh: {}",
                        safe_response_summary(status, &body)
                    );
                }
            }
        }

        if !status.is_success() {
            return Err(format!(
                "Ошибка API лимитов Codex ({}): {}",
                status.as_u16(),
                safe_error_detail(&body).unwrap_or_else(|| "подробности скрыты".to_string())
            ));
        }

        let payload: CodexApiUsagePayload = serde_json::from_str(&body)
            .map_err(|e| format!("Ошибка разбора ответа лимитов Codex: {}", e))?;
        Ok(Self::extract_usage_snapshot_from_api_payload(payload))
    }

    async fn resolve_usage_snapshot(
        profile_id: &str,
    ) -> Result<(Vec<CliUsageWindow>, Option<String>), String> {
        Ok(resolve_live_usage_result(
            Self::fetch_usage_snapshot_from_api(profile_id).await,
        ))
    }

    pub fn save_token(
        profile_id: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: u64,
        resource_url: Option<&str>, // repurposed: stores ChatGPT account_id
    ) -> Result<(), String> {
        let mut generations = token_generations();
        Self::write_token(
            profile_id,
            access_token,
            refresh_token,
            expires_at,
            resource_url,
        )?;
        advance_token_generation(&mut generations, profile_id);
        crate::app_log!(force: true, "[Codex] Token saved for profile {}, expires_at={}", profile_id, expires_at);
        Ok(())
    }

    fn write_token(
        profile_id: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: u64,
        resource_url: Option<&str>,
    ) -> Result<(), String> {
        let data = serde_json::json!({
            "access_token": access_token,
            "refresh_token": refresh_token,
            "expires_at": expires_at,
            "account_id": resource_url,
        })
        .to_string();
        let encrypted = crate::crypto::encrypt_string(&data).map_err(|e| e.to_string())?;
        let path = Self::token_file_path(profile_id);
        std::fs::write(&path, encrypted)
            .map_err(|e| format!("Не удалось записать токен: {}", e))?;
        Ok(())
    }

    fn get_token_snapshot(
        profile_id: &str,
    ) -> Result<(Option<(String, Option<String>, u64, Option<String>)>, u64), String> {
        let generations = token_generations();
        let generation = token_generation(&generations, profile_id);
        let token = Self::get_token(profile_id)?;
        Ok((token, generation))
    }

    fn save_token_if_generation(
        profile_id: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: u64,
        resource_url: Option<&str>,
        expected_generation: u64,
    ) -> Result<bool, String> {
        let mut generations = token_generations();
        if !token_generation_matches(&generations, profile_id, expected_generation) {
            return Ok(false);
        }
        Self::write_token(
            profile_id,
            access_token,
            refresh_token,
            expires_at,
            resource_url,
        )?;
        advance_token_generation(&mut generations, profile_id);
        crate::app_log!(force: true, "[Codex] Refreshed token saved for profile {}, expires_at={}", profile_id, expires_at);
        Ok(true)
    }

    /// Returns `(access_token, refresh_token, expires_at, account_id)`
    pub fn get_token(
        profile_id: &str,
    ) -> Result<Option<(String, Option<String>, u64, Option<String>)>, String> {
        let path = Self::token_file_path(profile_id);
        if !path.exists() {
            return Ok(None);
        }
        let encrypted = std::fs::read_to_string(&path)
            .map_err(|e| format!("Не удалось прочитать токен: {}", e))?;
        let decrypted = crate::crypto::decrypt_string(&encrypted)
            .map_err(|e| format!("Не удалось расшифровать токен: {}", e))?;
        let data: serde_json::Value = serde_json::from_str(&decrypted)
            .map_err(|e| format!("Ошибка разбора токена: {}", e))?;
        let access_token = data["access_token"]
            .as_str()
            .ok_or("No access_token in storage")?
            .to_string();
        let refresh_token = data["refresh_token"].as_str().map(|s| s.to_string());
        let expires_at = data["expires_at"]
            .as_u64()
            .ok_or("No expires_at in storage")?;
        let account_id = data["account_id"].as_str().map(|s| s.to_string());
        Ok(Some((access_token, refresh_token, expires_at, account_id)))
    }

    pub async fn refresh_access_token(
        profile_id: &str,
        attempted_access_token: &str,
        attempted_refresh_token: &str,
    ) -> Result<(), String> {
        let refresh_lock = token_refresh_lock(profile_id);
        let _refresh_guard = refresh_lock.lock().await;

        let (current_token, token_generation) = Self::get_token_snapshot(profile_id)?;
        let current_token = current_token
            .ok_or_else(|| "Токен Codex был удалён до начала обновления".to_string())?;
        if !refresh_attempt_matches_current(
            &current_token.0,
            current_token.1.as_deref(),
            attempted_access_token,
            attempted_refresh_token,
        ) {
            crate::app_log!(
                force: true,
                "[Codex] Token refresh skipped for profile {}: credentials already changed",
                profile_id
            );
            return Ok(());
        }

        let client = crate::http_client::http_client_builder()?
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        let params = [
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", attempted_refresh_token),
        ];

        let resp = client
            .post(TOKEN_URL)
            .form(&params)
            .header("Accept", "application/json")
            .header(
                "User-Agent",
                "codex_cli_rs/0.114.0 (Windows NT 10.0; x86_64)",
            )
            .send()
            .await
            .map_err(|e| format!("Ошибка сети при обновлении токена: {}", e))?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        crate::app_log!(
            force: true,
            "[Codex] Token refresh response: {}",
            safe_response_summary(status, &body)
        );

        if !status.is_success() {
            return Err(format!(
                "Обновление токена: ошибка {}: {}",
                status.as_u16(),
                safe_error_detail(&body).unwrap_or_else(|| "подробности скрыты".to_string())
            ));
        }

        let data: CodexTokenResponse = serde_json::from_str(&body)
            .map_err(|e| format!("Ошибка разбора ответа refresh: {}", e))?;

        let expires_at = Utc::now() + Duration::seconds(data.expires_in.unwrap_or(3600) as i64);
        let account_id = data
            .id_token
            .as_deref()
            .and_then(extract_account_id_from_id_token)
            .or(current_token.3);
        let refresh_token_to_store =
            select_refresh_token(data.refresh_token.as_deref(), attempted_refresh_token);
        if !Self::save_token_if_generation(
            profile_id,
            &data.access_token,
            Some(refresh_token_to_store),
            expires_at.timestamp() as u64,
            account_id.as_deref(),
            token_generation,
        )? {
            crate::app_log!(
                force: true,
                "[Codex] Token refresh result discarded for profile {}: credentials changed while request was in flight",
                profile_id
            );
            return Ok(());
        }

        crate::app_log!(force: true, "[Codex] Token refreshed for profile {}, expires_in={}s", profile_id, data.expires_in.unwrap_or(0));
        Ok(())
    }

    pub fn logout(profile_id: &str) -> Result<(), String> {
        let mut generations = token_generations();
        let path = Self::token_file_path(profile_id);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| format!("Не удалось удалить токен: {}", e))?;
        }
        advance_token_generation(&mut generations, profile_id);
        Ok(())
    }

    // ── Status ───────────────────────────────────────────────────────────────

    pub async fn get_status(profile_id: &str) -> Result<CliStatus, String> {
        let token_info = Self::get_token(profile_id)?;
        match token_info {
            None => Ok(CliStatus {
                is_authenticated: false,
                auth_expires_at: None,
                usage: None,
                usage_windows: None,
                usage_plan: None,
            }),
            Some((access_token, refresh_token, expires_at, _account_id)) => {
                let is_expired = Utc::now().timestamp() as u64 > expires_at;

                if is_expired {
                    if let Some(rt) = refresh_token.as_deref() {
                        crate::app_log!(force: true, "[Codex] get_status: token expired, attempting silent refresh for profile {}", profile_id);
                        match Self::refresh_access_token(profile_id, &access_token, rt).await {
                            Ok(()) => {
                                if let Ok(Some((_, _, new_exp, _))) = Self::get_token(profile_id) {
                                    let (usage_windows, usage_plan) =
                                        Self::resolve_usage_snapshot(profile_id).await?;
                                    let expires_str =
                                        chrono::DateTime::<Utc>::from_timestamp(new_exp as i64, 0)
                                            .map(|dt| dt.to_rfc3339());
                                    return Ok(CliStatus {
                                        is_authenticated: true,
                                        auth_expires_at: expires_str,
                                        usage: None,
                                        usage_windows: if usage_windows.is_empty() {
                                            None
                                        } else {
                                            Some(usage_windows)
                                        },
                                        usage_plan,
                                    });
                                }
                            }
                            Err(e) => {
                                crate::app_log!(force: true, "[Codex] get_status: silent refresh failed: {}", e);
                            }
                        }
                    }
                    return Ok(CliStatus {
                        is_authenticated: false,
                        auth_expires_at: None,
                        usage: None,
                        usage_windows: None,
                        usage_plan: None,
                    });
                }

                let (usage_windows, usage_plan) = Self::resolve_usage_snapshot(profile_id).await?;
                let expires_str = chrono::DateTime::<Utc>::from_timestamp(expires_at as i64, 0)
                    .map(|dt| dt.to_rfc3339());
                Ok(CliStatus {
                    is_authenticated: true,
                    auth_expires_at: expires_str,
                    usage: None,
                    usage_windows: if usage_windows.is_empty() {
                        None
                    } else {
                        Some(usage_windows)
                    },
                    usage_plan,
                })
            }
        }
    }

    fn parse_models_response(body: &str) -> Result<Vec<crate::llm::providers::Model>, String> {
        let payload: CodexModelsResponse = serde_json::from_str(body).map_err(|error| {
            format!("Не удалось разобрать официальный каталог Codex: {}", error)
        })?;

        let mut models = payload
            .models
            .into_iter()
            .filter_map(|model| {
                if model.visibility != "list" || !model.slug.starts_with("gpt-") {
                    return None;
                }
                let context_window = model.context_window?;

                Some(crate::llm::providers::Model {
                    id: model.slug,
                    name: model.display_name,
                    context_window,
                    description: model.description,
                    cost_in: None,
                    cost_out: None,
                    default_reasoning_effort: model.default_reasoning_level,
                    supported_reasoning_efforts: model
                        .supported_reasoning_levels
                        .into_iter()
                        .map(|level| level.effort)
                        .collect(),
                    priority: Some(model.priority),
                    supported_in_api: model.supported_in_api,
                })
            })
            .collect::<Vec<_>>();

        models.sort_by_key(|model| model.priority.unwrap_or(u32::MAX));
        Ok(models)
    }

    async fn send_models_request(
        access_token: &str,
        account_id: &str,
    ) -> Result<(reqwest::StatusCode, String), String> {
        let client = crate::http_client::http_client_builder()?
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|error| error.to_string())?;
        let response = client
            .get(CODEX_MODELS_URL)
            .query(&[("client_version", env!("CARGO_PKG_VERSION"))])
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("ChatGPT-Account-Id", account_id)
            .header("Originator", "codex-cli")
            .header("User-Agent", CODEX_USER_AGENT)
            .send()
            .await
            .map_err(|error| format!("Ошибка сети при загрузке каталога Codex: {}", error))?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Ok((status, body))
    }

    async fn fetch_live_models(
        profile_id: &str,
    ) -> Result<Vec<crate::llm::providers::Model>, String> {
        let (mut access_token, refresh_token, _, mut account_id) = Self::get_token(profile_id)?
            .ok_or_else(|| "Для профиля Codex отсутствует OAuth-токен".to_string())?;
        let current_account_id = account_id
            .clone()
            .ok_or_else(|| "Для профиля Codex отсутствует ChatGPT account id".to_string())?;

        let (mut status, mut body) =
            Self::send_models_request(&access_token, &current_account_id).await?;
        crate::app_log!(
            force: true,
            "[Codex] Models response: {}",
            safe_response_summary(status, &body)
        );

        if status.as_u16() == 401 {
            if let Some(refresh_token) = refresh_token.as_deref() {
                Self::refresh_access_token(profile_id, &access_token, refresh_token).await?;
                if let Some((new_access_token, _, _, new_account_id)) = Self::get_token(profile_id)?
                {
                    access_token = new_access_token;
                    account_id = new_account_id.or(account_id);
                    let refreshed_account_id = account_id.ok_or_else(|| {
                        "После обновления токена отсутствует ChatGPT account id".to_string()
                    })?;
                    (status, body) =
                        Self::send_models_request(&access_token, &refreshed_account_id).await?;
                    crate::app_log!(
                        force: true,
                        "[Codex] Models response after refresh: {}",
                        safe_response_summary(status, &body)
                    );
                }
            }
        }

        if !status.is_success() {
            return Err(format!(
                "Официальный каталог Codex вернул HTTP {}",
                status.as_u16()
            ));
        }

        let models = Self::parse_models_response(&body)?;
        if models.is_empty() {
            return Err("Официальный каталог Codex не содержит доступных GPT-моделей".to_string());
        }
        Ok(models)
    }

    /// Loads the official Codex catalog for the authenticated ChatGPT account.
    /// Falls back to a source-verified snapshot when the endpoint is unavailable.
    pub async fn fetch_models(
        profile_id: &str,
    ) -> Result<Vec<crate::llm::providers::Model>, String> {
        match Self::fetch_live_models(profile_id).await {
            Ok(models) => Ok(models),
            Err(error) => {
                crate::app_log!(
                    force: true,
                    "[Codex] Live model catalog unavailable; using verified fallback: {}",
                    error
                );
                Ok(crate::llm::providers::static_codex_models())
            }
        }
    }
}

// ─── Serde types ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CodexTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CodexModelsResponse {
    models: Vec<CodexApiModel>,
}

#[derive(Debug, Deserialize)]
struct CodexApiModel {
    slug: String,
    display_name: String,
    description: Option<String>,
    default_reasoning_level: Option<String>,
    #[serde(default)]
    supported_reasoning_levels: Vec<CodexReasoningLevel>,
    visibility: String,
    supported_in_api: Option<bool>,
    priority: u32,
    context_window: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct CodexReasoningLevel {
    effort: String,
}

#[derive(Debug, Deserialize)]
struct CodexApiUsagePayload {
    #[serde(alias = "planType")]
    plan_type: Option<String>,
    #[serde(alias = "rateLimit")]
    rate_limit: Option<CodexApiRateLimit>,
}

#[derive(Debug, Deserialize)]
struct CodexApiRateLimit {
    allowed: Option<bool>,
    #[serde(alias = "limitReached")]
    limit_reached: Option<bool>,
    #[serde(alias = "primaryWindow")]
    primary_window: Option<CodexApiUsageWindow>,
    #[serde(alias = "secondaryWindow")]
    secondary_window: Option<CodexApiUsageWindow>,
}

#[derive(Debug, Deserialize)]
struct CodexApiUsageWindow {
    #[serde(alias = "usedPercent")]
    used_percent: Option<serde_json::Value>,
    #[serde(alias = "limitWindowSeconds")]
    limit_window_seconds: Option<serde_json::Value>,
    #[serde(alias = "resetAfterSeconds")]
    reset_after_seconds: Option<serde_json::Value>,
    #[serde(alias = "resetAt")]
    reset_at: Option<serde_json::Value>,
}

/// Extract account_id (ChatGPT workspace) from id_token JWT claims.
/// JWT = header.payload.signature — we decode the payload (base64url → JSON).
fn extract_account_id_from_id_token(id_token: &str) -> Option<String> {
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() < 2 {
        crate::app_log!(force: true, "[Codex] id_token has {} parts, expected >=3", parts.len());
        return None;
    }
    // base64url may need padding
    let payload_b64 = parts[1];
    let payload = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .or_else(|_| {
            use base64::engine::general_purpose::URL_SAFE;
            URL_SAFE.decode(payload_b64)
        })
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    crate::app_log!(
        force: true,
        "[Codex] id_token claims parsed successfully: has_auth_claim={}",
        claims.get("https://api.openai.com/auth").is_some()
    );
    // OpenAI id_token: "https://api.openai.com/auth" → { "chatgpt_account_id": "..." }
    let auth_claim = claims.get("https://api.openai.com/auth");
    let account_id = auth_claim
        .and_then(|auth| {
            auth.get("chatgpt_account_id")
                .or_else(|| auth.get("account_id"))
        })
        .or_else(|| claims.get("chatgpt_account_id"))
        .or_else(|| claims.get("account_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    crate::app_log!(
        force: true,
        "[Codex] account_id extracted: {}",
        account_id.is_some()
    );
    account_id
}

#[cfg(test)]
mod tests {
    use super::{
        advance_token_generation, read_callback, refresh_attempt_matches_current,
        resolve_live_usage_result, select_refresh_token, set_callback, token_generation_matches,
        CallbackResult, CodexApiUsagePayload, CodexCliProvider, CALLBACK_SERVER,
    };
    use std::collections::HashMap;

    static AUTH_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[tokio::test(flavor = "current_thread")]
    async fn restarting_auth_replaces_previous_callback_listener() {
        let _test_guard = AUTH_TEST_LOCK.lock().expect("auth test lock");

        CodexCliProvider::auth_start()
            .await
            .expect("first OAuth listener should start");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        CodexCliProvider::auth_start()
            .await
            .expect("restarted OAuth listener should replace the previous listener");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        assert!(
            matches!(read_callback(), CallbackResult::Pending),
            "restarting OAuth must not report that its own callback port is occupied"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn restarting_auth_discards_callback_written_before_lifecycle_lock() {
        let _test_guard = AUTH_TEST_LOCK.lock().expect("auth test lock");

        CodexCliProvider::auth_start()
            .await
            .expect("first OAuth listener should start");
        let lifecycle_guard = CALLBACK_SERVER.lock().await;
        let restart = tokio::spawn(CodexCliProvider::auth_start());
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        set_callback(CallbackResult::Success {
            code: "stale-code".to_string(),
            state: Some("stale-state".to_string()),
        });
        drop(lifecycle_guard);
        restart
            .await
            .expect("restart task should complete")
            .expect("restarted OAuth listener should start");

        assert!(
            matches!(read_callback(), CallbackResult::Pending),
            "new OAuth session must reset callback state after stopping the previous listener"
        );
    }

    #[test]
    fn refresh_attempt_only_matches_unchanged_credentials() {
        assert!(refresh_attempt_matches_current(
            "access-old",
            Some("refresh-old"),
            "access-old",
            "refresh-old",
        ));
        assert!(!refresh_attempt_matches_current(
            "access-new",
            Some("refresh-new"),
            "access-old",
            "refresh-old",
        ));
        assert!(!refresh_attempt_matches_current(
            "access-old",
            None,
            "access-old",
            "refresh-old",
        ));
    }

    #[test]
    fn refresh_token_uses_rotated_value_or_preserves_existing_value() {
        assert_eq!(
            select_refresh_token(Some("refresh-new"), "refresh-old"),
            "refresh-new"
        );
        assert_eq!(select_refresh_token(None, "refresh-old"), "refresh-old");
    }

    #[test]
    fn credential_generation_invalidates_an_in_flight_refresh_snapshot() {
        let mut generations = HashMap::new();
        let refresh_generation = 0;

        assert!(token_generation_matches(
            &generations,
            "profile-race",
            refresh_generation,
        ));
        advance_token_generation(&mut generations, "profile-race");
        assert!(!token_generation_matches(
            &generations,
            "profile-race",
            refresh_generation,
        ));
    }

    #[test]
    fn missing_or_failed_live_usage_does_not_reuse_cross_profile_session_data() {
        let missing = resolve_live_usage_result(Ok(None));
        let failed = resolve_live_usage_result(Err("offline".to_string()));

        assert!(missing.0.is_empty());
        assert!(missing.1.is_none());
        assert!(failed.0.is_empty());
        assert!(failed.1.is_none());
    }

    #[test]
    fn parse_models_response_keeps_only_listed_gpt_models_with_source_metadata() {
        let models = CodexCliProvider::parse_models_response(
            r#"{
                "models": [
                    {
                        "slug": "gpt-5.6-sol",
                        "display_name": "GPT-5.6-Sol",
                        "description": "Latest frontier agentic coding model.",
                        "default_reasoning_level": "low",
                        "supported_reasoning_levels": [
                            {"effort": "low", "description": "Fast"},
                            {"effort": "max", "description": "Deep"},
                            {"effort": "ultra", "description": "Orchestrated"}
                        ],
                        "visibility": "list",
                        "supported_in_api": true,
                        "priority": 1,
                        "context_window": 272000
                    },
                    {
                        "slug": "gpt-hidden",
                        "display_name": "Hidden",
                        "visibility": "hide",
                        "priority": 2,
                        "context_window": 272000
                    },
                    {
                        "slug": "not-gpt",
                        "display_name": "Other",
                        "visibility": "list",
                        "priority": 3,
                        "context_window": 128000
                    },
                    {
                        "slug": "gpt-without-context",
                        "display_name": "No context",
                        "visibility": "list",
                        "priority": 4
                    }
                ]
            }"#,
        )
        .expect("official response should parse");

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "gpt-5.6-sol");
        assert_eq!(models[0].name, "GPT-5.6-Sol");
        assert_eq!(models[0].context_window, 272_000);
        assert_eq!(
            models[0].description.as_deref(),
            Some("Latest frontier agentic coding model.")
        );
        assert_eq!(models[0].default_reasoning_effort.as_deref(), Some("low"));
        assert_eq!(
            models[0].supported_reasoning_efforts,
            vec!["low", "max", "ultra"]
        );
        assert_eq!(models[0].priority, Some(1));
        assert_eq!(models[0].supported_in_api, Some(true));
    }

    #[test]
    fn extract_usage_snapshot_from_api_payload_uses_live_windows() {
        let payload: CodexApiUsagePayload = serde_json::from_str(
            r#"{
                "plan_type": "plus",
                "rate_limit": {
                    "primary_window": {
                        "used_percent": 53,
                        "limit_window_seconds": 18000,
                        "reset_at": 1775051447
                    },
                    "secondary_window": {
                        "used_percent": "16",
                        "limit_window_seconds": "604800",
                        "reset_after_seconds": 60
                    }
                }
            }"#,
        )
        .unwrap();

        let (windows, plan_type) =
            CodexCliProvider::extract_usage_snapshot_from_api_payload(payload).unwrap();

        assert_eq!(plan_type.as_deref(), Some("plus"));
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].key, "5h");
        assert_eq!(windows[0].used_percent, 53.0);
        assert_eq!(windows[0].remaining_percent, 47.0);
        assert_eq!(windows[1].key, "weekly");
        assert_eq!(windows[1].used_percent, 16.0);
        assert_eq!(windows[1].remaining_percent, 84.0);
    }

    #[test]
    fn extract_usage_snapshot_from_api_payload_deduplicates_identical_weekly_windows() {
        let payload: CodexApiUsagePayload = serde_json::from_str(
            r#"{
                "plan_type": "prolite",
                "rate_limit": {
                    "primary_window": {
                        "used_percent": 9,
                        "limit_window_seconds": 604800,
                        "reset_at": 1785689529
                    },
                    "secondary_window": {
                        "used_percent": 9,
                        "limit_window_seconds": 604800,
                        "reset_at": 1785689529
                    }
                }
            }"#,
        )
        .unwrap();

        let (windows, plan_type) =
            CodexCliProvider::extract_usage_snapshot_from_api_payload(payload).unwrap();

        assert_eq!(plan_type.as_deref(), Some("prolite"));
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].key, "weekly");
        assert_eq!(windows[0].remaining_percent, 91.0);
    }

    #[test]
    fn extract_usage_snapshot_from_api_payload_ignores_windows_without_source_duration() {
        let payload: CodexApiUsagePayload = serde_json::from_str(
            r#"{
                "plan_type": "pro",
                "rate_limit": {
                    "primary_window": {
                        "used_percent": 20,
                        "reset_at": 1775051447
                    },
                    "secondary_window": {
                        "used_percent": 40,
                        "reset_at": 1775656247
                    }
                }
            }"#,
        )
        .unwrap();

        assert!(CodexCliProvider::extract_usage_snapshot_from_api_payload(payload).is_none());
    }
}
