/** Платформо-зависимое имя Rust-бинарника MCP-сервера (например mcp-1c-skills). */
export function rustMcpBinaryName(baseName: string): string {
    return navigator.platform.toLowerCase().includes('win') ? `${baseName}.exe` : baseName;
}
