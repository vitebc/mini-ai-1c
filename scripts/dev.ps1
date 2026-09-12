# Mini AI 1C — Dev mode
# CARGO_TARGET_DIR на короткий путь, если проект не на C: (cargo os error 87 на мапленных дисках)
$srcTauri = (Resolve-Path "$PSScriptRoot\..\tauri-app\src-tauri").Path
if ($srcTauri -match '^[A-Za-z]:' -and $srcTauri[0].ToString().ToUpper() -ne 'C') {
    $env:CARGO_TARGET_DIR = Join-Path $env:TEMP ("cargo-target\" + ($srcTauri -replace '[:\\]','_'))
}
cd "$PSScriptRoot\..\tauri-app"
npm run app:dev
