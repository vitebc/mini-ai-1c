//! Парсер `ibases.v8i` — списка информационных баз 1С.
//!
//! Порт jvv-1c.ts (TypeScript). Поддерживает три формата строки подключения:
//! - `File="C:\path"` — файловая база;
//! - `Srvr="server";Ref="name"` — серверная база;
//! - `S="server/db"` — серверная база (сокращённая форма).

use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct InfobaseInfo {
    pub name: String,
    pub connection: String,
    pub r#type: String, // "file" | "server"
    pub id: Option<String>,
    pub folder: Option<String>,
}

fn home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

fn program_data() -> Option<PathBuf> {
    std::env::var_os("ProgramData").map(PathBuf::from)
}

/// Стандартные пути к ibases.v8i.
pub fn default_v8i_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = home() {
        paths.push(home.join("AppData").join("Roaming").join("1C").join("1CEStart").join("ibases.v8i"));
        paths.push(home.join("AppData").join("Roaming").join("1C").join("1cv8").join("ibases.v8i"));
    }
    if let Some(pd) = program_data() {
        paths.push(pd.join("1C").join("1CEStart").join("ibases.v8i"));
        paths.push(pd.join("1C").join("1cv8").join("ibases.v8i"));
    }
    paths
}

fn find_path_after_quote(rest: &str) -> Option<String> {
    let start = rest.find('"')?;
    let after = &rest[start + 1..];
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

/// Разбирает строку подключения в (connection, type).
pub fn parse_connection_string(connect: &str) -> Option<(String, String)> {
    let lower = connect.to_lowercase();

    // File="..."
    if lower.contains("file=") {
        let idx = lower.find("file=")?;
        let rest = &connect[idx..];
        if let Some(path) = find_path_after_quote(rest) {
            return Some((format!("File=\"{}\"", path), "file".to_string()));
        }
    }

    // Srvr="...";Ref="..."
    if lower.contains("srvr=") && lower.contains("ref=") {
        return Some((connect.to_string(), "server".to_string()));
    }

    // S="server/db"
    if lower.starts_with("s=") {
        return Some((connect.to_string(), "server".to_string()));
    }

    None
}

fn push_base(
    bases: &mut Vec<InfobaseInfo>,
    name: &str,
    connect: &str,
    id: &str,
    folder: &str,
) {
    if name.is_empty() || connect.is_empty() {
        return;
    }
    if let Some((connection, r#type)) = parse_connection_string(connect) {
        bases.push(InfobaseInfo {
            name: name.to_string(),
            connection,
            r#type,
            id: if id.is_empty() { None } else { Some(id.to_string()) },
            folder: if folder.is_empty() { None } else { Some(folder.to_string()) },
        });
    }
}

/// Разбирает содержимое ibases.v8i.
pub fn parse_v8i_content(content: &str) -> Vec<InfobaseInfo> {
    let mut bases = Vec::new();
    let mut current_name = String::new();
    let mut current_connect = String::new();
    let mut current_id = String::new();
    let mut current_folder = String::new();

    // Убираем BOM
    let text = content.strip_prefix('\u{FEFF}').unwrap_or(content);

    for line in text.split('\n') {
        let trimmed = line.trim();

        // Секция [Name]
        if let Some(section) = trimmed.strip_prefix('[') {
            if let Some(name) = section.strip_suffix(']') {
                push_base(&mut bases, &current_name, &current_connect, &current_id, &current_folder);
                current_name = name.to_string();
                current_connect.clear();
                current_id.clear();
                current_folder.clear();
                continue;
            }
        }

        // Key=Value
        if let Some(eq_idx) = trimmed.find('=') {
            let key = trimmed[..eq_idx].trim().to_lowercase();
            let value = trimmed[eq_idx + 1..].trim().to_string();
            match key.as_str() {
                "connect" => current_connect = value,
                "id" => current_id = value,
                "folder" => current_folder = value,
                _ => {}
            }
        }
    }

    // Последняя секция
    push_base(&mut bases, &current_name, &current_connect, &current_id, &current_folder);

    bases
}

/// Читает и разбирает ibases.v8i. Если `v8i_path` задан — только его.
pub fn parse_v8i_file(v8i_path: Option<&str>) -> Vec<InfobaseInfo> {
    let paths: Vec<PathBuf> = if let Some(p) = v8i_path {
        vec![PathBuf::from(p)]
    } else {
        default_v8i_paths()
    };

    for p in &paths {
        if !p.exists() {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(p) {
            let bases = parse_v8i_content(&content);
            if !bases.is_empty() {
                return bases;
            }
        }
    }
    Vec::new()
}

/// Находит первый существующий путь к ibases.v8i.
pub fn find_v8i_path() -> Option<String> {
    default_v8i_paths()
        .into_iter()
        .find(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_file_connection() {
        let (conn, ty) = parse_connection_string("File=\"C:\\base\\demo\"").unwrap();
        assert_eq!(conn, "File=\"C:\\base\\demo\"");
        assert_eq!(ty, "file");
    }

    #[test]
    fn parses_server_connection() {
        let (conn, ty) = parse_connection_string("Srvr=\"srv1\";Ref=\"УПП\"").unwrap();
        assert_eq!(conn, "Srvr=\"srv1\";Ref=\"УПП\"");
        assert_eq!(ty, "server");
    }

    #[test]
    fn parses_short_server_connection() {
        let (_, ty) = parse_connection_string("S=\"srv/db\"").unwrap();
        assert_eq!(ty, "server");
    }

    #[test]
    fn parses_full_v8i_content() {
        let content = r#"[ДемоБаза]
Connect=File="C:\1C\Demo"
ID=12345
Folder=C:\1C

[Серверная]
Connect=Srvr="SRV01";Ref="УПП"
"#;
        let bases = parse_v8i_content(content);
        assert_eq!(bases.len(), 2);
        assert_eq!(bases[0].name, "ДемоБаза");
        assert_eq!(bases[0].r#type, "file");
        assert_eq!(bases[0].id.as_deref(), Some("12345"));
        assert_eq!(bases[0].folder.as_deref(), Some("C:\\1C"));
        assert_eq!(bases[1].name, "Серверная");
        assert_eq!(bases[1].r#type, "server");
        assert_eq!(bases[1].id, None);
    }

    #[test]
    fn skips_broken_connections() {
        let content = "[БазаБезСоединения]\nID=1\n\n[ОК]\nConnect=File=\"C:\\ok\"\n";
        let bases = parse_v8i_content(content);
        assert_eq!(bases.len(), 1);
        assert_eq!(bases[0].name, "ОК");
    }

    #[test]
    fn strips_bom() {
        let content = "\u{FEFF}[База]\nConnect=File=\"C:\\b\"\n";
        let bases = parse_v8i_content(content);
        assert_eq!(bases.len(), 1);
        assert_eq!(bases[0].name, "База");
    }
}
