import test from 'node:test';
import assert from 'node:assert/strict';
import { rustMcpBinaryName } from '../mcpNodePath';

test('rustMcpBinaryName appends .exe on Windows', () => {
    assert.equal(rustMcpBinaryName('mcp-1c-skills'), 'mcp-1c-skills');
    assert.equal(rustMcpBinaryName('mcp-1c-search'), 'mcp-1c-search');
});
