import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { McpToolInfo } from '@/types/mcp';
import type { McpServerConfig } from '@/types/settings';
import { Wrench, RefreshCw, Info, AlertCircle } from 'lucide-react';
import { formatMcpTokenCount, summarizeMcpTokenUsage } from '@/utils/mcpTokenUsage';

interface McpToolsViewProps {
    serverName?: string | null;
    mcpServersOverride?: McpServerConfig[];
    bslEnabledOverride?: boolean;
}

function getToolIdentity(tool: McpToolInfo) {
    return `${tool.server_name}::${tool.tool_name}`;
}

export function McpToolsView({
    serverName,
    mcpServersOverride,
    bslEnabledOverride,
}: McpToolsViewProps) {
    const [tools, setTools] = useState<McpToolInfo[]>([]);
    const [loading, setLoading] = useState<boolean>(true);
    const [error, setError] = useState<string | null>(null);
    const [expandedTool, setExpandedTool] = useState<string | null>(null);

    const fetchTools = async (force = false) => {
        setLoading(true);
        setError(null);
        try {
            // Tauri invoke: command name and single arg object
            const res = (await invoke('list_mcp_tools', {
                force_refresh: force,
                mcp_servers_override: mcpServersOverride,
                bsl_enabled_override: bslEnabledOverride,
            })) as McpToolInfo[];
            const filtered = serverName ? res.filter(t => t.server_name === serverName) : res;
            // Deduplicate tools by server_name + tool_name to avoid collisions across servers.
            const seen = new Set<string>();
            const deduped: McpToolInfo[] = [];
            for (const t of filtered) {
                const toolId = getToolIdentity(t);
                if (!seen.has(toolId)) {
                    seen.add(toolId);
                    deduped.push(t);
                }
            }
            setTools(deduped);
        } catch (e: any) {
            setError(e?.toString() || 'Failed to fetch tools');
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchTools(false);
    }, [serverName, mcpServersOverride, bslEnabledOverride]);

    const grouped = tools.reduce<Record<string, McpToolInfo[]>>((acc, t) => {
        acc[t.server_name] = acc[t.server_name] || [];
        acc[t.server_name].push(t);
        return acc;
    }, {});
    const tokenUsage = summarizeMcpTokenUsage(tools);

    if (loading) {
        return <div className="p-4"><div className="text-sm text-zinc-500">Loading MCP tools...</div></div>;
    }

    if (error) {
        return (
            <div className="p-4">
                <div className="text-sm text-red-400 mb-2">Error: {error}</div>
                <button onClick={() => fetchTools(true)} className="px-3 py-1 bg-zinc-800 rounded">Retry</button>
            </div>
        );
    }

    const serverNames = Object.keys(grouped);
    if (serverNames.length === 0) {
        return <div className="p-4 text-sm text-zinc-500">No MCP tools available.</div>;
    }

    return (
        <div className="p-3 overflow-auto max-h-full">
            <div className="flex items-center justify-between mb-3">
                <div className="min-w-0">
                    <div className="flex items-center gap-2 text-sm font-semibold">
                        <Wrench className="w-4 h-4" /> MCP Tools
                    </div>
                    {tokenUsage.totalTools > 0 && (
                        <div className="mt-1 text-[11px] font-mono text-zinc-500">
                            Всего: ≈{formatMcpTokenCount(tokenUsage.totalEstimatedTokens)} токенов · {tokenUsage.totalTools} tools
                        </div>
                    )}
                </div>
                <div>
                    <button onClick={() => fetchTools(true)} className="px-2 py-1 rounded bg-zinc-800 hover:bg-zinc-700 text-sm flex items-center gap-2">
                        <RefreshCw className="w-3.5 h-3.5" /> Refresh
                    </button>
                </div>
            </div>
            <div className="flex flex-col gap-3">
                {serverNames.map(server => (
                    <div key={server} className="border border-zinc-800 rounded-lg p-3">
                        <div className="flex items-center justify-between mb-2">
                            <div className="text-sm font-semibold">{server}</div>
                            {tokenUsage.byServerName[server] && (
                                <div className="rounded bg-zinc-900 px-2 py-1 text-[11px] font-mono text-zinc-500">
                                    ≈{formatMcpTokenCount(tokenUsage.byServerName[server].estimatedTokens)} токенов
                                </div>
                            )}
                        </div>
                        <div className="grid grid-cols-1 gap-2">
                            {grouped[server].map(tool => {
                                const toolId = getToolIdentity(tool);
                                const isExpanded = expandedTool === toolId;
                                const isUnavailableTool = tool.tool_name === '__server_unavailable__';

                                if (isUnavailableTool) {
                                    return (
                                        <div
                                            key={toolId}
                                            className="rounded-lg border border-amber-500/20 bg-amber-500/5 p-3 text-sm"
                                        >
                                            <div className="flex items-start gap-2">
                                                <AlertCircle className="mt-0.5 h-4 w-4 shrink-0 text-amber-400" />
                                                <div>
                                                    <div className="font-medium text-amber-300">Инструменты недоступны</div>
                                                    <div className="mt-1 text-xs leading-relaxed text-zinc-400">
                                                        {tool.description ?? 'Сервер не готов отдавать список инструментов.'}
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    );
                                }

                                return (
                                    <div
                                        key={toolId}
                                        className={`p-2 border border-zinc-700 rounded transition-colors ${isExpanded ? 'bg-zinc-900' : 'hover:bg-zinc-900'} cursor-pointer`}
                                        onClick={() => setExpandedTool(isExpanded ? null : toolId)}
                                        role="button"
                                        tabIndex={0}
                                    >
                                        <div className="flex items-center justify-between">
                                            <div className="min-w-0">
                                                <div className="text-sm font-medium truncate">{tool.tool_name}</div>
                                                <div
                                                    className="text-xs text-zinc-500"
                                                    style={{
                                                        overflow: 'hidden',
                                                        maxHeight: isExpanded ? '1000px' : '3.2rem',
                                                        transition: 'max-height 220ms ease'
                                                    }}
                                                >
                                                    <div className={isExpanded ? '' : 'line-clamp-2'}>
                                                        {tool.description ?? ''}
                                                    </div>
                                                </div>
                                            </div>
                                            <div className="ml-3 flex items-center gap-2">
                                                {tool.estimated_tokens > 0 && (
                                                    <span
                                                        className="rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-mono text-zinc-500"
                                                        title="Оценка токенов tool payload"
                                                    >
                                                        ≈{formatMcpTokenCount(tool.estimated_tokens)}
                                                    </span>
                                                )}
                                                <div className={`w-2.5 h-2.5 rounded-full ${tool.is_enabled ? 'bg-emerald-400' : 'bg-zinc-600'}`} />
                                                <button
                                                    className="text-zinc-400 hover:text-zinc-200 p-1"
                                                    title="Info"
                                                    onClick={(e) => {
                                                        e.stopPropagation();
                                                        setExpandedTool(isExpanded ? null : toolId);
                                                    }}
                                                >
                                                    <Info className="w-4 h-4" />
                                                </button>
                                            </div>
                                        </div>
                                    </div>
                                );
                            })}
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
}

export default McpToolsView;

