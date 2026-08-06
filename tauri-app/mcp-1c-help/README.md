# 1c-help MCP Server

MCP-сервер для **официальной справки платформы 1С:Предприятие 8.3**. Читает `.hbk` файлы напрямую (нативный парсер, без Java/JAR), индексирует в SQLite FTS5, ищет по встроенному языку, объектной модели и языку запросов.

Порт `1c-help.ts` + `lib/hbk-parser.ts` (TypeScript/Node.js) на Rust. Standalone-бинарник — **не требует Node.js** и Tauri. Подключается к любому MCP-клиенту: opencode, Cursor, Claude Desktop.

## 📦 Сборка

```bash
cargo build --release
# → target/release/mcp-1c-help (один исполняемый файл)
```

## 🛠 Инструменты

| Инструмент | Описание |
|---|---|
| `search_1c_help` | Полнотекстовый поиск по справке (FTS5), фильтр по разделу: syntax/query/language |
| `get_1c_help_topic` | Полное содержимое темы по `topic_id` |
| `list_1c_help_versions` | Версия платформы, количество тем, дата индексации |
| `reindex_1c_help` | Принудительная переиндексация справки |

## 📁 Поиск платформы 1С

- Windows: `C:\Program Files\1cv8`, `C:\Program Files (x86)\1cv8`
- Linux: `/opt/1cv8`, `/opt/1cv8/x86_64`, `/usr/share/1cv8`
- Кастомный путь: переменная окружения `ONEC_HELP_PATH` (папка с `8.x.x.x/bin/shcntx_ru.hbk`)

## 🗂 Хранилище

База данных: `~/.config/mini-ai-1c/help/help.db` (SQLite FTS5).
Переиндексируется автоматически при смене версии платформы или пустой базе.

## 🔌 Подключение (opencode)

```json
{
  "mcpServers": {
    "1c-help": {
      "command": "/path/to/mcp-1c-help",
      "env": { "ONEC_HELP_PATH": "/opt/1cv8" }
    }
  }
}
```

## 📤 Статус (stderr)

```
HELP_STATUS:unavailable            — платформа не найдена
HELP_STATUS:indexing:N:TOTAL:msg   — идёт индексация
HELP_STATUS:ready:VERSION:COUNT    — готов к работе
```

## ⚙️ Формат HBK

- 16-байтовый заголовок → цепочки блоков (CRLF + hex-размеры + next-адрес);
- TOC из 7 записей по 12 байт (прямые байтовые смещения);
- `FileStorage` — ZIP-архив с HTML-страницами (метод 0 stored / 8 deflate);
- Извлечение текста: срезание тегов + схлопывание пробелов, заголовок из `<title>`/`<h1>`.
