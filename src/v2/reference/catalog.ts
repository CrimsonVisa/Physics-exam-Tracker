import type { V2CustomUnitConversionKind, V2CustomUnitSiClassification } from '../api';

export const CONSTANT_CATEGORIES = [
  'Fundamental',
  'Electromagnetic',
  'Atomic and nuclear',
  'Thermodynamic',
  'Mathematical',
  'Astrophysical',
  'User-defined',
] as const;

export const SI_CLASSIFICATION_LABELS: Record<V2CustomUnitSiClassification, string> = {
  si_base: 'SI base unit',
  si_derived: 'SI derived unit',
  non_si: 'Non-SI unit',
};

export const CONVERSION_KIND_LABELS: Record<V2CustomUnitConversionKind, string> = {
  none: 'No conversion',
  scalar: 'Scalar conversion',
  contextual: 'Contextual conversion',
};

export const CONVERSION_KIND_HINTS: Record<V2CustomUnitConversionKind, string> = {
  none: 'Use when this entry has no defined conversion to another unit.',
  scalar: 'Use only when multiplying by one finite factor is sufficient.',
  contextual: 'Use when conversion depends on a context or convention.',
};

export function conversionSummary(unit: {
  symbol: string;
  conversionKind: V2CustomUnitConversionKind;
  conversionFactor: string;
  conversionReferenceUnit: string;
  conversionNote: string;
}): string {
  switch (unit.conversionKind) {
    case 'scalar':
      return unit.conversionFactor && unit.conversionReferenceUnit
        ? `1 ${unit.symbol || 'unit'} = ${unit.conversionFactor} ${unit.conversionReferenceUnit}`
        : 'Scalar conversion data is incomplete.';
    case 'contextual':
      return unit.conversionNote || 'Contextual conversion';
    default:
      return 'No conversion defined';
  }
}
