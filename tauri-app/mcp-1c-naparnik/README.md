# 1c-naparnik MCP Server

MCP-сервер для **1С:Напарник** (`code.1c.ai`) — ИИ-консультанта по платформе 1С, стандартам разработки и БСП.

Порт `1c-naparnik.ts` (TypeScript/Node.js) на Rust. Standalone-бинарник — **не требует Node.js** и Tauri. Подключается к любому MCP-клиенту: opencode, Cursor, Claude Desktop.

## 📦 Сборка

```bash
cargo build --release
# → target/release/mcp-1c-naparnik (один исполняемый файл)
```

## 🔑 Требуется токен

```bash
export ONEC_AI_TOKEN="ваш_токен_code.1c.ai"
```

Без токена сервер завершает работу с ошибкой.

## 🛠 Инструменты

| Инструмент | Описание |
|---|---|
| `ask_1c_ai` | Вопрос ИИ-консультанту (1С, стандарты, БСП) |
| `explain_1c_syntax` | Объяснение синтаксиса метода/функции/объекта 1С |
| `check_1c_code` | Проверка кода 1С на ошибки/производительность/стандарты |

## 🔁 Сессии

- Переиспользуются (до 10 сессий, TTL 1 час).
- `ask_1c_ai(create_new_session=true)` — принудительно новая сессия.
- При получении `tool_calls` от сервера отправляется статус `rejected` (сервер не исполняет инструменты локально).

## 🔌 Подключение (opencode)

```json
{
  "mcpServers": {
    "1c-naparnik": {
      "command": "/path/to/mcp-1c-naparnik",
      "env": { "ONEC_AI_TOKEN": "ваш_токен" }
    }
  }
}
```

## 🐛 Отладка

`ONEC_AI_DEBUG=true` — выводит SSE-поток и payload'ы в stderr.
