import type { KatexRenderer } from '../katex';

export type MathSegment =
  | { kind: 'text'; value: string }
  | { kind: 'math'; value: string; display: boolean; source: string };

type Delimiter = {
  open: string;
  close: string;
  display: boolean;
};

function openingDelimiter(text: string, index: number): Delimiter | null {
  if (text.startsWith('$$', index)) {
    return { open: '$$', close: '$$', display: true };
  }
  if (text.startsWith('\\[', index)) {
    return { open: '\\[', close: '\\]', display: true };
  }
  if (text.startsWith('\\(', index)) {
    return { open: '\\(', close: '\\)', display: false };
  }
  if (text[index] === '$') {
    return { open: '$', close: '$', display: false };
  }
  return null;
}

function closingDelimiterIndex(text: string, start: number, close: string): number {
  let index = start;
  while (index < text.length) {
    if (text.startsWith(close, index)) {
      return index;
    }
    if (text[index] === '\\') {
      index += 2;
    } else {
      index += 1;
    }
  }
  return -1;
}

/**
 * Split mixed prose into plain-text and explicitly delimited math segments.
 *
 * Supported delimiters:
 * - `$...$` and `$$...$$`
 * - `\(...\)` and `\[...\]`
 *
 * Ordinary text is never treated as LaTeX. Escaped dollars (`\$`) remain
 * literal. Unclosed delimiters stay in the text instead of swallowing the
 * remainder of the field.
 */
export function parseMathText(text: string): MathSegment[] {
  const segments: MathSegment[] = [];
  let buffer = '';
  let index = 0;

  function flushText() {
    if (buffer) {
      segments.push({ kind: 'text', value: buffer });
      buffer = '';
    }
  }

  while (index < text.length) {
    if (text.startsWith('\\\\', index)) {
      buffer += '\\\\';
      index += 2;
      continue;
    }
    if (text[index] === '\\' && text[index + 1] === '$') {
      buffer += '$';
      index += 2;
      continue;
    }

    const delimiter = openingDelimiter(text, index);
    if (delimiter) {
      const start = index + delimiter.open.length;
      const end = closingDelimiterIndex(text, start, delimiter.close);
      if (end >= 0) {
        flushText();
        segments.push({
          kind: 'math',
          value: text.slice(start, end),
          display: delimiter.display,
          source: text.slice(index, end + delimiter.close.length),
        });
        index = end + delimiter.close.length;
        continue;
      }
    }

    buffer += text[index];
    index += 1;
  }

  flushText();
  return segments;
}

export function containsExplicitMath(text: string): boolean {
  return parseMathText(text).some((segment) => segment.kind === 'math');
}

/**
 * Dedicated math fields accept either raw LaTeX or one complete pair of
 * explicit delimiters. The delimiters are removed only for rendering; the
 * original input remains the stored source.
 */
export function unwrapMathDelimiters(source: string): {
  latex: string;
  display: boolean | null;
} {
  const trimmed = source.trim();
  const pairs: Array<[string, string, boolean]> = [
    ['$$', '$$', true],
    ['\\[', '\\]', true],
    ['\\(', '\\)', false],
    ['$', '$', false],
  ];
  for (const [open, close, display] of pairs) {
    if (
      trimmed.length >= open.length + close.length &&
      trimmed.startsWith(open) &&
      trimmed.endsWith(close)
    ) {
      return {
        latex: trimmed.slice(open.length, trimmed.length - close.length).trim(),
        display,
      };
    }
  }
  return { latex: trimmed, display: null };
}

/**
 * Render one LaTeX expression to KaTeX's HTML + MathML output.
 *
 * User input is never interpreted as HTML. Invalid expressions return `null`
 * so callers can show a readable source/plain-text fallback.
 */
export function renderMathHtml(
  katex: KatexRenderer,
  expression: string,
  display: boolean,
): string | null {
  if (!expression.trim()) {
    return null;
  }
  try {
    return katex.renderToString(expression, {
      displayMode: display,
      throwOnError: true,
      strict: 'warn',
      trust: false,
      output: 'htmlAndMathml',
    });
  } catch {
    return null;
  }
}
