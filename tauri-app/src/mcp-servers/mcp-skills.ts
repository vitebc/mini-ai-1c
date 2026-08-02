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

function isValidSkillId(id: string): boolean {
    if (!id) return false;
    if (id.startsWith('/') || id.startsWith('\\')) return false;
    if (id.includes('..')) return false;
    return true;
}

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
    if (!SKILLS_DIR) {
        return [];
    }

    const skills: SkillInfo[] = [];
    function readSkillFrom(category: string, skillDir: string, name: string) {
        const sp = join(skillDir, 'SKILL.md');
        if (!existsSync(sp)) return;
        try {
            const raw = readFileSync(sp, 'utf-8');
            const { metadata } = parseSkillFrontmatter(raw);
            const id = category ? `${category}/${name}` : name;
            skills.push({
                id,
                name: (metadata.name as string) || name,
                description: (metadata.description as string) || '',
                category: category || (metadata as any).category || (metadata as any).domain || '',
            });
        } catch { /* skip */ }
    }

    try {
        const entries = readdirSync(SKILLS_DIR, { withFileTypes: true });
        for (const entry of entries) {
            if (!entry.isDirectory()) continue;
            const fullPath = join(SKILLS_DIR, entry.name);
            // Case 1: flat skill (SKILL.md directly)
            if (existsSync(join(fullPath, 'SKILL.md'))) {
                readSkillFrom('', fullPath, entry.name);
                continue;
            }
            // Case 2: category directory with sub-skills
            const subs = readdirSync(fullPath, { withFileTypes: true });
            for (const sub of subs) {
                if (!sub.isDirectory()) continue;
                readSkillFrom(entry.name, join(fullPath, sub.name), sub.name);
            }
        }
    } catch { /* no skills dir */ }

    skills.sort((a, b) => a.name.localeCompare(b.name));
    return skills;
}

function skillDirFromId(id: string): string | null {
    if (!isValidSkillId(id)) return null;
    return join(SKILLS_DIR, id);
}

function getSkillFiles(skillId: string): string[] {
    if (!SKILLS_DIR) return [];
    const skillDir = skillDirFromId(skillId);
    if (!skillDir) return [];
    if (!existsSync(skillDir)) return [];

    const sd = skillDir;
    const files: string[] = [];
    function walk(dir: string) {
        try {
            const entries = readdirSync(dir, { withFileTypes: true });
            for (const e of entries) {
                const full = join(dir, e.name);
                if (e.isDirectory()) {
                    if (e.name !== 'node_modules' && !e.name.startsWith('.')) walk(full);
                } else {
                    files.push(relative(sd, full));
                }
            }
        } catch { /* skip */ }
    }
    walk(sd);
    return files.sort();
}

function readSkillFile(skillId: string, filePath: string): string {
    const skillDir = skillDirFromId(skillId);
    if (!skillDir) return '';
    try {
        return readFileSync(join(skillDir, filePath), 'utf-8');
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
                description: 'Получить полное содержимое SKILL.md + список файлов скилла. Содержимое файлов (скрипты, документы) читай через отдельный инструмент get_skill_file.',
                inputSchema: {
                    type: 'object',
                    properties: {
                        id: {
                            type: 'string',
                            description: 'ID скилла (например: cc-1c-skills/form-add, rust-engineer, typescript-pro)',
                        },
                    },
                    required: ['id'],
                },
            },
            {
                name: 'get_skill_file',
                description: 'Прочитать содержимое конкретного файла скилла (PS1-скрипт, документация и т.д.). Сначала вызови get_skill чтобы увидеть список доступных файлов, затем вызови этот инструмент с путём.',
                inputSchema: {
                    type: 'object',
                    properties: {
                        id: {
                            type: 'string',
                            description: 'ID скилла (например: cc-1c-skills/form-add)',
                        },
                        path: {
                            type: 'string',
                            description: 'Относительный путь к файлу (например: scripts/form-add.ps1)',
                        },
                    },
                    required: ['id', 'path'],
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
            if (!isValidSkillId(id)) throw new Error('Invalid skill id');

            const all = scanSkills();
            const info = all.find(s => s.id === id);
            if (!info) throw new Error(`Skill "${id}" not found`);

            const skillDir = skillDirFromId(id);
            if (!skillDir) throw new Error('Invalid skill path');
            const skillPath = join(skillDir, 'SKILL.md');
            const raw = readFileSync(skillPath, 'utf-8');
            const { metadata, body } = parseSkillFrontmatter(raw);

            const files = getSkillFiles(id);
            const nonMdFiles = files.filter(
                f => f !== 'SKILL.md' && !f.endsWith('.exe') && !f.endsWith('.pyc')
            );

            return {
                content: [{
                    type: 'text' as const,
                    text: [
                        `# ${info.name}`,
                        ``,
                        `**ID:** \`${info.id}\``,
                        `**Описание:** ${info.description}`,
                        `**Директория скилла:** \`${skillDir}\``,
                        `**Файлов:** ${files.length}`,
                        ``,
                        `---`,
                        ``,
                        body,
                        ...(nonMdFiles.length > 0 ? [
                            ``,
                            `---`,
                            `## Доступные файлы`,
                            ``,
                            `Для чтения содержимого файлов используй инструмент \`get_skill_file\` с параметрами \`id\` = \`${id}\` и \`path\` = относительный путь.`,
                            ``,
                            ...nonMdFiles.map(f => `- \`${f}\``),
                        ] : []),
                    ].join('\n'),
                }],
            };
        }

        case 'get_skill_file': {
            const id = (args as any)?.id as string;
            const filePath = (args as any)?.path as string;
            if (!id || !filePath) throw new Error('Parameters "id" and "path" are required');
            if (!isValidSkillId(id)) throw new Error('Invalid skill id');
            // Path traversal protection
            if (filePath.includes('..') || filePath.startsWith('/') || filePath.startsWith('\\')) {
                throw new Error('Invalid file path');
            }
            const content = readSkillFile(id, filePath);
            if (!content) throw new Error(`File "${filePath}" not found in skill "${id}"`);
            return {
                content: [{ type: 'text' as const, text: `### ${filePath}\n\n${content}` }],
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
