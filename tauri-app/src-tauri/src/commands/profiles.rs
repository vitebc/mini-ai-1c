use crate::llm::cli_providers::codex::CodexCliProvider;
use crate::llm::cli_providers::qwen::QwenCliProvider;
use crate::llm_profiles::{self, LLMProfile, ProfileStore};
use tauri::{AppHandle, Emitter};

fn sync_legacy_active_profile(active_profile_id: &str) {
    if active_profile_id.is_empty() {
        return;
    }

    let mut settings = crate::settings::load_settings();
    if settings.active_llm_profile == active_profile_id {
        return;
    }

    settings.active_llm_profile = active_profile_id.to_string();
    if let Err(err) = crate::settings::save_settings(&settings) {
        crate::app_log!(
            force: true,
            "[Profiles] Failed to sync settings.active_llm_profile: {}",
            err
        );
    }
}

fn persist_profile_store(store: &ProfileStore, app_handle: &AppHandle) -> Result<(), String> {
    llm_profiles::save_profiles(store)?;
    sync_legacy_active_profile(&store.active_profile_id);
    let _ = app_handle.emit("profiles-changed", &store.active_profile_id);
    Ok(())
}

/// Get all LLM profiles
#[tauri::command]
pub fn get_profiles() -> ProfileStore {
    llm_profiles::load_profiles()
}

/// Save profile
#[tauri::command]
pub fn save_profile(
    mut profile: LLMProfile,
    api_key: Option<String>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut store = llm_profiles::load_profiles();
    let existing_encrypted = store
        .profiles
        .iter()
        .find(|p| p.id == profile.id)
        .map(|p| p.api_key_encrypted.clone());

    match api_key {
        Some(key) if !key.trim().is_empty() => {
            profile.set_api_key(&key);
        }
        _ => {
            let incoming = profile.api_key_encrypted.trim();
            let incoming_is_placeholder =
                incoming.is_empty() || incoming.eq_ignore_ascii_case("set");

            if incoming_is_placeholder {
                if let Some(existing) = existing_encrypted {
                    if existing.trim().is_empty() || existing.eq_ignore_ascii_case("set") {
                        profile.api_key_encrypted.clear();
                    } else {
                        profile.api_key_encrypted = existing;
                    }
                } else {
                    profile.api_key_encrypted.clear();
                }
            }
        }
    }

    // Update or add profile
    if let Some(pos) = store.profiles.iter().position(|p| p.id == profile.id) {
        store.profiles[pos] = profile;
    } else {
        store.profiles.push(profile);
    }

    persist_profile_store(&store, &app_handle)
}

/// Delete a profile
#[tauri::command]
pub fn delete_profile(profile_id: String, app_handle: AppHandle) -> Result<(), String> {
    let mut store = llm_profiles::load_profiles();

    // Check if profile exists
    let profile = store.profiles.iter().find(|p| p.id == profile_id).cloned();
    if profile.is_none() {
        return Err("Профиль не найден".to_string());
    }

    // Don't allow deleting the last profile
    if store.profiles.len() <= 1 {
        return Err("Нельзя удалить последний профиль".to_string());
    }

    // If it's a CLI provider — clear the stored token from keychain
    if let Some(p) = &profile {
        if matches!(p.provider, crate::llm_profiles::LLMProvider::QwenCli) {
            let _ = QwenCliProvider::logout(&profile_id); // ignore error if no token exists
        } else if matches!(p.provider, crate::llm_profiles::LLMProvider::CodexCli) {
            let _ = CodexCliProvider::logout(&profile_id); // ignore error if no token exists
        }
    }

    // Remove the profile
    store.profiles.retain(|p| p.id != profile_id);

    // If we deleted the active profile, pick the first available one
    if store.active_profile_id == profile_id {
        if let Some(first) = store.profiles.first() {
            store.active_profile_id = first.id.clone();
        }
    }

    persist_profile_store(&store, &app_handle)
}

/// Set active profile
#[tauri::command]
pub fn set_active_profile(profile_id: String, app_handle: AppHandle) -> Result<(), String> {
    let mut store = llm_profiles::load_profiles();

    if !store.profiles.iter().any(|p| p.id == profile_id) {
        return Err("Profile not found".to_string());
    }

    store.active_profile_id = profile_id;
    persist_profile_store(&store, &app_handle)
}

/// Fetch models for a profile (using stored profile settings)
#[tauri::command]
pub async fn fetch_models_cmd(profile_id: String) -> Result<Vec<String>, String> {
    let store = llm_profiles::load_profiles();
    let profile = store
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or("Profile not found")?;

    crate::ai::fetch_models(profile).await
}

/// Test connection for a profile
#[tauri::command]
pub async fn test_llm_connection_cmd(profile_id: String) -> Result<String, String> {
    let store = llm_profiles::load_profiles();
    let profile = store
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or("Profile not found")?;

    crate::ai::test_connection(profile).await
}

/// Fetch models from a specific provider using API and Registry
#[tauri::command]
pub async fn fetch_models_from_provider(
    provider_id: String,
    base_url: String,
    api_key: String,
) -> Result<Vec<crate::llm::providers::Model>, String> {
    use crate::llm::providers;

    // 1. Fetch from API
    let api_models = providers::fetch_models_from_api(&provider_id, &base_url, &api_key).await?;

    if api_models.is_empty() {
        return Err("Provider returned empty model list".to_string());
    }

    // 2. Fetch Registry
    let registry = providers::fetch_registry().await.unwrap_or_else(|e| {
        crate::app_log!(force: true, "Failed to fetch registry: {}", e);
        providers::RegistryData {
            providers: std::collections::HashMap::new(),
        }
    });

    // 3. Merge
    let merged = providers::merge_models(api_models, &registry, &provider_id);

    Ok(merged)
}

/// Fetch models for an existing profile (using stored key)
#[tauri::command]
pub async fn fetch_models_for_profile(
    profile_id: String,
) -> Result<Vec<crate::llm::providers::Model>, String> {
    use crate::llm::providers;

    let store = llm_profiles::load_profiles();
    let profile = store
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or("Profile not found")?;

    // CLI providers use OAuth token, not API key — route to their own fetch_models
    if profile.provider.to_string() == "CodexCli" {
        return CodexCliProvider::fetch_models(&profile_id).await;
    }

    let api_key = profile.try_get_api_key()?;
    let base_url = profile.get_base_url();

    // 1. Fetch from API
    let api_models =
        providers::fetch_models_from_api(&profile.provider.to_string(), &base_url, &api_key)
            .await?;

    if api_models.is_empty() {
        return Err("Provider returned empty model list".to_string());
    }

    // 2. Fetch Registry
    let registry = providers::fetch_registry().await.unwrap_or_else(|e| {
        crate::app_log!(force: true, "Failed to fetch registry: {}", e);
        providers::RegistryData {
            providers: std::collections::HashMap::new(),
        }
    });

    // 3. Merge
    let merged = providers::merge_models(api_models, &registry, &profile.provider.to_string());

    Ok(merged)
}
