//! Поиск установленной платформы 1С:Предприятие.
//!
//! Порт `findPlatform` из 1c-help.ts. Если задан ONEC_HELP_PATH —
//! ищет только там. Иначе — стандартные пути для Windows/Linux.
//! Возвращает последнюю (максимальную) версию с файлом shcntx_ru.hbk.

use regex::Regex;
use std::path::{Path, PathBuf};

pub struct Platform {
    pub version: String,
    pub bin_path: PathBuf,
}

fn version_regex() -> Regex {
    Regex::new(r"^\d+\.\d+\.\d+\.\d+$").unwrap()
}

fn default_search_paths() -> Vec<PathBuf> {
    if cfg!(windows) {
        vec![
            PathBuf::from(r"C:\Program Files\1cv8"),
            PathBuf::from(r"C:\Program Files (x86)\1cv8"),
        ]
    } else {
        vec![
            PathBuf::from("/opt/1cv8"),
            PathBuf::from("/opt/1cv8/x86_64"),
            PathBuf::from("/usr/share/1cv8"),
        ]
    }
}

/// Находит платформу (максимальная версия с shcntx_ru.hbk).
pub fn find_platform() -> Option<Platform> {
    let custom = std::env::var("ONEC_HELP_PATH").unwrap_or_default();
    let custom = custom.trim();
    let search_paths: Vec<PathBuf> = if !custom.is_empty() {
        vec![PathBuf::from(custom)]
    } else {
        default_search_paths()
    };

    let version_re = version_regex();
    let mut platforms = Vec::new();

    for base in &search_paths {
        if !base.is_dir() {
            continue;
        }
        let entries = match std::fs::read_dir(base) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !version_re.is_match(&name) {
                continue;
            }
            let bin_path = entry.path().join("bin");
            if !bin_path.is_dir() {
                continue;
            }
            let hbk = bin_path.join("shcntx_ru.hbk");
            if !hbk.exists() {
                continue;
            }
            platforms.push(Platform {
                version: name,
                bin_path,
            });
        }
    }

    platforms.sort_by(|a, b| {
        let va: Vec<u32> = a.version.split('.').filter_map(|p| p.parse().ok()).collect();
        let vb: Vec<u32> = b.version.split('.').filter_map(|p| p.parse().ok()).collect();
        for i in 0..4 {
            let da = va.get(i).copied().unwrap_or(0);
            let db = vb.get(i).copied().unwrap_or(0);
            if da != db {
                return db.cmp(&da);
            }
        }
        std::cmp::Ordering::Equal
    });

    platforms.into_iter().next()
}

#[allow(dead_code)]
fn _is_hbk(p: &Path) -> bool {
    p.extension().map(|e| e == "hbk").unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_regex_matches_four_octets() {
        let re = version_regex();
        assert!(re.is_match("8.3.27.1989"));
        assert!(!re.is_match("8.3"));
        assert!(!re.is_match("8.3.27"));
        assert!(!re.is_match("common"));
    }

    #[test]
    fn sorts_by_version_desc() {
        let mk = |v: &str| Platform {
            version: v.to_string(),
            bin_path: PathBuf::new(),
        };
        let mut p = vec![mk("8.3.15.1000"), mk("8.3.27.1989"), mk("8.3.9.500")];
        p.sort_by(|a, b| {
            let va: Vec<u32> = a.version.split('.').filter_map(|x| x.parse().ok()).collect();
            let vb: Vec<u32> = b.version.split('.').filter_map(|x| x.parse().ok()).collect();
            for i in 0..4 {
                let da = va.get(i).copied().unwrap_or(0);
                let db = vb.get(i).copied().unwrap_or(0);
                if da != db {
                    return db.cmp(&da);
                }
            }
            std::cmp::Ordering::Equal
        });
        assert_eq!(p[0].version, "8.3.27.1989");
        assert_eq!(p[1].version, "8.3.15.1000");
        assert_eq!(p[2].version, "8.3.9.500");
    }
}
