const CHANNEL_MARKER_RE = /<\|?\s*channel\s*\|?>\s*(thought|thinking|analysis|reasoning|final|commentary)?/gi;
const FINAL_CHANNEL_MARKER_RE = /<\|?\s*channel\s*\|?>\s*final\b/gi;
const SERVICE_TOKEN_RE = /<\|(?:start|end|eot|endoftext|start_header_id|end_header_id|reserved_special_token_\d+)\|>/gi;

const INLINE_LATEX_SYMBOLS: Record<string, string> = {
    '\\rightarrow': '→',
    '\\to': '→',
    '\\leftarrow': '←',
    '\\Rightarrow': '⇒',
    '\\Leftarrow': '⇐',
    '\\leftrightarrow': '↔',
    '\\ge': '≥',
    '\\geq': '≥',
    '\\le': '≤',
    '\\leq': '≤',
    '\\neq': '≠',
    '\\ne': '≠',
};

function splitMarkdownFencedBlocks(content: string): Array<{ text: string; isCode: boolean }> {
    const lines = content.match(/[^\r\n]*(?:\r\n|\n|\r|$)/g) ?? [];
    if (lines[lines.length - 1] === '') {
        lines.pop();
    }

    const segments: Array<{ text: string; isCode: boolean }> = [];
    let buffer = '';
    let isCode = false;
    let fenceChar = '';
    let fenceLength = 0;

    const flush = () => {
        if (!buffer) return;
        segments.push({ text: buffer, isCode });
        buffer = '';
    };

    for (const line of lines) {
        const opening = /^[ \t]*(`{3,}|~{3,})/.exec(line);
        if (!isCode && opening) {
            flush();
            isCode = true;
            fenceChar = opening[1][0];
            fenceLength = opening[1].length;
            buffer += line;
            continue;
        }

        if (isCode) {
            buffer += line;
            const closing = new RegExp(`^[ \\t]*\\${fenceChar}{${fenceLength},}[ \\t]*(?:\\r?\\n|\\r)?$`).exec(line);
            if (closing) {
                flush();
                isCode = false;
                fenceChar = '';
                fenceLength = 0;
            }
            continue;
        }

        buffer += line;
    }

    flush();
    return segments;
}

function transformOutsideInlineCode(text: string, transform: (value: string) => string): string {
    let result = '';
    let cursor = 0;

    while (cursor < text.length) {
        const open = text.indexOf('`', cursor);
        if (open === -1) {
            result += transform(text.slice(cursor));
            break;
        }

        let markerEnd = open + 1;
        while (markerEnd < text.length && text[markerEnd] === '`') {
            markerEnd += 1;
        }
        const marker = text.slice(open, markerEnd);
        const close = text.indexOf(marker, markerEnd);

        result += transform(text.slice(cursor, open));
        if (close === -1) {
            result += text.slice(open);
            break;
        }

        result += text.slice(open, close + marker.length);
        cursor = close + marker.length;
    }

    return result;
}

function transformOutsideMarkdownCode(content: string, transform: (value: string) => string): string {
    return splitMarkdownFencedBlocks(content)
        .map(segment => segment.isCode
            ? segment.text
            : transformOutsideInlineCode(segment.text, transform))
        .join('');
}

function stripServiceChannelMarkup(text: string): string {
    FINAL_CHANNEL_MARKER_RE.lastIndex = 0;
    const finalMatch = FINAL_CHANNEL_MARKER_RE.exec(text);
    const visibleText = finalMatch ? text.slice(finalMatch.index + finalMatch[0].length) : text;

    return visibleText
        .replace(CHANNEL_MARKER_RE, '')
        .replace(SERVICE_TOKEN_RE, '');
}

function normalizeInlineLatexSymbols(text: string): string {
    return text.replace(/\$\s*(\\[A-Za-z]+)\s*\$/g, (match, command: string) => {
        return INLINE_LATEX_SYMBOLS[command] ?? match;
    });
}

export function sanitizeModelMarkdown(content: string): string {
    if (!content) return '';

    const cleaned = transformOutsideMarkdownCode(content, value => (
        normalizeInlineLatexSymbols(stripServiceChannelMarkup(value))
    ));

    return cleaned
        .replace(/[ \t]+\n/g, '\n')
        .replace(/\n{3,}/g, '\n\n')
        .trim();
}
