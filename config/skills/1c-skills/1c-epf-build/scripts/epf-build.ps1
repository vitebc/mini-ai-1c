# epf-build v1.0 — Build external data processor or report (EPF/ERF) from XML sources
# Source: https://github.com/Desko77/claude-code-skills-1c
<#
.SYNOPSIS
    Сборка внешней обработки/отчёта 1С из XML-исходников

.DESCRIPTION
    Собирает EPF/ERF-файл из XML-исходников с помощью платформы 1С.
    Общий скрипт для epf-build и erf-build.

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

.PARAMETER SourceFile
    Путь к корневому XML-файлу исходников

.PARAMETER OutputFile
    Путь к выходному EPF/ERF-файлу

.PARAMETER StrictLog
    Отказ в журнале платформы поднимает код возврата до 1, даже если платформа вернула 0

.PARAMETER AdditionalV8Arguments
    Дополнительные аргументы платформы списком через запятую (доходят и до временной базы)

.EXAMPLE
    .\epf-build.ps1 -InfoBasePath "C:\Bases\MyDB" -SourceFile "src\МояОбработка.xml" -OutputFile "build\МояОбработка.epf"

.EXAMPLE
    .\epf-build.ps1 -InfoBasePath "C:\Bases\MyDB" -SourceFile "src\МойОтчёт.xml" -OutputFile "build\МойОтчёт.erf"
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
    [string]$SourceFile,

    [Parameter(Mandatory=$true)]
    [string]$OutputFile,

    [Parameter(Mandatory=$false)]
    [string]$AdditionalV8Arguments,

    [Parameter(Mandatory=$false)]
    [switch]$StrictLog
)

$OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# --- Дополнительные аргументы платформы (эталон skills/1c-epf-build) ---
# Список разделяется запятой, а не пробелом: аргумент платформы несет пробел внутри значения
# (/C "имя значение", путь с пробелом), и разбор по пробелу разорвал бы такой аргумент.
function Split-PlatformArguments {
    param([string]$Raw)
    if ([string]::IsNullOrWhiteSpace($Raw)) { return @() }
    return @($Raw -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' })
}

# Настройки проекта ищутся вверх по дереву от каталога исходников: скрипт запускают из
# любого места, а файл настроек лежит в корне проекта.
function Find-V8ProjectFile {
    param([string]$StartDir)
    $d = if ([string]::IsNullOrEmpty($StartDir)) {
        (Get-Location).Path
    } elseif ([System.IO.Path]::IsPathRooted($StartDir)) {
        $StartDir
    } else {
        Join-Path (Get-Location).Path $StartDir
    }
    $d = [System.IO.Path]::GetFullPath($d)
    for ($i = 0; $i -lt 20 -and $d; $i++) {
        $pj = Join-Path $d ".v8-project.json"
        if (Test-Path $pj) { return $pj }
        $parent = [System.IO.Path]::GetDirectoryName($d)
        if ($parent -eq $d) { break }
        $d = $parent
    }
    return $null
}

function Get-ProjectPlatformArguments {
    param([string]$StartDir, [string]$Key)
    try {
        $pj = Find-V8ProjectFile $StartDir
        if (-not $pj) { return @() }
        $settings = Get-Content $pj -Raw -Encoding UTF8 | ConvertFrom-Json
        $value = $settings.$Key
        if ($null -eq $value) { return @() }
        if ($value -is [string]) { return Split-PlatformArguments $value }
        return @($value | ForEach-Object { [string]$_ } | Where-Object { $_ -ne '' })
    } catch {
        return @()
    }
}

# Аргументы вызова заменяют значение из настроек проекта целиком, а не дополняют его:
# при сложении снять заданный в проекте аргумент было бы нечем.
# $null в Explicit = параметр не задавали (действуют настройки проекта);
# пустая строка = заданное пустое значение (снимает аргументы проекта на этот запуск).
function Resolve-PlatformArguments {
    param($Explicit, [string]$StartDir, [string]$Key)
    if ($null -ne $Explicit) { return Split-PlatformArguments ([string]$Explicit) }
    return Get-ProjectPlatformArguments -StartDir $StartDir -Key $Key
}
# --- Конец блока дополнительных аргументов ---

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

# Каталог поиска настроек проекта — рядом с исходниками обработки. Объявлен до цепочки
# временной базы: аргументы нужны обоим запускам.
$settingsDir = Split-Path $SourceFile -Parent

# Незаданный параметр и заданный пустым — разные случаи: первый оставляет в силе настройки
# проекта, второй снимает их на этот запуск.
$explicitV8Args = if ($PSBoundParameters.ContainsKey('AdditionalV8Arguments')) { $AdditionalV8Arguments } else { $null }

# --- Auto-create stub database if no connection specified ---
$autoCreatedBase = $null
if (-not $InfoBasePath -and (-not $InfoBaseServer -or -not $InfoBaseRef)) {
    $sourceDir = Split-Path $SourceFile -Parent
    $autoBasePath = Join-Path $env:TEMP "epf_stub_db_$(Get-Random)"
    $stubScript = Join-Path $PSScriptRoot "stub-db-create.ps1"
    Write-Host "No database specified. Creating temporary stub database..."
    $stubArgs = "-SourceDir `"$sourceDir`" -V8Path `"$V8Path`" -TempBasePath `"$autoBasePath`""
    # Аргументы разрешаются ДО цепочки: заданные только в настройках проекта иначе не дошли бы
    # до создания временной базы.
    $resolvedStubArgs = @(Resolve-PlatformArguments -Explicit $explicitV8Args -StartDir $settingsDir -Key "v8args")
    if ($resolvedStubArgs.Count -gt 0) {
        $stubArgs += " -AdditionalV8Arguments `"$($resolvedStubArgs -join ',')`""
    }
    $stubProc = Start-Process -FilePath "powershell.exe" -ArgumentList "-NoProfile -File `"$stubScript`" $stubArgs" -NoNewWindow -Wait -PassThru
    if ($stubProc.ExitCode -ne 0) {
        Write-Host "Error: failed to create stub database" -ForegroundColor Red
        exit 1
    }
    $InfoBasePath = $autoBasePath
    $autoCreatedBase = $autoBasePath
}

# --- Validate source file ---
if (-not (Test-Path $SourceFile)) {
    Write-Host "Error: source file not found: $SourceFile" -ForegroundColor Red
    exit 1
}

# --- Ensure output directory exists ---
$outDir = Split-Path $OutputFile -Parent
if ($outDir -and -not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Path $outDir -Force | Out-Null
}

# --- Temp dir ---
$tempDir = Join-Path $env:TEMP "epf_build_$(Get-Random)"
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

    $arguments += "/LoadExternalDataProcessorOrReportFromFiles", "`"$SourceFile`"", "`"$OutputFile`""

    # --- Output ---
    $outFile = Join-Path $tempDir "build_log.txt"
    $arguments += "/Out", "`"$outFile`""
    $arguments += "/DisableStartupDialogs"
    $arguments += @(Resolve-PlatformArguments -Explicit $explicitV8Args -StartDir $settingsDir -Key "v8args")

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
        Write-Host "Build completed successfully: $OutputFile" -ForegroundColor Green
    } else {
        Write-Host "Error building (code: $exitCode)" -ForegroundColor Red
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
    if ($autoCreatedBase -and (Test-Path $autoCreatedBase)) {
        Remove-Item -Path $autoCreatedBase -Recurse -Force -ErrorAction SilentlyContinue
    }
}
