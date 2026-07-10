import test from 'node:test';
import assert from 'node:assert/strict';

import { cleanDiffArtifacts } from '../MarkdownRenderer';

test('cleanDiffArtifacts removes complete Naparnik SEARCH/REPLACE blocks before malformed tails are processed', () => {
    const response = [
        'Applying fixes:',
        '',
        '<<<<<<< SEARCH',
        '    OldCall();',
        '=======',
        '    NewCall();',
        '>>>>>>> REPLACE',
        '',
        '<<<<<<< SEARCH',
        '// Old parameter docs',
        '=======',
        '// New parameter docs',
        '>>>>>>> REPLACE',
        '',
        'Fixed 2 diagnostics.',
    ].join('\n');

    const cleaned = cleanDiffArtifacts(response, '    OldCall();');

    assert.match(cleaned, /Applying fixes/);
    assert.match(cleaned, /Fixed 2 diagnostics/);
    assert.doesNotMatch(cleaned, /<<<<<<< SEARCH/);
    assert.doesNotMatch(cleaned, /^={7}$/m);
    assert.doesNotMatch(cleaned, />>>>>>> REPLACE/);
    assert.doesNotMatch(cleaned, /OldCall/);
    assert.doesNotMatch(cleaned, /NewCall/);
    assert.doesNotMatch(cleaned, /Old parameter docs/);
    assert.doesNotMatch(cleaned, /New parameter docs/);
});

test('cleanDiffArtifacts keeps visible BSL examples when response also contains diff blocks', () => {
    const response = [
        'Recommended fixes:',
        '',
        'Date logic fix:',
        '```bsl',
        'If FileDate <> Object.Date Then',
        '    Return False;',
        'EndIf;',
        '```',
        '',
        '<diff>',
        '<search>',
        'DateMatches = True;',
        '</search>',
        '<replace>',
        'DateMatches = CheckFileDate();',
        '</replace>',
        '</diff>',
    ].join('\n');

    const cleaned = cleanDiffArtifacts(response, 'DateMatches = True;');

    assert.match(cleaned, /```bsl/);
    assert.match(cleaned, /If FileDate <> Object\.Date Then/);
    assert.doesNotMatch(cleaned, /<diff>/);
    assert.doesNotMatch(cleaned, /DateMatches = CheckFileDate/);
});
