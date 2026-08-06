import { execSync } from 'node:child_process';
import { chmodSync, copyFileSync, existsSync, mkdirSync, readdirSync, rmSync } from 'node:fs';
import { homedir } from 'node:os';
import { delimiter, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(fileURLToPath(import.meta.url));
const appDir = join(root, '..');
const mcpOutDir = join(appDir, 'src-tauri', 'mcp-servers');
const searchDir = join(appDir, 'mcp-1c-search');

const mode = process.argv[2] ?? 'mcp';

// Все Rust MCP-серверы (кроме mcp-1c-search — собирается отдельно).
const RUST_SERVERS = [
    'mcp-1c-skills',
    'mcp-1c-jvv',
    'mcp-1c-filesystem',
    'mcp-1c-naparnik',
    'mcp-1c-help',
    'mcp-1c-metadata',
];

if (!existsSync(mcpOutDir)) {
    mkdirSync(mcpOutDir, { recursive: true });
}

function binaryName(serverName) {
    return process.platform === 'win32' ? `${serverName}.exe` : serverName;
}

function cargoBinDir() {
    return join(homedir(), '.cargo', 'bin');
}

function runCargoBuild(serverDir) {
    const manifest = join(serverDir, 'Cargo.toml');
    const env = {
        ...process.env,
        PATH: `${cargoBinDir()}${delimiter}${process.env.PATH ?? ''}`,
    };
    execSync(`cargo build --release --manifest-path ${JSON.stringify(manifest)}`, {
        stdio: 'inherit',
        env,
    });
}

function copyToOut(serverDir, serverName) {
    const binary = join(serverDir, 'target', 'release', binaryName(serverName));
    const dest = join(mcpOutDir, binaryName(serverName));
    copyFileSync(binary, dest);
    if (process.platform !== 'win32') {
        chmodSync(dest, 0o755);
    }
    console.log(`Copied to ${dest}`);
}

function buildMcp() {
    console.log('Building Rust MCP servers...');
    // Удаляем устаревшие .cjs-артефакты из выходного каталога
    if (existsSync(mcpOutDir)) {
        for (const f of readdirSync(mcpOutDir)) {
            if (f.endsWith('.cjs')) {
                rmSync(join(mcpOutDir, f), { force: true });
                console.log(`Removed stale ${f}`);
            }
        }
    }
    for (const server of RUST_SERVERS) {
        const serverDir = join(appDir, server);
        if (!existsSync(join(serverDir, 'Cargo.toml'))) {
            console.error(`[WARN] ${serverDir}/Cargo.toml not found, skipping`);
            continue;
        }
        console.log(`Building ${server}...`);
        runCargoBuild(serverDir);
        copyToOut(serverDir, server);
    }
    console.log(`MCP servers (Rust) built into ${mcpOutDir}`);
}

function searchBinaryName() {
    return process.platform === 'win32' ? 'mcp-1c-search.exe' : 'mcp-1c-search';
}

function buildMcpSearch({ alsoCopyToTargets = false } = {}) {
    console.log('Building mcp-1c-search (release)...');
    const manifest = join(searchDir, 'Cargo.toml');
    const env = {
        ...process.env,
        PATH: `${cargoBinDir()}${delimiter}${process.env.PATH ?? ''}`,
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
    buildMcp();
} else if (mode === 'mcp-search') {
    buildMcpSearch();
} else if (mode === 'mcp-search-release') {
    buildMcpSearch({ alsoCopyToTargets: true });
} else {
    console.error(`Unknown mode: ${mode}. Expected: mcp | mcp-search | mcp-search-release`);
    process.exit(1);
}
