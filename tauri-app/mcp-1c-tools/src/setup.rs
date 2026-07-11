use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{env, fs};

const CONFIG_SUBDIR: &str = r".config\mcp-1c-tools";
const CONFIG_FILE: &str = "config.json";
const BUILD_BASE_DIR: &str = "BuildBase";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub v8_path: String,
    pub ibcmd_path: String,
    pub build_base: String,
    pub first_run: bool,
}

impl AppConfig {
    pub fn config_dir() -> PathBuf {
        let home = env::var("USERPROFILE")
            .or_else(|_| env::var("HOME"))
            .unwrap_or_else(|_| "C:\\Users\\Default".into());
        PathBuf::from(home).join(CONFIG_SUBDIR)
    }

    fn config_path() -> PathBuf {
        Self::config_dir().join(CONFIG_FILE)
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = fs::read_to_string(&path)
                .context("Failed to read config.json")?;
            let cfg: AppConfig = serde_json::from_str(&content)
                .context("Failed to parse config.json")?;
            Ok(cfg)
        } else {
            Err(anyhow::anyhow!("Config not found"))
        }
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir).context("Failed to create config dir")?;
        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;
        fs::write(Self::config_path(), &content)
            .context("Failed to write config.json")?;
        Ok(())
    }
}

pub struct PlatformInfo {
    pub version: String,
    pub bin_path: PathBuf,
}

pub fn find_platform() -> Option<PlatformInfo> {
    let search_paths = [
        r"C:\Program Files\1cv8",
        r"C:\Program Files (x86)\1cv8",
    ];

    let mut platforms: Vec<PlatformInfo> = Vec::new();

    for base_str in &search_paths {
        let base = Path::new(base_str);
        if !base.exists() || !base.is_dir() {
            continue;
        }

        let entries = match fs::read_dir(base) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Only version folders like "8.3.27.1989"
            if !name_str.chars().all(|c| c.is_ascii_digit() || c == '.') {
                continue;
            }
            if name_str.matches('.').count() != 3 {
                continue;
            }

            let bin_path = entry.path().join("bin");
            let exe_path = bin_path.join("1cv8.exe");
            if !exe_path.exists() {
                continue;
            }

            platforms.push(PlatformInfo {
                version: name_str.to_string(),
                bin_path,
            });
        }
    }

    if platforms.is_empty() {
        return None;
    }

    // Sort by semantic version descending
    platforms.sort_by(|a, b| {
        let parts_a: Vec<u32> = a.version.split('.').filter_map(|s| s.parse().ok()).collect();
        let parts_b: Vec<u32> = b.version.split('.').filter_map(|s| s.parse().ok()).collect();
        for i in 0..4 {
            let va = parts_a.get(i).copied().unwrap_or(0);
            let vb = parts_b.get(i).copied().unwrap_or(0);
            if vb != va {
                return vb.cmp(&va);
            }
        }
        std::cmp::Ordering::Equal
    });

    Some(platforms.swap_remove(0))
}

pub fn ensure_build_base(v8_path: &str, build_base: &str) -> Result<()> {
    let bb = Path::new(build_base);
    if bb.join("1Cv8.1CD").exists() {
        eprintln!("[setup] BuildBase already exists: {}", build_base);
        return Ok(());
    }

    fs::create_dir_all(build_base).context("Failed to create BuildBase dir")?;

    eprintln!("[setup] Creating BuildBase at: {}...", build_base);

    let mut cmd = std::process::Command::new(v8_path);
    cmd.args([
        "CREATEINFOBASE",
        &format!("File=\"{}\"", build_base),
    ]);
    let output = cmd.output().context("Failed to execute 1cv8.exe CREATEINFOBASE")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already exists") {
            eprintln!("[setup] BuildBase already exists (detected via error msg)");
            return Ok(());
        }
        return Err(anyhow::anyhow!(
            "Failed to create BuildBase: {}",
            stderr
        ));
    }

    eprintln!("[setup] BuildBase created successfully at: {}", build_base);
    Ok(())
}

pub fn first_run_setup(v8_path_override: Option<&str>) -> Result<AppConfig> {
    let config_dir = AppConfig::config_dir();
    fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

    let v8_path = if let Some(path) = v8_path_override {
        let p = Path::new(path);
        if !p.exists() {
            anyhow::bail!("Specified v8_path does not exist: {}", path);
        }
        path.to_string()
    } else if let Some(platform) = find_platform() {
        let exe = platform.bin_path.join("1cv8.exe");
        exe.to_string_lossy().to_string()
    } else {
        eprintln!("REQUIRE_V8_PATH:1cv8.exe not found. Please specify --v8-path or set ONEC_V8_PATH env var");
        anyhow::bail!("1cv8.exe not found. Provide --v8-path argument")
    };

    let v8_dir = Path::new(&v8_path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let ibcmd_path = v8_dir.join("ibcmd.exe");
    let ibcmd_path = if ibcmd_path.exists() {
        ibcmd_path.to_string_lossy().to_string()
    } else {
        v8_dir.join("ibcmd.exe").to_string_lossy().to_string()
    };

    let build_base = AppConfig::config_dir().join(BUILD_BASE_DIR);
    let build_base_str = build_base.to_string_lossy().to_string();

    let _ = ensure_build_base(&v8_path, &build_base_str);

    let config = AppConfig {
        v8_path,
        ibcmd_path,
        build_base: build_base_str,
        first_run: false,
    };

    config.save()?;
    eprintln!("[setup] Config saved to: {}", AppConfig::config_path().display());

    Ok(config)
}
