# План: MCP-сервер доступа к файлам (sandbox)

## Проблема

Агент (LLM) в Mini AI 1C не имеет доступа к файловой системе. Весь доступ к файлам
идёт только через специализированные MCP-серверы, каждый из которых имеет узкую
область видимости:

| MCP-сервер | Что может читать |
|---|---|
| `mcp-1c-search` | Только `.bsl`/`.xml` в папках конфигурации (`ONEC_CONFIG_PATH`) |
| `1c-help` | Только `.hbk` файлы платформы 1С |
| `1c-naparnik` | Ничего — HTTP-прокси на code.1c.ai |
| `1c-metadata` | Ничего — HTTP-прокси на localhost |

Общего доступа к файлам нет. Нужен MCP-сервер с доступом к файлам,
ограниченным sandbox-директорией.

## Решение

Новый **Stdio MCP-сервер** на Node.js/TypeScript (по образцу `1c-help.ts`,
`1c-naparnik.ts`). Все операции с файлами — только внутри sandbox-директории,
задаваемой через env-переменную.

```
AI Agent (LLM)
  → MCP Client (mcp_client.rs)
    → 1c-filesystem.cjs (Node.js, stdio)
      → файлы ТОЛЬКО внутри sandbox-папки
```

## 1. Новый файл: `src/mcp-servers/1c-filesystem.ts`

Тип: MCP Server (`StdioServerTransport`, `@modelcontextprotocol/sdk`).

### Env-конфигурация

- `MINI_AI_1C_SANDBOX_PATH` — корень sandbox

### Статусы (stderr, как у 1c-help / 1c-search)

- `FS_STATUS:ready` — sandbox задан и существует
- `FS_STATUS:unavailable` — sandbox не задан или не существует
- `FS_STATUS:error:<msg>` — ошибка

### Инструменты (только внутри sandbox)

| Tool | Параметры | Возвращает |
|---|---|---|
| `read_file` | `path` | `{ content, encoding: 'text'\|'base64', size }` |
| `write_file` | `path`, `content`, `encoding?` | `{ success, size }` |
| `edit_file` | `path`, `old_string`, `new_string` | `{ success, changes }` |
| `list_directory` | `path`, `pattern?` | `{ entries: [{ name, type, size, modified }] }` |
| `file_info` | `path` | `{ exists, type, size, modified, permissions }` |
| `search_files` | `pattern`, `root?` | `{ files: string[] }` |
| `create_directory` | `path` | `{ success }` |
| `delete_file` | `path` | `{ success }` — файл или пустая папка |
| `delete_directory` | `path`, `recursive?` | `{ success }` — папка, опционально рекурсивно |
| `move_file` | `source`, `destination` | `{ success }` — перемещение/переименование |

### Безопасность sandbox

- `path.resolve(sandbox, requested)` → проверка, что результат начинается с sandbox
- Блокировка `..` path traversal
- `delete_file` / `move_file` не могут выйти за sandbox
- Ошибки валидации возвращаются как tool error, не паника

## 2. Файлы для изменения

| Файл | Что делать |
|---|---|
| `src/mcp-servers/1c-filesystem.ts` | **Создать** — MCP сервер с 10 инструментами и sandbox-валидацией |
| `package.json` | Добавить `1c-filesystem.ts` в скрипт `build:mcp` |
| `src-tauri/src/mcp_client.rs:800` | Добавить `"1c-filesystem.cjs"` в `embedded_mcp_resource_bytes` |
| `src-tauri/src/settings.rs:722` | Добавить `"builtin-1c-filesystem"` в `is_builtin_node_mcp_server` |
| `src/components/settings/MCPSettings.tsx` | Константа, injection в `useEffect`, карточка с directory picker, ORDER |
| `src-tauri/src/enterprise.rs:134` | Добавить `"builtin-1c-filesystem"` в `server_ids` |

### Сборка — `package.json`

В скрипт `build:mcp` добавить:

```
&& npx esbuild src/mcp-servers/1c-filesystem.ts --bundle --platform=node --outfile=src-tauri/mcp-servers/1c-filesystem.cjs
```

### Встраивание — `mcp_client.rs:800`

```rust
"1c-filesystem.cjs" => Some(include_bytes!("../mcp-servers/1c-filesystem.cjs")),
```

### Регистрация built-in — `settings.rs:722`

```rust
"builtin-1c-naparnik" | "builtin-1c-metadata" | "builtin-1c-help" | "builtin-1c-filesystem"
```

### Frontend — `MCPSettings.tsx`

- Константа `BUILTIN_1C_FILESYSTEM_ID = 'builtin-1c-filesystem'`
- Injection в `useEffect` (строки 244-386) по аналогии с help:
  - `enabled: false`, `transport: 'stdio'`
  - `command: effectiveNodePath`
  - `args: ['mcp-servers/1c-filesystem.cjs']`
  - `env: { 'MINI_AI_1C_SANDBOX_PATH': '' }`
- В карточку сервера (строки 588-594): добавить `isFilesystem` и в `isBuiltin`
- В блок `isBuiltin`: UI с directory picker для sandbox-папки (как `ONEC_HELP_PATH`)
  и индикатором статуса
- В сортировку `ORDER` (строка 363): между BSL LS и Справкой

### Карточка в UI

- Заголовок: **«Файловая система (Sandbox)»**
- Иконка: `FolderOpen`
- Поле: выбор sandbox-директории через нативный диалог
- Статус: `ready` / `unavailable` (если sandbox не задан)
- Подсказка: «Агент сможет читать и изменять файлы только внутри этой папки»

## 3. Что НЕ меняется

- Tauri capabilities (`fs:` permissions) — не нужны, MCP работает как дочерний процесс
- `ai/tools.rs` — авто-подхват через `get_available_tools()`
- `mcp_client.rs` lifecycle — Stdio-серверы обрабатываются автоматически
- Rust-структуры данных — `McpServerConfig` / `McpTransport` уже покрывают всё

## 4. Оценка объёма

| Часть | Строк |
|---|---|
| `1c-filesystem.ts` | ~250 |
| MCPSettings.tsx (UI) | ~80 |
| `build:mcp` + константы | ~10 |
| `mcp_client.rs` include_bytes | ~3 |
| `settings.rs` is_builtin | ~3 |
| `enterprise.rs` | ~1 |
| **Итого** | **~350** |
