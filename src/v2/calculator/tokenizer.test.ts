import { describe, expect, it } from 'vitest';
import { CalcError } from './errors';
import { tokenize } from './tokenizer';

function code(input: string): string | null {
  try {
    tokenize(input);
    return null;
  } catch (error) {
    return error instanceof CalcError ? error.code : null;
  }
}

describe('calculator tokenizer', () => {
  it('tokenizes numbers, identifiers, operators and postfix symbols', () => {
    const tokens = tokenize('sin(30) + 2.5E-3 * 4! % mod');
    expect(tokens.map((token) => token.type)).toEqual([
      'identifier',
      'lparen',
      'number',
      'rparen',
      'operator',
      'number',
      'operator',
      'number',
      'factorial',
      'percent',
      'operator',
    ]);
    expect(tokens[0]).toMatchObject({ type: 'identifier', value: 'sin' });
    expect(tokens[2]).toMatchObject({ type: 'number', value: 30 });
    expect(tokens[5]).toMatchObject({ type: 'number', value: 0.0025 });
    expect(tokens[10]).toMatchObject({ type: 'operator', value: 'mod' });
  });

  it('supports unicode operators and pi', () => {
    const tokens = tokenize('2 × 3 ÷ 4 − π');
    expect(tokens[1]).toMatchObject({ type: 'operator', value: '*' });
    expect(tokens[3]).toMatchObject({ type: 'operator', value: '/' });
    expect(tokens[5]).toMatchObject({ type: 'operator', value: '-' });
    expect(tokens[6]).toMatchObject({ type: 'identifier', value: 'pi' });
  });

  it('supports compact modulus and decimal forms', () => {
    const tokens = tokenize('5mod2 + .5');
    expect(tokens[1]).toMatchObject({ type: 'operator', value: 'mod' });
    expect(tokens[4]).toMatchObject({ type: 'number', value: 0.5 });
  });

  it('rejects unsupported characters', () => {
    expect(code('2 @ 3')).toBe('syntax');
  });

  it('rejects malformed scientific notation', () => {
    expect(code('1e')).toBe('invalid_number');
    expect(code('1e+')).toBe('invalid_number');
  });

  it('rejects over-long expressions before tokenizing', () => {
    expect(code('1'.repeat(513))).toBe('too_long');
  });

  it('rejects numeric overflow and underflow literals', () => {
    expect(code('1e400')).toBe('overflow');
    expect(code('1e-400')).toBe('underflow');
  });
});
