//! BSL Language Server installer
//! Downloads and installs BSL LS from GitHub releases

use serde::{Deserialize, Serialize};

use futures::StreamExt;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

use crate::settings::{get_settings_dir, load_settings, save_settings};

#[derive(Deserialize, Debug)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize, Debug)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    #[serde(default)]
    digest: Option<String>,
}

struct TemporaryInstallArtifacts {
    archive_path: PathBuf,
    staging_dir: PathBuf,
}

impl TemporaryInstallArtifacts {
    fn new(archive_path: PathBuf, staging_dir: PathBuf) -> Self {
        Self {
            archive_path,
            staging_dir,
        }
    }
}

impl Drop for TemporaryInstallArtifacts {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.archive_path);
        let _ = std::fs::remove_dir_all(&self.staging_dir);
    }
}

fn select_windows_asset(release: &GitHubRelease) -> Result<&GitHubAsset, String> {
    release
        .assets
        .iter()
        .find(|asset| asset.name == "bsl-language-server_win.zip")
        .ok_or_else(|| {
            "Could not find bsl-language-server_win.zip in the latest release".to_string()
        })
}

#[cfg(test)]
fn verify_asset_digest(bytes: &[u8], digest: &str) -> Result<(), String> {
    let actual = format!("{:x}", Sha256::digest(bytes));
    verify_digest_hex(&actual, digest)
}

fn verify_digest_hex(actual: &str, digest: &str) -> Result<(), String> {
    let expected = digest
        .strip_prefix("sha256:")
        .ok_or_else(|| format!("Unsupported release asset digest: {digest}"))?;

    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!(
            "BSL Language Server checksum mismatch: expected {expected}, got {actual}"
        ))
    }
}

fn safe_archive_destination(base_dir: &Path, entry_name: &str) -> Result<PathBuf, String> {
    let entry = Path::new(entry_name);
    if entry_name.contains('\\') {
        return Err(format!("Unsafe archive entry: {entry_name}"));
    }

    let mut relative = PathBuf::new();
    for component in entry.components() {
        match component {
            Component::Normal(value) => relative.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("Unsafe archive entry: {entry_name}"));
            }
        }
    }

    if relative.as_os_str().is_empty() {
        return Err("Archive entry path is empty".to_string());
    }

    Ok(base_dir.join(relative))
}

fn sanitized_release_version(tag_name: &str) -> Result<String, String> {
    let version = tag_name.trim().trim_start_matches('v');
    if version.is_empty()
        || !version
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character))
    {
        return Err(format!(
            "Unsafe BSL Language Server release tag: {tag_name}"
        ));
    }
    Ok(version.to_string())
}

fn extract_windows_archive(archive_path: &Path, target_dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(target_dir).map_err(|error| {
        format!(
            "Failed to create BSL Language Server staging directory '{}': {error}",
            target_dir.display()
        )
    })?;

    let file = std::fs::File::open(archive_path)
        .map_err(|error| format!("Failed to open downloaded archive: {error}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| format!("Invalid ZIP archive: {error}"))?;

    let mut launcher_path = None;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("Failed to read ZIP entry #{index}: {error}"))?;
        let entry_name = entry.name().to_string();
        let destination = safe_archive_destination(target_dir, &entry_name)?;

        if entry.is_dir() {
            std::fs::create_dir_all(&destination).map_err(|error| {
                format!(
                    "Failed to create archive directory '{}': {error}",
                    destination.display()
                )
            })?;
            continue;
        }

        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "Failed to create archive directory '{}': {error}",
                    parent.display()
                )
            })?;
        }

        let mut output = std::fs::File::create(&destination).map_err(|error| {
            format!(
                "Failed to create extracted file '{}': {error}",
                destination.display()
            )
        })?;
        std::io::copy(&mut entry, &mut output).map_err(|error| {
            format!("Failed to extract archive entry '{}': {error}", entry_name)
        })?;
        output
            .flush()
            .map_err(|error| format!("Failed to flush '{}': {error}", destination.display()))?;

        if destination
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("bsl-language-server.exe"))
        {
            launcher_path = Some(destination);
        }
    }

    launcher_path.ok_or_else(|| {
        "Downloaded BSL Language Server archive does not contain bsl-language-server.exe"
            .to_string()
    })
}

fn validate_native_launcher(path: &Path, expected_version: &str) -> Result<(), String> {
    let metadata = std::fs::metadata(path).map_err(|error| {
        format!(
            "BSL Language Server launcher '{}' is missing: {error}",
            path.display()
        )
    })?;
    if metadata.len() < 64 * 1024 {
        return Err(format!(
            "BSL Language Server launcher '{}' is unexpectedly small",
            path.display()
        ));
    }

    let mut command = std::process::Command::new(path);
    command.arg("version");
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    let output = command.output().map_err(|error| {
        format!(
            "Failed to validate BSL Language Server launcher '{}': {error}",
            path.display()
        )
    })?;
    if !output.status.success() {
        return Err(format!(
            "BSL Language Server launcher validation failed with status {}",
            output.status
        ));
    }

    let version_output = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !version_output.contains(expected_version) {
        return Err(format!(
            "Installed BSL Language Server version does not match release {expected_version}: {}",
            version_output.trim()
        ));
    }

    Ok(())
}

#[derive(Serialize, Clone)]
struct DownloadProgress {
    progress: u64,
    total: u64,
    percent: u64,
}

/// Download BSL Language Server from GitHub
/// Returns the absolute path to the installed native launcher on Windows.
pub async fn download_bsl_ls(app: AppHandle) -> Result<String, String> {
    crate::app_log!("[BSL Installer] Starting download...");

    // Emit initial progress
    let _ = app.emit(
        "bsl-download-progress",
        DownloadProgress {
            progress: 0,
            total: 0,
            percent: 0,
        },
    );

    // Create HTTP client with redirect support and timeout
    let client = crate::http_client::http_client_builder()?
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(600)) // 10 minutes timeout
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // 1. Get latest release info from GitHub API
    crate::app_log!("[BSL Installer] Fetching latest release info...");
    let api_response = client
        .get("https://api.github.com/repos/1c-syntax/bsl-language-server/releases/latest")
        .header("User-Agent", "mini-ai-1c")
        .send()
        .await
        .map_err(|e| {
            format!(
                "Нет доступа к api.github.com: {}. \
            Проверьте подключение к интернету. \
            Если GitHub заблокирован файрволом — скачайте JAR вручную с \
            https://github.com/1c-syntax/bsl-language-server/releases/latest \
            и укажите путь в настройках.",
                e
            )
        })?;

    if !api_response.status().is_success() {
        let status = api_response.status();
        let body = api_response.text().await.unwrap_or_default();
        let extra = if status.as_u16() == 403 || body.contains("rate limit") {
            " (GitHub API rate limit — попробуйте позже или скачайте JAR вручную)".to_string()
        } else {
            String::new()
        };
        return Err(format!(
            "GitHub API вернул ошибку {}{}\n\
            Скачайте JAR вручную: https://github.com/1c-syntax/bsl-language-server/releases/latest",
            status, extra
        ));
    }

    let release: GitHubRelease = api_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    crate::app_log!("[BSL Installer] Found release: {}", release.tag_name);

    // 2. Use the official Windows distribution with its bundled runtime.
    let asset = select_windows_asset(&release)?;
    let asset_digest = asset.digest.as_deref().ok_or_else(|| {
        format!(
            "GitHub release asset '{}' does not provide a SHA-256 digest",
            asset.name
        )
    })?;
    let release_version = sanitized_release_version(&release.tag_name)?;

    let total_size = asset.size;
    crate::app_log!(
        "[BSL Installer] Asset: {} ({} bytes)",
        asset.name,
        total_size
    );

    // 3. Determine install path (absolute path in app data dir)
    let bin_dir = get_settings_dir().join("bin");
    if !bin_dir.exists() {
        tokio::fs::create_dir_all(&bin_dir)
            .await
            .map_err(|e| format!("Failed to create bin dir: {}", e))?;
    }

    let install_dir = bin_dir.join(format!("bsl-language-server-{release_version}"));
    let existing_launcher = install_dir
        .join("bsl-language-server")
        .join("bsl-language-server.exe");
    if existing_launcher.exists()
        && validate_native_launcher(&existing_launcher, &release_version).is_ok()
    {
        let path_str = normalize_windows_path(&existing_launcher)?;
        save_native_settings(path_str.clone(), release_version)?;
        crate::app_log!(
            "[BSL Installer] Reusing validated installation: {}",
            path_str
        );
        return Ok(path_str);
    }

    let unique_suffix = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let archive_path = bin_dir.join(format!(".bsl-language-server-{unique_suffix}.zip"));
    let staging_dir = bin_dir.join(format!(".bsl-language-server-{unique_suffix}.installing"));
    let _temporary_artifacts =
        TemporaryInstallArtifacts::new(archive_path.clone(), staging_dir.clone());
    crate::app_log!(
        "[BSL Installer] Temporary archive: {}",
        archive_path.display()
    );

    // 4. Download file with progress
    crate::app_log!("[BSL Installer] Downloading...");
    let response = client
        .get(&asset.browser_download_url)
        .header("User-Agent", "mini-ai-1c")
        .header("Accept", "application/octet-stream")
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    // 5. Stream download with progress
    let mut file = tokio::fs::File::create(&archive_path)
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let mut stream = response.bytes_stream();
    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;
    let mut last_percent: u64 = 0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Error downloading: {}", e))?;

        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Error writing file: {}", e))?;

        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        let percent = if total_size > 0 {
            (downloaded * 100) / total_size
        } else {
            0
        };

        // Emit progress every 5%
        if percent >= last_percent + 5 {
            crate::app_log!("[BSL Installer] Progress: {}%", percent);
            let _ = app.emit(
                "bsl-download-progress",
                DownloadProgress {
                    progress: downloaded,
                    total: total_size,
                    percent,
                },
            );
            last_percent = percent;
        }
    }

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush: {}", e))?;
    drop(file);

    if downloaded != total_size {
        let _ = tokio::fs::remove_file(&archive_path).await;
        return Err(format!(
            "BSL Language Server download size mismatch: expected {total_size}, got {downloaded}"
        ));
    }

    let actual_digest = format!("{:x}", hasher.finalize());
    if let Err(error) = verify_digest_hex(&actual_digest, asset_digest) {
        let _ = tokio::fs::remove_file(&archive_path).await;
        return Err(error);
    }

    // Emit 100%
    let _ = app.emit(
        "bsl-download-progress",
        DownloadProgress {
            progress: total_size,
            total: total_size,
            percent: 100,
        },
    );

    crate::app_log!("[BSL Installer] Download complete!");

    // 6. Extract to a staging directory, validate, then atomically activate.
    let archive_for_extract = archive_path.clone();
    let staging_for_extract = staging_dir.clone();
    let staged_launcher = tokio::task::spawn_blocking(move || {
        extract_windows_archive(&archive_for_extract, &staging_for_extract)
    })
    .await
    .map_err(|error| format!("BSL Language Server extraction task failed: {error}"))??;

    validate_native_launcher(&staged_launcher, &release_version)?;

    let activated_dir = if install_dir.exists() {
        bin_dir.join(format!(
            "bsl-language-server-{release_version}-{unique_suffix}"
        ))
    } else {
        install_dir
    };
    tokio::fs::rename(&staging_dir, &activated_dir)
        .await
        .map_err(|error| {
            format!(
                "Failed to activate BSL Language Server installation '{}': {error}",
                activated_dir.display()
            )
        })?;
    let relative_launcher = staged_launcher
        .strip_prefix(&staging_dir)
        .map_err(|error| format!("Invalid extracted launcher path: {error}"))?;
    let launcher_path = activated_dir.join(relative_launcher);
    let path_str = normalize_windows_path(&launcher_path)?;

    // 7. Save native launcher and release version to settings.
    save_native_settings(path_str.clone(), release_version)?;

    crate::app_log!("[BSL Installer] Saved to settings: {}", path_str);

    Ok(path_str)
}

fn normalize_windows_path(path: &Path) -> Result<String, String> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|error| format!("Failed to resolve path '{}': {error}", path.display()))?;
    let mut path_str = canonical.to_string_lossy().to_string();

    #[cfg(windows)]
    if let Some(rest) = path_str.strip_prefix(r"\\?\UNC\") {
        path_str = format!(r"\\{rest}");
    } else if let Some(rest) = path_str.strip_prefix(r"\\?\") {
        path_str = rest.to_string();
    }

    Ok(path_str)
}

fn save_native_settings(path: String, version: String) -> Result<(), String> {
    let mut settings = load_settings();
    settings.bsl_server.executable_path = path;
    settings.bsl_server.installed_version = version;
    save_settings(&settings).map_err(|error| format!("Failed to save settings: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn release_with_assets(names: &[&str]) -> GitHubRelease {
        GitHubRelease {
            tag_name: "v1.0.5".to_string(),
            assets: names
                .iter()
                .map(|name| GitHubAsset {
                    name: (*name).to_string(),
                    browser_download_url: format!("https://example.invalid/{name}"),
                    size: 42,
                    digest: Some(
                        "sha256:d927af45d3fcb009399a1fa0d4ae56969ab4c0414a71eb81054b4b4e2c5fd86e"
                            .to_string(),
                    ),
                })
                .collect(),
        }
    }

    #[test]
    fn selects_official_windows_distribution_instead_of_legacy_jar() {
        let release = release_with_assets(&[
            "bsl-language-server-1.0.5-exec.jar",
            "bsl-language-server_linux.tar.gz",
            "bsl-language-server_win.zip",
        ]);

        let asset = select_windows_asset(&release).expect("Windows asset must be selected");

        assert_eq!(asset.name, "bsl-language-server_win.zip");
    }

    #[test]
    fn verifies_github_sha256_digest_and_rejects_modified_bytes() {
        let digest = "sha256:d927af45d3fcb009399a1fa0d4ae56969ab4c0414a71eb81054b4b4e2c5fd86e";

        assert!(verify_asset_digest(b"bsl-language-server", digest).is_ok());
        assert!(verify_asset_digest(b"modified", digest).is_err());
    }

    #[test]
    fn archive_entries_cannot_escape_install_directory() {
        let install_dir = Path::new(r"C:\MiniAI1C\bin\bsl-language-server-1.0.5");

        assert!(
            safe_archive_destination(install_dir, "bsl-language-server/app/server.jar").is_ok()
        );
        assert!(safe_archive_destination(install_dir, "../../outside.exe").is_err());
        assert!(safe_archive_destination(install_dir, r"C:\outside.exe").is_err());
    }

    #[test]
    fn temporary_install_artifacts_are_removed_on_early_return() {
        let test_dir = std::env::temp_dir().join(format!(
            "mini-ai-1c-bsl-cleanup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let archive_path = test_dir.join("server.zip");
        let staging_dir = test_dir.join("server.installing");
        std::fs::create_dir_all(&staging_dir).expect("create staging directory");
        std::fs::write(&archive_path, b"archive").expect("create temporary archive");
        std::fs::write(staging_dir.join("partial.bin"), b"partial")
            .expect("create partial extracted file");

        {
            let _guard = TemporaryInstallArtifacts::new(archive_path.clone(), staging_dir.clone());
        }

        assert!(!archive_path.exists());
        assert!(!staging_dir.exists());
        let _ = std::fs::remove_dir(&test_dir);
    }
}
