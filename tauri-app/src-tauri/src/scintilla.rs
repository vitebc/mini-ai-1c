//! Direct Scintilla editor API for 1C Configurator.
//!
//! 1C Configurator embeds a Scintilla control (class "Scintilla") as its code editor.
//! Scintilla accepts Win32 SendMessage from any external process, so we can read/write
//! code without touching the clipboard or stealing window focus.

#![cfg(windows)]

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};
use windows::Win32::Globalization::{
    MultiByteToWideChar, WideCharToMultiByte, MULTI_BYTE_TO_WIDE_CHAR_FLAGS,
};
use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, GetClassNameW, SendMessageW};

const SCI_GETTEXT: u32 = 2182;
const SCI_SETTEXT: u32 = 2181;
const SCI_GETLENGTH: u32 = 2006;
const SCI_GETSELTEXT: u32 = 2161;
const SCI_REPLACESEL: u32 = 2170;
const SCI_GETCURRENTPOS: u32 = 2008;
const SCI_LINEFROMPOSITION: u32 = 2167;
const SCI_GETSELSTART: u32 = 2143;
const SCI_GETSELEND: u32 = 2144;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveFragmentInfo {
    pub text: String,
    pub start_line: usize,
    pub end_line: usize,
}

struct FindContext {
    found: Option<HWND>,
}

unsafe extern "system" fn enum_child_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut FindContext);

    let mut class_buf = [0u16; 256];
    let len = GetClassNameW(hwnd, &mut class_buf);
    if len > 0 {
        let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
        if class_name.eq_ignore_ascii_case("Scintilla") {
            ctx.found = Some(hwnd);
            return BOOL(0);
        }
    }

    BOOL(1)
}

pub fn find_scintilla_control(parent_hwnd: HWND) -> Option<HWND> {
    let mut ctx = FindContext { found: None };
    unsafe {
        let _ = EnumChildWindows(
            parent_hwnd,
            Some(enum_child_proc),
            LPARAM(&mut ctx as *mut FindContext as isize),
        );
    }
    ctx.found
}

pub fn sci_get_text(sci_hwnd: HWND) -> Result<String, String> {
    unsafe {
        let len = SendMessageW(sci_hwnd, SCI_GETLENGTH, WPARAM(0), LPARAM(0)).0 as usize;
        if len == 0 {
            return Ok(String::new());
        }

        let mut buf = vec![0u8; len + 1];
        SendMessageW(
            sci_hwnd,
            SCI_GETTEXT,
            WPARAM(buf.len()),
            LPARAM(buf.as_mut_ptr() as isize),
        );

        bytes_to_string(&buf[..len])
    }
}

pub fn sci_get_seltext(sci_hwnd: HWND) -> Result<String, String> {
    unsafe {
        let needed = SendMessageW(sci_hwnd, SCI_GETSELTEXT, WPARAM(0), LPARAM(0)).0 as usize;
        if needed <= 1 {
            return Ok(String::new());
        }

        let mut buf = vec![0u8; needed];
        SendMessageW(
            sci_hwnd,
            SCI_GETSELTEXT,
            WPARAM(0),
            LPARAM(buf.as_mut_ptr() as isize),
        );

        bytes_to_string(&buf[..needed - 1])
    }
}

pub fn sci_set_text(sci_hwnd: HWND, text: &str) -> Result<(), String> {
    let bytes = string_to_bytes(text);
    unsafe {
        SendMessageW(
            sci_hwnd,
            SCI_SETTEXT,
            WPARAM(0),
            LPARAM(bytes.as_ptr() as isize),
        );
    }
    Ok(())
}

pub fn sci_replace_sel(sci_hwnd: HWND, text: &str) -> Result<(), String> {
    let bytes = string_to_bytes(text);
    unsafe {
        SendMessageW(
            sci_hwnd,
            SCI_REPLACESEL,
            WPARAM(0),
            LPARAM(bytes.as_ptr() as isize),
        );
    }
    Ok(())
}

pub fn sci_get_current_pos(sci_hwnd: HWND) -> usize {
    unsafe { SendMessageW(sci_hwnd, SCI_GETCURRENTPOS, WPARAM(0), LPARAM(0)).0 as usize }
}

pub fn sci_get_sel_start(sci_hwnd: HWND) -> usize {
    unsafe { SendMessageW(sci_hwnd, SCI_GETSELSTART, WPARAM(0), LPARAM(0)).0 as usize }
}

pub fn sci_get_sel_end(sci_hwnd: HWND) -> usize {
    unsafe { SendMessageW(sci_hwnd, SCI_GETSELEND, WPARAM(0), LPARAM(0)).0 as usize }
}

pub fn sci_line_from_position(sci_hwnd: HWND, pos: usize) -> usize {
    unsafe { SendMessageW(sci_hwnd, SCI_LINEFROMPOSITION, WPARAM(pos), LPARAM(0)).0 as usize }
}

pub fn sci_has_selection(sci_hwnd: HWND) -> bool {
    sci_get_sel_start(sci_hwnd) != sci_get_sel_end(sci_hwnd)
}

pub fn sci_get_active_fragment(sci_hwnd: HWND) -> Result<String, String> {
    if sci_has_selection(sci_hwnd) {
        return sci_get_seltext(sci_hwnd);
    }

    let text = sci_get_text(sci_hwnd)?;
    if text.trim().is_empty() {
        return Ok(text);
    }

    let current_pos = sci_get_current_pos(sci_hwnd);
    let current_line = sci_line_from_position(sci_hwnd, current_pos);

    Ok(extract_active_fragment_info(&text, current_line)
        .map(|fragment| fragment.text)
        .unwrap_or(text))
}

pub fn sci_get_active_fragment_info(sci_hwnd: HWND) -> Result<Option<ActiveFragmentInfo>, String> {
    if sci_has_selection(sci_hwnd) {
        return Ok(None);
    }

    let text = sci_get_text(sci_hwnd)?;
    if text.trim().is_empty() {
        return Ok(None);
    }

    let current_pos = sci_get_current_pos(sci_hwnd);
    let current_line = sci_line_from_position(sci_hwnd, current_pos);

    Ok(extract_active_fragment_info(&text, current_line))
}

#[cfg(test)]
fn extract_active_fragment(text: &str, current_line: usize) -> Option<String> {
    extract_active_fragment_info(text, current_line).map(|fragment| fragment.text)
}

pub(crate) fn extract_active_fragment_info(
    text: &str,
    current_line: usize,
) -> Option<ActiveFragmentInfo> {
    let lines: Vec<&str> = if text.is_empty() {
        Vec::new()
    } else {
        text.split_inclusive('\n').collect()
    };

    if lines.is_empty() {
        return None;
    }

    let clamped_line = current_line.min(lines.len().saturating_sub(1));
    let (mut start_line, end_line) = find_bsl_fragment_bounds(&lines, clamped_line)?;

    while start_line > 0 {
        let previous = lines[start_line - 1].trim();
        if previous.starts_with('&') {
            start_line -= 1;
            continue;
        }
        break;
    }

    let start_offset: usize = lines[..start_line].iter().map(|line| line.len()).sum();
    let end_offset: usize = lines[..=end_line].iter().map(|line| line.len()).sum();

    text.get(start_offset..end_offset)
        .map(|fragment| ActiveFragmentInfo {
            text: fragment.to_string(),
            start_line,
            end_line,
        })
}

fn find_bsl_fragment_bounds(lines: &[&str], current_line: usize) -> Option<(usize, usize)> {
    let start_line = (0..=current_line)
        .rev()
        .find(|&idx| is_bsl_routine_start(lines[idx].trim()))?;

    let end_line = (start_line..lines.len()).find(|&idx| is_bsl_routine_end(lines[idx].trim()))?;

    Some((start_line, end_line))
}

fn is_bsl_routine_start(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.starts_with("процедура ")
        || lower.starts_with("функция ")
        || lower.starts_with("procedure ")
        || lower.starts_with("function ")
}

fn is_bsl_routine_end(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.starts_with("конецпроцедуры")
        || lower.starts_with("конецфункции")
        || lower.starts_with("endprocedure")
        || lower.starts_with("endfunction")
}

fn bytes_to_string(raw: &[u8]) -> Result<String, String> {
    if let Ok(text) = std::str::from_utf8(raw) {
        return Ok(text.to_string());
    }

    Ok(cp1251_to_utf8(raw))
}

fn string_to_bytes(text: &str) -> Vec<u8> {
    let mut bytes = utf8_to_cp1251(text);
    bytes.push(0);
    bytes
}

fn utf8_to_cp1251(text: &str) -> Vec<u8> {
    let wide: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        let needed = WideCharToMultiByte(1251, 0, &wide, None, None, None);
        if needed <= 0 {
            crate::app_log!("[Scintilla] utf8_to_cp1251: WideCharToMultiByte size query failed");
            return Vec::new();
        }
        let mut buf = vec![0u8; needed as usize];
        let written = WideCharToMultiByte(1251, 0, &wide, Some(&mut buf), None, None);
        if written <= 0 {
            crate::app_log!("[Scintilla] utf8_to_cp1251: WideCharToMultiByte conversion failed");
            return Vec::new();
        }
        buf.truncate(written as usize);
        buf
    }
}

fn cp1251_to_utf8(bytes: &[u8]) -> String {
    unsafe {
        let needed = MultiByteToWideChar(1251, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), bytes, None);
        if needed <= 0 {
            return String::from_utf8_lossy(bytes).into_owned();
        }

        let mut buffer = vec![0u16; needed as usize];
        let written = MultiByteToWideChar(
            1251,
            MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0),
            bytes,
            Some(buffer.as_mut_slice()),
        );
        if written <= 0 {
            return String::from_utf8_lossy(bytes).into_owned();
        }

        String::from_utf16_lossy(&buffer[..written as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf8_passthrough() {
        let text = "Процедура ТестПроц()";
        let result = bytes_to_string(text.as_bytes()).unwrap();
        assert_eq!(result, text);
    }

    #[test]
    fn test_cp1251_conversion() {
        let cp1251 = vec![0xCF, 0xF0, 0xE8, 0xE2, 0xE5, 0xF2];
        let result = bytes_to_string(&cp1251).unwrap();
        assert_eq!(result, "Привет");
    }

    #[test]
    fn test_extract_active_fragment_returns_current_routine() {
        let text = concat!(
            "&НаСервере\n",
            "Процедура Первая()\n",
            "\tСообщить(\"1\");\n",
            "КонецПроцедуры\n",
            "\n",
            "Функция Вторая()\n",
            "\tВозврат 2;\n",
            "КонецФункции\n",
        );

        let fragment = extract_active_fragment(text, 6).unwrap();

        assert_eq!(fragment, "Функция Вторая()\n\tВозврат 2;\nКонецФункции\n");
    }

    #[test]
    fn test_extract_active_fragment_info_returns_bounds() {
        let text = concat!(
            "&НаСервере\n",
            "Процедура Первая()\n",
            "\tСообщить(\"1\");\n",
            "КонецПроцедуры\n",
            "\n",
            "Функция Вторая()\n",
            "\tВозврат 2;\n",
            "КонецФункции\n",
        );

        let fragment = extract_active_fragment_info(text, 6).unwrap();

        assert_eq!(fragment.start_line, 5);
        assert_eq!(fragment.end_line, 7);
        assert_eq!(
            fragment.text,
            "Функция Вторая()\n\tВозврат 2;\nКонецФункции\n"
        );
    }

    #[test]
    fn test_extract_active_fragment_includes_attached_attributes() {
        let text = concat!(
            "&НаКлиенте\n",
            "&Вместо(\"СтараяКоманда\")\n",
            "Процедура Тест()\n",
            "\tВозврат;\n",
            "КонецПроцедуры\n",
        );

        let fragment = extract_active_fragment(text, 2).unwrap();

        assert_eq!(
            fragment,
            "&НаКлиенте\n&Вместо(\"СтараяКоманда\")\nПроцедура Тест()\n\tВозврат;\nКонецПроцедуры\n"
        );
    }

    #[test]
    fn test_extract_active_fragment_returns_none_outside_routine() {
        let text = "Перем ГлобальнаяПеременная;\n\n// комментарий\n";
        assert_eq!(extract_active_fragment(text, 0), None);
    }
}
