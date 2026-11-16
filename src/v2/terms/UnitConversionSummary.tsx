import type { V2CustomUnit } from '../api';
import { MathText } from '../MathText';
import { MathExpression } from '../math/MathExpression';

type UnitConversionSummaryProps = {
  unit: V2CustomUnit;
  className?: string;
};

/** Shared mathematical rendering for unit conversion reference text. */
export function UnitConversionSummary({ unit, className }: UnitConversionSummaryProps) {
  if (unit.conversionKind === 'scalar') {
    if (!unit.conversionFactor || !unit.conversionReferenceUnit) {
      return <span className={className}>Scalar conversion data is incomplete.</span>;
    }
    return (
      <span
        aria-label={`1 ${unit.symbol || 'unit'} = ${unit.conversionFactor} ${unit.conversionReferenceUnit}`}
        className={className}
        data-unit-conversion="scalar"
      >
        1 <MathExpression latex={unit.symbol || 'unit'} plainText={unit.symbol || 'unit'} /> ={' '}
        {unit.conversionFactor}{' '}
        <MathExpression
          latex={unit.conversionReferenceUnit}
          plainText={unit.conversionReferenceUnit}
        />
      </span>
    );
  }

  if (unit.conversionKind === 'contextual') {
    return <MathText className={className} text={unit.conversionNote || 'Contextual conversion'} />;
  }

  return <span className={className}>No conversion defined</span>;
}
