import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import {
    readFileSync,
    writeFileSync,
    readdirSync,
    statSync,
    existsSync,
    mkdirSync,
    unlinkSync,
    rmSync,
    renameSync,
} from 'fs';
import { resolve, relative, join, basename, dirname } from 'path';

// ─── Config ──────────────────────────────────────────────────────

const SANDBOX = (process.env.MINI_AI_1C_SANDBOX_PATH || '').trim();

// ─── Status reporting ────────────────────────────────────────────

function emitStatus(status: string) {
    process.stderr.write(`FS_STATUS:${status}\n`);
}

function reportStatus() {
    if (!SANDBOX) {
        emitStatus('unavailable');
    } else if (!existsSync(SANDBOX) || !statSync(SANDBOX).isDirectory()) {
        emitStatus('error:sandbox directory does not exist');
    } else {
        emitStatus('ready');
    }
}

// ─── Sandbox validation ──────────────────────────────────────────

function resolveSandboxPath(requested: string): string | null {
    const resolved = resolve(SANDBOX, requested);
    if (!resolved.startsWith(SANDBOX)) return null;
    if (resolved.includes('..')) return null;
    return resolved;
}

// ─── MCP Server ──────────────────────────────────────────────────

const server = new Server(
    { name: '1c-filesystem', version: '1.0.0' },
    { capabilities: { tools: {} } },
);

server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: [
        {
            name: 'read_file',
            description: 'Read file content. Returns content as text or base64 with size.',
            inputSchema: {
                type: 'object',
                properties: {
                    path: { type: 'string', description: 'Relative path from sandbox root' },
                },
                required: ['path'],
            },
        },
        {
            name: 'write_file',
            description: 'Write content to a file (creates or overwrites).',
            inputSchema: {
                type: 'object',
                properties: {
                    path: { type: 'string', description: 'Relative path from sandbox root' },
                    content: { type: 'string', description: 'Content to write' },
                },
                required: ['path', 'content'],
            },
        },
        {
            name: 'edit_file',
            description: 'Find and replace exact string in a file.',
            inputSchema: {
                type: 'object',
                properties: {
                    path: { type: 'string', description: 'Relative path from sandbox root' },
                    old_string: { type: 'string', description: 'Exact string to find (must be unique or provide enough context)' },
                    new_string: { type: 'string', description: 'Replacement string' },
                },
                required: ['path', 'old_string', 'new_string'],
            },
        },
        {
            name: 'list_directory',
            description: 'List entries in a directory with type, size, modified date.',
            inputSchema: {
                type: 'object',
                properties: {
                    path: { type: 'string', description: 'Relative path from sandbox root (default: .)' },
                },
            },
        },
        {
            name: 'file_info',
            description: 'Get file/directory metadata: exists, type, size, modified, permissions.',
            inputSchema: {
                type: 'object',
                properties: {
                    path: { type: 'string', description: 'Relative path from sandbox root' },
                },
                required: ['path'],
            },
        },
        {
            name: 'search_files',
            description: 'Search for files by glob pattern (recursively).',
            inputSchema: {
                type: 'object',
                properties: {
                    pattern: { type: 'string', description: 'Glob pattern (e.g. "**/*.bsl", "*.xml")' },
                    root: { type: 'string', description: 'Relative subdirectory to search in (default: sandbox root)' },
                },
                required: ['pattern'],
            },
        },
        {
            name: 'create_directory',
            description: 'Create a directory (including parent dirs).',
            inputSchema: {
                type: 'object',
                properties: {
                    path: { type: 'string', description: 'Relative path from sandbox root' },
                },
                required: ['path'],
            },
        },
        {
            name: 'delete_file',
            description: 'Delete a file or empty directory.',
            inputSchema: {
                type: 'object',
                properties: {
                    path: { type: 'string', description: 'Relative path from sandbox root' },
                },
                required: ['path'],
            },
        },
        {
            name: 'delete_directory',
            description: 'Delete a directory, optionally recursively.',
            inputSchema: {
                type: 'object',
                properties: {
                    path: { type: 'string', description: 'Relative path from sandbox root' },
                    recursive: { type: 'boolean', description: 'Delete all contents recursively (default: false)' },
                },
                required: ['path'],
            },
        },
        {
            name: 'move_file',
            description: 'Move or rename a file/directory within sandbox.',
            inputSchema: {
                type: 'object',
                properties: {
                    source: { type: 'string', description: 'Source relative path from sandbox root' },
                    destination: { type: 'string', description: 'Destination relative path from sandbox root' },
                },
                required: ['source', 'destination'],
            },
        },
    ],
}));

server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;

    if (!SANDBOX || !existsSync(SANDBOX)) {
        return {
            content: [{ type: 'text' as const, text: `Error: sandbox not configured (MINI_AI_1C_SANDBOX_PATH=${SANDBOX || '(empty)'})` }],
            isError: true,
        };
    }

    const a = (args || {}) as Record<string, any>;

    try {
        switch (name) {
            case 'read_file': {
                const p = resolveSandboxPath(a.path);
                if (!p) return err('Path escapes sandbox');
                if (!existsSync(p)) return err('File not found');
                const stat = statSync(p);
                if (stat.isDirectory()) return err('Path is a directory');
                const content = readFileSync(p, 'utf-8');
                return ok({ content, encoding: 'text', size: stat.size });
            }

            case 'write_file': {
                const p = resolveSandboxPath(a.path);
                if (!p) return err('Path escapes sandbox');
                mkdirSync(dirname(p), { recursive: true });
                writeFileSync(p, a.content, 'utf-8');
                const size = statSync(p).size;
                return ok({ success: true, size });
            }

            case 'edit_file': {
                const p = resolveSandboxPath(a.path);
                if (!p) return err('Path escapes sandbox');
                if (!existsSync(p)) return err('File not found');
                const content = readFileSync(p, 'utf-8');
                if (!content.includes(a.old_string)) return err('String not found in file');
                const count = content.split(a.old_string).length - 1;
                const updated = content.replace(a.old_string, a.new_string);
                writeFileSync(p, updated, 'utf-8');
                return ok({ success: true, changes: count });
            }

            case 'list_directory': {
                const p = resolveSandboxPath(a.path || '.');
                if (!p) return err('Path escapes sandbox');
                if (!existsSync(p)) return err('Directory not found');
                const entries = readdirSync(p, { withFileTypes: true });
                const items = entries.map((e) => {
                    const full = join(p, e.name);
                    const s = statSync(full);
                    return {
                        name: e.name,
                        type: e.isDirectory() ? 'directory' : 'file',
                        size: s.size,
                        modified: s.mtime.toISOString(),
                    };
                });
                return ok({ entries: items });
            }

            case 'file_info': {
                const p = resolveSandboxPath(a.path);
                if (!p) return err('Path escapes sandbox');
                if (!existsSync(p)) return ok({ exists: false });
                const s = statSync(p);
                return ok({
                    exists: true,
                    type: s.isDirectory() ? 'directory' : s.isFile() ? 'file' : 'other',
                    size: s.size,
                    modified: s.mtime.toISOString(),
                    permissions: (s.mode & 0o777).toString(8),
                });
            }

            case 'search_files': {
                const root = resolveSandboxPath(a.root || '.');
                if (!root) return err('Path escapes sandbox');
                if (!existsSync(root)) return err('Root directory not found');
                const results: string[] = [];
                function walk(dir: string, rel: string) {
                    try {
                        const entries = readdirSync(dir, { withFileTypes: true });
                        for (const e of entries) {
                            const full = join(dir, e.name);
                            const relPath = rel ? `${rel}/${e.name}` : e.name;
                            if (e.isDirectory()) {
                                walk(full, relPath);
                            } else if (matchGlob(a.pattern, e.name)) {
                                results.push(relPath);
                            }
                        }
                    } catch { /* skip */ }
                }
                walk(root, '');
                return ok({ files: results });
            }

            case 'create_directory': {
                const p = resolveSandboxPath(a.path);
                if (!p) return err('Path escapes sandbox');
                mkdirSync(p, { recursive: true });
                return ok({ success: true });
            }

            case 'delete_file': {
                const p = resolveSandboxPath(a.path);
                if (!p) return err('Path escapes sandbox');
                if (!existsSync(p)) return err('File not found');
                const s = statSync(p);
                if (s.isDirectory()) {
                    const items = readdirSync(p);
                    if (items.length > 0) return err('Directory not empty — use delete_directory with recursive');
                    rmdirSync(p);
                } else {
                    unlinkSync(p);
                }
                return ok({ success: true });
            }

            case 'delete_directory': {
                const p = resolveSandboxPath(a.path);
                if (!p) return err('Path escapes sandbox');
                if (!existsSync(p)) return err('Directory not found');
                if (a.recursive) {
                    rmSync(p, { recursive: true });
                } else {
                    const items = readdirSync(p);
                    if (items.length > 0) return err('Directory not empty — set recursive=true');
                    rmSync(p);
                }
                return ok({ success: true });
            }

            case 'move_file': {
                const src = resolveSandboxPath(a.source);
                const dst = resolveSandboxPath(a.destination);
                if (!src || !dst) return err('Path escapes sandbox');
                if (!existsSync(src)) return err('Source not found');
                mkdirSync(dirname(dst), { recursive: true });
                renameSync(src, dst);
                return ok({ success: true });
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

function rmdirSync(p: string) {
    try { unlinkSync(p); } catch { /* ignore */ }
}

function matchGlob(pattern: string, filename: string): boolean {
    const parts = pattern.split('/');
    const last = parts[parts.length - 1];
    const starIdx = last.indexOf('*');
    if (starIdx === -1) return filename === last;
    const prefix = last.slice(0, starIdx);
    const suffix = last.slice(starIdx + 1);
    return filename.startsWith(prefix) && filename.endsWith(suffix);
}

// ─── Startup ─────────────────────────────────────────────────────

async function main() {
    reportStatus();
    const transport = new StdioServerTransport();
    await server.connect(transport);
}

main().catch((e) => {
    process.stderr.write(`[1c-filesystem] Fatal: ${e}\n`);
    process.exit(1);
});
