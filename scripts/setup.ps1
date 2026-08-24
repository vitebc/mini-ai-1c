<# Mini AI 1C — Setup dependencies for Windows
Usage: .\scripts\setup.ps1 (в PowerShell от администратора) #>

$ErrorActionPreference = "Stop"

function Write-Info { Write-Host "[INFO] $args" -ForegroundColor Green }
function Write-Warn { Write-Host "[WARN] $args" -ForegroundColor Yellow }
function Write-Err  { Write-Host "[ERR]  $args" -ForegroundColor Red }

function Test-Command($name) { Get-Command $name -ErrorAction SilentlyContinue }

# --- Rust ---
if (Test-Command cargo) {
    Write-Info "Rust уже установлен: $(cargo --version)"
} else {
    Write-Info "Установка Rust через winget (MSVC toolchain)..."
    winget install --id Rustlang.Rust.MSVC --source winget --accept-source-agreements --accept-package-agreements
    $rustup = "$env:USERPROFILE\.cargo\bin\rustup.exe"
    & $rustup component add rustfmt clippy
    $env:PATH += ";$env:USERPROFILE\.cargo\bin"
    [Environment]::SetEnvironmentVariable("PATH", $env:PATH, "User")
    Write-Info "Rust установлен: $(cargo --version)"
}

# --- Node.js ---
if (Test-Command node -and Test-Command npm) {
    Write-Info "Node.js уже установлен: $(node --version), npm: $(npm --version)"
} else {
    Write-Info "Установка Node.js 20 LTS через winget..."
    if (Test-Command winget) {
        winget install --id OpenJS.NodeJS.LTS --source winget --accept-source-agreements --accept-package-agreements
    } else {
        Write-Warn "winget не найден. Скачайте Node.js 20+ с https://nodejs.org"
        exit 1
    }
    $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH","User")
    Write-Info "Node.js: $(node --version), npm: $(npm --version)"
}

# --- Java 17+ ---
if (Test-Command java) {
    $javaVer = java -version 2>&1 | Select-String 'version "(\d+)' | ForEach-Object { $_.Matches.Groups[1].Value }
    if ($javaVer -ge 17) {
        Write-Info "Java уже установлена: $(java -version 2>&1 | Select-Object -First 1)"
    } else {
        Write-Warn "Java версия $javaVer < 17"
    }
}
if (-not (Test-Command java) -or $javaVer -lt 17) {
    Write-Info "Установка Eclipse Temurin JDK 17 через winget..."
    winget install --id EclipseAdoptium.Temurin.17.JDK --source winget --accept-source-agreements --accept-package-agreements
    $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH","User")
    Write-Info "Java: $(java -version 2>&1 | Select-Object -First 1)"
}

# --- .NET 8 SDK ---
if (Test-Command dotnet) {
    $dotnetVer = (dotnet --version).Split('.')[0]
    if ($dotnetVer -ge 8) {
        Write-Info ".NET уже установлен: $(dotnet --version)"
    } else {
        Write-Warn ".NET версия $dotnetVer < 8"
    }
}
if (-not (Test-Command dotnet) -or $dotnetVer -lt 8) {
    Write-Info "Установка .NET 8 SDK через winget..."
    winget install --id Microsoft.DotNet.SDK.8 --source winget --accept-source-agreements --accept-package-agreements
    $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH","User")
    Write-Info ".NET: $(dotnet --version)"
}

# --- WebView2 Runtime (для Tauri) ---
Write-Info "Проверка WebView2 Runtime..."
try {
    $webview2 = Get-ItemProperty "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction Stop
    Write-Info "WebView2 Runtime уже установлен: $($webview2.pv)"
} catch {
    Write-Info "Установка WebView2 Runtime через winget..."
    winget install --id Microsoft.WebView2Runtime --source winget --accept-source-agreements --accept-package-agreements
}

# --- Visual Studio Build Tools (для нативных зависимостей) ---
Write-Info "Проверка Visual Studio Build Tools..."
if (-not (Test-Path "HKLM:\SOFTWARE\Microsoft\VisualStudio\SxS\VS7" -ErrorAction SilentlyContinue)) {
    Write-Warn "Visual Studio Build Tools не найдены."
    Write-Warn "Для компиляции нативных модулей (mcp-1c-search) нужны:"
    Write-Warn "  winget install --id Microsoft.VisualStudio.2022.BuildTools --source winget --accept-source-agreements --accept-package-agreements"
    Write-Warn "Выберите рабочую нагрузку: 'Desktop development with C++'"
} else {
    Write-Info "Visual Studio Build Tools найдены"
}

# --- Tauri CLI ---
if (Test-Command cargo-tauri) {
    Write-Info "Tauri CLI уже установлен"
} else {
    Write-Info "Установка Tauri CLI..."
    cargo install tauri-cli --version "^2.0"
    Write-Info "Tauri CLI установлен"
}

# --- Git ---
if (-not (Test-Command git)) {
    Write-Info "Установка Git через winget..."
    winget install --id Git.Git --source winget --accept-source-agreements --accept-package-agreements
}

Write-Host ""
Write-Info "=== Готово! ==="
Write-Info "Перезапустите PowerShell (или терминал), чтобы обновить PATH."
Write-Info "Затем выполните:"
Write-Info "  cd tauri-app"
Write-Info "  npm install"
Write-Info "  npm run build:mcp"