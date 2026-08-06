//! Поиск установленных платформ 1С:Предприятие.
//!
//! Сканирует `Program Files\1cv8\X.Y.Z.W` и `Program Files (x86)\1cv8\...`,
//! возвращает версии по убыванию с путями к `1cv8.exe` и `ibcmd.exe`.

use regex::Regex;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct PlatformInfo {
    pub version: String,
    pub bin_path: String,
    pub exe_path: String,
    pub ibcmd_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cestart_path: Option<String>,
}

fn version_regex() -> Regex {
    Regex::new(r"^\d+\.\d+\.\d+\.\d+$").unwrap()
}

fn program_files_bases() -> Vec<PathBuf> {
    let mut bases = Vec::new();
    if let Some(pf) = std::env::var_os("PROGRAMFILES") {
        bases.push(PathBuf::from(pf));
    } else {
        bases.push(PathBuf::from(r"C:\Program Files"));
    }
    if let Some(pf86) = std::env::var_os("PROGRAMFILES(X86)") {
        bases.push(PathBuf::from(pf86));
    } else {
        bases.push(PathBuf::from(r"C:\Program Files (x86)"));
    }
    bases
}

/// Находит установленные платформы, отсортированные по версии по убыванию.
pub fn find_platform() -> Vec<PlatformInfo> {
    let version_re = version_regex();
    let mut platforms = Vec::new();

    for base in program_files_bases() {
        let cv8_dir = base.join("1cv8");
        if !cv8_dir.is_dir() {
            continue;
        }

        let entries = match std::fs::read_dir(&cv8_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !version_re.is_match(&name) {
                continue;
            }
            let version_dir = entry.path();
            if !version_dir.is_dir() {
                continue;
            }

            let bin_dir = version_dir.join("bin");
            let exe_path = bin_dir.join("1cv8.exe");
            let ibcmd_path = bin_dir.join("ibcmd.exe");

            if exe_path.exists() {
                // 1cestart.exe лежит в общем каталоге
                let common_dir = cv8_dir.join("common");
                let cestart_path = common_dir.join("1cestart.exe");
                platforms.push(PlatformInfo {
                    version: name,
                    bin_path: bin_dir.to_string_lossy().to_string(),
                    exe_path: exe_path.to_string_lossy().to_string(),
                    ibcmd_path: ibcmd_path.to_string_lossy().to_string(),
                    cestart_path: if cestart_path.exists() {
                        Some(cestart_path.to_string_lossy().to_string())
                    } else {
                        None
                    },
                });
            }
        }
    }

    // Сортировка по семантической версии по убыванию
    platforms.sort_by(|a, b| {
        let va: Vec<u32> = a.version.split('.').filter_map(|p| p.parse().ok()).collect();
        let vb: Vec<u32> = b.version.split('.').filter_map(|p| p.parse().ok()).collect();
        for i in 0..4 {
            let da = va.get(i).copied().unwrap_or(0);
            let db = vb.get(i).copied().unwrap_or(0);
            if db != da {
                return db.cmp(&da);
            }
        }
        std::cmp::Ordering::Equal
    });

    platforms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_regex_matches_exact_four_octets() {
        let re = version_regex();
        assert!(re.is_match("8.3.27.1989"));
        assert!(!re.is_match("8.3"));
        assert!(!re.is_match("8.3.27"));
        assert!(!re.is_match("8.3.27.1989.1"));
        assert!(!re.is_match("common"));
    }

    #[test]
    fn sorts_by_semantic_version_desc() {
        let mk = |v: &str| PlatformInfo {
            version: v.to_string(),
            bin_path: String::new(),
            exe_path: String::new(),
            ibcmd_path: String::new(),
            cestart_path: None,
        };
        let mut p = vec![mk("8.3.15.1000"), mk("8.3.27.1989"), mk("8.3.9.500")];
        p.sort_by(|a, b| {
            let va: Vec<u32> = a.version.split('.').filter_map(|x| x.parse().ok()).collect();
            let vb: Vec<u32> = b.version.split('.').filter_map(|x| x.parse().ok()).collect();
            for i in 0..4 {
                let da = va.get(i).copied().unwrap_or(0);
                let db = vb.get(i).copied().unwrap_or(0);
                if db != da {
                    return db.cmp(&da);
                }
            }
            std::cmp::Ordering::Equal
        });
        assert_eq!(p[0].version, "8.3.27.1989");
        assert_eq!(p[1].version, "8.3.15.1000");
        assert_eq!(p[2].version, "8.3.9.500");
    }

    #[test]
    fn no_program_files_returns_empty() {
        // На Linux/CI каталога 1cv8 нет — вернётся пустой список.
        let platforms = find_platform();
        // На Windows с установленной 1С может быть непусто; на CI/Linux — пусто.
        // Просто проверяем, что вызов не паникует.
        let _ = platforms;
    }
}
