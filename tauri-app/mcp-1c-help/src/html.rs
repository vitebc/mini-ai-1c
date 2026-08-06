//! Извлечение заголовка и читаемого текста из HTML-страницы справки.
//!
//! Порт `extractText` из 1c-help.ts. Без полноценного HTML-парсера:
//! удаляет script/style/nav, вытаскивает заголовок из `<title>`/`<h1>`/`<h2>`,
//! срезает теги регулярками и схлопывает пробелы.

/// Возвращает (title, text).
pub fn extract_text(html: &str) -> (String, String) {
    // Заголовок
    let title = extract_title(html);

    // Убираем script/style/nav — содержимое до 10000 символов достаточно.
    let mut cleaned = html.to_string();
    cleaned = remove_tags_with_content(&cleaned, &["script", "style", "nav"]);
    cleaned = remove_by_class(&cleaned, &["toc", "navigation"]);

    // Срезаем все теги
    let text = strip_tags(&cleaned);
    let text = collapse_ws(&text);
    let text: String = text.chars().take(10000).collect();

    (title, text)
}

fn extract_title(html: &str) -> String {
    // <title>...</title>
    if let Some(t) = between(&html, "<title", "</title>") {
        let cleaned = strip_tags(&t);
        let cleaned = collapse_ws(&cleaned);
        if !cleaned.is_empty() {
            return cleaned;
        }
    }
    // <h1 ...>...</h1> или <h2 ...>...</h2>
    for tag in ["<h1", "<h2"] {
        let open = format!("{}", tag);
        let close = tag.replace("h1", "/h1").replace("h2", "/h2");
        if let Some(t) = between(&html, &open, &close) {
            let cleaned = collapse_ws(&strip_tags(&t));
            if !cleaned.is_empty() {
                return cleaned;
            }
        }
    }
    "Без названия".to_string()
}

fn between<'a>(haystack: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = haystack.find(open)?;
    let after = &haystack[start..];
    // Пропускаем до конца открывающего тега (после '>')
    let gt = after.find('>')?;
    let after = &after[gt + 1..];
    let end = after.find(close)?;
    Some(&after[..end])
}

/// Удаляет `<script>...</script>` и подобные блоки (без учёта атрибутов внутри).
fn remove_tags_with_content(input: &str, tags: &[&str]) -> String {
    let mut out = input.to_string();
    for tag in tags {
        let re = regex::Regex::new(&format!(r"(?is)<{tag}[\s>].*?</{tag}\s*>")).unwrap();
        out = re.replace_all(&out, " ").to_string();
    }
    out
}

/// Удаляет элементы с class="toc"/"navigation" и их содержимое.
fn remove_by_class(input: &str, classes: &[&str]) -> String {
    let mut out = input.to_string();
    for class in classes {
        let re = regex::Regex::new(&format!(
            r#"(?is)<(?:div|span|table|ul|ol|nav)[^>]*class\s*=\s*["'][^"']*{class}[^"']*["'][^>]*>.*?</(?:div|span|table|ul|ol|nav)>"#
        ))
        .unwrap();
        out = re.replace_all(&out, " ").to_string();
    }
    out
}

fn strip_tags(input: &str) -> String {
    let re = regex::Regex::new(r"(?s)<[^>]*>").unwrap();
    re.replace_all(input, " ").to_string()
}

fn collapse_ws(input: &str) -> String {
    let re = regex::Regex::new(r"\s+").unwrap();
    re.replace_all(input, " ").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_title_and_text() {
        let html = r#"<html><head><title>Метод Выполнить</title></head><body>
            <script>var x = 1;</script>
            <div class="toc">Оглавление</div>
            <h1>Метод Выполнить()</h1>
            <p>Выполняет   команду.</p>
            <p>Второй абзац.</p>
        </body></html>"#;
        let (title, text) = extract_text(html);
        assert_eq!(title, "Метод Выполнить");
        assert!(text.contains("Выполняет"));
        assert!(text.contains("Второй абзац"));
        assert!(!text.contains("var x = 1"));
        assert!(!text.contains("Оглавление"));
    }

    #[test]
    fn falls_back_when_no_title() {
        let (title, _) = extract_text("<body><p>текст</p></body>");
        assert_eq!(title, "Без названия");
    }

    #[test]
    fn collapses_whitespace() {
        assert_eq!(collapse_ws("a   b\n\t c"), "a b c");
    }
}
