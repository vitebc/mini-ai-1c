import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import { readFileSync, readdirSync, existsSync, statSync } from 'fs';
import { join, resolve, relative } from 'path';

// ─── Path resolution ────────────────────────────────────────────

const SKILLS_DIR = process.env.SKILLS_DIR || resolveSkillsDir();
const TIMEOUT_MS = 30_000;

function resolveSkillsDir(): string {
    // 1. Relative to current exe
    const exeDir = process.argv[1] ? resolve(process.argv[1], '..') : process.cwd();
    const candidates = [
        join(exeDir, '.agents', 'skills'),
        join(exeDir, '..', '.agents', 'skills'),
        join(exeDir, '..', '..', '.agents', 'skills'),
        join(process.cwd(), '.agents', 'skills'),
    ];
    for (const p of candidates) {
        if (existsSync(p) && statSync(p).isDirectory()) return p;
    }
    return '';
}

// ─── Types ───────────────────────────────────────────────────────

interface SkillInfo {
    id: string;
    name: string;
    description: string;
    category?: string;
}

interface SkillContent {
    id: string;
    name: string;
    description: string;
    category?: string;
    metadata: Record<string, unknown>;
    content: string;
    files: string[];
}

// ─── Skills index ────────────────────────────────────────────────

let skillsCache: SkillInfo[] | null = null;

function parseSkillFrontmatter(content: string): { metadata: Record<string, unknown>; body: string } {
    const metadata: Record<string, unknown> = {};
    if (content.startsWith('---\n')) {
        const end = content.indexOf('\n---\n', 4);
        if (end !== -1) {
            const fm = content.slice(4, end);
            for (const line of fm.split('\n')) {
                const idx = line.indexOf(': ');
                if (idx !== -1) {
                    const key = line.slice(0, idx).trim();
                    let val: unknown = line.slice(idx + 2).trim();
                    if (val === 'true') val = true;
                    else if (val === 'false') val = false;
                    else if (!isNaN(Number(val))) val = Number(val);
                    metadata[key] = val;
                }
            }
            return { metadata, body: content.slice(end + 5).trim() };
        }
    }
    return { metadata, body: content };
}

function scanSkills(): SkillInfo[] {
    if (skillsCache) return skillsCache;
    if (!SKILLS_DIR) {
        skillsCache = [];
        return skillsCache;
    }

    const skills: SkillInfo[] = [];
    try {
        const entries = readdirSync(SKILLS_DIR, { withFileTypes: true });
        for (const entry of entries) {
            if (!entry.isDirectory()) continue;
            const skillDir = join(SKILLS_DIR, entry.name);
            const skillPath = join(skillDir, 'SKILL.md');
            if (!existsSync(skillPath)) continue;

            try {
                const raw = readFileSync(skillPath, 'utf-8');
                const { metadata } = parseSkillFrontmatter(raw);
                skills.push({
                    id: entry.name,
                    name: (metadata.name as string) || entry.name,
                    description: (metadata.description as string) || '',
                    category: (metadata as any).category || (metadata as any).domain || '',
                });
            } catch { /* skip malformed */ }
        }
    } catch { /* no skills dir */ }

    skills.sort((a, b) => a.name.localeCompare(b.name));
    skillsCache = skills;
    return skills;
}

function getSkillFiles(skillId: string): string[] {
    if (!SKILLS_DIR) return [];
    const skillDir = join(SKILLS_DIR, skillId);
    if (!existsSync(skillDir)) return [];

    const files: string[] = [];
    function walk(dir: string) {
        try {
            const entries = readdirSync(dir, { withFileTypes: true });
            for (const e of entries) {
                const full = join(dir, e.name);
                if (e.isDirectory()) {
                    if (e.name !== 'node_modules' && !e.name.startsWith('.')) walk(full);
                } else {
                    files.push(relative(skillDir, full));
                }
            }
        } catch { /* skip */ }
    }
    walk(skillDir);
    return files.sort();
}

function readSkillFile(skillId: string, filePath: string): string {
    try {
        return readFileSync(join(SKILLS_DIR, skillId, filePath), 'utf-8');
    } catch {
        return '';
    }
}

// ─── MCP Server ──────────────────────────────────────────────────

const server = new Server(
    { name: 'mcp-skills', version: '1.0.0' },
    { capabilities: { tools: {} } },
);

server.setRequestHandler(ListToolsRequestSchema, async () => {
    const skills = scanSkills();
    return {
        tools: [
            {
                name: 'list_skills',
                description: 'Получить список всех доступных скиллов (наборов знаний и инструкций). Каждый скилл — это структурированное руководство по конкретной технологии или подходу: Rust, TypeScript, дизайн UI, Tauri, MCP и т.д.',
                inputSchema: {
                    type: 'object',
                    properties: {
                        category: {
                            type: 'string',
                            description: 'Фильтр по категории (опционально)',
                        },
                    },
                },
            },
            {
                name: 'get_skill',
                description: 'Получить полное содержимое скилла: его SKILL.md + все файлы (референсы, примеры). Используй этот инструмент когда нужно получить глубокие знания по конкретной технологии — Rust, TypeScript, React, Tauri, MCP и т.д.',
                inputSchema: {
                    type: 'object',
                    properties: {
                        id: {
                            type: 'string',
                            description: 'ID скилла (например: rust-engineer, typescript-pro, frontend-design, desktop-framework-tauri, mcp-builder)',
                        },
                    },
                    required: ['id'],
                },
            },
            {
                name: 'search_skills',
                description: 'Поиск по названиям и описаниям скиллов. Вернёт список подходящих скиллов с их ID и описанием.',
                inputSchema: {
                    type: 'object',
                    properties: {
                        query: {
                            type: 'string',
                            description: 'Поисковый запрос',
                        },
                    },
                    required: ['query'],
                },
            },
        ],
    };
});

server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;

    switch (name) {
        case 'list_skills': {
            const category = (args as any)?.category as string | undefined;
            const all = scanSkills();
            const filtered = category
                ? all.filter(s => s.category?.toLowerCase().includes(category.toLowerCase()))
                : all;

            return {
                content: [{
                    type: 'text' as const,
                    text: filtered.length > 0
                        ? filtered.map(s =>
                            `### ${s.name}\nID: \`${s.id}\`\n${s.description}\n${s.category ? `Категория: ${s.category}` : ''}`
                        ).join('\n\n')
                        : 'Скиллы не найдены.',
                }],
            };
        }

        case 'get_skill': {
            const id = (args as any)?.id as string;
            if (!id) throw new Error('Parameter "id" is required');

            const all = scanSkills();
            const info = all.find(s => s.id === id);
            if (!info) throw new Error(`Skill "${id}" not found`);

            const skillDir = join(SKILLS_DIR, id);
            const skillPath = join(skillDir, 'SKILL.md');
            const raw = readFileSync(skillPath, 'utf-8');
            const { metadata, body } = parseSkillFrontmatter(raw);

            const files = getSkillFiles(id);
            const references: { path: string; content: string }[] = [];

            for (const file of files) {
                if (file === 'SKILL.md' || file.endsWith('.exe') || file.endsWith('.pyc')) continue;
                try {
                    references.push({ path: file, content: readSkillFile(id, file) });
                } catch { /* skip */ }
            }

            return {
                content: [{
                    type: 'text' as const,
                    text: [
                        `# ${info.name}`,
                        ``,
                        `**ID:** \`${info.id}\``,
                        `**Описание:** ${info.description}`,
                        `**Файлов:** ${files.length}`,
                        ``,
                        `---`,
                        ``,
                        body,
                        ...(references.length > 0 ? [
                            ``,
                            `---`,
                            `## Референсы (${references.length} файлов)`,
                            ``,
                            ...references.map(r => [
                                `### ${r.path}`,
                                ``,
                                r.content,
                            ]).flat(),
                        ] : []),
                    ].join('\n'),
                }],
            };
        }

        case 'search_skills': {
            const query = ((args as any)?.query as string || '').toLowerCase();
            const all = scanSkills();
            const results = all.filter(s =>
                s.name.toLowerCase().includes(query) ||
                s.description.toLowerCase().includes(query) ||
                s.id.toLowerCase().includes(query)
            );

            return {
                content: [{
                    type: 'text' as const,
                    text: results.length > 0
                        ? results.map(s =>
                            `### ${s.name}\nID: \`${s.id}\`\n${s.description}`
                        ).join('\n\n')
                        : `По запросу "${query}" ничего не найдено.`,
                }],
            };
        }

        default:
            throw new Error(`Unknown tool: ${name}`);
    }
});

// ─── Startup ─────────────────────────────────────────────────────

async function main() {
    const skillsCount = scanSkills().length;
    process.stderr.write(`[mcp-skills] ${skillsCount} skills loaded from ${SKILLS_DIR || '(empty)'}\n`);

    const transport = new StdioServerTransport();
    await server.connect(transport);
}

main().catch((e) => {
    process.stderr.write(`[mcp-skills] Fatal: ${e}\n`);
    process.exit(1);
});
