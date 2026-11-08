import { CalcError } from './errors';
import type { BinaryOperator } from './types';

export const MAX_EXPRESSION_LENGTH = 512;
export const MAX_TOKEN_COUNT = 512;

export type Token =
  | { type: 'number'; value: number; raw: string; start: number; end: number }
  | { type: 'identifier'; value: string; start: number; end: number }
  | { type: 'operator'; value: BinaryOperator; start: number; end: number }
  | { type: 'lparen'; start: number; end: number }
  | { type: 'rparen'; start: number; end: number }
  | { type: 'comma'; start: number; end: number }
  | { type: 'factorial'; start: number; end: number }
  | { type: 'percent'; start: number; end: number };

const NUMBER_PATTERN = /^(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?/;
const IDENTIFIER_PATTERN = /^[A-Za-z_π][A-Za-z0-9_]*/;
const OPERATOR_CHARACTERS: Record<string, BinaryOperator> = {
  '+': '+',
  '-': '-',
  '−': '-',
  '*': '*',
  '×': '*',
  '/': '/',
  '÷': '/',
  '^': '^',
};

export function tokenize(input: string): Token[] {
  if (input.length > MAX_EXPRESSION_LENGTH) {
    throw new CalcError('too_long');
  }

  const tokens: Token[] = [];
  let index = 0;

  function push(token: Token) {
    tokens.push(token);
    if (tokens.length > MAX_TOKEN_COUNT) {
      throw new CalcError('too_many_tokens', token.start);
    }
  }

  while (index < input.length) {
    const character = input[index];

    if (/\s/.test(character)) {
      index += 1;
      continue;
    }

    const numberMatch = NUMBER_PATTERN.exec(input.slice(index));
    if (numberMatch) {
      const raw = numberMatch[0];
      const start = index;
      const end = index + raw.length;
      const value = Number(raw);
      const nextCharacter = input[end];
      if ((nextCharacter === 'e' || nextCharacter === 'E') && !/[eE]/.test(raw)) {
        throw new CalcError('invalid_number', start);
      }
      if (!Number.isFinite(value)) {
        throw new CalcError(
          value === Infinity || value === -Infinity ? 'overflow' : 'invalid_number',
          start,
        );
      }
      if (value === 0 && /[1-9]/.test(raw)) {
        throw new CalcError('underflow', start);
      }
      push({ type: 'number', value, raw, start, end });
      index = end;
      continue;
    }

    const identifierMatch = IDENTIFIER_PATTERN.exec(input.slice(index));
    if (identifierMatch) {
      const raw = identifierMatch[0];
      const start = index;
      const end = index + raw.length;
      const normalized = raw === 'π' ? 'pi' : raw.toLowerCase();
      if (normalized === 'mod') {
        push({ type: 'operator', value: 'mod', start, end });
      } else if (/^mod\d+$/.test(normalized)) {
        push({ type: 'operator', value: 'mod', start, end: start + 3 });
        index = start + 3;
        continue;
      } else {
        push({ type: 'identifier', value: normalized, start, end });
      }
      index = end;
      continue;
    }

    const operator = OPERATOR_CHARACTERS[character];
    if (operator) {
      push({ type: 'operator', value: operator, start: index, end: index + 1 });
      index += 1;
      continue;
    }

    if (character === '(') {
      push({ type: 'lparen', start: index, end: index + 1 });
      index += 1;
      continue;
    }
    if (character === ')') {
      push({ type: 'rparen', start: index, end: index + 1 });
      index += 1;
      continue;
    }
    if (character === ',') {
      push({ type: 'comma', start: index, end: index + 1 });
      index += 1;
      continue;
    }
    if (character === '!') {
      push({ type: 'factorial', start: index, end: index + 1 });
      index += 1;
      continue;
    }
    if (character === '%') {
      push({ type: 'percent', start: index, end: index + 1 });
      index += 1;
      continue;
    }

    throw new CalcError('syntax', index);
  }

  return tokens;
}
