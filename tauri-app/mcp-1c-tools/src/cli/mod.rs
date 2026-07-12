use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;

use crate::config::Config;

pub async fn create_infobase(args: Value, config: &Config) -> Result<String> {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if path.is_empty() { return Err(anyhow::anyhow!("Path is required")); }

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/C", "IBCreate", "/ParentNode:root\\", "/IBPath:", &path]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("Failed to execute 1cv8.exe")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(format!("Infobase created at: {}\n{}", path, String::from_utf8_lossy(&output.stdout)))
}

pub async fn dump_cf(args: Value, config: &Config) -> Result<String> {
    let ib_path = args.get("ib_path").and_then(|v| v.as_str()).unwrap_or("");
    let cf_path = args.get("cf_path").and_then(|v| v.as_str()).unwrap_or("");
    let user = args.get("user").and_then(|v| v.as_str()).unwrap_or("Admin");
    let pwd = args.get("password").and_then(|v| v.as_str()).unwrap_or("");

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/C", "IBDUMP", "/Destination:", cf_path, "/IBConnectionString:", ib_path, "/ConnectionUser:", user, "/ConnectionPassword:", pwd, "/Overwrite:"]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("IBDUMP failed")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(format!("Configuration dumped to: {}\n{}", cf_path, String::from_utf8_lossy(&output.stdout)))
}

pub async fn load_cf(args: Value, config: &Config) -> Result<String> {
    let ib_path = args.get("ib_path").and_then(|v| v.as_str()).unwrap_or("");
    let cf_path = args.get("cf_path").and_then(|v| v.as_str()).unwrap_or("");
    let user = args.get("user").and_then(|v| v.as_str()).unwrap_or("Admin");
    let pwd = args.get("password").and_then(|v| v.as_str()).unwrap_or("");

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/C", "IBLOAD", "/File:", cf_path, "/IBConnectionString:", ib_path, "/User:", user, "/Password:", pwd]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("IBLOAD failed")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(format!("Configuration loaded: {}\n{}", ib_path, String::from_utf8_lossy(&output.stdout)))
}

pub async fn dump_dt(args: Value, config: &Config) -> Result<String> {
    let ib_path = args.get("ib_path").and_then(|v| v.as_str()).unwrap_or("");
    let dt_path = args.get("dt_path").and_then(|v| v.as_str()).unwrap_or("");

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/C", "IBDUMP", "/Destination:", dt_path, "/IBConnectionString:", ib_path, "/ObjectType:dataProcessor", "/ExportFormat:ibdata"]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("IBDUMP .dt failed")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(format!("DB dumped to .dt: {}\n{}", dt_path, String::from_utf8_lossy(&output.stdout)))
}

pub async fn load_dt(args: Value, config: &Config) -> Result<String> {
    let ib_path = args.get("ib_path").and_then(|v| v.as_str()).unwrap_or("");
    let dt_path = args.get("dt_path").and_then(|v| v.as_str()).unwrap_or("");

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/C", "IBLOAD", "/File:", dt_path, "/IBConnectionString:", ib_path]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("IBLOAD .dt failed")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(format!("DB loaded from .dt: {}\n{}", ib_path, String::from_utf8_lossy(&output.stdout)))
}

pub async fn dump_xml(args: Value, config: &Config) -> Result<String> {
    let ib_path = args.get("ib_path").and_then(|v| v.as_str()).unwrap_or("");
    let out_dir = args.get("out_dir").and_then(|v| v.as_str()).unwrap_or("");

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/C", "IBLOAD", "/File:", out_dir, "/IBConnectionString:", ib_path]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("XML dump failed")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(format!("Config saved to XML: {}\n{}", out_dir, String::from_utf8_lossy(&output.stdout)))
}

pub async fn load_xml(args: Value, config: &Config) -> Result<String> {
    let ib_path = args.get("ib_path").and_then(|v| v.as_str()).unwrap_or("");
    let src_dir = args.get("src_dir").and_then(|v| v.as_str()).unwrap_or("");

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/C", "IBLOAD", "/File:", src_dir, "/IBConnectionString:", ib_path]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("XML load failed")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(format!("Config loaded from XML: {}\n{}", ib_path, String::from_utf8_lossy(&output.stdout)))
}

pub async fn update_infobase(args: Value, config: &Config) -> Result<String> {
    let ib_path = args.get("ib_path").and_then(|v| v.as_str()).unwrap_or("");
    let user = args.get("user").and_then(|v| v.as_str()).unwrap_or("Admin");
    let pwd = args.get("password").and_then(|v| v.as_str()).unwrap_or("");

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/C", "IBUPDATE", "/IBConnectionString:", ib_path, "/User:", user, "/Password:", pwd]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("IBUPDATE failed")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(format!("Infobase updated: {}\n{}", ib_path, String::from_utf8_lossy(&output.stdout)))
}

pub async fn run_1c(args: Value, config: &Config) -> Result<String> {
    let ib_path = args.get("ib_path").and_then(|v| v.as_str()).unwrap_or("");
    let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("ENTERPRISE");
    let user = args.get("user").and_then(|v| v.as_str()).unwrap_or("");
    let pwd = args.get("password").and_then(|v| v.as_str()).unwrap_or("");

    if ib_path.is_empty() {
        return Err(anyhow::anyhow!("ib_path is required"));
    }

    let mut cmd = Command::new(config.v8_path.clone());

    if ib_path.contains("Srvr=") || ib_path.contains("File=") {
        let escaped = format!("\"{}\"", ib_path.replace('\"', "\"\""));
        cmd.args([mode, "/IBConnectionString", &escaped]);
    } else {
        cmd.args([mode, "/F", &ib_path]);
    }

    if !user.is_empty() {
        cmd.arg("/User:").arg(&user);
    }
    if !pwd.is_empty() {
        cmd.arg("/Password:").arg(&pwd);
    }

    let _ = cmd.spawn()?;
    Ok(format!("{} started for: {}",
        if mode == "DESIGNER" { "Configurator" } else { "1C:Enterprise" },
        ib_path))
}

pub async fn list_infobases(args: Value, config: &Config) -> Result<String> {
    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/C", "IBLIST"]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("IBLIST failed")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(format!("Infobases:\n{}", String::from_utf8_lossy(&output.stdout)))
}

pub async fn build_epf(args: Value, config: &Config) -> Result<String> {
    let src_dir = args.get("src_dir").and_then(|v| v.as_str()).unwrap_or("");
    let out_file = args.get("out_file").and_then(|v| v.as_str()).unwrap_or("");

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/C", "EPFBuild", "/Source:", src_dir, "/Destination:", out_file, "/Overwrite:", "/IBConnectionString:", &config.build_base]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("EPF build failed")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(format!("EPF built: {}\n{}", out_file, String::from_utf8_lossy(&output.stdout)))
}

pub async fn dump_epf(args: Value, config: &Config) -> Result<String> {
    let epf_path = args.get("epf_path").and_then(|v| v.as_str()).unwrap_or("");
    let out_dir = args.get("out_dir").and_then(|v| v.as_str()).unwrap_or("");

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/C", "EPFExtract", "/Source:", epf_path, "/Destination:", out_dir, "/Overwrite:", "/IBConnectionString:", &config.build_base]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("EPF extract failed")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(format!("EPF extracted: {}\n{}", out_dir, String::from_utf8_lossy(&output.stdout)))
}

pub async fn build_erf(args: Value, config: &Config) -> Result<String> {
    let src_dir = args.get("src_dir").and_then(|v| v.as_str()).unwrap_or("");
    let out_file = args.get("out_file").and_then(|v| v.as_str()).unwrap_or("");

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/C", "ERFBuild", "/Source:", src_dir, "/Destination:", out_file, "/Overwrite:", "/IBConnectionString:", &config.build_base]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("ERF build failed")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(format!("ERF built: {}\n{}", out_file, String::from_utf8_lossy(&output.stdout)))
}

pub async fn dump_erf(args: Value, config: &Config) -> Result<String> {
    let erf_path = args.get("erf_path").and_then(|v| v.as_str()).unwrap_or("");
    let out_dir = args.get("out_dir").and_then(|v| v.as_str()).unwrap_or("");

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/C", "ERFFExtract", "/Source:", erf_path, "/Destination:", out_dir, "/Overwrite:", "/IBConnectionString:", &config.build_base]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("ERF extract failed")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(format!("ERF extracted: {}\n{}", out_dir, String::from_utf8_lossy(&output.stdout)))
}