use crate::enterprise;
use crate::settings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EnterpriseStatus {
    pub enabled: bool,
    pub server_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub available: bool,
    pub version: Option<String>,
    pub url: Option<String>,
    pub changelog: Option<String>,
}

/// Get enterprise mode status
#[tauri::command]
pub fn get_enterprise_status() -> EnterpriseStatus {
    let config = settings::load_enterprise_config();
    EnterpriseStatus {
        enabled: config.is_some(),
        server_url: config
            .as_ref()
            .map(|c| c.server_url.clone())
            .unwrap_or_default(),
    }
}

/// Fetch config from enterprise server and merge into local settings
#[tauri::command]
pub async fn fetch_enterprise_config() -> Result<bool, String> {
    let config = settings::load_enterprise_config().ok_or("No enterprise config found")?;
    let mut local_settings = settings::load_settings();

    let merged = enterprise::fetch_and_merge(&config, &mut local_settings).await?;

    if merged {
        settings::save_settings(&local_settings).map_err(|e| format!("Save failed: {}", e))?;
    }

    Ok(merged)
}

/// Check for client updates on the enterprise server
#[tauri::command]
pub async fn check_for_updates() -> Result<UpdateCheckResult, String> {
    let enterprise_config =
        settings::load_enterprise_config().ok_or("No enterprise config found")?;

    let current_version = env!("CARGO_PKG_VERSION");
    let url = format!(
        "{}/api/updater/check?version={}",
        enterprise_config.server_url.trim_end_matches('/'),
        current_version
    );

    let client = crate::http_client::http_client_builder()
        .map_err(|e| format!("HTTP client: {}", e))?
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client build: {}", e))?;

    let mut req = client.get(&url);
    if let Some(ref token) = enterprise_config.token {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let resp = req.send().await.map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        return Ok(UpdateCheckResult {
            available: false,
            version: None,
            url: None,
            changelog: None,
        });
    }

    resp.json::<UpdateCheckResult>()
        .await
        .map_err(|e| format!("Parse failed: {}", e))
}

/// Download and apply client update
#[tauri::command]
pub async fn download_update(version: String, url: String) -> Result<String, String> {
    let enterprise_config =
        settings::load_enterprise_config().ok_or("No enterprise config found")?;

    let client = crate::http_client::http_client_builder()
        .map_err(|e| format!("HTTP client: {}", e))?
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Client build: {}", e))?;

    let mut req = client.get(&url);
    if let Some(ref token) = enterprise_config.token {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let resp = req.send().await.map_err(|e| format!("Download failed: {}", e))?;

    let bytes = resp.bytes().await.map_err(|e| format!("Read failed: {}", e))?;

    // Save to temp directory
    let temp_dir = std::env::temp_dir().join(format!("mini-ai-1c-update-{}", version));
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let zip_path = temp_dir.join("update.zip");
    std::fs::write(&zip_path, &bytes).map_err(|e| e.to_string())?;

    // Extract
    let extract_dir = temp_dir.join("extracted");
    std::fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;

    let file = std::fs::File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    archive.extract(&extract_dir).map_err(|e| e.to_string())?;

    Ok(extract_dir.to_string_lossy().to_string())
}
