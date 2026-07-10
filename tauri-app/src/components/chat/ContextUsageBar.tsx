import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { AlertTriangle, Plus } from 'lucide-react';
import { resolveContextUsageDisplay, type ContextUsagePayload } from '../../utils/contextUsage';

interface ContextUsageBarProps {
    onNewChat?: () => void;
    /** Сбрасывать индикатор при смене профиля */
    profileId?: string;
    /** Сбрасывать индикатор при смене чата */
    chatId?: string | null;
    /** Пользовательский лимит из активного LLM-профиля */
    configuredContextWindow?: number | null;
    /** Принимаем новые usage-события только во время активной генерации */
    isLoading?: boolean;
}

function formatTokens(n: number): string {
    if (n >= 1000) return `~${(n / 1000).toFixed(1)}k`;
    return `~${n}`;
}

function formatWindow(n: number): string {
    if (n >= 1000) return `${Math.round(n / 1000)}k`;
    return String(n);
}

export function ContextUsageBar({
    onNewChat,
    profileId,
    chatId,
    configuredContextWindow,
    isLoading = false,
}: ContextUsageBarProps) {
    const [usage, setUsage] = useState<ContextUsagePayload | null>(null);

    // Сброс при смене профиля или чата
    useEffect(() => {
        setUsage(null);
    }, [profileId, chatId]);

    // Подписка на Tauri event
    useEffect(() => {
        const unlisten = listen<ContextUsagePayload>('context-usage', (event) => {
            if (!isLoading) return;
            setUsage(event.payload);
        });
        return () => {
            unlisten.then(fn => fn());
        };
    }, [isLoading]);

    const display = resolveContextUsageDisplay(usage, configuredContextWindow);

    if (!display) return null;

    const { estimatedTokens, contextWindow, percent, warningLevel } = display;

    const trackColor =
        warningLevel === 'critical' ? 'bg-red-500/20' :
        warningLevel === 'warning'  ? 'bg-yellow-500/20' :
                                       'bg-zinc-700/40';

    const fillColor =
        warningLevel === 'critical' ? 'bg-red-500' :
        warningLevel === 'warning'  ? 'bg-yellow-500' :
                                       'bg-zinc-500';

    const textColor =
        warningLevel === 'critical' ? 'text-red-400' :
        warningLevel === 'warning'  ? 'text-yellow-400' :
                                       'text-zinc-500';

    return (
        <div className="max-w-4xl mx-auto px-1 mb-2">
            <div className="flex items-center gap-2">
                {/* Прогресс-бар */}
                <div className={`flex-1 h-1 rounded-full ${trackColor} overflow-hidden`}>
                    <div
                        className={`h-full rounded-full transition-all duration-500 ${fillColor}`}
                        style={{ width: `${Math.min(percent, 100)}%` }}
                    />
                </div>

                {/* Текст */}
                <span className={`text-[10px] font-mono flex-shrink-0 ${textColor}`}>
                    {formatTokens(estimatedTokens)} / {formatWindow(contextWindow)}
                </span>

                {/* Иконка + кнопка при критическом уровне */}
                {warningLevel === 'critical' && (
                    <div className="flex items-center gap-1 flex-shrink-0">
                        <AlertTriangle className="w-3 h-3 text-red-400" />
                        {onNewChat && (
                            <button
                                onClick={onNewChat}
                                className="flex items-center gap-0.5 text-[10px] text-red-400 hover:text-red-300 transition-colors"
                                title="Начать новый чат"
                            >
                                <Plus className="w-3 h-3" />
                                Новый чат
                            </button>
                        )}
                    </div>
                )}
            </div>
        </div>
    );
}
