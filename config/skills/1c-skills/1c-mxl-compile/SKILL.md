---
name: 1c-mxl-compile
description: Компиляция табличного документа (MXL) из JSON-определения. Используй когда нужно создать макет печатной формы
argument-hint: <JsonPath> <OutputPath>
tags: mxl
allowed-tools:
  - Bash
  - Read
  - Write
  - Glob
---

# /mxl-compile — Компилятор макета из DSL

Принимает компактное JSON-определение макета и генерирует корректный Template.xml для табличного документа 1С. Claude описывает *что* нужно (области, параметры, стили), скрипт обеспечивает *корректность* XML (палитры, индексы, объединения, namespace).

## Использование

```
/mxl-compile <JsonPath> <OutputPath>
```

## Параметры

| Параметр   | Обязательный | Описание                           |
|------------|:------------:|------------------------------------|
| JsonPath   | да           | Путь к JSON-определению макета     |
| OutputPath | да           | Путь для генерации Template.xml    |

## Команда

```powershell
powershell.exe -NoProfile -File 1c-mxl-compile/scripts/mxl-compile.ps1 -JsonPath "<путь>.json" -OutputPath "<путь>/Template.xml"
```

## Рабочий процесс

1. Claude пишет JSON-определение (Write tool) → файл `.json`
2. Claude вызывает `/mxl-compile` для генерации Template.xml
3. Claude вызывает `/mxl-validate` для проверки корректности
4. Claude вызывает `/mxl-info` для верификации структуры

**Если макет создаётся по изображению** (скриншот, скан печатной формы) — сначала вызвать `/img-grid` для наложения сетки, по ней определить границы колонок и пропорции, затем использовать `"Nx"` ширины + `"page"` для автоматического расчёта размеров.

## JSON-схема DSL

Полная спецификация формата: **`docs/mxl-dsl-spec.md`** (прочитать через Read tool перед написанием JSON).

Краткая структура:

```
{ columns, page, defaultWidth, columnWidths,
  languages, currentLanguage, defaultLanguage, textLanguages,
  columnSets: { name: { columns, columnWidths, id } },
  fonts: { name: { face, size, bold, italic, underline, strikeout } },
  styles: { name: { font, align, valign, border, borderWidth, wrap, format, textColor } },
  rows: [...],
  areas: [{ name, columnSet, rows: [{ height, hidden, style, rowStyle, columnSet, cells: [
    { col, span, rowspan, style, param, detail, text, template }
  ]}]}],
  namedAreas: [{ name, rows, cols }]
}
```

Ключевые правила:
- `page` — формат страницы (`"A4-landscape"`, `"A4-portrait"` или число). Автоматически вычисляет `defaultWidth` из суммы пропорций `"Nx"`
- `col` — 1-based позиция колонки
- `rowStyle` — автозаполнение пустот стилем (рамки по всей ширине); `style` строки — собственный стиль без автозаполнения; `hidden` — скрытая строка
- Тип заполнения определяется автоматически: `param` → Parameter, `text` → Text, `template` → Template
- `rowspan` — объединение строк вниз (rowStyle учитывает занятые ячейки)
- Строка массивом: элемент на колонку (`"Текст"`, `"{Парам}"`, `"Текст [Парам]"`, `null` — пропуск, `">"` — span, `"|"` — rowspan, `{...}` — ячейка без `col`)
- Область без `name` — строки попадают в документ без именованной области; строки вне областей — поле `rows` верхнего уровня
- `namedAreas[]` — области по координатам: только `rows` → Rows, только `cols` → Columns, обе оси → Rectangle
- `columnSets` — свои ширины для части строк (`columnSet` области/строки); `id` без задания выводится как UUIDv3 имени
- `text`/`template` — строка или объект по языкам (`{ "ru": "...", "en": "..." }`); языки задают `languages`/`currentLanguage`/`defaultLanguage`
