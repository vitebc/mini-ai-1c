import test from 'node:test';
import assert from 'node:assert/strict';

import { MINIMAX_PROVIDER_DEFINITION } from '../providerCatalog';

test('MiniMax uses the current official M3 model and OpenAI-compatible endpoint', () => {
    assert.equal(MINIMAX_PROVIDER_DEFINITION.value, 'MiniMax');
    assert.equal(MINIMAX_PROVIDER_DEFINITION.defaultModel, 'MiniMax-M3');
    assert.equal(MINIMAX_PROVIDER_DEFINITION.defaultUrl, 'https://api.minimax.io/v1');
});
