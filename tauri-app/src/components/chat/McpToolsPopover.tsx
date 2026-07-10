import { useEffect, useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { McpToolInfo } from '@/types/mcp';
import type { McpServerConfig } from '@/types/settings';
import { RefreshCw } from 'lucide-react';
import { formatMcpTokenCount, summarizeMcpTokenUsage } from '@/utils/mcpTokenUsage';

interface Props {
    onToolSelect: (toolName: string) => void;
    onClose: () => void;
    mcpServersOverride?: McpServerConfig[];
    bslEnabledOverride?: boolean;
}

function getToolIdentity(tool: McpToolInfo) {
    return `${tool.server_name}::${tool.tool_name}`;
}

function sanitizeTools(tools: McpToolInfo[]) {
    const seen = new Set<string>();
    const deduped: McpToolInfo[] = [];

    for (const tool of tools) {
        if (!tool.is_enabled || tool.tool_name === '__server_unavailable__') {
            continue;
        }

        const toolId = getToolIdentity(tool);
        if (!seen.has(toolId)) {
            seen.add(toolId);
            deduped.push(tool);
        }
    }

    return deduped;
}

export default function McpToolsPopover({
    onToolSelect,
    onClose,
    mcpServersOverride,
    bslEnabledOverride,
}: Props) {
    const [tools, setTools] = useState<McpToolInfo[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const popoverRef = useRef<HTMLDivElement>(null);

    const fetchTools = async (force = false) => {
        setLoading(true);
        setError(null);
        try {
            const res = (await invoke('list_mcp_tools', {
                force_refresh: force,
                mcp_servers_override: mcpServersOverride,
                bsl_enabled_override: bslEnabledOverride,
            })) as McpToolInfo[];
            setTools(sanitizeTools(res));
        } catch (e: any) {
            setError(e?.toString() || 'Failed to fetch tools');
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        // The input popover should reflect just-saved MCP settings immediately.
        fetchTools(true);
    }, [mcpServersOverride, bslEnabledOverride]);

    useEffect(() => {
        const handleClickOutside = (e: MouseEvent) => {
            if (popoverRef.current && !popoverRef.current.contains(e.target as Node)) {
                onClose();
            }
        };
        const handleEsc = (e: KeyboardEvent) => {
            if (e.key === 'Escape') onClose();
        };
        document.addEventListener('mousedown', handleClickOutside);
        document.addEventListener('keydown', handleEsc);
        return () => {
            document.removeEventListener('mousedown', handleClickOutside);
            document.removeEventListener('keydown', handleEsc);
        };
    }, [onClose]);

    const grouped = tools.reduce<Record<string, McpToolInfo[]>>((acc, t) => {
        acc[t.server_name] = acc[t.server_name] || [];
        acc[t.server_name].push(t);
        return acc;
    }, {});
    const tokenUsage = summarizeMcpTokenUsage(tools);

    return (
        <div
            ref={popoverRef}
            className="absolute bottom-full right-0 mb-2 w-[420px] bg-[#09090b] border border-zinc-800 rounded-xl shadow-2xl overflow-hidden z-50 animate-in slide-in-from-bottom-2 duration-200"
        >
            <div className="flex items-center justify-between px-3 py-2 border-b border-zinc-800">
                <div className="min-w-0">
                    <div className="text-sm font-semibold text-zinc-200">MCP Tools</div>
                    {!loading && !error && tokenUsage.totalTools > 0 && (
                        <div className="mt-0.5 text-[10px] font-mono text-zinc-500">
                            Всего: ≈{formatMcpTokenCount(tokenUsage.totalEstimatedTokens)} токенов · {tokenUsage.totalTools} tools
                        </div>
                    )}
                </div>
                <div className="flex items-center gap-2">
                    <button onClick={() => fetchTools(true)} className="p-1 rounded hover:bg-zinc-800" title="Refresh">
                        <RefreshCw className="w-3.5 h-3.5 text-zinc-400" />
                    </button>
                    <button onClick={onClose} className="p-1 rounded hover:bg-zinc-800 text-zinc-500 hover:text-zinc-300 text-xs font-bold">
                        ✕
                    </button>
                </div>
            </div>
            <div className="max-h-72 overflow-y-auto custom-scrollbar">
                {loading && <div className="p-3 text-sm text-zinc-500">Loading...</div>}
                {error && <div className="p-3 text-sm text-red-400">Error: {error}</div>}
                {!loading && !error && Object.keys(grouped).length === 0 && (
                    <div className="p-3 text-sm text-zinc-500">No tools available</div>
                )}
                {!loading && !error && Object.entries(grouped).map(([server, arr]) => (
                    <div key={server} className="border-b border-zinc-800/50 last:border-b-0">
                        <div className="flex items-center justify-between gap-2 px-3 py-1.5 bg-zinc-900/50">
                            <div className="min-w-0 truncate text-[10px] font-bold text-zinc-500 uppercase tracking-wider">{server}</div>
                            {tokenUsage.byServerName[server] && (
                                <div className="shrink-0 text-[10px] font-mono text-zinc-500">
                                    ≈{formatMcpTokenCount(tokenUsage.byServerName[server].estimatedTokens)}
                                </div>
                            )}
                        </div>
                        <div className="flex flex-col">
                            {arr.map(t => (
                                <button
                                    key={getToolIdentity(t)}
                                    onClick={() => onToolSelect(t.tool_name)}
                                    className="flex items-center justify-between px-3 py-2 hover:bg-zinc-800/50 text-left transition-colors"
                                >
                                    <div className="min-w-0 flex-1">
                                        <div className="text-[12px] font-medium text-zinc-200">{t.tool_name}</div>
                                        {t.description && (
                                            <div className="text-[10px] text-zinc-500 line-clamp-2">{t.description}</div>
                                        )}
                                    </div>
                                    <div className="ml-2 flex items-center gap-1.5 flex-shrink-0">
                                        {t.estimated_tokens > 0 && (
                                            <span
                                                className="rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-mono text-zinc-500"
                                                title="Оценка токенов tool payload"
                                            >
                                                ≈{formatMcpTokenCount(t.estimated_tokens)}
                                            </span>
                                        )}
                                        <div className={`w-2 h-2 rounded-full ${t.is_enabled ? 'bg-emerald-400' : 'bg-zinc-600'}`} />
                                    </div>
                                </button>
                            ))}
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
}
