import { renderMathHtml, unwrapMathDelimiters } from './render';
import { useKatex } from './useKatex';

type MathExpressionProps = {
  latex: string;
  plainText?: string;
  display?: boolean;
  className?: string;
  ariaLabel?: string;
  fallbackTitle?: string;
};

function classNames(display: boolean, className?: string): string {
  return [
    'v2-math-expression',
    display ? 'v2-math-expression-display' : 'v2-math-expression-inline',
    className,
  ]
    .filter(Boolean)
    .join(' ');
}

/**
 * Render a complete LaTeX expression through the application's one KaTeX
 * pipeline. `plainText` is the readable fallback used when KaTeX is loading,
 * unavailable, or cannot parse the source.
 */
export function MathExpression({
  latex,
  plainText,
  display = false,
  className,
  ariaLabel,
  fallbackTitle,
}: MathExpressionProps) {
  const { katex, loadFailed } = useKatex();
  const unwrapped = unwrapMathDelimiters(latex);
  const source = unwrapped.latex;
  const effectiveDisplay = unwrapped.display ?? display;
  const classes = classNames(effectiveDisplay, className);
  const readable = plainText?.trim() || latex;
  const unavailableTitle = fallbackTitle ?? 'LaTeX could not be rendered';

  if (!source) {
    return <span aria-hidden="true" className={`${classes} v2-math-expression-empty`} />;
  }

  if (!katex) {
    return (
      <span
        className={`${classes} v2-math-fallback`}
        data-latex-source={latex}
        title={loadFailed ? unavailableTitle : 'Rendering LaTeX'}
      >
        {readable}
      </span>
    );
  }

  const html = renderMathHtml(katex, source, effectiveDisplay);

  if (html === null) {
    return (
      <span
        className={`${classes} v2-math-fallback`}
        data-latex-source={latex}
        title={unavailableTitle}
      >
        {readable}
      </span>
    );
  }

  return (
    <span
      aria-label={ariaLabel ?? readable}
      className={classes}
      data-latex-source={latex}
      role="math"
    >
      <span aria-hidden="true" dangerouslySetInnerHTML={{ __html: html }} />
    </span>
  );
}
