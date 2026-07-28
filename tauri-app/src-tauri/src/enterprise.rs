use crate::http_client::http_client_builder;
use crate::settings::{EnterpriseConfig, McpServerConfig, McpTransport, RemoteEnterpriseConfig};
use serde_json::Value;
use std::collections::HashMap;

/// Fetch remote config from enterprise server and merge into local settings
pub async fn fetch_and_merge(
    config: &EnterpriseConfig,
    local_settings: &mut crate::settings::AppSettings,
) -> Result<bool, String> {
    fetch_and_merge_config(&config.server_url, config.token.as_deref(), local_settings).await
}

/// Fetch remote config using server URL and optional token
pub async fn fetch_and_merge_config(
    server_url: &str,
    token: Option<&str>,
    local_settings: &mut crate::settings::AppSettings,
) -> Result<bool, String> {
    let url = format!("{}/api/client/config", server_url.trim_end_matches('/'));

    let client = http_client_builder()
        .map_err(|e| format!("HTTP client init: {}", e))?
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client build: {}", e))?;

    let mut req = client.get(&url);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let resp = req.send().await.map_err(|e| format!("Request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Server returned {}", resp.status()));
    }

    let remote: RemoteEnterpriseConfig = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;

    let enterprise_config = EnterpriseConfig {
        server_url: server_url.to_string(),
        token: token.map(|s| s.to_string()),
        auto_update: true,
    };
    merge_remote_config(local_settings, &remote, &enterprise_config);
    Ok(true)
}

fn merge_remote_config(
    local: &mut crate::settings::AppSettings,
    remote: &RemoteEnterpriseConfig,
    enterprise: &EnterpriseConfig,
) {
    let base_url = enterprise.server_url.trim_end_matches('/').to_string();
    local.enterprise_server_applied = enterprise.server_url.clone();

    // Override MCP servers: switch to HTTP transport pointing at server
    if !remote.mcp_servers.is_empty() {
        local.mcp_servers = remote
            .mcp_servers
            .iter()
            .map(|s| McpServerConfig {
                id: s.id.clone(),
                name: s.name.clone(),
                enabled: s.enabled,
                transport: McpTransport::Http,
                url: Some(format!("{}/api/mcp/{}", base_url, s.id)),
                login: s.login.clone(),
                password: s.password.clone(),
                headers: {
                    let mut h = s.headers.clone().unwrap_or_default();
                    if let Some(ref token) = enterprise.token {
                        h.insert("Authorization".into(), format!("Bearer {}", token));
                    }
                    Some(h)
                },
                command: None,
                args: None,
                env: None,
            })
            .collect();
    }

    // Override BSL LS remote URL
    if !remote.bsl_remote_url.is_empty() {
        local.bsl_server.remote_url = remote.bsl_remote_url.clone();
    }

    // Override LLM profile
    if !remote.active_llm_profile.is_empty() {
        local.active_llm_profile = remote.active_llm_profile.clone();
    }

    // Override LLM providers
    if !remote.llm.active_provider_id.is_empty() {
        local.llm = remote.llm.clone();
    }

    // Override theme
    if let Some(ref theme) = remote.theme {
        local.theme = Some(theme.clone());
    }

    // Merge extra settings (deep merge of JSON values)
    if let Some(ref extra) = remote.extra_settings {
        if let Ok(local_value) = serde_json::to_value(&*local) {
            let merged = deep_merge(local_value, extra.clone());
            if let Ok(merged_settings) = serde_json::from_value(merged) {
                *local = merged_settings;
            }
        }
    }
}

/// Deep merge two JSON values (b overrides a)
fn deep_merge(a: Value, b: Value) -> Value {
    match (a, b) {
        (Value::Object(mut a_map), Value::Object(b_map)) => {
            for (k, b_val) in b_map {
                if let Some(a_val) = a_map.remove(&k) {
                    a_map.insert(k, deep_merge(a_val, b_val));
                } else {
                    a_map.insert(k, b_val);
                }
            }
            Value::Object(a_map)
        }
        (_, b_val) => b_val,
    }
}

/// Build MCP server configs pointing to enterprise server
pub fn build_enterprise_mcp_configs(
    server_url: &str,
    token: Option<&str>,
) -> Vec<McpServerConfig> {
    let base_url = server_url.trim_end_matches('/').to_string();
    let mut headers = HashMap::new();
    if let Some(t) = token {
        headers.insert("Authorization".into(), format!("Bearer {}", t));
    }

    // These match the server's built-in MCP server IDs
    let server_ids = vec![
        "builtin-1c-search",
        "builtin-1c-help",
        "builtin-1c-naparnik",
        "builtin-1c-metadata",
    ];

    server_ids
        .into_iter()
        .enumerate()
        .map(|(i, id)| McpServerConfig {
            id: id.to_string(),
            name: format!("Enterprise MCP #{}", i + 1),
            enabled: true,
            transport: McpTransport::Http,
            url: Some(format!("{}/api/mcp/{}", base_url, id)),
            login: None,
            password: None,
            headers: {
                let h = if headers.is_empty() { None } else { Some(headers.clone()) };
                h
            },
            command: None,
            args: None,
            env: None,
        })
        .collect()
}
