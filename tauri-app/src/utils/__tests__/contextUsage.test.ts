import test from 'node:test';
import assert from 'node:assert/strict';

import { resolveContextUsageDisplay } from '../contextUsage';

test('uses configured profile max tokens as display window', () => {
    const display = resolveContextUsageDisplay({
        estimated_tokens: 1000,
        context_window: 128000,
        percent: 0.8,
        warning_level: 'ok',
    }, 8000);

    assert.equal(display?.estimatedTokens, 1000);
    assert.equal(display?.contextWindow, 8000);
    assert.equal(display?.percent, 12.5);
    assert.equal(display?.warningLevel, 'ok');
});

test('recalculates warning level from configured window', () => {
    const display = resolveContextUsageDisplay({
        estimated_tokens: 6000,
        context_window: 128000,
        percent: 4.7,
        warning_level: 'ok',
    }, 8000);

    assert.equal(display?.percent, 75);
    assert.equal(display?.warningLevel, 'warning');
});

test('recalculates critical level from configured window', () => {
    const display = resolveContextUsageDisplay({
        estimated_tokens: 7000,
        context_window: 128000,
        percent: 5.5,
        warning_level: 'ok',
    }, 8000);

    assert.equal(display?.percent, 87.5);
    assert.equal(display?.warningLevel, 'critical');
});

test('falls back to payload context window when configured window is absent', () => {
    const display = resolveContextUsageDisplay({
        estimated_tokens: 3200,
        context_window: 16000,
        percent: 20,
        warning_level: 'ok',
    });

    assert.equal(display?.contextWindow, 16000);
    assert.equal(display?.percent, 20);
});

test('returns null when no valid window is available', () => {
    const display = resolveContextUsageDisplay({
        estimated_tokens: 3200,
        context_window: 0,
        percent: 0,
        warning_level: 'ok',
    }, 0);

    assert.equal(display, null);
});
