# 1c-metadata MCP Server

Тонкий прокси к **HTTP-сервису метаданных 1С** (совместим с Kharin `1c_mcp`). Пересылает стандартные MCP-запросы (tools/resources/prompts) на 1C-расширение через JSON-RPC 2.0 по HTTP.

Порт `1c-metadata.ts` (TypeScript/Node.js) на Rust. Standalone-бинарник — **не требует Node.js** и Tauri.

## 📦 Сборка

```bash
cargo build --release
# → target/release/mcp-1c-metadata
```

## 🔌 Переменные окружения

| Переменная | Описание | По умолчанию |
|---|---|---|
| `ONEC_METADATA_URL` | Базовый URL HTTP-сервиса 1С | `http://localhost/base/hs/mcp` |
| `ONEC_USERNAME` | Логин (Basic auth) | пусто |
| `ONEC_PASSWORD` | Пароль (Basic auth) | пусто |
| `ONEC_AI_DEBUG` | `true` — логировать запросы в stderr | — |

## 🔁 Проксируемые методы

`tools/list`, `tools/call`, `resources/list`, `resources/read`, `prompts/list`, `prompts/get` → `{base}/rpc`.

## 🔌 Подключение (opencode)

```json
{
  "mcpServers": {
    "1c-metadata": {
      "command": "/path/to/mcp-1c-metadata",
      "env": {
        "ONEC_METADATA_URL": "http://server/base/hs/mcp",
        "ONEC_USERNAME": "login",
        "ONEC_PASSWORD": "password"
      }
    }
  }
}
```
