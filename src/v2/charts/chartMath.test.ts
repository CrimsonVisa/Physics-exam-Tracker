import { describe, expect, it } from 'vitest';
import { formatChartDuration } from './chartMath';

describe('formatChartDuration', () => {
  it.each([
    [0, '0 s'],
    [1, '1 s'],
    [23, '23 s'],
    [59, '59 s'],
    [60, '1 min'],
    [61, '1 min 1 s'],
    [3_600, '1 h'],
    [3_661, '1 h 1 min 1 s'],
    [7_325, '2 h 2 min 5 s'],
  ])('formats %i seconds as %s', (seconds, expected) => {
    expect(formatChartDuration(seconds)).toBe(expected);
  });

  it('never rounds a positive sub-minute duration to zero', () => {
    for (const seconds of [1, 5, 23, 59]) {
      expect(formatChartDuration(seconds)).not.toBe('0 s');
      expect(formatChartDuration(seconds)).toContain(`${seconds} s`);
    }
  });
});
