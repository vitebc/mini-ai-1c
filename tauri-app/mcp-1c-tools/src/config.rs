use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{env, fs};

#[derive(Debug, Clone)]
pub struct Config {
    pub v8_path: String,
    pub ibcmd_path: String,
    pub build_base: String,
    pub workspace: String,
}

pub async fn new(v8_path_override: Option<&str>) -> Result<Config> {
    // Try loading existing config
    if let Ok(cfg) = crate::setup::AppConfig::load() {
        eprintln!("[config] Loaded config: v8_path={}", cfg.v8_path);
        let workspace = find_workspace().await;
        return Ok(Config {
            v8_path: cfg.v8_path,
            ibcmd_path: cfg.ibcmd_path,
            build_base: cfg.build_base,
            workspace,
        });
    }

    // First run — run setup
    eprintln!("[config] First run detected. Running setup...");
    let app_cfg = crate::setup::first_run_setup(v8_path_override)
        .context("First run setup failed")?;

    let workspace = find_workspace().await;
    Ok(Config {
        v8_path: app_cfg.v8_path,
        ibcmd_path: app_cfg.ibcmd_path,
        build_base: app_cfg.build_base,
        workspace,
    })
}

async fn find_workspace() -> String {
    let cwd = env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let checks = [
        &cwd,
        r"E:\1C_AI\mini-ai-1c",
        r"C:\Projects",
        r"D:\Projects",
    ];
    for path in checks {
        let p = Path::new(path);
        if p.exists() && has_workspace_marker(p) {
            return path.to_string();
        }
    }
    cwd
}

fn has_workspace_marker(path: &Path) -> bool {
    path.join(".v8-project.json").exists()
        || path.join("src").join("cf").exists()
        || path.join("Configuration.xml").exists()
}
