import { useMemo } from 'react';
import { MathExpression } from './math/MathExpression';
import { parseMathText } from './math/render';

type MathTextProps = {
  text: string;
  className?: string;
};

/**
 * Render plain text with explicit `$...$`, `$$...$$`, `\(...\)` and `\[...\]`
 * mathematics. Ordinary prose stays untouched; only explicitly delimited
 * segments are parsed as LaTeX.
 */
export function MathText({ text, className }: MathTextProps) {
  const nodes = useMemo(
    () =>
      parseMathText(text).map((segment, index) =>
        segment.kind === 'math' ? (
          <MathExpression
            ariaLabel={segment.source}
            className={segment.display ? 'v2-math-display' : 'v2-math-inline'}
            display={segment.display}
            fallbackTitle="LaTeX rendering unavailable"
            key={`math-${index}`}
            latex={segment.value}
            plainText={segment.source}
          />
        ) : (
          <span key={`text-${index}`}>{segment.value}</span>
        ),
      ),
    [text],
  );

  return <span className={['v2-math-text', className].filter(Boolean).join(' ')}>{nodes}</span>;
}
