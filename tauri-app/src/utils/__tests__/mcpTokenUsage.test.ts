import test from 'node:test';
import assert from 'node:assert/strict';

import { summarizeMcpTokenUsage } from '../mcpTokenUsage';

test('summarizeMcpTokenUsage groups enabled tools by server and totals token estimates', () => {
    const summary = summarizeMcpTokenUsage([
        {
            server_name: '1С:Поиск',
            tool_name: 'find_symbol',
            description: 'Найти символ',
            input_schema: null,
            is_enabled: true,
            estimated_tokens: 120,
        },
        {
            server_name: '1С:Поиск',
            tool_name: 'search_code',
            description: 'Поиск кода',
            input_schema: null,
            is_enabled: true,
            estimated_tokens: 80,
        },
        {
            server_name: '1С:Напарник',
            tool_name: 'ask_1c_ai',
            description: 'Вопрос ИТС',
            input_schema: null,
            is_enabled: true,
            estimated_tokens: 40,
        },
        {
            server_name: 'Отключен',
            tool_name: 'disabled_tool',
            description: null,
            input_schema: null,
            is_enabled: false,
            estimated_tokens: 900,
        },
        {
            server_name: 'Недоступен',
            tool_name: '__server_unavailable__',
            description: null,
            input_schema: null,
            is_enabled: false,
            estimated_tokens: 1000,
        },
    ]);

    assert.equal(summary.totalEstimatedTokens, 240);
    assert.equal(summary.totalTools, 3);
    assert.deepEqual(
        summary.servers.map(server => ({
            serverName: server.serverName,
            toolCount: server.toolCount,
            estimatedTokens: server.estimatedTokens,
        })),
        [
            { serverName: '1С:Поиск', toolCount: 2, estimatedTokens: 200 },
            { serverName: '1С:Напарник', toolCount: 1, estimatedTokens: 40 },
        ],
    );
});
