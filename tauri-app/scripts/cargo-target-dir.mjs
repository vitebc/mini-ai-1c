import { existsSync, mkdirSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// На Windows при проекте на мапленном диске (Y: и т.п.) cargo падает с
// os error 87 при работе с .rlib/.tmp*.temp-archive. Решение — короткий
// CARGO_TARGET_DIR в %TEMP%. На C: и не-Windows возвращает null (дефолт cargo).
export function cargoTargetOverride(projectDir) {
    if (process.platform !== 'win32') return null;
    const m = /^[A-Za-z]:/.exec(projectDir);
    if (!m || m[0].toUpperCase() === 'C:') return null;
    const safe = projectDir.replace(/[:\\/]/g, '_');
    return join(process.env.TEMP || tmpdir() || 'C:\\Temp', 'cargo-target', safe);
}

export function cleanStaleTempArchives(targetDir) {
    try {
        const deps = join(targetDir, 'deps');
        if (!existsSync(deps)) return;
        for (const f of readdirSync(deps)) {
            if (f.startsWith('.tmp') && f.endsWith('.temp-archive')) {
                try { rmSync(join(deps, f), { recursive: true, force: true }); } catch {}
            }
        }
    } catch {}
}

// Чистый эффективный target dir без побочных эффектов.
export function effectiveTargetDir(projectDir) {
    return cargoTargetOverride(projectDir) ?? join(projectDir, 'target');
}

// Применяет CARGO_TARGET_DIR к process.env, создаёт каталог, чистит битые .tmp.
// Возвращает эффективный target dir (override или <projectDir>/target).
export function applyCargoTargetDir(projectDir) {
    const override = cargoTargetOverride(projectDir);
    if (!override) return join(projectDir, 'target');
    process.env.CARGO_TARGET_DIR = override;
    try { mkdirSync(override, { recursive: true }); } catch {}
    console.log(`[cargo-target] CARGO_TARGET_DIR=${override}`);
    cleanStaleTempArchives(override);
    return override;
}
