---
name: 1c-epf-build
description: Собрать внешнюю обработку 1С (EPF/ERF) из XML-исходников. Используй когда пользователь просит собрать, скомпилировать обработку или получить EPF/ERF файл из исходников
argument-hint: <ProcessorName>
tags: epf
allowed-tools:
  - Bash
  - Read
  - Glob
  - Grep
---

# /epf-build — Сборка обработки

## Usage

```
/epf-build <ProcessorName> [SrcDir] [OutDir]
```

| Параметр      | Обязательный | По умолчанию | Описание                             |
|---------------|:------------:|--------------|--------------------------------------|
| ProcessorName | да           | —            | Имя обработки (имя корневого XML)    |
| SrcDir        | нет          | `src`        | Каталог исходников                   |
| OutDir        | нет          | `build`      | Каталог для результата               |

## Параметры подключения (опционально)

1. Вызови `get_1c_environment` возьми путь к платформе
2. Проверь если временная база в ~/.config/mini-ai-1c/workspace/TempBase
3. Если база есть - используй ее
4. Если базы нет - создай пустую в ~/.config/mini-ai-1c/workspace/TempBase

## Команда

```powershell
powershell.exe -NoProfile -File 1c-epf-build/scripts/epf-build.ps1 <параметры>
```

### Параметры скрипта

| Параметр | Обязательный | Описание |
|----------|:------------:|----------|
| `-V8Path <путь>` | нет | Каталог bin платформы (или полный путь к 1cv8.exe) |
| `-InfoBasePath <путь>` | * | Файловая база |
| `-InfoBaseServer <сервер>` | * | Сервер 1С (для серверной базы) |
| `-InfoBaseRef <имя>` | * | Имя базы на сервере |
| `-UserName <имя>` | нет | Имя пользователя |
| `-Password <пароль>` | нет | Пароль |
| `-SourceFile <путь>` | да | Путь к корневому XML-файлу исходников |
| `-OutputFile <путь>` | да | Путь к выходному EPF/ERF-файлу |

> `*` — опционально. Если не указано — автоматически создаётся временная база со заглушками метаданных

## Примеры

```powershell
# Сборка обработки (файловая база)
powershell.exe -NoProfile -File 1c-epf-build/scripts/epf-build.ps1 -InfoBasePath "C:\Bases\MyDB" -SourceFile "src/МояОбработка.xml" -OutputFile "build/МояОбработка.epf"

# Серверная база
powershell.exe -NoProfile -File 1c-epf-build/scripts/epf-build.ps1 -InfoBaseServer "srv01" -InfoBaseRef "MyDB" -UserName "Admin" -Password "secret" -SourceFile "src/МояОбработка.xml" -OutputFile "build/МояОбработка.epf"
```
