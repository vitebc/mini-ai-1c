#!/usr/bin/env node
// ─── Скрипт сборки jvv-1c MCP-сервера для внешних агентов ────────
// Собирает standalone CJS-бандл, который можно запускать:
//   node jvv-1c.cjs --stdio              (по умолчанию)
//   node jvv-1c.cjs --sse --port 3000    (HTTP-режим)

import { build } from 'esbuild';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';
import { chmodSync, mkdirSync, existsSync } from 'fs';

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
        footer: {
            js: 'module.exports = exports;',
        },
        external: [],
        minify: false,
        sourcemap: false,
        logLevel: 'info',
    });

    chmodSync(outFile, 0o755);

    console.log('');
    console.log('=== Готово! ===');
    console.log(`Бандл: ${outFile}`);
    console.log('');
    console.log('Использование:');
    console.log(`  node ${outFile} --stdio              (stdio для локальных агентов)`);
    console.log(`  node ${outFile} --sse --port 3000    (HTTP-режим для удалённых агентов)`);
    console.log('');
    console.log('Пример конфигурации Claude Desktop (stdio):');
    console.log(`  { "mcpServers": { "jvv-1c": { "command": "node", "args": ["${outFile}"] } } }`);
    console.log('');
    console.log('Пример конфигурации (HTTP/SSE):');
    console.log(`  { "mcpServers": { "jvv-1c": { "url": "http://localhost:3000" } } }`);
} catch (e) {
    console.error('Ошибка сборки:', e);
    process.exit(1);
}
