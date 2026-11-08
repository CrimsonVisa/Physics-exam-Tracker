import { useEffect, useMemo, useState, type ReactNode } from 'react';

type MathSegment = {
  math: boolean;
  value: string;
  display: boolean;
};

type KatexRenderer = typeof import('katex').default;

let katexPromise: Promise<KatexRenderer> | null = null;

function loadKatex(): Promise<KatexRenderer> {
  if (!katexPromise) {
    katexPromise = import('katex').then((module) => module.default);
  }
  return katexPromise;
}

function parseMath(text: string): MathSegment[] {
  const segments: MathSegment[] = [];
  let buffer = '';
  let index = 0;

  function flushText() {
    if (buffer) {
      segments.push({ math: false, value: buffer, display: false });
      buffer = '';
    }
  }

  while (index < text.length) {
    const character = text[index];
    if (character === '\\' && text[index + 1] === '$') {
      buffer += '$';
      index += 2;
      continue;
    }
    if (character === '$') {
      const display = text[index + 1] === '$';
      const delimiter = display ? '$$' : '$';
      const start = index + delimiter.length;
      let end = start;
      while (end < text.length) {
        if (text[end] === '\\') {
          end += 2;
          continue;
        }
        if (text.startsWith(delimiter, end)) {
          break;
        }
        end += 1;
      }
      if (end < text.length) {
        flushText();
        segments.push({
          math: true,
          value: text.slice(start, end),
          display,
        });
        index = end + delimiter.length;
        continue;
      }
    }
    buffer += character;
    index += 1;
  }
  flushText();
  return segments;
}

function renderMath(segment: MathSegment, key: string, katex: KatexRenderer | null): ReactNode {
  if (!katex) {
    return (
      <span className="v2-math-fallback" key={key} title="LaTeX is loading">
        {segment.display ? `$$${segment.value}$$` : `$${segment.value}$`}
      </span>
    );
  }
  try {
    const html = katex.renderToString(segment.value, {
      displayMode: segment.display,
      throwOnError: false,
      strict: 'warn',
      trust: false,
      output: 'htmlAndMathml',
    });
    return (
      <span
        className={segment.display ? 'v2-math-display' : 'v2-math-inline'}
        dangerouslySetInnerHTML={{ __html: html }}
        key={key}
      />
    );
  } catch {
    return (
      <span className="v2-math-fallback" key={key} title="LaTeX could not be rendered">
        {segment.display ? `$$${segment.value}$$` : `$${segment.value}$`}
      </span>
    );
  }
}

/**
 * Renders plain text with explicit `$...$` inline and `$$...$$` display math.
 * Only KaTeX-generated markup is inserted into the DOM; user content is never
 * interpreted as raw HTML. KaTeX is loaded as a separate chunk on first use.
 */
export function MathText({ text }: { text: string }) {
  const [katex, setKatex] = useState<KatexRenderer | null>(null);

  useEffect(() => {
    let cancelled = false;
    void loadKatex().then((loaded) => {
      if (!cancelled) {
        setKatex(() => loaded);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const nodes = useMemo(() => {
    const segments = parseMath(text);
    return segments.map((segment, index) =>
      segment.math ? (
        renderMath(segment, `math-${index}`, katex)
      ) : (
        <span key={`text-${index}`}>{segment.value}</span>
      ),
    );
  }, [text, katex]);

  return <span className="v2-math-text">{nodes}</span>;
}
