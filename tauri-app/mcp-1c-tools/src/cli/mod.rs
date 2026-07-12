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
    cmd.arg(mode);

    let lower = ib_path.to_lowercase();
    if let Some(pos) = lower.find("file=") {
        let rest = &ib_path[pos + 5..];
        if let Some(start) = rest.find('"') {
            if let Some(end) = rest[start + 1..].find('"') {
                cmd.arg("/F").arg(&rest[start + 1..start + 1 + end]);
            } else {
                cmd.arg("/F").arg(rest.trim_matches('"'));
            }
        } else {
            cmd.arg("/F").arg(rest.trim());
        }
    } else if lower.contains("srvr=") && lower.contains("ref=") {
        cmd.arg("/S").arg(&ib_path);
    } else {
        cmd.arg("/F").arg(&ib_path);
    }

    if mode == "DESIGNER" {
        if !user.is_empty() { cmd.arg("/User:").arg(&user); }
        if !pwd.is_empty() { cmd.arg("/Password:").arg(&pwd); }
    } else {
        if !user.is_empty() { cmd.arg("/N").arg(&user); }
        if !pwd.is_empty() { cmd.arg("/P").arg(&pwd); }
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
    let source_file = args.get("source_file").and_then(|v| v.as_str()).unwrap_or("");
    let out_file = args.get("out_file").and_then(|v| v.as_str()).unwrap_or("");

    if source_file.is_empty() || out_file.is_empty() {
        return Err(anyhow::anyhow!("source_file and out_file are required"));
    }

    let temp_log = std::env::temp_dir().join("epf_build_log.txt");
    let log_path = temp_log.to_string_lossy().to_string();

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/F", &config.build_base]);
    cmd.arg("/LoadExternalDataProcessorOrReportFromFiles");
    cmd.arg(&source_file);
    cmd.arg(&out_file);
    cmd.arg("/Out").arg(&log_path);
    cmd.arg("/DisableStartupDialogs");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("EPF build failed")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed: {}", stderr));
    }
    Ok(format!("EPF built: {}\n{}", out_file, String::from_utf8_lossy(&output.stdout)))
}

pub async fn dump_epf(args: Value, config: &Config) -> Result<String> {
    let epf_path = args.get("epf_path").and_then(|v| v.as_str()).unwrap_or("");
    let out_dir = args.get("out_dir").and_then(|v| v.as_str()).unwrap_or("");

    if epf_path.is_empty() || out_dir.is_empty() {
        return Err(anyhow::anyhow!("epf_path and out_dir are required"));
    }

    let temp_log = std::env::temp_dir().join("epf_dump_log.txt");
    let log_path = temp_log.to_string_lossy().to_string();

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/F", &config.build_base]);
    cmd.arg("/DumpExternalDataProcessorOrReportToFiles");
    cmd.arg(&out_dir);
    cmd.arg(&epf_path);
    cmd.arg("-Format").arg("Hierarchical");
    cmd.arg("/Out").arg(&log_path);
    cmd.arg("/DisableStartupDialogs");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("EPF dump failed")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed: {}", stderr));
    }
    Ok(format!("EPF dumped: {}\n{}", out_dir, String::from_utf8_lossy(&output.stdout)))
}

pub async fn build_erf(args: Value, config: &Config) -> Result<String> {
    let source_file = args.get("source_file").and_then(|v| v.as_str()).unwrap_or("");
    let out_file = args.get("out_file").and_then(|v| v.as_str()).unwrap_or("");

    if source_file.is_empty() || out_file.is_empty() {
        return Err(anyhow::anyhow!("source_file and out_file are required"));
    }

    let temp_log = std::env::temp_dir().join("erf_build_log.txt");
    let log_path = temp_log.to_string_lossy().to_string();

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/F", &config.build_base]);
    cmd.arg("/LoadExternalDataProcessorOrReportFromFiles");
    cmd.arg(&source_file);
    cmd.arg(&out_file);
    cmd.arg("/Out").arg(&log_path);
    cmd.arg("/DisableStartupDialogs");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("ERF build failed")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed: {}", stderr));
    }
    Ok(format!("ERF built: {}\n{}", out_file, String::from_utf8_lossy(&output.stdout)))
}

pub async fn dump_erf(args: Value, config: &Config) -> Result<String> {
    let erf_path = args.get("erf_path").and_then(|v| v.as_str()).unwrap_or("");
    let out_dir = args.get("out_dir").and_then(|v| v.as_str()).unwrap_or("");

    if erf_path.is_empty() || out_dir.is_empty() {
        return Err(anyhow::anyhow!("erf_path and out_dir are required"));
    }

    let temp_log = std::env::temp_dir().join("erf_dump_log.txt");
    let log_path = temp_log.to_string_lossy().to_string();

    let mut cmd = Command::new(config.v8_path.clone());
    cmd.args(["DESIGNER", "/F", &config.build_base]);
    cmd.arg("/DumpExternalDataProcessorOrReportToFiles");
    cmd.arg(&out_dir);
    cmd.arg(&erf_path);
    cmd.arg("-Format").arg("Hierarchical");
    cmd.arg("/Out").arg(&log_path);
    cmd.arg("/DisableStartupDialogs");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().await.context("ERF dump failed")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed: {}", stderr));
    }
    Ok(format!("ERF dumped: {}\n{}", out_dir, String::from_utf8_lossy(&output.stdout)))
}