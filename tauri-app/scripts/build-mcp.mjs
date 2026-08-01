import { build } from 'esbuild';
import { execSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { delimiter, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(fileURLToPath(import.meta.url));
const appDir = join(root, '..');
const mcpSrcDir = join(appDir, 'src', 'mcp-servers');
const mcpOutDir = join(appDir, 'src-tauri', 'mcp-servers');
const searchDir = join(appDir, 'mcp-1c-search');

const mode = process.argv[2] ?? 'mcp';

const HELP_BANNER =
    "if(typeof File==='undefined'){global.File=class File extends Blob{constructor(c,n,o){super(c,o);this.name=n;this.lastModified=o?.lastModified??Date.now();}}}";

if (!existsSync(mcpOutDir)) {
    mkdirSync(mcpOutDir, { recursive: true });
}

async function buildMcp() {
    const entries = [
        { in: '1c-naparnik.ts', out: '1c-naparnik.cjs' },
        { in: '1c-metadata.ts', out: '1c-metadata.cjs' },
        { in: '1c-help.ts', out: '1c-help.cjs', banner: HELP_BANNER },
        { in: 'mcp-skills.ts', out: 'mcp-skills.cjs' },
        { in: '1c-filesystem.ts', out: '1c-filesystem.cjs' },
    ];

    for (const entry of entries) {
        await build({
            entryPoints: [join(mcpSrcDir, entry.in)],
            bundle: true,
            platform: 'node',
            outfile: join(mcpOutDir, entry.out),
            banner: entry.banner ? { js: entry.banner } : undefined,
            logLevel: 'info',
        });
    }
    console.log(`MCP servers bundled into ${mcpOutDir}`);
}

function searchBinaryName() {
    return process.platform === 'win32' ? 'mcp-1c-search.exe' : 'mcp-1c-search';
}

function buildMcpSearch({ alsoCopyToTargets = false } = {}) {
    console.log('Building mcp-1c-search (release)...');
    const manifest = join(searchDir, 'Cargo.toml');
    const cargoBin = join(homedir(), '.cargo', 'bin');
    const env = {
        ...process.env,
        PATH: `${cargoBin}${delimiter}${process.env.PATH ?? ''}`,
    };
    execSync(`cargo build --release --manifest-path ${JSON.stringify(manifest)}`, {
        stdio: 'inherit',
        env,
    });

    const binary = join(searchDir, 'target', 'release', searchBinaryName());
    const dest = join(mcpOutDir, searchBinaryName());
    copyFileSync(binary, dest);
    console.log(`Copied to ${dest}`);

    if (alsoCopyToTargets) {
        for (const profile of ['debug', 'release']) {
            const dir = join(appDir, 'src-tauri', 'target', profile, 'mcp-servers');
            if (existsSync(dir)) {
                copyFileSync(binary, join(dir, searchBinaryName()));
                console.log(`Copied to ${join(dir, searchBinaryName())}`);
            }
        }
        if (process.platform === 'win32') {
            const binDest = join(
                appDir,
                'src-tauri',
                'bin',
                'mcp-1c-search-x86_64-pc-windows-msvc.exe',
            );
            copyFileSync(binary, binDest);
            console.log(`Copied to ${binDest}`);
        }
    }
}

if (mode === 'mcp') {
    await buildMcp();
} else if (mode === 'mcp-search') {
    buildMcpSearch();
} else if (mode === 'mcp-search-release') {
    buildMcpSearch({ alsoCopyToTargets: true });
} else {
    console.error(`Unknown mode: ${mode}. Expected: mcp | mcp-search | mcp-search-release`);
    process.exit(1);
}
