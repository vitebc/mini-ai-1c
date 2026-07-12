use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub async fn support_edit(args: Value) -> Result<String> {
    let target_path = args.get("target_path")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Параметр 'target_path' обязателен"))?;

    let set_val = args.get("set").and_then(|v| v.as_str()).filter(|s| !s.is_empty());
    let capability = args.get("capability").and_then(|v| v.as_str()).filter(|s| !s.is_empty());

    let (has_set, has_cap) = (set_val.is_some(), capability.is_some());
    if has_set == has_cap {
        return Err(anyhow!("Укажите ровно одно: -Set editable|off-support|locked  ЛИБО  -Capability on|off"));
    }

    let path = PathBuf::from(target_path);
    if !path.exists() {
        return Err(anyhow!("Путь не найден: {}", target_path));
    }

    // Find bin file by walking up
    let (bin_path, elem_uuid) = find_bin_and_uuid(&path)?;

    if !bin_path.exists() {
        return Ok("Конфигурация не на поддержке (Ext/ParentConfigurations.bin отсутствует) — переключать нечего.".to_string());
    }

    let raw = fs::read(&bin_path).context("Не удалось прочитать ParentConfigurations.bin")?;
    if raw.len() <= 32 {
        return Ok("Поддержка снята полностью (пустой ParentConfigurations.bin) — переключать нечего.".to_string());
    }

    let start = if raw.len() >= 3 && raw[0] == 0xEF && raw[1] == 0xBB && raw[2] == 0xBF { 3 } else { 0 };
    let text = String::from_utf8(raw[start..].to_vec())
        .map_err(|_| anyhow!("ParentConfigurations.bin не является валидным UTF-8"))?;

    let header_re = Regex::new(r"^\{6,(\d+),(\d+),").map_err(|_| anyhow!("Regex error"))?;
    let cap = header_re.captures(&text)
        .ok_or_else(|| anyhow!("Неизвестный формат ParentConfigurations.bin"))?;

    let g: i32 = cap[1].parse().unwrap_or(1);
    let _k: i32 = cap[2].parse().unwrap_or(0);

    if let Some(cap_val) = capability {
        return handle_capability(&text, cap_val == "on", g, &bin_path);
    }

    if let Some(set_val) = set_val {
        if g == 1 {
            return Err(anyhow!("Возможность изменения конфигурации выключена — пообъектное переключение недоступно.\nСначала: support-edit -Path {} -Capability on", target_path));
        }
        if elem_uuid.is_empty() {
            return Err(anyhow!("Не удалось определить объект по пути: {}", target_path));
        }
        return handle_set(&text, set_val, &elem_uuid, &bin_path);
    }

    Ok("".to_string())
}

fn find_bin_and_uuid(path: &PathBuf) -> Result<(PathBuf, String)> {
    let rp = if path.is_dir() { path.clone() } else {
        path.parent().map(|p| p.to_path_buf()).unwrap_or(path.clone())
    };

    let mut elem_uuid = String::new();
    let mut cfg_dir: Option<PathBuf> = None;
    let mut bin_path: Option<PathBuf> = None;

    let mut d = Some(rp.clone());
    for _ in 0..12 {
        let dir = match d { Some(ref p) => p.clone(), None => break };

        if elem_uuid.is_empty() {
            let xml_path = dir.with_extension("xml");
            if xml_path.exists() {
                if let Ok(content) = fs::read_to_string(&xml_path) {
                    if let Some(u) = extract_uuid(&content) {
                        elem_uuid = u;
                    }
                }
            }
        }

        if cfg_dir.is_none() {
            let cand = dir.join("Ext").join("ParentConfigurations.bin");
            let cfg_xml = dir.join("Configuration.xml");
            if cand.exists() || cfg_xml.exists() {
                cfg_dir = Some(dir.clone());
                bin_path = Some(cand);
            }
        }

        if !elem_uuid.is_empty() && cfg_dir.is_some() { break; }
        d = dir.parent().map(|p| p.to_path_buf());
    }

    if elem_uuid.is_empty() && cfg_dir.is_some() {
        let cfg_xml = cfg_dir.as_ref().unwrap().join("Configuration.xml");
        if cfg_xml.exists() {
            if let Ok(content) = fs::read_to_string(&cfg_xml) {
                if let Some(u) = extract_uuid(&content) {
                    elem_uuid = u;
                }
            }
        }
    }

    let bin = bin_path.unwrap_or_else(|| rp.join("Ext").join("ParentConfigurations.bin"));
    Ok((bin, elem_uuid))
}

fn extract_uuid(xml: &str) -> Option<String> {
    if let Some(pos) = xml.find(" uuid=\"") {
        let rest = &xml[pos + 7..];
        if let Some(end) = rest.find('\"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn handle_capability(text: &str, turn_on: bool, g: i32, bin_path: &PathBuf) -> Result<String> {
    let target = if turn_on { "0" } else { "1" };
    if g == target.parse::<i32>().unwrap() {
        let word = if turn_on { "включена" } else { "выключена" };
        return Ok(format!("Возможность изменения конфигурации уже {} — изменений нет.", word));
    }

    let re1 = Regex::new(r"^(\{6,)\d+(,)").unwrap();
    let result = re1.replace(&text, |caps: &regex::Captures| format!("{}{}", &caps[1], target));

    let re2 = Regex::new(r"([0-9a-fA-F\-]{36}),\d+,([0-9a-fA-F\-]{36})").unwrap();
    let result = re2.replace_all(&result, |caps: &regex::Captures| format!("{},{},{}", &caps[1], target, &caps[2]));

    let re3 = Regex::new(r"[0-2],0,([0-9a-fA-F\-]{36})").unwrap();
    let result = re3.replace_all(&result, |caps: &regex::Captures| format!("{},0,{}", target, &caps[1]));

    save_bin(bin_path, &result)?;

    if turn_on {
        Ok("Возможность изменения конфигурации ВКЛЮЧЕНА. Все объекты поставщика — на замке.\nВключайте редактирование точечно: support-edit -Path <объект> -Set editable".to_string())
    } else {
        Ok("Возможность изменения конфигурации ВЫКЛЮЧЕНА. Вся конфигурация стала read-only; пообъектные правила сброшены.".to_string())
    }
}

fn handle_set(text: &str, set_val: &str, elem_uuid: &str, bin_path: &PathBuf) -> Result<String> {
    let new_f1 = match set_val { "editable" => "1", "off-support" => "2", "locked" => "0", _ => return Err(anyhow!("Неизвестное значение -Set: {}", set_val)) };

    let escaped_uuid = regex::escape(&elem_uuid.to_lowercase());
    let pattern = format!("([0-2]),0,{}", escaped_uuid);

    let re = Regex::new(&pattern).map_err(|_| anyhow!("Regex error"))?;
    if !re.is_match(&text) {
        return Ok(format!("Объект (uuid {}) не на поддержке (своё добавление или не найден в bin) — переключать нечего.", elem_uuid));
    }

    let result = re.replace_all(&text, |caps: &regex::Captures| format!("{},0,{}", new_f1, &caps[1]));
    save_bin(bin_path, &result)?;

    let msg = match set_val {
        "editable" => format!("Объект {} переведён в режим editable — правки разрешены (обновления вендора продолжают приходить).", elem_uuid),
        "off-support" => format!("Объект {} снят с поддержки — правки свободны, обновления вендора больше не приходят.", elem_uuid),
        "locked" => format!("Объект {} возвращён на замок.", elem_uuid),
        _ => unreachable!(),
    };
    Ok(msg)
}

fn save_bin(path: &PathBuf, text: &str) -> Result<()> {
    let bom: Vec<u8> = [0xEFu8, 0xBB, 0xBF]
        .iter()
        .chain(text.as_bytes().iter())
        .copied()
        .collect();
    fs::write(path, &bom).with_context(|| format!("Не удалось записать {}", path.display()))
}
