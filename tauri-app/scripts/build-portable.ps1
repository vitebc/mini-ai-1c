param(
    [string]$OutputDir = ".\dist-portable"
)

Write-Host "=== Mini AI 1C Portable Build ===" -ForegroundColor Cyan

# 1. Build the Tauri app
Write-Host "[1/4] Building Tauri app..." -ForegroundColor Yellow
npm run app:build
if ($LASTEXITCODE -ne 0) { Write-Host "Build failed!" -ForegroundColor Red; exit 1 }

# 2. Create portable directory
Write-Host "[2/4] Creating portable package..." -ForegroundColor Yellow
$exePath = ".\src-tauri\target\release\mini-ai-1c.exe"
$portableDir = Join-Path $OutputDir "MiniAI1C"
New-Item -ItemType Directory -Path $portableDir -Force | Out-Null

# 3. Copy executable
Copy-Item $exePath (Join-Path $portableDir "mini-ai-1c.exe") -Force

# 4. Copy MCP servers
$mcpDir = ".\src-tauri\mcp-servers"
if (Test-Path $mcpDir) {
    Copy-Item "$mcpDir\*" $portableDir -Recurse -Force
}

# 5. Create enterprise.json placeholder
$enterpriseJson = @{
    server_url = "http://localhost:9224"
    token = ""
    auto_update = $true
} | ConvertTo-Json
$enterpriseJson | Out-File (Join-Path $portableDir "enterprise.json") -Encoding utf8

# 6. Create .env with portable mode indicator
"# Mini AI 1C Portable" | Out-File (Join-Path $portableDir "README.txt") -Encoding utf8

# 7. ZIP it
Write-Host "[3/4] Creating ZIP archive..." -ForegroundColor Yellow
$zipPath = "$OutputDir\MiniAI1C-Portable.zip"
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path "$portableDir\*" -DestinationPath $zipPath

Write-Host "[4/4] Cleaning up..." -ForegroundColor Yellow
Remove-Item $portableDir -Recurse -Force

Write-Host "=== Done ===" -ForegroundColor Green
Write-Host "Portable build: $zipPath" -ForegroundColor Green
