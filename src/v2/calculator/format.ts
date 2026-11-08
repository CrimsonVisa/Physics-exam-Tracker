export type FormattedNumber = {
  text: string;
  approximate: boolean;
  value: number;
};

const DISPLAY_SIGNIFICANT_DIGITS = 12;
const SCIENTIFIC_UPPER_THRESHOLD = 1e12;
const SCIENTIFIC_LOWER_THRESHOLD = 1e-9;

export function formatNumber(value: number): FormattedNumber {
  if (Number.isNaN(value)) {
    return { text: 'Error', approximate: false, value };
  }
  if (!Number.isFinite(value)) {
    return {
      text: value > 0 ? 'Infinity' : '-Infinity',
      approximate: false,
      value,
    };
  }

  const normalized = Object.is(value, -0) ? 0 : value;
  const rounded = Number(normalized.toPrecision(DISPLAY_SIGNIFICANT_DIGITS));
  const approximate = !Object.is(rounded, normalized);

  if (rounded === 0) {
    return { text: '0', approximate: false, value: normalized };
  }

  if (
    Math.abs(rounded) >= SCIENTIFIC_UPPER_THRESHOLD ||
    Math.abs(rounded) < SCIENTIFIC_LOWER_THRESHOLD
  ) {
    const exponential = rounded.toExponential(DISPLAY_SIGNIFICANT_DIGITS - 1);
    const [mantissa, exponent] = exponential.split('e');
    const trimmedMantissa = mantissa.replace(/\.?0+$/, '');
    const trimmedExponent = exponent.replace(/^\+/, '');
    return {
      text: `${trimmedMantissa}E${trimmedExponent}`,
      approximate,
      value: normalized,
    };
  }

  let text = new Intl.NumberFormat('en-US', {
    maximumSignificantDigits: DISPLAY_SIGNIFICANT_DIGITS,
    useGrouping: false,
  }).format(rounded);
  if (text === '-0') {
    text = '0';
  }
  return { text, approximate, value: normalized };
}

export function formatResultForCopy(value: number): string {
  return formatNumber(value).text;
}
