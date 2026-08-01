#!/usr/bin/env node
// ─── 1C Environment MCP Server ─────────────────────────────────
// Standalone MCP-сервер для определения платформы 1С и списка баз.
// Поддерживает stdio (для локальных агентов) и HTTP (Streamable HTTP / SSE).
//
// Использование:
//   node 1c-env.cjs --stdio            (по умолчанию)
//   node 1c-env.cjs --sse --port 3000  (HTTP-режим)
//   node 1c-env.cjs --port 3001        (HTTP-режим, порт 3001)

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { CallToolRequestSchema, ListToolsRequestSchema, InitializeRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import { readFileSync, readdirSync, existsSync, statSync } from 'fs';
import { join, resolve } from 'path';
import { createServer, IncomingMessage, ServerResponse } from 'http';
import { randomUUID } from 'crypto';

// ─── Types ───────────────────────────────────────────────────────

interface InfobaseInfo {
    name: string;
    connection: string;
    type: 'file' | 'server';
    id: string | null;
    folder: string | null;
}

interface PlatformInfo {
    version: string;
    bin_path: string;
    exe_path: string;
    ibcmd_path: string;
    exists: boolean;
}

// ─── CLI args ────────────────────────────────────────────────────

function parseArgs(): { mode: 'stdio' | 'http'; port: number } {
    const args = process.argv.slice(2);
    let mode: 'stdio' | 'http' = 'stdio';
    let port = 3000;

    for (let i = 0; i < args.length; i++) {
        switch (args[i]) {
            case '--stdio':
                mode = 'stdio';
                break;
            case '--sse':
            case '--http':
                mode = 'http';
                break;
            case '--port':
            case '-p':
                port = parseInt(args[i + 1], 10) || 3000;
                i++;
                break;
            case '--help':
            case '-h':
                console.error('1C Environment MCP Server');
                console.error('Usage: node 1c-env.cjs [--stdio] [--sse|--http] [--port N]');
                process.exit(0);
        }
    }
    return { mode, port };
}

// ─── ibases.v8i Parser ───────────────────────────────────────────

const DEFAULT_V8I_PATHS: string[] = (() => {
    const home = process.env.USERPROFILE || process.env.HOME || '';
    const progData = process.env.ProgramData || '';
    const paths: string[] = [];

    if (home) {
        paths.push(
            join(home, 'AppData', 'Roaming', '1C', '1CEStart', 'ibases.v8i'),
            join(home, 'AppData', 'Roaming', '1C', '1cv8', 'ibases.v8i'),
        );
    }
    if (progData) {
        paths.push(
            join(progData, '1C', '1CEStart', 'ibases.v8i'),
            join(progData, '1C', '1cv8', 'ibases.v8i'),
        );
    }
    return paths;
})();

function parseConnectionString(connect: string): { connection: string; type: 'file' | 'server' } | null {
    const lower = connect.toLowerCase();

    if (lower.includes('file=')) {
        const idx = lower.indexOf('file=');
        const rest = connect.slice(idx);
        const start = rest.indexOf('"');
        if (start !== -1) {
            const end = rest.indexOf('"', start + 1);
            if (end !== -1) {
                const path = rest.slice(start + 1, end);
                return { connection: `File="${path}"`, type: 'file' };
            }
        }
    }

    if (lower.includes('srvr=') && lower.includes('ref=')) {
        return { connection: connect, type: 'server' };
    }

    if (lower.startsWith('s=')) {
        return { connection: connect, type: 'server' };
    }

    return null;
}

function parseV8iContent(content: string): InfobaseInfo[] {
    const bases: InfobaseInfo[] = [];
    let currentName = '';
    let currentConnect = '';
    let currentId = '';
    let currentFolder = '';

    const text = content.replace(/^\u{FEFF}/u, '');

    for (const line of text.split('\n')) {
        const trimmed = line.trim();
        const sectionMatch = trimmed.match(/^\[(.+)\]$/);
        if (sectionMatch) {
            if (currentName && currentConnect) {
                const parsed = parseConnectionString(currentConnect);
                if (parsed) {
                    bases.push({
                        name: currentName,
                        connection: parsed.connection,
                        type: parsed.type,
                        id: currentId || null,
                        folder: currentFolder || null,
                    });
                }
            }
            currentName = sectionMatch[1];
            currentConnect = '';
            currentId = '';
            currentFolder = '';
            continue;
        }

        const eqIdx = trimmed.indexOf('=');
        if (eqIdx !== -1) {
            const key = trimmed.slice(0, eqIdx).trim().toLowerCase();
            const value = trimmed.slice(eqIdx + 1).trim();
            switch (key) {
                case 'connect': currentConnect = value; break;
                case 'id': currentId = value; break;
                case 'folder': currentFolder = value; break;
            }
        }
    }

    if (currentName && currentConnect) {
        const parsed = parseConnectionString(currentConnect);
        if (parsed) {
            bases.push({
                name: currentName,
                connection: parsed.connection,
                type: parsed.type,
                id: currentId || null,
                folder: currentFolder || null,
            });
        }
    }

    return bases;
}

function parseV8iFile(v8iPath?: string): InfobaseInfo[] {
    const searchPaths = v8iPath ? [v8iPath] : DEFAULT_V8I_PATHS;
    for (const p of searchPaths) {
        if (!existsSync(p)) continue;
        try {
            const content = readFileSync(p, 'utf-8');
            const bases = parseV8iContent(content);
            if (bases.length > 0) return bases;
        } catch { /* skip */ }
    }
    return [];
}

function findV8iPath(): string | null {
    for (const p of DEFAULT_V8I_PATHS) {
        if (existsSync(p)) return p;
    }
    return null;
}

// ─── Platform Scanner ────────────────────────────────────────────

function findPlatform(): PlatformInfo[] {
    const programFiles = [
        process.env['PROGRAMFILES'] || 'C:\\Program Files',
        process.env['PROGRAMFILES(X86)'] || 'C:\\Program Files (x86)',
    ];
    const platforms: PlatformInfo[] = [];

    for (const base of programFiles) {
        const cv8Dir = join(base, '1cv8');
        if (!existsSync(cv8Dir)) continue;
        try {
            const entries = readdirSync(cv8Dir);
            for (const entry of entries) {
                if (!/^\d+\.\d+\.\d+\.\d+$/.test(entry)) continue;
                const versionDir = join(cv8Dir, entry);
                if (!statSync(versionDir).isDirectory()) continue;
                const binDir = join(versionDir, 'bin');
                const exePath = join(binDir, '1cv8.exe');
                const ibcmdPath = join(binDir, 'ibcmd.exe');
                if (existsSync(exePath)) {
                    platforms.push({ version: entry, bin_path: binDir, exe_path: exePath, ibcmd_path: ibcmdPath, exists: true });
                }
            }
        } catch { /* skip */ }
    }

    platforms.sort((a, b) => {
        const va = a.version.split('.').map(Number);
        const vb = b.version.split('.').map(Number);
        for (let i = 0; i < 4; i++) {
            const diff = (vb[i] || 0) - (va[i] || 0);
            if (diff !== 0) return diff;
        }
        return 0;
    });

    return platforms;
}

function getEnvironment() {
    const platforms = findPlatform();
    const infobases = parseV8iFile();
    const v8iPath = findV8iPath();
    return { platforms, infobases, v8i_path: v8iPath };
}

// ─── Status ──────────────────────────────────────────────────────

function emitStatus(status: string) {
    process.stderr.write(`1C_ENV_STATUS:${status}\n`);
}

function reportStatus() {
    const env = getEnvironment();
    if (env.platforms.length === 0 && env.infobases.length === 0) {
        emitStatus('unavailable');
    } else {
        emitStatus(`ready:${env.platforms.length} platforms, ${env.infobases.length} bases`);
    }
}

// ─── MCP Server Setup ────────────────────────────────────────────

function createServerInstance(): Server {
    const server = new Server(
        { name: '1c-env', version: '1.0.0' },
        { capabilities: { tools: {} } },
    );

    server.setRequestHandler(ListToolsRequestSchema, async () => ({
        tools: [
            {
                name: 'list_infobases',
                description: 'Список информационных баз 1С из ibases.v8i. Возвращает все зарегистрированные базы: имя, строку соединения, тип (файловая/серверная), ID и папку.',
                inputSchema: {
                    type: 'object',
                    properties: {
                        v8i_path: {
                            type: 'string',
                            description: 'Путь к ibases.v8i (по умолчанию — стандартные расположения %APPDATA%\\1C\\1CEStart)',
                        },
                    },
                },
            },
            {
                name: 'find_platform',
                description: 'Поиск установленных версий 1С:Предприятие (1cv8.exe). Сканирует Program Files и Program Files (x86). Возвращает список версий по убыванию с путями к 1cv8.exe и ibcmd.exe.',
                inputSchema: { type: 'object', properties: {} },
            },
            {
                name: 'get_1c_environment',
                description: 'Комбинированная информация: установленные платформы + список баз 1С + путь к ibases.v8i. Один вызов — вся картина.',
                inputSchema: { type: 'object', properties: {} },
            },
        ],
    }));

    server.setRequestHandler(CallToolRequestSchema, async (request) => {
        const { name, arguments: args } = request.params;
        try {
            switch (name) {
                case 'list_infobases': {
                    const v8iPath = (args as any)?.v8i_path as string | undefined;
                    const bases = parseV8iFile(v8iPath);
                    return ok({
                        count: bases.length,
                        v8i_path: v8iPath || findV8iPath(),
                        bases: bases.map(b => ({ name: b.name, connection: b.connection, type: b.type, id: b.id, folder: b.folder })),
                    });
                }
                case 'find_platform': {
                    const platforms = findPlatform();
                    return ok({
                        count: platforms.length,
                        latest: platforms[0] || null,
                        platforms: platforms.map(p => ({ version: p.version, bin_path: p.bin_path, exe_path: p.exe_path, ibcmd_path: p.ibcmd_path })),
                    });
                }
                case 'get_1c_environment': {
                    const env = getEnvironment();
                    return ok({
                        platforms: { count: env.platforms.length, latest_version: env.platforms[0]?.version || null, items: env.platforms.map(p => ({ version: p.version, exe_path: p.exe_path, ibcmd_path: p.ibcmd_path })) },
                        infobases: { count: env.infobases.length, v8i_path: env.v8i_path, items: env.infobases.map(b => ({ name: b.name, connection: b.connection, type: b.type })) },
                    });
                }
                default:
                    throw new Error(`Unknown tool: ${name}`);
            }
        } catch (e: any) {
            return err(e.message || String(e));
        }
    });

    return server;
}

function ok(data: any) {
    return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

function err(msg: string) {
    return { content: [{ type: 'text' as const, text: `Error: ${msg}` }], isError: true };
}

// ─── Stdio Mode ──────────────────────────────────────────────────

async function startStdio() {
    const server = createServerInstance();
    const transport = new StdioServerTransport();
    await server.connect(transport);
    emitStatus('stdio:connected');
}

// ─── HTTP Mode (Streamable HTTP / SSE) ───────────────────────────

async function startHttp(port: number) {
    const httpServer = createServer(async (req: IncomingMessage, res: ServerResponse) => {
        // CORS
        res.setHeader('Access-Control-Allow-Origin', '*');
        res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
        res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Accept, mcp-session-id');
        res.setHeader('Access-Control-Expose-Headers', 'mcp-session-id');

        if (req.method === 'OPTIONS') {
            res.writeHead(204);
            res.end();
            return;
        }

        const url = new URL(req.url || '/', `http://localhost:${port}`);

        // POST / — Streamable HTTP (main endpoint)
        if (req.method === 'POST' && url.pathname === '/') {
            let body = '';
            for await (const chunk of req) body += chunk;
            try {
                const parsedBody = JSON.parse(body);
                const server = createServerInstance();
                const transport = new StreamableHTTPServerTransport({
                    sessionIdGenerator: () => randomUUID(),
                });
                await server.connect(transport);
                await transport.handleRequest(req as any, res, parsedBody);
            } catch (e: any) {
                res.writeHead(500, { 'Content-Type': 'application/json' });
                res.end(JSON.stringify({ jsonrpc: '2.0', error: { code: -32603, message: e.message } }));
            }
            return;
        }

        // GET /health
        if (req.method === 'GET' && url.pathname === '/health') {
            const env = getEnvironment();
            res.writeHead(200, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ status: 'ok', platforms: env.platforms.length, bases: env.infobases.length }));
            return;
        }

        // GET / — info page
        if (req.method === 'GET' && url.pathname === '/') {
            res.writeHead(200, { 'Content-Type': 'text/html' });
            res.end(`<!DOCTYPE html><html><body><h2>1C Environment MCP Server</h2>
<p>Tools: <code>list_infobases</code>, <code>find_platform</code>, <code>get_1c_environment</code></p>
<p>POST <a href="/">/</a> — Streamable HTTP (MCP)</p>
<p>GET /health — Health check</p>
</body></html>`);
            return;
        }

        res.writeHead(404);
        res.end('Not found');
    });

    httpServer.listen(port, () => {
        emitStatus(`http:listening:${port}`);
        console.error(`[1c-env] HTTP server on http://localhost:${port}`);
        console.error(`[1c-env] MCP endpoint: POST http://localhost:${port}/`);
        console.error(`[1c-env] Health check: GET http://localhost:${port}/health`);
    });
}

// ─── Entry ───────────────────────────────────────────────────────

async function main() {
    reportStatus();
    const { mode, port } = parseArgs();

    if (mode === 'stdio') {
        await startStdio();
    } else {
        await startHttp(port);
    }
}

main().catch((e) => {
    process.stderr.write(`[1c-env] Fatal: ${e}\n`);
    process.exit(1);
});
