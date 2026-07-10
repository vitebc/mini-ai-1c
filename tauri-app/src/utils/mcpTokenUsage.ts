import type { McpToolInfo } from '../types/mcp';

export interface McpTokenToolSummary {
    toolName: string;
    description: string | null;
    estimatedTokens: number;
}

export interface McpTokenServerSummary {
    serverName: string;
    toolCount: number;
    estimatedTokens: number;
    tools: McpTokenToolSummary[];
}

export interface McpTokenUsageSummary {
    totalTools: number;
    totalEstimatedTokens: number;
    servers: McpTokenServerSummary[];
    byServerName: Record<string, McpTokenServerSummary>;
}

function isRealEnabledTool(tool: McpToolInfo): boolean {
    return tool.is_enabled && tool.tool_name !== '__server_unavailable__';
}

export function summarizeMcpTokenUsage(tools: McpToolInfo[]): McpTokenUsageSummary {
    const byServerName: Record<string, McpTokenServerSummary> = {};
    const servers: McpTokenServerSummary[] = [];

    for (const tool of tools) {
        if (!isRealEnabledTool(tool)) continue;

        let server = byServerName[tool.server_name];
        if (!server) {
            server = {
                serverName: tool.server_name,
                toolCount: 0,
                estimatedTokens: 0,
                tools: [],
            };
            byServerName[tool.server_name] = server;
            servers.push(server);
        }

        const estimatedTokens = Math.max(0, Math.round(tool.estimated_tokens || 0));
        server.toolCount += 1;
        server.estimatedTokens += estimatedTokens;
        server.tools.push({
            toolName: tool.tool_name,
            description: tool.description,
            estimatedTokens,
        });
    }

    return {
        totalTools: servers.reduce((sum, server) => sum + server.toolCount, 0),
        totalEstimatedTokens: servers.reduce((sum, server) => sum + server.estimatedTokens, 0),
        servers,
        byServerName,
    };
}

export function formatMcpTokenCount(tokens: number): string {
    const rounded = Math.max(0, Math.round(tokens));
    if (rounded >= 1000) {
        return `${(rounded / 1000).toFixed(1)}k`;
    }
    return String(rounded);
}
