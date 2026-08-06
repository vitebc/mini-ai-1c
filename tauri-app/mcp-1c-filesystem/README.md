# 1c-filesystem MCP Server

MCP-сервер для безопасных файловых операций в изолированном каталоге (sandbox). Работает с BSL-модулями, XML-выгрузками конфигурации, внешними обработками и другими файлами.

Порт `1c-filesystem.ts` (TypeScript/Node.js) на Rust. Standalone-бинарник — **не требует Node.js** и Tauri. Подключается к любому MCP-клиенту: opencode, Cursor, Claude Desktop.

## 📦 Сборка

```bash
cargo build --release
# → target/release/mcp-1c-filesystem (один исполняемый файл)
```

## 🛠 Инструменты (11)

| Инструмент | Описание |
|---|---|
| `read_file` | Чтение файла (текст + размер) |
| `write_file` | Запись/перезапись файла (создаёт родительские каталоги) |
| `edit_file` | Поиск и замена точной строки в файле |
| `list_directory` | Список содержимого каталога (тип, размер, дата) |
| `file_info` | Метаданные файла/каталога: существует, тип, размер, права |
| `search_files` | Рекурсивный поиск по glob-паттерну (например `**/*.bsl`) |
| `create_directory` | Создание каталога (включая родительские) |
| `delete_file` | Удаление файла или пустого каталога |
| `delete_directory` | Удаление каталога (рекурсивно опционально) |
| `move_file` | Перемещение/переименование в пределах sandbox |
| `run_command` | Запуск shell-команды внутри sandbox (stdout/stderr/exit_code) |

## 🔒 Безопасность

- Все пути резолвятся относительно `MINI_AI_1C_SANDBOX_PATH` (переменная окружения).
- Выход за пределы sandbox запрещён: `..`, абсолютные пути и симлинки отклоняются.
- `run_command` ограничен таймаутом (1–300 сек) и буфером вывода 10 МБ.

## 🔌 Подключение (opencode)

```json
{
  "mcpServers": {
    "1c-filesystem": {
      "command": "/path/to/mcp-1c-filesystem",
      "env": { "MINI_AI_1C_SANDBOX_PATH": "/path/to/sandbox" }
    }
  }
}
```

## 📤 Статус

При старте в stderr выводится `FS_STATUS:ready` / `FS_STATUS:unavailable` / `FS_STATUS:error:...` — парсится клиентом.
