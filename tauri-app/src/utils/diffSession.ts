interface FinalizeDiffSessionOptions {
    remainingGroupCount: number;
    finalCode: string;
    onCodeChange: (code: string) => void;
    onDiffChange?: (content: string) => void;
    defer: (callback: () => void) => void;
}

interface ShouldOpenDiffPreviewOptions {
    hasBaseCode: boolean;
    isLargeDiff: boolean;
    alreadyShown: boolean;
}

export interface DiffPreviewSource {
    messageKey: string;
    content: string;
}

interface ResolveDiffPreviewTransitionOptions {
    source: DiffPreviewSource | null;
    previousContent: string;
    currentContent: string;
}

interface DiffPreviewTransition {
    nextSource: DiffPreviewSource | null;
    dismissedMessageKey: string | null;
}

/**
 * Явно завершает diff-сессию после обработки последней визуальной группы.
 *
 * Monaco пересчитывает diff асинхронно, поэтому событие с пустым списком
 * изменений не является надёжной точкой завершения пользовательского действия.
 */
export function finalizeDiffSessionIfLastGroup({
    remainingGroupCount,
    finalCode,
    onCodeChange,
    onDiffChange,
    defer,
}: FinalizeDiffSessionOptions): boolean {
    if (remainingGroupCount !== 1) {
        return false;
    }

    onCodeChange(finalCode);
    if (onDiffChange) {
        defer(() => onDiffChange(''));
    }
    return true;
}

/**
 * Последнее сообщение может повторно вызвать эффект ChatArea во время фиксации
 * кода. Уже показанный preview не открываем снова после его обработки.
 */
export function shouldOpenUnseenDiffPreview({
    hasBaseCode,
    isLargeDiff,
    alreadyShown,
}: ShouldOpenDiffPreviewOptions): boolean {
    return hasBaseCode && !isLargeDiff && !alreadyShown;
}

/**
 * Связывает закрытие preview только с тем chat-сообщением, которое его открыло.
 * Если активный diff заменён внешним источником (например overlay), chat-связь
 * сбрасывается и чужое сообщение не помечается обработанным.
 */
export function resolveDiffPreviewTransition({
    source,
    previousContent,
    currentContent,
}: ResolveDiffPreviewTransitionOptions): DiffPreviewTransition {
    if (source && currentContent && currentContent !== source.content) {
        return {
            nextSource: null,
            dismissedMessageKey: null,
        };
    }

    if (!previousContent || currentContent) {
        return {
            nextSource: source,
            dismissedMessageKey: null,
        };
    }

    return {
        nextSource: null,
        dismissedMessageKey: source?.messageKey ?? null,
    };
}
