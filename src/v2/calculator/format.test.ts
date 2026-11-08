import { describe, expect, it } from 'vitest';
import { formatNumber, formatResultForCopy } from './format';

describe('calculator formatting', () => {
  it('formats exact integers without an approximation marker', () => {
    expect(formatNumber(5)).toEqual({ text: '5', approximate: false, value: 5 });
    expect(formatNumber(-12)).toEqual({ text: '-12', approximate: false, value: -12 });
    expect(formatNumber(0)).toEqual({ text: '0', approximate: false, value: 0 });
    expect(formatNumber(-0).text).toBe('0');
  });

  it('marks floating-point display rounding', () => {
    const sum = formatNumber(0.1 + 0.2);
    expect(sum.text).toBe('0.3');
    expect(sum.approximate).toBe(true);

    const third = formatNumber(1 / 3);
    expect(third.text).toBe('0.333333333333');
    expect(third.approximate).toBe(true);
  });

  it('formats large and small values in scientific notation', () => {
    expect(formatNumber(1e12).text).toBe('1E12');
    expect(formatNumber(1.234567890123e15).text).toBe('1.23456789012E15');
    expect(formatNumber(1e-10).text).toBe('1E-10');
    expect(formatNumber(-2.5e-12).text).toBe('-2.5E-12');
  });

  it('keeps values at the scientific-notation boundaries readable', () => {
    expect(formatNumber(999999999999).text).toBe('999999999999');
    expect(formatNumber(1e-9).text).toBe('0.000000001');
  });

  it('trims insignificant trailing zeros', () => {
    expect(formatNumber(1000).text).toBe('1000');
    expect(formatNumber(1.5).text).toBe('1.5');
    expect(formatNumber(2.5e6).text).toBe('2500000');
  });

  it('exposes the copy formatter', () => {
    expect(formatResultForCopy(0.1 + 0.2)).toBe('0.3');
    expect(formatResultForCopy(2 + 3)).toBe('5');
  });

  it('handles non-finite values defensively', () => {
    expect(formatNumber(Number.NaN).text).toBe('Error');
    expect(formatNumber(Number.POSITIVE_INFINITY).text).toBe('Infinity');
    expect(formatNumber(Number.NEGATIVE_INFINITY).text).toBe('-Infinity');
  });
});
