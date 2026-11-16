import { MathExpression } from '../math/MathExpression';

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

/**
 * Backwards-compatible formula wrapper around the shared mathematical
 * renderer. Kept so every formula surface uses one KaTeX implementation.
 */
export function FormulaEquation({
  latex,
  plainText,
  display = false,
  className,
}: FormulaEquationProps) {
  return (
    <MathExpression
      className={classNames(display, className)}
      display={display}
      fallbackTitle="Formula rendering unavailable"
      latex={latex}
      plainText={plainText}
    />
  );
}
