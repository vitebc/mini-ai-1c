#!/usr/bin/env node
// Mini AI 1C — Clean node_modules with long-path support (Windows)
// Usage: npm run clean  OR  node scripts/clean.mjs

import { rmSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const targets = [
  'node_modules',
  join('tauri-app', 'node_modules'),
];

function clean(p) {
  if (!existsSync(p)) {
    console.log(`[clean] skip ${p} (not found)`);
    return;
  }
  // На Windows используем \\?\ префикс для обхода MAX_PATH 260
  const abs = join(process.cwd(), p);
  const winPath = process.platform === 'win32' ? `\\\\?\\${abs}` : abs;
  try {
    rmSync(winPath, { recursive: true, force: true, maxRetries: 3, retryDelay: 300 });
    console.log(`[clean] removed ${p}`);
  } catch (e) {
    // Фолбэк без префикса (для не-Windows или если \\?\ не сработал)
    try {
      rmSync(abs, { recursive: true, force: true, maxRetries: 3, retryDelay: 300 });
      console.log(`[clean] removed ${p} (fallback)`);
    } catch (e2) {
      console.error(`[clean] failed ${p}:`, e2.message);
      console.error(`[clean] Закрой VS Code / tauri dev / антивирус и запусти от администратора`);
      process.exitCode = 1;
    }
  }
}

for (const t of targets) clean(t);
if (!process.exitCode) console.log('[clean] done. Запусти: npm run install-all');
