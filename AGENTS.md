# Mini AI 1C — Enterprise агент для разработки на 1С:Предприятие

## О проекте

Mini AI 1C — десктопное приложение (Tauri 2 + React 19) для AI-ассистированной разработки на платформе 1С:Предприятие.  
Работает с Конфигуратором через EditorBridge (.NET named pipe), анализирует BSL-код, генерирует и применяет изменения.

**Разработка переезжает на VPS-сервер.**  
Репозиторий серверной части: `E:\1C_AI\ai-1c-server` / `https://github.com/vitebc/ai-1c-server`

---

## Структура проекта

```
mini-ai-1c/
├── tauri-app/                    # Основное приложение
│   ├── src/                      # Frontend (React 19, TypeScript, Vite, Tailwind v4, Monaco)
│   │   ├── api/                  # Tauri invoke-обёртки
│   │   ├── components/           # UI компоненты
│   │   │   ├── chat/             # Чат, SessionPanel, MCP-панели
│   │   │   ├── layout/           # Header, MainLayout
│   │   │   ├── settings/         # Настройки: LLM, MCP, BSL, Configurator, Skills
│   │   │   └── ...
│   │   ├── contexts/             # React Context: Chat, Settings, Configurator, BSL, Profile
│   │   ├── hooks/                # useChatSessions, useCodeSession
│   │   ├── mcp-servers/          # MCP-серверы TypeScript (1c-help, naparnik, metadata, skills)
│   │   └── utils/                # Утилиты
│   ├── src-tauri/                # Backend (Rust)
│   │   └── src/
│   │       ├── lib.rs            # Точка входа, Tauri setup
│   │       ├── settings.rs       # Настройки, AppSettings, EnterpriseConfig, пути
│   │       ├── mcp_client.rs     # MCP менеджер (Http/Stdio/Internal transport)
│   │       ├── enterprise.rs     # Enterprise режим: fetch + merge конфига с сервера
│   │       ├── bsl_client.rs     # BSL Language Server WebSocket клиент
│   │       ├── editor_bridge.rs  # EditorBridge (.NET named pipe)
│   │       ├── llm_profiles.rs   # LLM профили с AES-GCM шифрованием ключей
│   │       └── commands/         # Tauri команды
│   │           ├── skills.rs     # CRUD локальных скиллов
│   │           ├── enterprise.rs # Команды enterprise-режима
│   │           └── ...
│   ├── mcp-1c-search/            # Отдельный Rust MCP-сервер для поиска по 1С-конфигурации
│   └── scripts/                  # dev.ps1, build.ps1, portable.ps1, mock-enterprise-server.mjs
├── scripts/                      # dev.ps1, build.ps1 (запуск из корня)
├── .agents/skills/               # Скиллы для AI-агента (копируются в ~/.config/mini-ai-1c/.agents/skills/)
└── AGENTS.md                     # Этот файл
```

---

## Что сделано (реализованные фичи)

### 1. Панель сессий (SessionsPanel)
- Левая боковая панель с деревом сессий
- Иерархия: Конфигурация → Объект 1С → Модуль → Чаты
- Переключение чатов, удаление, создание новых
- Ресайз панели (280-480px), темы (light/dark), состояние открыта/закрыта

### 2. Enterprise-режим
- `enterprise.json` рядом с EXE → при старте `GET {server}/api/client/config`
- Deep-merge конфига с сервера в локальные настройки
- Graceful degradation при недоступности сервера
- Авто-очистка устаревших enterprise-настроек при отсутствии `enterprise.json`

### 3. Единый конфиг-путь
- Все данные перенесены в `$HOME/.config/mini-ai-1c/`
- Миграция со старых путей при первом запуске
- EditorBridge path auto-fix после миграции

### 4. MCP-сервер mcp-skills
- TypeScript stdio-сервер (как 1c-help, naparnik, metadata)
- Инструменты: `list_skills`, `get_skill`, `search_skills`
- Двухуровневая структура скиллов: `<category>/<skill>/SKILL.md`
- Встроен `include_bytes!` + авто-билд через esbuild

### 5. Вкладка «Скиллы» в настройках
- Слева список скиллов, справа Markdown-редактор + превью
- CRUD через Rust-команды (list, get, save, delete)

### 6. Portable сборка
- `scripts/build-portable.ps1` — ZIP с EXE + MCP-серверами + enterprise.json

---

## Архитектура конфигурации

```
$HOME/.config/mini-ai-1c/
├── settings.json                 # Основные настройки
├── llm_profiles.json             # LLM-профили (API ключи зашифрованы AES-GCM)
├── .key                          # Мастер-ключ шифрования (AES-256-GCM)
├── bin/                          # EditorBridge.exe, BSL LS .jar
├── bsl-workspace/                # Рабочая директория BSL Language Server
├── search-index/                 # SQLite-индексы mcp-1c-search
├── .agents/skills/               # Скиллы (категория/скилл/SKILL.md)
│   ├── bsl/                      
│   │   ├── common-module/SKILL.md
│   │   └── managed-form/SKILL.md
│   └── cc-1c/
│       └── ...
└── qwen-usage-*.json             # Счётчики использования Qwen CLI
```

---

## Enterprise-архитектура (клиент-сервер)

```
┌────────────────────────────────┐
│  Сервер (ai-1c-server, Rust)   │
│  - MCP Gateway (HTTP→stdio)    │
│  - BSL LS WebSocket            │
│  - Config API                  │
│  - Skills Registry             │
│  - File Watcher + Indexer      │
│  - Admin Dashboard (React SPA) │
│  - Updater (версии клиента)    │
└──────────────┬─────────────────┘
               │ HTTP / WebSocket
┌──────────────▼─────────────────┐
│  Клиент (mini-ai-1c portable)  │
│  - enterprise.json → настройки │
│  - MCP через HTTP к серверу    │
│  - BSL LS через WebSocket      │
│  - EditorBridge локально       │
│  - Авто-обновление с сервера   │
└────────────────────────────────┘
```

---

## Команды

| Действие | Команда (из `tauri-app/`) |
|----------|--------------------------|
| Dev сервер | `npm run app:dev` |
| Production build | `npm run app:build` |
| Portable ZIP | `powershell -File scripts/build-portable.ps1` |
| TypeScript check | `npx tsc --noEmit` |
| Rust check | `cargo check` (из `src-tauri/`) |
| Собрать MCP-серверы | `npm run build:mcp` |
| Собрать mcp-1c-search | `npm run build:mcp-search` |
| Скрипты из корня | `scripts/dev.ps1`, `scripts/build.ps1`, `scripts/portable.ps1` |

---

## Планы (ai-1c-server)

| Этап | Описание |
|------|----------|
| 1 | Каркас сервера: axum + SQLite + CLI args |
| 2 | MCP Gateway: subprocess manager, HTTP→stdio proxy |
| 3 | Config API: endpoint для клиентов |
| 4 | File Watcher + интеграция mcp-1c-search |
| 5 | Skills Registry: CRUD + API |
| 6 | Admin Dashboard: React SPA |
| 7 | Updater: версионирование, авто-обновление клиента |
| 8 | Аутентификация: токены |
| 9 | Linux systemd service + Windows service |
| 10 | Документация |

---

## Git workflow

- **Коммитить после каждого изменения**, даже мелкого.
- **Пушить сразу после каждого коммита** (`git push origin main`).
- Перед коммитом: `git status`, `git diff`, проверить артефакты.
- Формат сообщения: кратко, на русском, суть.
- Ветка `main` — основная разработка.
- Серверная часть — отдельный репозиторий `vitebc/ai-1c-server`.

---

## Зависимости

- **Rust**: Tauri 2, axum, tokio, rusqlite, reqwest, notify, aes-gcm
- **Frontend**: React 19, TypeScript, Vite 7, Tailwind CSS v4, Monaco Editor, Radix UI, lucide-react
- **MCP**: @modelcontextprotocol/sdk (TypeScript), tree-sitter-bsl (Rust)
- **Системные**: Java 17+ (BSL LS), Node.js 18+ (MCP-серверы), WebView2 Runtime
- **EditorBridge**: .NET 8 self-contained single-file EXE
