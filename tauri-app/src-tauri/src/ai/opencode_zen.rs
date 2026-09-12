//! OpenCode Zen session support for Custom/OpenAI-compatible profiles.
//!
//! Hosted Zen/Go routing requires `x-opencode-session`. Profiles may provide
//! that value manually through `extra_headers`; when it is absent, this module
//! creates and caches a Zen session automatically for compatible Zen base URLs.

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use crate::llm_profiles::LLMProfile;

pub const OPENCODE_SESSION_HEADER: &str = "x-opencode-session";
const OPENCODE_ZEN_SESSION_TTL: Duration = Duration::from_secs(30 * 60);
const OPENCODE_ZEN_SESSION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
struct CachedZenSession {
    session_id: String,
    expires_at: Instant,
}

static ZEN_SESSIONS: OnceLock<Mutex<HashMap<String, CachedZenSession>>> = OnceLock::new();

fn zen_session_cache() -> &'static Mutex<HashMap<String, CachedZenSession>> {
    ZEN_SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Whether a base URL points at hosted OpenCode Zen.
pub fn is_opencode_zen_url(url: &str) -> bool {
    url.to_ascii_lowercase().contains("opencode.ai/zen")
}

/// Derives a Zen API root from an OpenAI-compatible base URL.
///
/// `https://opencode.ai/zen/go/v1` becomes `https://opencode.ai/zen`.
pub fn zen_api_root(base_url: &str) -> String {
    let mut root = base_url.trim().trim_end_matches('/').to_string();
    let suffixes = [
        "/v1/chat/completions",
        "/chat/completions",
        "/v1/models",
        "/go/v1",
        "/go",
        "/v1",
    ];

    loop {
        let lower = root.to_ascii_lowercase();
        let mut shortened = false;
        for suffix in suffixes {
            if root.len() > suffix.len() && lower.ends_with(suffix) {
                root.truncate(root.len() - suffix.len());
                root = root.trim_end_matches('/').to_string();
                shortened = true;
                break;
            }
        }
        if !shortened {
            break;
        }
    }

    root
}

fn zen_session_endpoints(root: &str) -> Vec<String> {
    vec![
        format!("{root}/v1/sessions"),
        format!("{root}/sessions"),
        format!("{root}/api/sessions"),
    ]
}

fn zen_cache_key(base_url: &str, api_key: &str) -> String {
    format!("{}|{}", base_url.trim().trim_end_matches('/'), api_key.trim())
}

fn cached_zen_session_id(base_url: &str, api_key: &str) -> Option<String> {
    let key = zen_cache_key(base_url, api_key);
    let cache = zen_session_cache().lock().ok()?;
    let entry = cache.get(&key)?;
    if entry.expires_at > Instant::now() {
        Some(entry.session_id.clone())
    } else {
        None
    }
}

fn store_zen_session_id(base_url: &str, api_key: &str, session_id: String) {
    if let Ok(mut cache) = zen_session_cache().lock() {
        cache.insert(
            zen_cache_key(base_url, api_key),
            CachedZenSession {
                session_id,
                expires_at: Instant::now() + OPENCODE_ZEN_SESSION_TTL,
            },
        );
    }
}

/// Forgets a cached Zen session after a MissingSessionID response.
pub fn invalidate_zen_session(base_url: &str, api_key: &str) {
    if let Ok(mut cache) = zen_session_cache().lock() {
        cache.remove(&zen_cache_key(base_url, api_key));
    }
}

fn parse_zen_session_id(body: &serde_json::Value) -> Option<String> {
    for candidate in [
        body.get("id"),
        body.get("session_id"),
        body.get("sessionId"),
        body.get("session").and_then(|v| v.get("id")),
        body.get("session").and_then(|v| v.get("session_id")),
        body.get("data").and_then(|v| v.get("id")),
        body.get("data").and_then(|v| v.get("session_id")),
    ] {
        if let Some(id) = candidate.and_then(serde_json::Value::as_str) {
            let id = id.trim();
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    None
}

/// Whether an LLM error payload reports an unusable/absent OpenCode session.
pub fn is_missing_opencode_session_error(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("missing") && lower.contains("session")
        || lower.contains("missingsessionid")
        || lower.contains("invalid") && lower.contains("x-opencode-session")
        || lower.contains("expired") && lower.contains("x-opencode-session")
}

/// Returns a manually configured OpenCode session, if present.
pub fn configured_opencode_session_id(
    configured: Option<&HashMap<String, String>>,
) -> Option<String> {
    configured?.iter().find_map(|(name, value)| {
        if name.trim().eq_ignore_ascii_case(OPENCODE_SESSION_HEADER) {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_string())
        } else {
            None
        }
    })
}

async fn create_zen_session(base_url: &str, api_key: &str) -> Result<String, String> {
    let root = zen_api_root(base_url);
    let client = crate::http_client::build_http_client()?;
    let mut attempts = Vec::new();

    for endpoint in zen_session_endpoints(&root) {
        let mut request = client
            .post(&endpoint)
            .header(CONTENT_TYPE, "application/json")
            .timeout(OPENCODE_ZEN_SESSION_TIMEOUT)
            .json(&json!({
                "source": "mini-ai-1c",
                "client": "mini-ai-1c",
            }));
        if !api_key.trim().is_empty() {
            request = request.bearer_auth(api_key.trim());
        }

        match request.send().await {
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                if status.is_success() {
                    match serde_json::from_str::<serde_json::Value>(&body)
                        .ok()
                        .and_then(|json| parse_zen_session_id(&json))
                    {
                        Some(session_id) => return Ok(session_id),
                        None => attempts.push(format!(
                            "{endpoint}: HTTP {status} without a parsable session id"
                        )),
                    }
                } else {
                    attempts.push(format!(
                        "{endpoint}: HTTP {status} {}",
                        truncate_error_body(&body)
                    ));
                }
            }
            Err(error) => {
                attempts.push(format!("{endpoint}: request error {error}"));
            }
        }
    }

    Err(format!(
        "Не удалось создать OpenCode Zen-сессию ({}). \
         Укажите действующую сессию вручную в дополнительных заголовках Custom-профиля: {}.",
        attempts.join("; "),
        OPENCODE_SESSION_HEADER
    ))
}

fn truncate_error_body(body: &str) -> String {
    const LIMIT: usize = 500;
    let body = body.trim();
    if body.len() <= LIMIT {
        return body.to_string();
    }
    format!("{}…", body[..LIMIT].to_string())
}

/// Creates or reuses a Zen session and caches it for 30 minutes.
pub async fn ensure_zen_session_id(base_url: &str, api_key: &str) -> Result<String, String> {
    if let Some(session_id) = cached_zen_session_id(base_url, api_key) {
        return Ok(session_id);
    }

    let session_id = create_zen_session(base_url, api_key).await?;
    store_zen_session_id(base_url, api_key, session_id.clone());
    Ok(session_id)
}

/// Applies safe user-configured headers without overriding auth/content type.
pub fn apply_configured_headers(headers: &mut HeaderMap, configured: Option<&HashMap<String, String>>) {
    let Some(configured) = configured else {
        return;
    };

    for (name, value) in configured {
        let name = name.trim();
        let value = value.trim();
        if name.is_empty() || value.is_empty() {
            continue;
        }
        if name.eq_ignore_ascii_case("authorization") || name.eq_ignore_ascii_case("content-type") {
            continue;
        }
        let Ok(header_name) = name.parse::<HeaderName>() else {
            crate::app_log!("[Zen] Пропущен некорректный заголовок: {name}");
            continue;
        };
        let Ok(header_value) = HeaderValue::from_str(value) else {
            crate::app_log!("[Zen] Пропущено некорректное значение заголовка: {name}");
            continue;
        };
        headers.insert(header_name, header_value);
    }
}

async fn resolve_zen_session_id(
    base_url: &str,
    api_key: &str,
    configured: Option<&HashMap<String, String>>,
) -> Result<Option<String>, String> {
    if !is_opencode_zen_url(base_url) {
        return Ok(None);
    }
    if let Some(session_id) = configured_opencode_session_id(configured) {
        return Ok(Some(session_id));
    }

    Ok(Some(ensure_zen_session_id(base_url, api_key).await?))
}

/// Resolves profile headers, automatically supplying a Zen session when required.
pub async fn request_headers_for_profile(
    profile: &LLMProfile,
    api_key: &str,
) -> Result<HeaderMap, String> {
    request_headers_for_custom_endpoint(&profile.get_base_url(), api_key, profile.extra_headers.as_ref()).await
}

/// Resolves headers for an ad-hoc provider/base/key combination.
pub async fn request_headers_for_custom_endpoint(
    base_url: &str,
    api_key: &str,
    configured: Option<&HashMap<String, String>>,
) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    if !api_key.trim().is_empty() {
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key.trim()))
                .map_err(|e| format!("Invalid auth token: {e}"))?,
        );
    }

    apply_configured_headers(&mut headers, configured);
    if let Some(session_id) = resolve_zen_session_id(base_url, api_key, configured).await? {
        headers.insert(
            OPENCODE_SESSION_HEADER,
            HeaderValue::from_str(&session_id).map_err(|e| format!("Invalid Zen session: {e}"))?,
        );
    }

    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_opencode_zen_urls() {
        assert!(is_opencode_zen_url("https://opencode.ai/zen/go/v1"));
        assert!(is_opencode_zen_url("HTTPS://OPENCODE.AI/ZEN/GO/"));
        assert!(!is_opencode_zen_url("https://api.openai.com/v1"));
    }

    #[test]
    fn derives_zen_api_root() {
        assert_eq!(
            zen_api_root("https://opencode.ai/zen/go/v1"),
            "https://opencode.ai/zen"
        );
        assert_eq!(
            zen_api_root("https://opencode.ai/zen/go/v1/chat/completions"),
            "https://opencode.ai/zen"
        );
        assert_eq!(
            zen_api_root("https://opencode.ai/zen/"),
            "https://opencode.ai/zen"
        );
    }

    #[test]
    fn detects_missing_session_errors() {
        assert!(is_missing_opencode_session_error(
            r#"{"type":"MissingSessionID","message":"Request is missing x-opencode-session"}"#
        ));
        assert!(!is_missing_opencode_session_error("rate limited"));
    }

    #[test]
    fn finds_manual_session_without_network() {
        let configured = HashMap::from([(
            "X-OpenCode-Session".to_string(),
            "manual-session".to_string(),
        )]);
        assert_eq!(
            configured_opencode_session_id(Some(&configured)).as_deref(),
            Some("manual-session")
        );
        assert_eq!(configured_opencode_session_id(None), None);
    }

    #[test]
    fn skips_reserved_and_invalid_configured_headers() {
        let mut headers = HeaderMap::new();
        let configured = HashMap::from([
            ("Authorization".to_string(), "Bearer overridden".to_string()),
            ("X-Custom".to_string(), "custom-value".to_string()),
            ("Not A Header".to_string(), "ignored".to_string()),
        ]);
        apply_configured_headers(&mut headers, Some(&configured));
        assert!(headers.get(AUTHORIZATION).is_none());
        assert_eq!(
            headers.get("X-Custom").unwrap().to_str().unwrap(),
            "custom-value"
        );
    }
}
