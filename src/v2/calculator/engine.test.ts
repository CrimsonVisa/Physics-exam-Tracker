import { describe, expect, it } from 'vitest';
import { applyRepeatedEquals, evaluate } from './engine';
import { CalcError } from './errors';

function value(expression: string, options?: Parameters<typeof evaluate>[1]): number {
  return evaluate(expression, options).value;
}

function errorCode(expression: string): string | null {
  try {
    evaluate(expression);
    return null;
  } catch (error) {
    return error instanceof CalcError ? error.code : null;
  }
}

describe('calculator engine', () => {
  it('evaluates arithmetic with standard precedence', () => {
    expect(value('2 + 3 * 4')).toBe(14);
    expect(value('(2 + 3) * 4')).toBe(20);
    expect(value('10 - 2 - 3')).toBe(5);
    expect(value('24 / 4 / 2')).toBe(3);
    expect(value('2 + 3 * 4 - 5 / 5')).toBe(13);
  });

  it('handles unary signs and right-associative powers', () => {
    expect(value('2 ^ 3 ^ 2')).toBe(512);
    expect(value('-2 ^ 2')).toBe(-4);
    expect(value('(-2) ^ 2')).toBe(4);
    expect(value('2 ^ -2')).toBe(0.25);
    expect(value('-2 ^ -2')).toBe(-0.25);
    expect(value('2 ^ 3!')).toBe(64);
  });

  it('handles postfix factorial and percentage', () => {
    expect(value('5!')).toBe(120);
    expect(value('0!')).toBe(1);
    expect(value('50%')).toBe(0.5);
    expect(value('200 + 10%')).toBe(200.1);
    expect(value('3!!')).toBe(720);
  });

  it('handles reciprocal, absolute value and modulus', () => {
    expect(value('recip(4)')).toBe(0.25);
    expect(value('abs(-7.5)')).toBe(7.5);
    expect(value('5 mod 3')).toBe(2);
    expect(value('-5 mod 3')).toBe(-2);
    expect(value('5 mod -3')).toBe(2);
    expect(value('5mod2')).toBe(1);
  });

  it('handles powers and roots', () => {
    expect(value('2 ^ 10')).toBe(1024);
    expect(value('sqrt(9)')).toBe(3);
    expect(value('cbrt(-8)')).toBe(-2);
    expect(value('root(27, 3)')).toBeCloseTo(3, 12);
    expect(value('root(-8, 3)')).toBeCloseTo(-2, 12);
    expect(value('2 ^ 0')).toBe(1);
    expect(value('0 ^ 0')).toBe(1);
  });

  it('handles trigonometry in degrees and radians', () => {
    expect(value('sin(30)')).toBeCloseTo(0.5, 12);
    expect(value('cos(60)')).toBeCloseTo(0.5, 12);
    expect(value('tan(45)')).toBeCloseTo(1, 12);
    expect(value('asin(0.5)')).toBeCloseTo(30, 12);
    expect(value('acos(0.5)')).toBeCloseTo(60, 12);
    expect(value('atan(1)')).toBeCloseTo(45, 12);
    expect(value('sin(pi / 6)', { angleMode: 'rad' })).toBeCloseTo(0.5, 12);
    expect(value('asin(0.5)', { angleMode: 'rad' })).toBeCloseTo(Math.PI / 6, 12);
    expect(value('sin(180)')).toBeCloseTo(0, 12);
  });

  it('handles hyperbolic and inverse hyperbolic functions', () => {
    expect(value('sinh(0)')).toBe(0);
    expect(value('cosh(0)')).toBe(1);
    expect(value('tanh(0)')).toBe(0);
    expect(value('asinh(0)')).toBe(0);
    expect(value('acosh(1)')).toBe(0);
    expect(value('atanh(0)')).toBe(0);
  });

  it('handles logarithms and exponentials', () => {
    expect(value('ln(e)')).toBeCloseTo(1, 12);
    expect(value('log(1000)')).toBeCloseTo(3, 12);
    expect(value('log10(1000)')).toBeCloseTo(3, 12);
    expect(value('log2(8)')).toBeCloseTo(3, 12);
    expect(value('logb(8, 2)')).toBeCloseTo(3, 12);
    expect(value('log(8, 2)')).toBeCloseTo(3, 12);
    expect(value('exp(1)')).toBeCloseTo(Math.E, 12);
    expect(value('pow10(3)')).toBe(1000);
    expect(value('e ^ 2')).toBeCloseTo(Math.E ** 2, 12);
    expect(value('10 ^ 2')).toBe(100);
  });

  it('handles constants and previous answer', () => {
    expect(value('pi')).toBe(Math.PI);
    expect(value('π')).toBe(Math.PI);
    expect(value('e')).toBe(Math.E);
    expect(value('Ans * 2', { ans: 5 })).toBe(10);
    expect(value('ans + 1', { ans: 2 })).toBe(3);
  });

  it('handles scientific notation literals', () => {
    expect(value('1.2E3')).toBe(1200);
    expect(value('1E-3')).toBe(0.001);
    expect(value('2.5e+6')).toBe(2500000);
  });

  it('returns the last binary operation for repeated equals', () => {
    expect(evaluate('2 + 3').lastBinary).toEqual({ operator: '+', operand: 3 });
    expect(evaluate('2 + 3 * 4').lastBinary).toEqual({ operator: '+', operand: 12 });
    expect(evaluate('2 * 3 + 4').lastBinary).toEqual({ operator: '+', operand: 4 });
    expect(evaluate('2 ^ 3 ^ 2').lastBinary).toEqual({ operator: '^', operand: 9 });
    expect(evaluate('(2 + 3)').lastBinary).toBeNull();
    expect(evaluate('5').lastBinary).toBeNull();
    expect(applyRepeatedEquals(5, { operator: '+', operand: 3 })).toBe(8);
    expect(applyRepeatedEquals(5, null)).toBe(5);
  });

  it('rejects empty, malformed and unbalanced expressions', () => {
    expect(errorCode('')).toBe('syntax');
    expect(errorCode('   ')).toBe('syntax');
    expect(errorCode('2 +')).toBe('syntax');
    expect(errorCode('2 + * 3')).toBe('syntax');
    expect(errorCode('(2 + 3')).toBe('unbalanced_parentheses');
    expect(errorCode('2 + 3)')).toBe('unbalanced_parentheses');
    expect(errorCode('foo(1)')).toBe('unknown_function');
    expect(errorCode('2 @ 3')).toBe('syntax');
    expect(errorCode('1e')).toBe('invalid_number');
  });

  it('rejects division by zero and invalid domains', () => {
    expect(errorCode('1 / 0')).toBe('division_by_zero');
    expect(errorCode('5 mod 0')).toBe('division_by_zero');
    expect(errorCode('0 ^ -1')).toBe('division_by_zero');
    expect(errorCode('recip(0)')).toBe('division_by_zero');
    expect(errorCode('sqrt(-1)')).toBe('domain_error');
    expect(errorCode('log(0)')).toBe('domain_error');
    expect(errorCode('log(-1)')).toBe('domain_error');
    expect(errorCode('ln(0)')).toBe('domain_error');
    expect(errorCode('log2(0)')).toBe('domain_error');
    expect(errorCode('logb(8, 1)')).toBe('domain_error');
    expect(errorCode('asin(2)')).toBe('domain_error');
    expect(errorCode('acos(-2)')).toBe('domain_error');
    expect(errorCode('acosh(0)')).toBe('domain_error');
    expect(errorCode('atanh(1)')).toBe('domain_error');
    expect(errorCode('root(-8, 2)')).toBe('domain_error');
    expect(errorCode('(-8) ^ 0.5')).toBe('domain_error');
  });

  it('rejects invalid factorial inputs and overflow', () => {
    expect(errorCode('(-1)!')).toBe('factorial_domain');
    expect(errorCode('3.5!')).toBe('factorial_domain');
    expect(errorCode('171!')).toBe('overflow');
    expect(errorCode('2 ^ 10000')).toBe('overflow');
    expect(errorCode('exp(1000)')).toBe('overflow');
    expect(errorCode('pow10(400)')).toBe('overflow');
    expect(errorCode('1e400')).toBe('overflow');
    expect(errorCode('1e-400')).toBe('underflow');
  });

  it('rejects tangent singularities', () => {
    expect(errorCode('tan(90)')).toBe('angle_domain');
    expect(errorCode('tan(270)')).toBe('angle_domain');
  });

  it('enforces nesting and expression limits', () => {
    expect(errorCode(`${'('.repeat(65)}1${')'.repeat(65)}`)).toBe('nesting_too_deep');
    expect(errorCode('1+'.repeat(300) + '1')).toBe('too_long');
  });
});
