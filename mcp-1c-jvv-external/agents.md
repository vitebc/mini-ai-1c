# mcp-1c-jvv — MCP-сервер платформ 1С и списка баз (локальное подключение)

> Standalone Rust-бинарник, транспорт `stdio`. Не требует Node.js, Tauri, ключей или `enterprise.json`. Читает локальные каталоги Windows / `ibases.v8i`.

Исходник: `tauri-app/mcp-1c-jvv/` — порт `tauri-app/src/mcp-servers/jvv-1c.ts` (`PORTED_FROM` → коммит `67629d2`).
Собранный бинарник: `tauri-app/src-tauri/mcp-servers/mcp-1c-jvv` (`.exe` на Windows), собирается скриптом `tauri-app/scripts/build-mcp.mjs`.

**Готовые архивы в этой папке:**
- `mcp-1c-jvv-linux-x64.tar.gz` — Linux x64 (856K)
- `mcp-1c-jvv-macos-arm64.tar.gz` — macOS Apple Silicon (744K, `aarch64`)
- `mcp-1c-jvv-macos-x64.tar.gz` — macOS Intel (800K, `x86_64`)
- `mcp-1c-jvv-macos-universal.tar.gz` — macOS Universal (1.6M, оба арха, `lipo`)
- `mcp-1c-jvv-windows-x64.tar.gz` / `.zip` — Windows x64 (806K, `PE32+ x86-64`, `gnu`)

---

## 1. Что делает

| Инструмент | Описание |
|---|---|
| `list_infobases` | Список баз из `ibases.v8i`: `name`, `connection`, `type` (`file`/`server`), `id`, `folder`. Параметр `v8i_path` — опциональный путь к конкретному файлу |
| `find_platform` | Установленные платформы 1С (`1cv8.exe`) — сканирует `Program Files\1cv8\X.Y.Z.W\bin`, сортирует по версии по убыванию. Отдаёт `version`, `bin_path`, `exe_path`, `ibcmd_path`, `cestart_path` |
| `get_1c_environment` | Комбо: `platforms` + `infobases` + `v8i_path` за один вызов |

Где ищет `ibases.v8i` (`mcp-1c-jvv/src/v8i.rs:31`):

```
%USERPROFILE%\AppData\Roaming\1C\1CEStart\ibases.v8i
%USERPROFILE%\AppData\Roaming\1C\1cv8\ibases.v8i
%ProgramData%\1C\1CEStart\ibases.v8i
%ProgramData%\1C\1cv8\ibases.v8i
```

Где ищет платформы (`mcp-1c-jvv/src/platform.rs:24`):

```
%PROGRAMFILES%\1cv8\X.Y.Z.W\bin\1cv8.exe
%PROGRAMFILES(X86)%\1cv8\X.Y.Z.W\bin\1cv8.exe
→ common\1cestart.exe
```

Статус в `stderr` при старте (`mcp-1c-jvv/src/main.rs:16`): `1C_ENV_STATUS:ready:N platforms, M bases` или `unavailable` — парсится `mcp_client.rs`.

Env-переменные **не требуются**. Единственный опциональный параметр — `v8i_path` в `list_infobases`.

---

## 2. Сборка (локально, один раз)

### 2.0 Linux / Windows (общий)

```bash
# Из корня репо — соберёт все 6 Rust MCP (включая jvv) в src-tauri/mcp-servers/
npm run build:mcp --workspace=tauri-app
# или напрямую:
node tauri-app/scripts/build-mcp.mjs mcp

# Только jvv (вручную):
cargo build --release --manifest-path tauri-app/mcp-1c-jvv/Cargo.toml
# → tauri-app/mcp-1c-jvv/target/release/mcp-1c-jvv[.exe]
# скрипт скопирует его в tauri-app/src-tauri/mcp-servers/
```

### 2.1 macOS (Intel и Apple Silicon)

На macOS бинарник **собирается так же** — одной командой. Кросc-сборка с Linux на macOS не нужна, просто собери на самом Mac.

```bash
# 1. Rust (если нет):
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 2. Клонируй ветку и собери:
git clone https://github.com/vitebc/mini-ai-1c.git
cd mini-ai-1c
git checkout feature/mcp-1c-jvv-external

# вариант A — через скрипт (соберёт все 6 в src-tauri/mcp-servers/)
npm run build:mcp --workspace=tauri-app

# вариант B — только jvv (быстрее, ~12 сек на M1):
cargo build --release --manifest-path tauri-app/mcp-1c-jvv/Cargo.toml
# → tauri-app/mcp-1c-jvv/target/release/mcp-1c-jvv
# → копия → tauri-app/src-tauri/mcp-servers/mcp-1c-jvv
chmod +x tauri-app/src-tauri/mcp-servers/mcp-1c-jvv
```

Архитектура определяется автоматически: на Apple Silicon (`arm64`) соберётся `aarch64`, на Intel — `x86_64`. Универсальный бинарь (опционально):

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --manifest-path tauri-app/mcp-1c-jvv/Cargo.toml --target aarch64-apple-darwin
cargo build --release --manifest-path tauri-app/mcp-1c-jvv/Cargo.toml --target x86_64-apple-darwin
lipo -create \
  tauri-app/mcp-1c-jvv/target/aarch64-apple-darwin/release/mcp-1c-jvv \
  tauri-app/mcp-1c-jvv/target/x86_64-apple-darwin/release/mcp-1c-jvv \
  -output tauri-app/src-tauri/mcp-servers/mcp-1c-jvv
```

Снятие карантина (если скачивал архив с Linux):

```bash
xattr -d com.apple.quarantine tauri-app/src-tauri/mcp-servers/mcp-1c-jvv 2>/dev/null; true
```

Проверка бинаря:

```bash
ls -lh tauri-app/src-tauri/mcp-servers/mcp-1c-jvv*
file tauri-app/src-tauri/mcp-servers/mcp-1c-jvv
# Mach-O 64-bit executable arm64/x86_64
./tauri-app/src-tauri/mcp-servers/mcp-1c-jvv --help 2>&1 | head
# Должен стартовать как MCP stdio — без аргументов ждёт JSON-RPC на stdin
```

На macOS/Linux без установленной 1С `find_platform` и `list_infobases` вернут пустые списки — это нормально (1С на Mac редка, сервер ищет только `ibases.v8i` по Windows-путям).

---

## 3. Подключение к внешним агентам (локально, stdio)

### opencode (`opencode.json` / `opencode.jsonc` в корне проекта)

`opencode` понимает `type: "local"` + `command` как массив:

```jsonc
{
  "mcp": {
    "1c-jvv": {
      "type": "local",
      "command": ["./tauri-app/src-tauri/mcp-servers/mcp-1c-jvv"],
      "enabled": true
    }
  }
}
```

Абсолютный путь (если запускаешь opencode вне репо):

```jsonc
{
  "mcp": {
    "1c-jvv": {
      "type": "local",
      "command": ["/abs/path/to/mini-ai-1c/tauri-app/src-tauri/mcp-servers/mcp-1c-jvv"],
      "enabled": true
    }
  }
}
```

Windows:

```jsonc
{
  "mcp": {
    "1c-jvv": {
      "type": "local",
      "command": ["./tauri-app/src-tauri/mcp-servers/mcp-1c-jvv.exe"],
      "enabled": true
    }
  }
}
```

macOS (путь с `$HOME` — раскрой заранее):

```jsonc
{
  "mcp": {
    "1c-jvv": {
      "type": "local",
      // Apple Silicon / Intel — один бинарь, без .exe
      "command": ["/Users/you/mini-ai-1c/tauri-app/src-tauri/mcp-servers/mcp-1c-jvv"],
      "enabled": true
    }
  }
}
```

macOS — относительный путь (если `opencode` запускаешь из корня репо):

```jsonc
{
  "mcp": {
    "1c-jvv": {
      "type": "local",
      "command": ["./tauri-app/src-tauri/mcp-servers/mcp-1c-jvv"],
      "enabled": true
    }
  }
}
```

### Claude Code / Cursor / Windsurf / Zed (`.mcp.json`)

Тот же формат — поле `command` + `args` (env не нужен):

```json
{
  "mcpServers": {
    "1c-jvv": {
      "command": "/abs/path/to/mcp-1c-jvv",
      "args": []
    }
  }
}
```

macOS пример (`.mcp.json`):

```json
{
  "mcpServers": {
    "1c-jvv": {
      "command": "/Users/you/mini-ai-1c/tauri-app/src-tauri/mcp-servers/mcp-1c-jvv",
      "args": []
    }
  }
}
```

### MCP Inspector (проверка без агента)

```bash
npx @modelcontextprotocol/inspector ./tauri-app/src-tauri/mcp-servers/mcp-1c-jvv
# → открой http://localhost:6274, вызови tools/list, затем find_platform / get_1c_environment

# macOS — тот же вызов:
npx @modelcontextprotocol/inspector /Users/you/mini-ai-1c/tauri-app/src-tauri/mcp-servers/mcp-1c-jvv
```

---

## 4. Проверка что работает

1. `opencode mcp list` — должен показать `1c-jvv` со статусом `connected`.
2. В чате агента вызови `get_1c_environment` — вернётся JSON с `platforms.count` и `infobases.count`.
3. Если `0 platforms, 0 bases` — проверь наличие `ibases.v8i` и `C:\Program Files\1cv8\`.

Ручная проверка через stdio (без клиента):

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | ./tauri-app/src-tauri/mcp-servers/mcp-1c-jvv
# → {"result":{"tools":[{"name":"list_infobases",...},{"name":"find_platform",...},{"name":"get_1c_environment",...}]}}
```

---

## 5. Примечания

- Бинарник релизный (`opt-level=3`, `lto`, `strip` — `Cargo.toml:16`), зависимости: `tokio`, `serde`, `regex`, `dirs`.
- TS-оригинал `tauri-app/src/mcp-servers/jvv-1c.ts` — эталон, не менять. При изменениях TS синхронизируй Rust-порт (`scripts/check-mcp-parity.sh`, `PORTED_FROM`).
- Для внешнего использования вне Tauri HTTP-обёртка не нужна — достаточно stdio. HTTP→stdio Gateway есть в `ai-1c-server`, но для локального сценария избыточен.
