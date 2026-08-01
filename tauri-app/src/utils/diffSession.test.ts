import { strict as assert } from 'node:assert';
import {
    finalizeDiffSessionIfLastGroup,
    resolveDiffPreviewTransition,
    shouldOpenUnseenDiffPreview,
} from './diffSession';

const events: string[] = [];
const defer = (callback: () => void) => callback();

const finalized = finalizeDiffSessionIfLastGroup({
    remainingGroupCount: 1,
    finalCode: 'Procedure Fixed()\nEndProcedure',
    onCodeChange: code => events.push(`code:${code}`),
    onDiffChange: diff => events.push(`diff:${diff}`),
    defer,
});

assert.equal(finalized, true, 'Последняя группа должна завершать diff-сессию');
assert.deepEqual(events, [
    'code:Procedure Fixed()\nEndProcedure',
    'diff:',
], 'Сначала фиксируется код, затем очищается активный diff');

events.length = 0;
const finalizedEarly = finalizeDiffSessionIfLastGroup({
    remainingGroupCount: 2,
    finalCode: 'Procedure Partial()\nEndProcedure',
    onCodeChange: code => events.push(`code:${code}`),
    onDiffChange: diff => events.push(`diff:${diff}`),
    defer,
});

assert.equal(finalizedEarly, false, 'Промежуточная группа не должна завершать diff-сессию');
assert.deepEqual(events, [], 'Промежуточная группа не должна фиксировать или очищать состояние');

assert.equal(shouldOpenUnseenDiffPreview({
    hasBaseCode: true,
    isLargeDiff: false,
    alreadyShown: true,
}), false, 'Уже показанный diff нельзя повторно открывать после применения');

assert.equal(shouldOpenUnseenDiffPreview({
    hasBaseCode: true,
    isLargeDiff: false,
    alreadyShown: false,
}), true, 'Новый применимый diff должен открываться в preview');

assert.deepEqual(resolveDiffPreviewTransition({
    source: { messageKey: 'message-1', content: 'diff-from-chat' },
    previousContent: 'diff-from-chat',
    currentContent: '',
}), {
    nextSource: null,
    dismissedMessageKey: 'message-1',
}, 'Закрытие chat-preview должно скрыть действие только у исходного сообщения');

assert.deepEqual(resolveDiffPreviewTransition({
    source: { messageKey: 'message-1', content: 'diff-from-chat' },
    previousContent: 'diff-from-chat',
    currentContent: 'diff-from-overlay',
}), {
    nextSource: null,
    dismissedMessageKey: null,
}, 'Overlay diff не должен менять состояние chat-сообщения');

console.log('✅ PASS  Жизненный цикл применения diff');
