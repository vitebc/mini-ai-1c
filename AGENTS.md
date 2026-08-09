# AGENTS.md — config-skills (worktree `config/skills-content`)

## Назначение

Это worktree репозитория mini-ai-1c на ветке `config/skills-content`. Здесь собирается **личная коллекция** скиллов, правил и документации для разработки на 1С:Предприятие, которая потом переносится в другой проект.

Рабочий цикл: **брать из эталонов → редактировать под себя → коммитить в `config/`**.

⚠️ Это НЕ основной репозиторий mini-ai-1c (приложение Tauri/Rust). AGENTS.md корня (`/root/project/mini-ai-1c/AGENTS.md`) описывает приложение — он здесь не применим. Всё, что нужно знать, живёт в `config/`.

## Структура

```
config/
├── etalon/     # GITIGNORED. 4 upstream-репозитория-источника (полные git-клоны):
│               #   1c-ai-development-kit, 1c-ssl-skills, cc-1c-skills, cursor-1c-skills
├── skills/
│   ├── 1c-skills/<skill>/   # 35 скиллов. Каждый: SKILL.md + scripts/*.{ps1,py}
│   │                        #   + evals/evals.json, иногда references/, presets/
│   └── bsp/                 # Скилл БСП 3.1.11: SKILL.md + references/ (24 топика) + scripts/bsp_api.py
├── rules/                   # 29 правил: *.md с YAML frontmatter `paths:` (glob по BSL)
└── docs/                    # спецификации (1c-*-spec.md), гайды (*-guide.md),
                             # DSL-спеки (*-dsl-spec.md), index (1c-specs-index.md)
```

## Эталоны (`config/etalon/`) — только чтение

- Каждая папка — самостоятельный git-клонированный upstream-репозиторий (у каждой свой `origin`).
- **Не редактировать и не коммитить сюда** — это исходники. При необходимости правь копию в `config/`, а источник используй для сравнения (`git -C config/etalon/<name> log`, diff).
- Полезно при выборе реализации: `config/skills/1c-skills/<skill>` часто является кастомизацией скилла из `cursor-1c-skills` или `1c-ai-development-kit/.claude/skills`.

## Форматы (соблюдать при создании/редактировании)

**Скилл** (`config/skills/1c-skills/<skill>/SKILL.md`):
- **Папка скилла = `name` в frontmatter**, с префиксом `1c-` (пример: папка `1c-form-add`, `name: 1c-form-add`). Исключение — `1c-composing-1c-queries`. Не допускай расхождения папка/`name`/`skill_name` в evals.
- **Окончания строк — LF** (не CRLF): парсер frontmatter в mcp-1c-skills падает на `---\r\n` и description молча становится пустым (BM25/поиск ломается). Конвертируй: `sed -i 's/\r$//'`.
- YAML frontmatter: `name`, `description` (начинать с «Используй когда…» — ключевой сигнал для BM25-поиска и выбора скилла агентом), `argument-hint`, `allowed-tools`, `tags` (через запятую, домены: `epf`, `erf`, `forms`, `mxl`, `skd`, `db`, `query`, `template`, `bsp`, `workflow`, `help`), `depends_on` (только для workflow-скиллов: `1c-epf-full-cycle` перечисляет под-скиллы цепочки). Без кавычек вокруг значений; без многострочного YAML `description: >` — парсер его не понимает.
- Заголовок `# /<command>` — имя команды без префикса (пример: папка `1c-form-add`, заголовок `# /form-add`). Заголовки команд должны быть **уникальны** — не допускай двух скиллов с одной командой (был конфликт `1c-epf-init`/`1c-epf-scaffold`, scaffold удалён).
- Описание формата, таблица параметров, блок «## Команда» с вызовом `powershell.exe -NoProfile -File ...`. Пути в командах — **только** `<name>/scripts/<script>.ps1` (напр. `1c-form-add/scripts/form-add.ps1`), т.е. относительно каталога скиллов, **без префикса `skills/`** — агент запускает скрипт из расположения скилла, и `skills/` ломает путь. Не используй `.claude/skills/...` или `${CLAUDE_SKILL_DIR}`. Примечание: ERF-скиллы не имеют своих скриптов и переиспользуют EPF-скрипты (напр. `1c-erf-build` → `1c-epf-build/scripts/epf-build.ps1`).
- `evals/evals.json`: массив `{prompt, expected_output, expectations[]}` для проверки скилла; `skill_name` должен совпадать с `name` в frontmatter.

**PS1 vs Python** (см. `config/docs/python-porting-guide.md`):
- PS1 — **мастер-версия**: правишь `.ps1` → тестируешь → переносишь в `.py`. Не дорабатывай `.py` без идентичного изменения `.ps1`.
- Каждый `.py` самодостаточен, общие утилиты (`esc_xml`, `emit_mltext` и т.п.) дублируются в каждом скрипте.

**Правило** (`config/rules/*.md`): frontmatter `paths:` с glob-масками (напр. `**/*.bsl`), далее правило. Правила ссылаются друг на друга по имени файла (напр. «Дополняет `1c-coding-standards.md`»).

**Документация** (`config/docs/`): спецификации XML-формата 1С и DSL-спеки взаимосвязаны; `1c-specs-index.md` — точка входа. При правке спецификаций обновляй перекрёстные ссылки в индексе.

## Добавление нового скилла (чеклист)

Собери все правила из раздела «Форматы» в один порядок действий:

1. **Выбери имя.** Папка = `name` = `skill_name` в evals, с префиксом `1c-` (напр. `1c-meta-compile`). Если скилл берётся из эталона — обычно это кастомизация из `config/etalon/cursor-1c-skills/skills/<name>` или `1c-ai-development-kit/.claude/skills/<name>`; скопируй и адаптируй.
2. **Создай структуру** `config/skills/1c-skills/<name>/`: `SKILL.md`, `scripts/` (`.ps1` + `.py`), `evals/evals.json`. Скрипты клади по парам PS1/Python (см. `python-porting-guide.md`).
3. **Frontmatter** (все поля, без кавычек, LF-окончания):
   - `name: 1c-<...>`
   - `description:` — начинается с «Используй когда …», затем суть (тип объекта, операция) — это ключевой сигнал для BM25-поиска и выбора агентом. Добавляй слова-синонимы из предметной области, чтобы поиск находил скилл.
   - `argument-hint:` — формат вызова в `<скобках>`.
   - `allowed-tools:` — список через `-`.
   - `tags:` — домены через запятую (`epf`, `erf`, `forms`, `mxl`, `skd`, `db`, `query`, `template`, `bsp`, `workflow`, `help`).
   - `depends_on:` — только для workflow-скиллов: под-скиллы цепочки через запятую (как `1c-epf-full-cycle`).
4. **Заголовок** `# /<command>` — команда без префикса `1c-`; команда должна быть **уникальна** (проверь: `grep -rn "^# /" config/skills/1c-skills/*/SKILL.md`).
5. **«## Команда»** — вызов `powershell.exe -NoProfile -File <name>/scripts/<script>.ps1 ...`. Путь — от каталога скиллов (`1c-form-add/scripts/form-add.ps1`), **без `skills/`**, `.claude/` и `${CLAUDE_SKILL_DIR}`.
6. **Evals** — `skill_name` строго равен `name`; опиши 2-3 сценария с ожидаемыми параметрами скрипта.
7. **Верификация** (обязательно, по аналогии с живыми проверками из истории):
   - `sed -i 's/\r$//'` на `.md` (LF, иначе парсер не увидит description);
   - проверь, что папка = `name` и evals согласованы;
   - проверь, что все пути `<name>/scripts/*` существуют;
   - прогони живые MCP-запросы на сервере `mcp-1c-skills` (см. main-репо, `tauri-app/mcp-1c-skills`): `list_skills`, `search` по ключевым словам, `get_skill` — убедись, что скилл находится и корректно отображается (Вызов/Теги/Зависит от).
8. **Коммит** кратко на русском, затем **пуш** (по правилу «всегда пуш после коммита»).

## Knowledge graph (graphify)

Граф по `config/` (skills + rules + docs) собран в `graphify-out/` (gitignored):
- `graphify query "<вопрос>"` — быстрый ответ по корпусу (BFS по графу).
- `graphify path "A" "B"`, `graphify explain "<узел>"`.
- `graph.html` — визуализация в браузере; `GRAPH_REPORT.md` — отчёт.
- После крупных правок содержимого: `/graphify --update` для инкрементального перестроения.

## Git workflow

- Worktree на ветке `config/skills-content`; коммиты только по `config/` (и AGENTS.md).
- Перед коммитом: `git status`, `git diff`; НЕ добавлять `config/etalon/` (gitignored) и `graphify-out/` (gitignored).
- Сообщения коммитов — кратко, на русском.
