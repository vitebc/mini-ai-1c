# db-dump-xml v1.0 — Dump 1C configuration to XML files
# Source: https://github.com/Desko77/claude-code-skills-1c
<#
.SYNOPSIS
    Выгрузка конфигурации 1С в XML-файлы

.DESCRIPTION
    Выполняет выгрузку конфигурации 1С в файлы в четырёх режимах:
    - Full: полная выгрузка всей конфигурации
    - Changes: инкрементальная выгрузка изменённых объектов
    - Partial: выгрузка конкретных объектов из списка
    - UpdateInfo: обновление только ConfigDumpInfo.xml

.PARAMETER V8Path
    Путь к каталогу bin платформы или к 1cv8.exe

.PARAMETER InfoBasePath
    Путь к файловой информационной базе

.PARAMETER InfoBaseServer
    Сервер 1С (для серверной базы)

.PARAMETER InfoBaseRef
    Имя базы на сервере

.PARAMETER UserName
    Имя пользователя 1С

.PARAMETER Password
    Пароль пользователя

.PARAMETER ConfigDir
    Каталог для выгрузки конфигурации

.PARAMETER Mode
    Режим выгрузки: Full, Changes, Partial, UpdateInfo (по умолчанию Changes)

.PARAMETER Objects
    Имена объектов метаданных через запятую (для режима Partial)

.PARAMETER Extension
    Имя расширения для выгрузки

.PARAMETER AllExtensions
    Выгрузить все расширения

.PARAMETER Format
    Формат выгрузки: Hierarchical или Plain (по умолчанию Hierarchical)

.PARAMETER StrictLog
    Отказ в журнале платформы поднимает код возврата до 1, даже если платформа вернула 0

.EXAMPLE
    .\db-dump-xml.ps1 -InfoBasePath "C:\Bases\MyDB" -ConfigDir "C:\src" -Mode Full

.EXAMPLE
    .\db-dump-xml.ps1 -InfoBasePath "C:\Bases\MyDB" -ConfigDir "C:\src" -Mode Partial -Objects "Справочник.Номенклатура,Документ.Заказ"
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory=$false)]
    [string]$V8Path,

    [Parameter(Mandatory=$false)]
    [string]$InfoBasePath,

    [Parameter(Mandatory=$false)]
    [string]$InfoBaseServer,

    [Parameter(Mandatory=$false)]
    [string]$InfoBaseRef,

    [Parameter(Mandatory=$false)]
    [string]$UserName,

    [Parameter(Mandatory=$false)]
    [string]$Password,

    [Parameter(Mandatory=$true)]
    [string]$ConfigDir,

    [Parameter(Mandatory=$false)]
    [ValidateSet("Full", "Changes", "Partial", "UpdateInfo")]
    [string]$Mode = "Changes",

    [Parameter(Mandatory=$false)]
    [string]$Objects,

    [Parameter(Mandatory=$false)]
    [string]$Extension,

    [Parameter(Mandatory=$false)]
    [switch]$AllExtensions,

    [Parameter(Mandatory=$false)]
    [ValidateSet("Hierarchical", "Plain")]
    [string]$Format = "Hierarchical",

    [Parameter(Mandatory=$false)]
    [switch]$StrictLog
)

$OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# --- Resolve V8Path ---
if (-not $V8Path) {
    $found = Get-ChildItem "C:\Program Files\1cv8\*\bin\1cv8.exe" -ErrorAction SilentlyContinue | Sort-Object FullName -Descending | Select-Object -First 1
    if ($found) {
        $V8Path = $found.FullName
    } else {
        Write-Host "Error: 1cv8.exe not found. Specify -V8Path" -ForegroundColor Red
        exit 1
    }
} elseif (Test-Path $V8Path -PathType Container) {
    $V8Path = Join-Path $V8Path "1cv8.exe"
}

if (-not (Test-Path $V8Path)) {
    Write-Host "Error: 1cv8.exe not found at $V8Path" -ForegroundColor Red
    exit 1
}

# --- Validate connection ---
if (-not $InfoBasePath -and (-not $InfoBaseServer -or -not $InfoBaseRef)) {
    Write-Host "Error: specify -InfoBasePath or -InfoBaseServer + -InfoBaseRef" -ForegroundColor Red
    exit 1
}

# --- Validate Partial mode ---
if ($Mode -eq "Partial" -and -not $Objects) {
    Write-Host "Error: -Objects required for Partial mode" -ForegroundColor Red
    exit 1
}

# --- Create output dir if needed ---
if (-not (Test-Path $ConfigDir)) {
    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
    Write-Host "Created output directory: $ConfigDir"
}

# --- Temp dir ---
$tempDir = Join-Path $env:TEMP "db_dump_xml_$(Get-Random)"
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

try {
    # --- Build arguments ---
    $arguments = @("DESIGNER")

    if ($InfoBaseServer -and $InfoBaseRef) {
        $arguments += "/S", "`"$InfoBaseServer/$InfoBaseRef`""
    } else {
        $arguments += "/F", "`"$InfoBasePath`""
    }

    if ($UserName) { $arguments += "/N`"$UserName`"" }
    if ($Password) { $arguments += "/P`"$Password`"" }

    $arguments += "/DumpConfigToFiles", "`"$ConfigDir`""
    $arguments += "-Format", $Format

    switch ($Mode) {
        "Full" {
            Write-Host "Executing full configuration dump..."
        }
        "Changes" {
            Write-Host "Executing incremental configuration dump..."
            $arguments += "-update"
            $arguments += "-force"
        }
        "Partial" {
            Write-Host "Executing partial configuration dump..."
            $objectList = $Objects -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ }

            $listFile = Join-Path $tempDir "dump_list.txt"
            $utf8Bom = New-Object System.Text.UTF8Encoding($true)
            [System.IO.File]::WriteAllLines($listFile, $objectList, $utf8Bom)

            $arguments += "-listFile", "`"$listFile`""
            Write-Host "Objects to dump: $($objectList.Count)"
            foreach ($obj in $objectList) { Write-Host "  $obj" }
        }
        "UpdateInfo" {
            Write-Host "Updating ConfigDumpInfo.xml..."
            $arguments += "-configDumpInfoOnly"
        }
    }

    # --- Extensions ---
    if ($Extension) {
        $arguments += "-Extension", "`"$Extension`""
    } elseif ($AllExtensions) {
        $arguments += "-AllExtensions"
    }

    # --- Output ---
    $outFile = Join-Path $tempDir "dump_log.txt"
    $arguments += "/Out", "`"$outFile`""
    $arguments += "/DisableStartupDialogs"

    # --- Execute ---
    Write-Host "Running: 1cv8.exe $($arguments -join ' ')"
    $process = Start-Process -FilePath $V8Path -ArgumentList $arguments -NoNewWindow -Wait -PassThru
    $exitCode = $process.ExitCode

    # --- Read log ---
    $logContent = $null
    if (Test-Path $outFile) {
        $logContent = Get-Content $outFile -Raw -ErrorAction SilentlyContinue
    }

    # --- Scan log for silent rejections (эталон: вердикт пакетного запуска, 1.7.0) ---
    # Платформа штатно возвращает 0 при проваленной операции — отказ виден только в журнале.
    $fatalLogPatterns = @(
        'неверное свойство объекта метаданных',
        'не входит в состав объекта метаданных',
        'неизвестное имя типа',
        'неизвестный объект метаданных',
        'ни один из документов не является регистратором для регистра',
        'неверное значение перечисления',
        'не может быть приведен к типу',
        'необходима версия платформы не меньше',
        'не найден метод',
        'не может быть применен'
    )
    $cleanLogPatterns = @(
        'ошибок не обнаружено',
        'ошибки не обнаружены',
        'предупреждений не обнаружено',
        'ошибок: 0',
        'предупреждений: 0',
        'errors were not found',
        '0 errors'
    )
    $silentFailures = @()
    if ($logContent) {
        foreach ($line in ($logContent -split "`r?`n")) {
            $trimmed = $line.Trim()
            if (-not $trimmed) { continue }
            $lower = $trimmed.ToLowerInvariant()
            $isClean = $false
            foreach ($pat in $cleanLogPatterns) {
                if ($lower.Contains($pat)) { $isClean = $true; break }
            }
            if ($isClean) { continue }
            foreach ($pat in $fatalLogPatterns) {
                if ($lower.Contains($pat)) { $silentFailures += $trimmed; break }
            }
        }
    }

    # --- Result ---
    # По умолчанию — вердикт платформы по коду возврата; журнал всегда печатается.
    # С -StrictLog отказ в журнале поднимает код возврата до 1, даже если платформа вернула 0.
    if ($exitCode -eq 0) {
        Write-Host "Dump completed successfully" -ForegroundColor Green
        Write-Host "Configuration dumped to: $ConfigDir"
    } else {
        Write-Host "Error dumping configuration (code: $exitCode)" -ForegroundColor Red
    }

    if ($logContent) {
        Write-Host "--- Log ---"
        Write-Host $logContent
        Write-Host "--- End ---"
    }

    if ($silentFailures.Count -gt 0) {
        $msg = "[warning] platform reported success, but the log contains $($silentFailures.Count) problem(s)"
        if (-not $StrictLog) { $msg += " (pass -StrictLog to treat as error)" }
        Write-Host $msg -ForegroundColor Yellow
        foreach ($f in $silentFailures) { Write-Host "  $f" -ForegroundColor Yellow }
        if ($StrictLog -and $exitCode -eq 0) { $exitCode = 1 }
    }

    exit $exitCode

} finally {
    if (Test-Path $tempDir) {
        Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
