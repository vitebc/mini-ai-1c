#!/usr/bin/env node
// ─── JVV-1C MCP Server ─────────────────────────────────────────
// MCP-сервер для определения платформы 1С и списка баз.
// Использует @modelcontextprotocol/sdk (единообразно с другими серверами mini-ai-1c).
//
// Внутри mini-ai-1c: node jvv-1c.cjs (stdio, по умолчанию)
// Внешние агенты:     npm run build:jvv-1c → dist/jvv-1c.cjs (zero-dep standalone)

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import { readFileSync, readdirSync, existsSync, statSync } from 'fs';
import { join } from 'path';
import { homedir } from 'os';
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

// ─── CLI Args ────────────────────────────────────────────────────

function parseArgs(): { mode: 'stdio' | 'http'; port: number } {
    const args = process.argv.slice(2);
    let mode: 'stdio' | 'http' = 'stdio';
    let port = 3000;

    for (let i = 0; i < args.length; i++) {
        switch (args[i]) {
            case '--stdio': mode = 'stdio'; break;
            case '--http': mode = 'http'; break;
            case '--port': case '-p':
                port = parseInt(args[i + 1], 10) || 3000;
                i++;
                break;
            case '--help': case '-h':
                console.error('JVV-1C MCP Server v1.0.0');
                console.error('Usage: node jvv-1c.cjs [--stdio] [--http] [--port N]');
                process.exit(0);
        }
    }
    return { mode, port };
}

// ─── ibases.v8i Parser ───────────────────────────────────────────

const V8I_PATHS: string[] = (() => {
    const home = homedir();
    const progData = process.env.ProgramData || '';
    const paths: string[] = [];

    if (home) {
        const appData = process.env.APPDATA || join(home, 'AppData', 'Roaming');
        paths.push(
            join(appData, '1C', '1CEStart', 'ibases.v8i'),
            join(appData, '1C', '1cv8', 'ibases.v8i'),
        );
        paths.push(
            join(home, '.1cv8', 'ibases.v8i'),
            join(home, '.1C', '1CEStart', 'ibases.v8i'),
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
                    bases.push({ name: currentName, connection: parsed.connection, type: parsed.type, id: currentId || null, folder: currentFolder || null });
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
            bases.push({ name: currentName, connection: parsed.connection, type: parsed.type, id: currentId || null, folder: currentFolder || null });
        }
    }

    return bases;
}

function parseV8iFile(v8iPath?: string): InfobaseInfo[] {
    const searchPaths = v8iPath ? [v8iPath] : V8I_PATHS;
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
    for (const p of V8I_PATHS) {
        if (existsSync(p)) return p;
    }
    return null;
}

// ─── Platform Scanner ────────────────────────────────────────────

function findPlatform(): PlatformInfo[] {
    const searchPaths: string[] = [];
    const pf = process.env['PROGRAMFILES'] || 'C:\\Program Files';
    const pf86 = process.env['PROGRAMFILES(X86)'] || 'C:\\Program Files (x86)';
    searchPaths.push(join(pf, '1cv8'), join(pf86, '1cv8'));

    const home = homedir();
    searchPaths.push('/opt/1cv8', '/opt/1C/v8.3', join(home, '1cv8'));

    const platforms: PlatformInfo[] = [];

    for (const cv8Dir of searchPaths) {
        if (!existsSync(cv8Dir)) continue;
        try {
            const entries = readdirSync(cv8Dir);
            for (const entry of entries) {
                if (!/^\d+\.\d+\.\d+\.\d+$/.test(entry)) continue;
                const versionDir = join(cv8Dir, entry);
                try {
                    if (!statSync(versionDir).isDirectory()) continue;
                } catch { continue; }

                const binDir = join(versionDir, 'bin');
                const winExe = join(binDir, '1cv8.exe');
                const unixExe = join(binDir, '1cv8');
                const winIbcmd = join(binDir, 'ibcmd.exe');
                const unixIbcmd = join(binDir, 'ibcmd');

                const exePath = existsSync(winExe) ? winExe : existsSync(unixExe) ? unixExe : winExe;
                const ibcmdPath = existsSync(winIbcmd) ? winIbcmd : existsSync(unixIbcmd) ? unixIbcmd : winIbcmd;

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
    return { platforms: findPlatform(), infobases: parseV8iFile(), v8i_path: findV8iPath() };
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

// ─── MCP Server (SDK-based, like other mini-ai-1c servers) ──────

function createServerInstance(): Server {
    const server = new Server(
        { name: 'jvv-1c', version: '1.0.0' },
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
                        v8i_path: { type: 'string', description: 'Путь к ibases.v8i (автоопределение по умолчанию)' },
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
                    return {
                        content: [{
                            type: 'text' as const,
                            text: JSON.stringify({
                                count: bases.length,
                                v8i_path: v8iPath || findV8iPath(),
                                bases: bases.map(b => ({ name: b.name, connection: b.connection, type: b.type, id: b.id, folder: b.folder })),
                            }, null, 2),
                        }],
                    };
                }
                case 'find_platform': {
                    const platforms = findPlatform();
                    return {
                        content: [{
                            type: 'text' as const,
                            text: JSON.stringify({
                                count: platforms.length,
                                latest: platforms[0] || null,
                                platforms: platforms.map(p => ({ version: p.version, bin_path: p.bin_path, exe_path: p.exe_path, ibcmd_path: p.ibcmd_path })),
                            }, null, 2),
                        }],
                    };
                }
                case 'get_1c_environment': {
                    const env = getEnvironment();
                    return {
                        content: [{
                            type: 'text' as const,
                            text: JSON.stringify({
                                platforms: { count: env.platforms.length, latest_version: env.platforms[0]?.version || null, items: env.platforms.map(p => ({ version: p.version, exe_path: p.exe_path, ibcmd_path: p.ibcmd_path })) },
                                infobases: { count: env.infobases.length, v8i_path: env.v8i_path, items: env.infobases.map(b => ({ name: b.name, connection: b.connection, type: b.type })) },
                            }, null, 2),
                        }],
                    };
                }
                default:
                    throw new Error(`Unknown tool: ${name}`);
            }
        } catch (e: any) {
            return { content: [{ type: 'text' as const, text: `Error: ${e.message || String(e)}` }], isError: true };
        }
    });

    return server;
}

// ─── Stdio Mode ──────────────────────────────────────────────────

async function startStdio() {
    const server = createServerInstance();
    const transport = new StdioServerTransport();
    await server.connect(transport);
}

// ─── HTTP Mode (Streamable HTTP) ─────────────────────────────────

async function startHttp(port: number) {
    const httpServer = createServer(async (req: IncomingMessage, res: ServerResponse) => {
        res.setHeader('Access-Control-Allow-Origin', '*');
        res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
        res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Accept, mcp-session-id');

        if (req.method === 'OPTIONS') { res.writeHead(204); res.end(); return; }

        const url = new URL(req.url || '/', `http://localhost:${port}`);

        if (req.method === 'GET' && url.pathname === '/health') {
            const env = getEnvironment();
            res.writeHead(200, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ status: 'ok', name: 'jvv-1c', version: '1.0.0', platforms: env.platforms.length, bases: env.infobases.length }));
            return;
        }

        if (req.method === 'GET' && url.pathname === '/') {
            res.writeHead(200, { 'Content-Type': 'text/html' });
            res.end(`<!DOCTYPE html><html><body><h2>JVV-1C MCP Server v1.0.0</h2>
<p>Tools: list_infobases, find_platform, get_1c_environment</p>
<p>POST <a href="/">/</a> — Streamable HTTP (MCP)</p>
<p>GET /health — Health check</p></body></html>`);
            return;
        }

        if (req.method === 'POST') {
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
                res.end(JSON.stringify({ jsonrpc: '2.0', error: { code: -32603, message: e.message || String(e) } }));
            }
            return;
        }

        res.writeHead(404);
        res.end('Not found');
    });

    httpServer.listen(port, () => {
        process.stderr.write(`[jvv-1c] HTTP server on http://localhost:${port}\n`);
    });
}

// ─── Entry ───────────────────────────────────────────────────────

async function main() {
    reportStatus();
    const { mode, port } = parseArgs();
    if (mode === 'stdio') { await startStdio(); } else { await startHttp(port); }
}

main().catch((e) => {
    process.stderr.write(`[jvv-1c] Fatal: ${e}\n`);
    process.exit(1);
});
