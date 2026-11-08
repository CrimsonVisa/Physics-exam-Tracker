import { useEffect, useState } from 'react';
import { loadKatex, type KatexRenderer } from '../katex';

type FormulaEquationProps = {
  latex: string;
  plainText: string;
  display?: boolean;
  className?: string;
};

function classNames(display: boolean, className?: string): string {
  return [
    'v2-formula-equation',
    display ? 'v2-formula-equation-display' : 'v2-formula-equation-inline',
    className,
  ]
    .filter(Boolean)
    .join(' ');
}

function renderToHtml(katex: KatexRenderer, latex: string, display: boolean): string | null {
  try {
    return katex.renderToString(latex, {
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

export function FormulaEquation({
  latex,
  plainText,
  display = false,
  className,
}: FormulaEquationProps) {
  const [katex, setKatex] = useState<KatexRenderer | null>(null);
  const [loadFailed, setLoadFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void loadKatex()
      .then((loaded) => {
        if (!cancelled) {
          setKatex(() => loaded);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setLoadFailed(true);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!katex) {
    return (
      <span
        className={`${classNames(display, className)} v2-formula-equation-fallback`}
        title={loadFailed ? 'Formula rendering unavailable' : 'Rendering formula'}
      >
        {plainText}
      </span>
    );
  }

  const html = renderToHtml(katex, latex, display);

  if (html === null) {
    return (
      <span
        className={`${classNames(display, className)} v2-formula-equation-fallback`}
        title="Formula rendering unavailable"
      >
        {plainText}
      </span>
    );
  }

  return (
    <span className={classNames(display, className)} dangerouslySetInnerHTML={{ __html: html }} />
  );
}
