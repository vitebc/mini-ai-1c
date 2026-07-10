# AGENTS.md — Mini AI 1C

## Monorepo structure

- **Root** `package.json` with npm workspaces (`tauri-app`).
- **Two independent Rust crates** (separate `Cargo.toml`, own targets):
  - `tauri-app/src-tauri/` — main Tauri 2 app (lib crate `mini_ai_1c_lib`, entrypoint `lib.rs::run()`)
  - `tauri-app/mcp-1c-search/` — standalone MCP search binary
- **Frontend** (`tauri-app/`): React 19, TypeScript, Vite, Tailwind CSS v4, Monaco Editor, Radix UI.

## Key commands

All commands run from the **`tauri-app/`** directory unless noted.

| Action | Command |
|--------|---------|
| Install deps | `npm install` (root, for workspace resolution) |
| Dev server | `npm run app:dev` (builds MCP search Rust binary → bundles TS MCP servers → `tauri dev`) |
| Production build | `npm run app:build` |
| Full rebuild | `npm run full:build` (release MCP search + app build) |
| Build MCP servers only | `npm run build:mcp` (esbuild TS → .cjs) |
| Build MCP search only | `npm run build:mcp-search` (Rust release + copy) |
| TypeScript check | `npm run build` (runs `tsc` then `vite build`) |
| Rust tests | `npm run test:rust` (cargo test in src-tauri) |
| Diff tests | `npm run test:diff` (from any workspace-aware dir) |

## Dev server quirks

- Vite listens on **port 1440** (not 5173).
- Two HTML entrypoints in Rollup config: `index.html` (main window) and `overlay.html` (overlay popup).
- `tsc` type-checking is a separate step before Vite build.

## Rust architecture

- `main.rs` is just `mini_ai_1c_lib::run()`.
- Windows-only modules are gated with `#[cfg(windows)]` — `configurator`, `editor_bridge`, `mouse_hook`, `scintilla`.
- `mcp-1c-search/` is a **separate** Rust binary (not a workspace member), built independently with `cargo build --release`.
- App data migration from `com.miniai1c.agent` → `com.mini-ai-1c` runs on startup in `lib.rs`.

## Windows-only features

The app is designed for **Windows 10/11** with the 1C:Enterprise Configurator. Key Windows-specific components:
- **EditorBridge** (.NET, named pipe `\\.\pipe\mini-ai-editor-bridge-<USERNAME>`)
- **BSL Language Server** (requires Java 17+)
- **Quick Actions** overlay via global mouse hook (`Ctrl + right-click` in Configurator)
- WebView2 Runtime must be installed (Tauri 2 requirement on Windows)

## Testing

- `tests/` directory is in `.gitignore` — test sources are not committed.
- `tests/scenarios/configurator_quick_actions_remaining.md` is the only tracked test-related file (scenario doc, not executable).
- No CI workflows found (no `.github/`).

## TypeScript path aliases

`@/` and `@src/` both resolve to `tauri-app/src/`.

## Git-ignored files to know about

- `settings.json`, `llm_profiles.json`, `.env` — sensitive runtime config, never committed.
- `tests/`, `docs/`, `temp/`, `artifacts/`, `backup/` — development artifacts.

## Bundle & packaging

- `tauri build` outputs to `src-tauri/target/release/bundle/`.
- MCP server binaries are embedded via `include_bytes!` and auto-extracted on first run.
- Prod installers are MSI/NSIS (`npm run repack:installers`).
