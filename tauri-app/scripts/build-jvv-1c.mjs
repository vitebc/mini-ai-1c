#!/usr/bin/env node
// ─── Скрипт standalone сборки jvv-1c MCP-сервера ─────────────────
// Бандлит jvv-1c.ts + все зависимости (включая @modelcontextprotocol/sdk)
// в один CJS-файл. Результат можно запускать без npm install.

import { build } from 'esbuild';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';
import { chmodSync, mkdirSync, existsSync, statSync } from 'fs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const srcDir = join(__dirname, '..', 'src', 'mcp-servers');
const outDir = join(__dirname, '..', 'dist');
const outFile = join(outDir, 'jvv-1c.cjs');

if (!existsSync(outDir)) mkdirSync(outDir, { recursive: true });

try {
    await build({
        entryPoints: [join(srcDir, 'jvv-1c.ts')],
        bundle: true,
        platform: 'node',
        target: 'node18',
        format: 'cjs',
        outfile: outFile,
        footer: { js: 'module.exports = exports;' },
        minify: false,
        sourcemap: false,
        logLevel: 'info',
        // Все зависимости включаются в бандл (SDK, crypto, http, fs, path, os)
        // Результат: один файл, zero external deps
        external: [],
    });

    chmodSync(outFile, 0o755);

    const size = existsSync(outFile) ? statSync(outFile).size : 0;
    const sizeKB = (size / 1024).toFixed(1);

    console.log('');
    console.log('=== JVV-1C Standalone Build ===');
    console.log(`Бандл: ${outFile} (${sizeKB} KB)`);
    console.log('');
    console.log('Содержит: jvv-1c + @modelcontextprotocol/sdk + все зависимости');
    console.log('Запуск на целевой машине: только Node.js 18+, npm install не нужен.');
    console.log('');
    console.log('Usage:');
    console.log(`  node ${outFile} --stdio`);
    console.log(`  node ${outFile} --http --port 3000`);
    console.log('');
    console.log('Claude Desktop config:');
    console.log(`  { "mcpServers": { "jvv-1c": { "command": "node", "args": ["${outFile}"] } } }`);
} catch (e) {
    console.error('Ошибка сборки:', e);
    process.exit(1);
}
