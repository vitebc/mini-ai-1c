# 1c-jvv MCP Server

MCP-сервер для обнаружения установленных платформ 1С:Предприятие и списка информационных баз из `ibases.v8i`.

Порт `jvv-1c.ts` (TypeScript/Node.js) на Rust. Standalone-бинарник — **не требует Node.js** и Tauri. Подключается к любому MCP-клиенту: opencode, Cursor, Claude Desktop.

## 📦 Сборка

```bash
cargo build --release
# → target/release/mcp-1c-jvv (один исполняемый файл)
```

## 🛠 Инструменты

| Инструмент | Описание |
|---|---|
| `list_infobases` | Список баз из ibases.v8i: имя, строка подключения, тип (файловая/серверная), ID, папка |
| `find_platform` | Установленные версии 1С (1cv8.exe) по убыванию, пути к 1cv8.exe/ibcmd.exe/1cestart.exe |
| `get_1c_environment` | Всё сразу: платформы + базы + путь к ibases.v8i |

## 📁 Где ищет ibases.v8i

```
%USERPROFILE%\AppData\Roaming\1C\1CEStart\ibases.v8i
%USERPROFILE%\AppData\Roaming\1C\1cv8\ibases.v8i
%ProgramData%\1C\1CEStart\ibases.v8i
%ProgramData%\1C\1cv8\ibases.v8i
```

## 🔍 Где ищет платформы

```
%PROGRAMFILES%\1cv8\X.Y.Z.W\bin\1cv8.exe
%PROGRAMFILES(X86)%\1cv8\X.Y.Z.W\bin\1cv8.exe
```

1cestart.exe ищется в `%PROGRAMFILES%\1cv8\common\1cestart.exe`.

## 🔌 Подключение (opencode)

```json
{
  "mcpServers": {
    "1c-jvv": {
      "command": "/path/to/mcp-1c-jvv"
    }
  }
}
```

## 📤 Статус

При старте в stderr выводится строка `1C_ENV_STATUS:ready:N platforms, M bases` или `1C_ENV_STATUS:unavailable` — парсится клиентом для отображения состояния.
