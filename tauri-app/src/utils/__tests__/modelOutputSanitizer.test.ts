import test from 'node:test';
import assert from 'node:assert/strict';

import { sanitizeModelMarkdown } from '../modelOutputSanitizer';

test('removes malformed thought channel markers from visible text', () => {
    const input = '<|channel>thought <channel|>\nОтвет пользователю';

    assert.equal(sanitizeModelMarkdown(input), 'Ответ пользователю');
});

test('keeps final answer when thought and final channels are present', () => {
    const input = [
        '<|channel|>thought',
        'Не показывать этот внутренний текст.',
        '<|channel|>final',
        'Показывать только это.',
    ].join('\n');

    assert.equal(sanitizeModelMarkdown(input), 'Показывать только это.');
});

test('does not alter service-looking text inside fenced code blocks', () => {
    const input = [
        'До',
        '```bsl',
        'Сообщить("<|channel|>thought");',
        '```',
        'После',
    ].join('\n');

    assert.equal(sanitizeModelMarkdown(input), input);
});

test('normalizes common inline latex arrows in regular text', () => {
    assert.equal(
        sanitizeModelMarkdown('Переход A $\\rightarrow$ B и B $\\Leftarrow$ A'),
        'Переход A → B и B ⇐ A',
    );
});

test('does not normalize latex-looking text inside inline code', () => {
    assert.equal(
        sanitizeModelMarkdown('Команда `$\\rightarrow$` остается как есть'),
        'Команда `$\\rightarrow$` остается как есть',
    );
});
