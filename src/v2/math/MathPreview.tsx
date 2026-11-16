import { MathText } from '../MathText';
import { MathExpression } from './MathExpression';
import { containsExplicitMath } from './render';

type MathPreviewProps = {
  source: string;
  /** Render the whole source as one expression, or only its delimited parts. */
  mode?: 'expression' | 'text';
  display?: boolean;
  label?: string;
  emptyLabel?: string;
  compact?: boolean;
};

/**
 * Live rendered preview for mathematical input. In text mode, previews are
 * shown only when the source contains explicit math delimiters, so ordinary
 * prose fields remain uncluttered.
 */
export function MathPreview({
  source,
  mode = 'expression',
  display = false,
  label = 'Rendered preview',
  emptyLabel,
  compact = false,
}: MathPreviewProps) {
  const empty = source.trim().length === 0;
  if (mode === 'text' && !containsExplicitMath(source)) {
    return null;
  }
  if (empty && !emptyLabel) {
    return null;
  }

  return (
    <span
      className={['v2-math-preview', compact ? 'v2-math-preview-compact' : '']
        .filter(Boolean)
        .join(' ')}
      data-math-preview="true"
    >
      <span className="v2-math-preview-label">{label}</span>
      {empty ? (
        <span className="v2-math-preview-empty">{emptyLabel}</span>
      ) : mode === 'text' ? (
        <MathText text={source} />
      ) : (
        <MathExpression display={display} latex={source} />
      )}
    </span>
  );
}
