#!/usr/bin/env node
// ─── Скрипт сборки 1c-env MCP-сервера для внешних агентов ────────
// Собирает standalone CJS-бандл, который можно запускать:
//   node 1c-env.cjs --stdio              (по умолчанию)
//   node 1c-env.cjs --sse --port 3000    (HTTP-режим)

import { build } from 'esbuild';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';
import { chmodSync, mkdirSync, existsSync } from 'fs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const srcDir = join(__dirname, '..', 'src', 'mcp-servers');
const outDir = join(__dirname, '..', 'dist');
const outFile = join(outDir, '1c-env.cjs');

if (!existsSync(outDir)) mkdirSync(outDir, { recursive: true });

try {
    await build({
        entryPoints: [join(srcDir, '1c-env.ts')],
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
    console.log(`  { "mcpServers": { "1c-env": { "command": "node", "args": ["${outFile}"] } } }`);
    console.log('');
    console.log('Пример конфигурации (HTTP/SSE):');
    console.log(`  { "mcpServers": { "1c-env": { "url": "http://localhost:3000" } } }`);
} catch (e) {
    console.error('Ошибка сборки:', e);
    process.exit(1);
}
