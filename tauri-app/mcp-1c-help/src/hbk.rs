//! Парсер HBK-файлов справки 1С:Предприятие.
//!
//! Порт hbk-parser.ts. Формат:
//! - [Header 16 bytes]: firstFreeBlock(4) + defaultBlockSize(4) + unknown(8);
//! - Блоки: CRLF + payloadSize(8 hex) + SPACE + blockSize(8 hex) + SPACE + nextBlock(8 hex) + SPACE+CRLF
//!   = 31 байт заголовок; nextBlock = 0x7fffffff означает конец цепочки;
//! - TOC (первый блок, offset=16): 7 записей FileInfo по 12 байт
//!   (headerAddr 4b, bodyAddr 4b, reserved 4b) — прямые байтовые смещения;
//! - FileStorage: ZIP-архив с HTML-страницами (метод 0 stored / 8 deflate).

use flate2::read::DeflateDecoder;
use std::io::Read;
use std::path::Path;

/// Страница справки: имя внутри ZIP + HTML-содержимое.
pub struct HbkPage {
    pub name: String,
    pub html: String,
}

const ZIP_LFH_SIG: u32 = 0x0403_4b50;
const MAX_BLOCKS: usize = 100_000; // защита от бесконечных цепочек

fn hex_usize(buf: &[u8], start: usize, len: usize) -> usize {
    std::str::from_utf8(&buf[start..start + len])
        .ok()
        .and_then(|s| usize::from_str_radix(s.trim(), 16).ok())
        .unwrap_or(0)
}

/// Читает заголовок блока по прямому байтовому смещению.
fn read_block(buf: &[u8], raw_offset: usize) -> Option<(usize, usize, Option<usize>, usize)> {
    if raw_offset + 31 > buf.len() {
        return None;
    }
    let p = raw_offset + 2; // skip CRLF
    let payload_size = hex_usize(buf, p, 8);
    let p = p + 9; // hex + SPACE
    let _block_size = hex_usize(buf, p, 8);
    let p = p + 9; // hex + SPACE
    let next_hex = hex_usize(buf, p, 8);
    let data_start = p + 11; // hex + SPACE + CRLF
    let next_raw = if next_hex == 0x7fff_ffff {
        None
    } else {
        Some(next_hex)
    };
    Some((payload_size, _block_size, next_raw, data_start))
}

/// Читает всю цепочку блоков начиная с raw_offset, конкатенирует данные.
fn read_entity_full(buf: &[u8], raw_offset: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut off = Some(raw_offset);
    let mut blocks = 0;
    while let Some(o) = off {
        blocks += 1;
        if blocks > MAX_BLOCKS {
            break;
        }
        let Some((payload_size, _block_size, next_raw, data_start)) = read_block(buf, o) else {
            break;
        };
        let end = (data_start + payload_size).min(buf.len());
        if data_start < end {
            out.extend_from_slice(&buf[data_start..end]);
        }
        off = next_raw;
    }
    out
}

/// Читает TOC из первого блока HBK.
fn parse_toc(buf: &[u8]) -> Vec<(usize, usize)> {
    let Some((payload_size, _bs, _next, data_start)) = read_block(buf, 16) else {
        return Vec::new();
    };
    let end = (data_start + payload_size).min(buf.len());
    let toc_data = &buf[data_start..end];
    let count = toc_data.len() / 12;
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 12;
        let header_addr = u32::from_le_bytes([
            toc_data[base],
            toc_data[base + 1],
            toc_data[base + 2],
            toc_data[base + 3],
        ]) as usize;
        let body_addr = u32::from_le_bytes([
            toc_data[base + 4],
            toc_data[base + 5],
            toc_data[base + 6],
            toc_data[base + 7],
        ]) as usize;
        entries.push((header_addr, body_addr));
    }
    entries
}

/// Читает имя файла из header-блока (UTF-16LE, после 20 байт доп. полей).
fn read_file_name(buf: &[u8], header_raw: usize) -> String {
    let Some((payload_size, _bs, _next, data_start)) = read_block(buf, header_raw) else {
        return String::new();
    };
    let name_len = payload_size.saturating_sub(20);
    if name_len == 0 {
        return String::new();
    }
    let start = data_start + 20;
    let end = (start + name_len).min(buf.len());
    if start >= end {
        return String::new();
    }
    let bytes = &buf[start..end];
    // UTF-16LE (may contain nulls)
    let mut name = String::with_capacity(bytes.len() / 2);
    let mut chars = bytes.chunks_exact(2);
    for pair in &mut chars {
        let u = u16::from_le_bytes([pair[0], pair[1]]);
        if u != 0 {
            name.push(char::from_u32(u as u32).unwrap_or('\u{FFFD}'));
        }
    }
    name
}

// ─── ZIP ─────────────────────────────────────────────────────────────────────

struct ZipEntry<'a> {
    name: String,
    compressed_data: &'a [u8],
    comp_method: u16,
}

/// Итерирует ZIP-записи (local file headers).
fn iter_zip_entries(zip_buf: &[u8]) -> Vec<ZipEntry<'_>> {
    let mut entries = Vec::new();
    let mut pos = 0usize;
    while pos + 30 <= zip_buf.len() {
        let sig = u32::from_le_bytes([
            zip_buf[pos],
            zip_buf[pos + 1],
            zip_buf[pos + 2],
            zip_buf[pos + 3],
        ]);
        if sig != ZIP_LFH_SIG {
            break;
        }
        let comp_method = u16::from_le_bytes([zip_buf[pos + 8], zip_buf[pos + 9]]);
        let comp_size = u32::from_le_bytes([
            zip_buf[pos + 18],
            zip_buf[pos + 19],
            zip_buf[pos + 20],
            zip_buf[pos + 21],
        ]) as usize;
        let _uncomp_size = u32::from_le_bytes([
            zip_buf[pos + 22],
            zip_buf[pos + 23],
            zip_buf[pos + 24],
            zip_buf[pos + 25],
        ]) as usize;
        let name_len = u16::from_le_bytes([zip_buf[pos + 26], zip_buf[pos + 27]]) as usize;
        let extra_len = u16::from_le_bytes([zip_buf[pos + 28], zip_buf[pos + 29]]) as usize;
        let name_start = pos + 30;
        let data_start = name_start + name_len + extra_len;
        if data_start > zip_buf.len() {
            break;
        }
        let name = String::from_utf8_lossy(&zip_buf[name_start..data_start]).to_string();
        let end = (data_start + comp_size).min(zip_buf.len());
        let compressed_data = &zip_buf[data_start..end];
        entries.push(ZipEntry {
            name,
            compressed_data,
            comp_method,
        });
        pos = end;
    }
    entries
}

/// Разжимает deflate-raw или возвращает данные как есть (stored).
fn decompress(entry: &ZipEntry<'_>) -> Option<Vec<u8>> {
    match entry.comp_method {
        0 => Some(entry.compressed_data.to_vec()),
        8 => {
            let mut decoder = DeflateDecoder::new(entry.compressed_data);
            let mut out = Vec::new();
            let _ = decoder.read_to_end(&mut out);
            if out.is_empty() && !entry.compressed_data.is_empty() {
                None
            } else {
                Some(out)
            }
        }
        _ => None,
    }
}

/// Итерирует HTML-страницы из HBK файла.
pub fn parse_hbk(file_path: &Path) -> Vec<HbkPage> {
    let Ok(buf) = std::fs::read(file_path) else {
        return Vec::new();
    };
    let toc = parse_toc(&buf);

    // Ищем FileStorage по имени; fallback — второй элемент TOC
    let mut fs_body_addr: Option<usize> = None;
    for (header_addr, body_addr) in &toc {
        let name = read_file_name(&buf, *header_addr);
        if name.to_lowercase().contains("filestorage") {
            fs_body_addr = Some(*body_addr);
            break;
        }
    }
    if fs_body_addr.is_none() && toc.len() >= 2 {
        fs_body_addr = Some(toc[1].1);
    }
    let Some(fs_body_addr) = fs_body_addr else {
        return Vec::new();
    };

    let zip_buf = read_entity_full(&buf, fs_body_addr);
    if zip_buf.len() < 4 || u32::from_le_bytes([zip_buf[0], zip_buf[1], zip_buf[2], zip_buf[3]]) != ZIP_LFH_SIG {
        eprintln!("[hbk-parser] FileStorage is not a ZIP archive in {}", file_path.display());
        return Vec::new();
    }

    let entries = iter_zip_entries(&zip_buf);
    let mut pages = Vec::new();
    for entry in entries {
        if !entry.name.to_lowercase().ends_with(".html") {
            continue;
        }
        if let Some(raw) = decompress(&entry) {
            pages.push(HbkPage {
                name: entry.name.clone(),
                html: String::from_utf8_lossy(&raw).to_string(),
            });
        }
    }
    pages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_usize_parses() {
        assert_eq!(hex_usize(b"0000012C", 0, 8), 300);
        assert_eq!(hex_usize(b"00000000", 0, 8), 0);
    }

    #[test]
    fn decompress_stored_vs_deflate() {
        // stored
        let stored = ZipEntry {
            name: "a".to_string(),
            compressed_data: b"hello".as_slice(),
            comp_method: 0,
        };
        assert_eq!(decompress(&stored).unwrap(), b"hello".to_vec());
        // deflate raw
        let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        use std::io::Write;
        enc.write_all(b"deflated-data").unwrap();
        let compressed = enc.finish().unwrap();
        let deflated = ZipEntry {
            name: "b".to_string(),
            compressed_data: &compressed,
            comp_method: 8,
        };
        assert_eq!(decompress(&deflated).unwrap(), b"deflated-data".to_vec());
        // unsupported method
        let other = ZipEntry {
            name: "c".to_string(),
            compressed_data: b"x".as_slice(),
            comp_method: 99,
        };
        assert!(decompress(&other).is_none());
    }

    #[test]
    fn parse_hbk_missing_file_returns_empty() {
        let pages = parse_hbk(Path::new("/nonexistent/file.hbk"));
        assert!(pages.is_empty());
    }
}
