import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import { readFileSync, readdirSync, existsSync, statSync } from 'fs';
import { join, resolve } from 'path';

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
    cestart_path: string;
    exists: boolean;
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

    // File="..."
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

    // Srvr="...";Ref="..."
    if (lower.includes('srvr=') && lower.includes('ref=')) {
        return { connection: connect, type: 'server' };
    }

    // S="server/db"
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

        // Section header [Name]
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

        // Key=Value
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

    // Save last
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
                // Version folders: "8.3.27.1989" (exactly 3 dots)
                if (!/^\d+\.\d+\.\d+\.\d+$/.test(entry)) continue;

                const versionDir = join(cv8Dir, entry);
                if (!statSync(versionDir).isDirectory()) continue;

                const binDir = join(versionDir, 'bin');
                const exePath = join(binDir, '1cv8.exe');
                const ibcmdPath = join(binDir, 'ibcmd.exe');

                if (existsSync(exePath)) {
                    // Check for 1cestart.exe in common directory
                    const commonDir = join(cv8Dir, 'common');
                    const cestartPath = join(commonDir, '1cestart.exe');
                    platforms.push({
                        version: entry,
                        bin_path: binDir,
                        exe_path: exePath,
                        ibcmd_path: ibcmdPath,
                        cestart_path: existsSync(cestartPath) ? cestartPath : '',
                        exists: true,
                    });
                }
            }
        } catch { /* skip */ }
    }

    // Sort by semantic version descending
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

// ─── Combined Environment ────────────────────────────────────────

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

// ─── MCP Server ──────────────────────────────────────────────────

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
            inputSchema: {
                type: 'object',
                properties: {},
            },
        },
        {
            name: 'get_1c_environment',
            description: 'Комбинированная информация: установленные платформы + список баз 1С + путь к ibases.v8i. Один вызов — вся картина.',
            inputSchema: {
                type: 'object',
                properties: {},
            },
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
                    bases: bases.map(b => ({
                        name: b.name,
                        connection: b.connection,
                        type: b.type,
                        id: b.id,
                        folder: b.folder,
                    })),
                });
            }

            case 'find_platform': {
                const platforms = findPlatform();
                return ok({
                    count: platforms.length,
                    latest: platforms[0] || null,
                    platforms: platforms.map(p => ({
                        version: p.version,
                        bin_path: p.bin_path,
                        exe_path: p.exe_path,
                        ibcmd_path: p.ibcmd_path,
                        cestart_path: p.cestart_path || null,
                    })),
                });
            }

            case 'get_1c_environment': {
                const env = getEnvironment();
                return ok({
                    platforms: {
                        count: env.platforms.length,
                        latest_version: env.platforms[0]?.version || null,
                        items: env.platforms.map(p => ({
                            version: p.version,
                            exe_path: p.exe_path,
                            ibcmd_path: p.ibcmd_path,
                        })),
                    },
                    infobases: {
                        count: env.infobases.length,
                        v8i_path: env.v8i_path,
                        items: env.infobases.map(b => ({
                            name: b.name,
                            connection: b.connection,
                            type: b.type,
                        })),
                    },
                });
            }

            default:
                throw new Error(`Unknown tool: ${name}`);
        }
    } catch (e: any) {
        return err(e.message || String(e));
    }
});

// ─── Helpers ─────────────────────────────────────────────────────

function ok(data: any) {
    return {
        content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }],
    };
}

function err(msg: string) {
    return {
        content: [{ type: 'text' as const, text: `Error: ${msg}` }],
        isError: true,
    };
}

// ─── Startup ─────────────────────────────────────────────────────

async function main() {
    reportStatus();
    const transport = new StdioServerTransport();
    await server.connect(transport);
}

main().catch((e) => {
    process.stderr.write(`[jvv-1c] Fatal: ${e}\n`);
    process.exit(1);
});
