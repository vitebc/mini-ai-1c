#!/usr/bin/env bash
# Проверяет, менялись ли исходные TypeScript MCP-серверы с момента портирования на Rust.
# Для каждого mcp-1c-*/PORTED_FROM показывает коммиты TS-оригинала после портирования.
#
# Использование: scripts/check-mcp-parity.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FOUND=0

for ported in "$ROOT"/tauri-app/mcp-1c-*/PORTED_FROM; do
    [ -f "$ported" ] || continue
    server_dir="$(dirname "$ported")"
    server="$(basename "$server_dir")"

    # Извлекаем коммит и TS-пути из PORTED_FROM
    commit="$(grep -m1 'Синхронизирован до коммита:' "$ported" | sed 's/.*коммита: \([0-9a-f]\+\).*/\1/')"
    ts_files="$(grep -m1 'Исходный TypeScript:' "$ported" | sed 's/.*TypeScript: //')"

    [ -n "$commit" ] || { echo "[$server] PORTED_FROM без коммита"; continue; }

    # Собираем пути из списка (могут быть несколько через пробел)
    files=""
    for f in $ts_files; do
        files="$files $ROOT/$f"
    done

    log_out="$(cd "$ROOT" && git log --oneline "$commit..HEAD" -- $files 2>/dev/null)"
    if [ -n "$log_out" ]; then
        echo "=== $server: TS-оригинал изменился ==="
        echo "$log_out" | head -20
        FOUND=1
    else
        echo "[$server] OK — TS не менялся"
    fi
done

exit $FOUND
