export type ContextWarningLevel = 'ok' | 'warning' | 'critical';

export interface ContextUsagePayload {
    estimated_tokens: number;
    context_window: number;
    percent: number;
    warning_level: ContextWarningLevel;
}

export interface ContextUsageDisplay {
    estimatedTokens: number;
    contextWindow: number;
    percent: number;
    warningLevel: ContextWarningLevel;
}

function positiveWindow(value: number | null | undefined): number | null {
    if (!Number.isFinite(value) || !value || value <= 0) {
        return null;
    }
    return Math.round(value);
}

function warningLevel(percent: number): ContextWarningLevel {
    if (percent >= 85) return 'critical';
    if (percent >= 70) return 'warning';
    return 'ok';
}

export function resolveContextUsageDisplay(
    usage: ContextUsagePayload | null,
    configuredContextWindow?: number | null,
): ContextUsageDisplay | null {
    if (!usage) return null;

    const contextWindow = positiveWindow(configuredContextWindow) ?? positiveWindow(usage.context_window);
    if (!contextWindow) return null;

    const estimatedTokens = Math.max(0, Math.round(Number.isFinite(usage.estimated_tokens) ? usage.estimated_tokens : 0));
    const percent = Math.min(100, (estimatedTokens / contextWindow) * 100);

    return {
        estimatedTokens,
        contextWindow,
        percent,
        warningLevel: warningLevel(percent),
    };
}
