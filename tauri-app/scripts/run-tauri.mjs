import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { applyCargoTargetDir } from './cargo-target-dir.mjs';

// Wrapper для `tauri dev` / `tauri build`: выставляет CARGO_TARGET_DIR на
// короткий путь при проекте на мапленном диске (os error 87) и чистит битые .tmp.
const scriptsDir = dirname(fileURLToPath(import.meta.url));
const appDir = join(scriptsDir, '..');
const srcTauriDir = join(appDir, 'src-tauri');

applyCargoTargetDir(srcTauriDir);

function findTauriBin() {
    const exe = process.platform === 'win32' ? 'tauri.cmd' : 'tauri';
    const candidates = [
        join(appDir, 'node_modules', '.bin', exe),
        join(scriptsDir, '..', '..', 'node_modules', '.bin', exe),
    ];
    for (const c of candidates) {
        if (existsSync(c)) return c;
    }
    return 'tauri';
}

const bin = findTauriBin();
const args = process.argv.slice(2);
console.log(`[run-tauri] ${bin} ${args.join(' ')}`);
const res = spawnSync(bin, args, {
    stdio: 'inherit',
    shell: process.platform === 'win32',
    cwd: appDir,
});
process.exit(res.status ?? 1);
