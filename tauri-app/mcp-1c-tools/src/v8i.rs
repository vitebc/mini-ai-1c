use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfobaseInfo {
    pub name: String,
    pub connection: String,
    #[serde(rename = "type")]
    pub base_type: InfobaseType,
    pub id: Option<String>,
    pub folder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InfobaseType {
    File,
    Server,
}

const DEFAULT_V8I_PATHS: &[&str] = &[
    "AppData\\Roaming\\1C\\1CEStart\\ibases.v8i",
    "AppData\\Roaming\\1C\\1cv8\\ibases.v8i",
    "ProgramData\\1C\\1CEStart\\ibases.v8i",
    "ProgramData\\1C\\1cv8\\ibases.v8i",
];

fn parse_connection_string(connect: &str) -> Option<(String, InfobaseType)> {
    // File="E:\1cBase\KA_TD"
    if let Some(cap) = connect.to_lowercase().find("file=") {
        let rest = &connect[cap..];
        if let Some(start) = rest.find('"') {
            if let Some(end) = rest[start + 1..].find('"') {
                let path = &rest[start + 1..start + 1 + end];
                return Some((format!("File=\"{}\"", path), InfobaseType::File));
            }
        }
    }

    // Srvr="server";Ref="db"
    let has_srvr = connect.to_lowercase().contains("srvr=");
    let has_ref = connect.to_lowercase().contains("ref=");
    if has_srvr && has_ref {
        return Some((connect.to_string(), InfobaseType::Server));
    }

    // S="server/db"
    if connect.to_lowercase().starts_with("s=") {
        return Some((connect.to_string(), InfobaseType::Server));
    }

    None
}

fn parse_v8i_content(content: &str) -> Vec<InfobaseInfo> {
    let mut bases: Vec<InfobaseInfo> = Vec::new();
    let mut current_name = String::new();
    let mut current_connect = String::new();
    let mut current_id = String::new();
    let mut current_folder = String::new();

    let content = content.trim_start_matches('\u{FEFF}');

    for line in content.lines() {
        let trimmed = line.trim();

        // Section header [Name]
        if let Some(name) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            // Save previous base
            if !current_name.is_empty() && !current_connect.is_empty() {
                if let Some((conn, btype)) = parse_connection_string(&current_connect) {
                    bases.push(InfobaseInfo {
                        name: current_name.clone(),
                        connection: conn,
                        base_type: btype,
                        id: if current_id.is_empty() { None } else { Some(current_id.clone()) },
                        folder: if current_folder.is_empty() { None } else { Some(current_folder.clone()) },
                    });
                }
            }

            current_name = name.to_string();
            current_connect.clear();
            current_id.clear();
            current_folder.clear();
            continue;
        }

        // Key=Value
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim();
            let value = trimmed[eq_pos + 1..].trim();

            match key.to_lowercase().as_str() {
                "connect" => current_connect = value.to_string(),
                "id" => current_id = value.to_string(),
                "folder" => current_folder = value.to_string(),
                _ => {}
            }
        }
    }

    // Save last base
    if !current_name.is_empty() && !current_connect.is_empty() {
        if let Some((conn, btype)) = parse_connection_string(&current_connect) {
            bases.push(InfobaseInfo {
                name: current_name.clone(),
                connection: conn,
                base_type: btype,
                id: if current_id.is_empty() { None } else { Some(current_id.clone()) },
                folder: if current_folder.is_empty() { None } else { Some(current_folder.clone()) },
            });
        }
    }

    bases
}

pub fn parse_v8i_file(v8i_path: Option<&str>) -> Vec<InfobaseInfo> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    let prog_data = std::env::var("ProgramData").unwrap_or_default();

    let search_paths: Vec<String> = if let Some(path) = v8i_path {
        vec![path.to_string()]
    } else {
        DEFAULT_V8I_PATHS.iter().map(|rel| {
            if rel.starts_with("ProgramData") {
                Path::new(&prog_data).join(rel.trim_start_matches("ProgramData\\"))
                    .to_string_lossy().to_string()
            } else {
                Path::new(&home).join(rel)
                    .to_string_lossy().to_string()
            }
        }).collect()
    };

    for p in &search_paths {
        let path = Path::new(p);
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    let bases = parse_v8i_content(&content);
                    if !bases.is_empty() {
                        return bases;
                    }
                }
                Err(_) => continue,
            }
        }
    }

    Vec::new()
}

pub fn find_infobase_by_name(name: &str) -> Option<InfobaseInfo> {
    let bases = parse_v8i_file(None);
    let search = name.to_lowercase();

    // Exact match first
    if let Some(found) = bases.iter().find(|b| b.name.to_lowercase() == search) {
        return Some(found.clone());
    }

    // Partial match
    bases.into_iter().find(|b| b.name.to_lowercase().contains(&search))
}

pub fn get_infobase_names() -> Vec<String> {
    parse_v8i_file(None).into_iter().map(|b| b.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_v8i_content() {
        let content = r#"[TestBase]
Connect=File="C:\bases\test"
ID=12345
Folder=C:\bases

[MyServer]
Connect=Srvr="mysql";Ref="my_db"
"#;
        let bases = parse_v8i_content(content);
        assert_eq!(bases.len(), 2);
        assert_eq!(bases[0].name, "TestBase");
        assert!(matches!(bases[0].base_type, InfobaseType::File));
        assert_eq!(bases[1].name, "MyServer");
        assert!(matches!(bases[1].base_type, InfobaseType::Server));
    }
}
