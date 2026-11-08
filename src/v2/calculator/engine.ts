import { CalcError } from './errors';
import { tokenize, type Token } from './tokenizer';
import type { AngleMode, BinaryOperator, LastBinaryOperation } from './types';

export type EvaluationOptions = {
  angleMode?: AngleMode;
  ans?: number;
};

export type EvaluationResult = {
  value: number;
  lastBinary: LastBinaryOperation | null;
};

const MAX_NESTING = 64;
const MIN_NONZERO_ABS = 1e-323;
const MAX_ABS = 1e308;
const TANGENT_EPSILON = 1e-12;

function checkResult(value: number, position: number | null = null): number {
  if (Number.isNaN(value)) {
    throw new CalcError('domain_error', position);
  }
  if (!Number.isFinite(value) || Math.abs(value) > MAX_ABS) {
    throw new CalcError('overflow', position);
  }
  if (value !== 0 && Math.abs(value) < MIN_NONZERO_ABS) {
    throw new CalcError('underflow', position);
  }
  return value;
}

function applyBinary(operator: BinaryOperator, left: number, right: number): number {
  switch (operator) {
    case '+':
      return checkResult(left + right);
    case '-':
      return checkResult(left - right);
    case '*':
      return checkResult(left * right);
    case '/':
      if (right === 0) {
        throw new CalcError('division_by_zero');
      }
      return checkResult(left / right);
    case '^': {
      if (left === 0 && right < 0) {
        throw new CalcError('division_by_zero');
      }
      const value = Math.pow(left, right);
      if (Number.isNaN(value)) {
        throw new CalcError('domain_error');
      }
      return checkResult(value);
    }
    case 'mod': {
      if (right === 0) {
        throw new CalcError('division_by_zero');
      }
      return checkResult(left - right * Math.trunc(left / right));
    }
    default:
      throw new CalcError('syntax');
  }
}

function factorial(value: number, position: number | null): number {
  if (!Number.isInteger(value) || value < 0) {
    throw new CalcError('factorial_domain', position);
  }
  if (value > 170) {
    throw new CalcError('overflow', position);
  }
  let result = 1;
  for (let factor = 2; factor <= value; factor += 1) {
    result *= factor;
  }
  return checkResult(result, position);
}

function toRadians(value: number, angleMode: AngleMode): number {
  return angleMode === 'deg' ? (value * Math.PI) / 180 : value;
}

function fromRadians(value: number, angleMode: AngleMode): number {
  return angleMode === 'deg' ? (value * 180) / Math.PI : value;
}

function expectArgumentCount(args: number[], count: number, position: number): void {
  if (args.length !== count) {
    throw new CalcError('syntax', position);
  }
}

function applyFunction(
  name: string,
  args: number[],
  angleMode: AngleMode,
  position: number,
): number {
  switch (name) {
    case 'sin':
      expectArgumentCount(args, 1, position);
      return checkResult(Math.sin(toRadians(args[0], angleMode)), position);
    case 'cos':
      expectArgumentCount(args, 1, position);
      return checkResult(Math.cos(toRadians(args[0], angleMode)), position);
    case 'tan': {
      expectArgumentCount(args, 1, position);
      const radians = toRadians(args[0], angleMode);
      if (Math.abs(Math.cos(radians)) < TANGENT_EPSILON) {
        throw new CalcError('angle_domain', position);
      }
      return checkResult(Math.tan(radians), position);
    }
    case 'asin':
      expectArgumentCount(args, 1, position);
      if (args[0] < -1 || args[0] > 1) {
        throw new CalcError('domain_error', position);
      }
      return checkResult(fromRadians(Math.asin(args[0]), angleMode), position);
    case 'acos':
      expectArgumentCount(args, 1, position);
      if (args[0] < -1 || args[0] > 1) {
        throw new CalcError('domain_error', position);
      }
      return checkResult(fromRadians(Math.acos(args[0]), angleMode), position);
    case 'atan':
      expectArgumentCount(args, 1, position);
      return checkResult(fromRadians(Math.atan(args[0]), angleMode), position);
    case 'sinh':
      expectArgumentCount(args, 1, position);
      return checkResult(Math.sinh(args[0]), position);
    case 'cosh':
      expectArgumentCount(args, 1, position);
      return checkResult(Math.cosh(args[0]), position);
    case 'tanh':
      expectArgumentCount(args, 1, position);
      return checkResult(Math.tanh(args[0]), position);
    case 'asinh':
      expectArgumentCount(args, 1, position);
      return checkResult(Math.asinh(args[0]), position);
    case 'acosh':
      expectArgumentCount(args, 1, position);
      if (args[0] < 1) {
        throw new CalcError('domain_error', position);
      }
      return checkResult(Math.acosh(args[0]), position);
    case 'atanh':
      expectArgumentCount(args, 1, position);
      if (Math.abs(args[0]) >= 1) {
        throw new CalcError('domain_error', position);
      }
      return checkResult(Math.atanh(args[0]), position);
    case 'ln':
      expectArgumentCount(args, 1, position);
      if (args[0] <= 0) {
        throw new CalcError('domain_error', position);
      }
      return checkResult(Math.log(args[0]), position);
    case 'log':
    case 'log10': {
      expectArgumentCount(args, name === 'log' && args.length === 2 ? 2 : 1, position);
      if (name === 'log' && args.length === 2) {
        if (args[0] <= 0 || args[1] <= 0 || args[1] === 1) {
          throw new CalcError('domain_error', position);
        }
        return checkResult(Math.log(args[0]) / Math.log(args[1]), position);
      }
      if (args[0] <= 0) {
        throw new CalcError('domain_error', position);
      }
      return checkResult(Math.log10(args[0]), position);
    }
    case 'log2':
      expectArgumentCount(args, 1, position);
      if (args[0] <= 0) {
        throw new CalcError('domain_error', position);
      }
      return checkResult(Math.log2(args[0]), position);
    case 'logb':
    case 'log_base':
      expectArgumentCount(args, 2, position);
      if (args[0] <= 0 || args[1] <= 0 || args[1] === 1) {
        throw new CalcError('domain_error', position);
      }
      return checkResult(Math.log(args[0]) / Math.log(args[1]), position);
    case 'exp':
      expectArgumentCount(args, 1, position);
      return checkResult(Math.exp(args[0]), position);
    case 'pow10':
      expectArgumentCount(args, 1, position);
      return checkResult(Math.pow(10, args[0]), position);
    case 'sqrt':
      expectArgumentCount(args, 1, position);
      if (args[0] < 0) {
        throw new CalcError('domain_error', position);
      }
      return checkResult(Math.sqrt(args[0]), position);
    case 'cbrt':
      expectArgumentCount(args, 1, position);
      return checkResult(Math.cbrt(args[0]), position);
    case 'root': {
      expectArgumentCount(args, 2, position);
      const [value, degree] = args;
      if (!Number.isInteger(degree) || degree === 0) {
        throw new CalcError('domain_error', position);
      }
      if (value < 0) {
        if (Math.abs(degree) % 2 !== 1) {
          throw new CalcError('domain_error', position);
        }
        return checkResult(-Math.pow(-value, 1 / degree), position);
      }
      return checkResult(Math.pow(value, 1 / degree), position);
    }
    case 'abs':
      expectArgumentCount(args, 1, position);
      return checkResult(Math.abs(args[0]), position);
    case 'recip':
      expectArgumentCount(args, 1, position);
      if (args[0] === 0) {
        throw new CalcError('division_by_zero', position);
      }
      return checkResult(1 / args[0], position);
    case 'fact':
    case 'factorial':
      expectArgumentCount(args, 1, position);
      return factorial(args[0], position);
    case 'mod':
      expectArgumentCount(args, 2, position);
      return applyBinary('mod', args[0], args[1]);
    default:
      throw new CalcError('unknown_function', position);
  }
}

class Parser {
  private readonly tokens: Token[];
  private index = 0;
  private nesting = 0;
  private readonly angleMode: AngleMode;
  private readonly ans: number;

  constructor(tokens: Token[], angleMode: AngleMode, ans: number) {
    this.tokens = tokens;
    this.angleMode = angleMode;
    this.ans = ans;
  }

  parse(): EvaluationResult {
    const result = this.parseAdditive();
    const next = this.peek();
    if (next) {
      if (next.type === 'rparen') {
        throw new CalcError('unbalanced_parentheses', next.start);
      }
      throw new CalcError('syntax', next.start);
    }
    return result;
  }

  private parseAdditive(): EvaluationResult {
    let left = this.parseMultiplicative();
    while (true) {
      const operatorToken = this.peek();
      if (
        operatorToken?.type !== 'operator' ||
        (operatorToken.value !== '+' && operatorToken.value !== '-')
      ) {
        return left;
      }
      this.index += 1;
      const right = this.parseMultiplicative();
      const value = checkResult(
        applyBinary(operatorToken.value, left.value, right.value),
        operatorToken.start,
      );
      left = {
        value,
        lastBinary: { operator: operatorToken.value, operand: right.value },
      };
    }
  }

  private parseMultiplicative(): EvaluationResult {
    let left = this.parseUnary();
    while (true) {
      const operatorToken = this.peek();
      if (
        operatorToken?.type !== 'operator' ||
        (operatorToken.value !== '*' &&
          operatorToken.value !== '/' &&
          operatorToken.value !== 'mod')
      ) {
        return left;
      }
      this.index += 1;
      const right = this.parseUnary();
      const value = checkResult(
        applyBinary(operatorToken.value, left.value, right.value),
        operatorToken.start,
      );
      left = {
        value,
        lastBinary: { operator: operatorToken.value, operand: right.value },
      };
    }
  }

  private parseUnary(): EvaluationResult {
    const token = this.peek();
    if (token?.type === 'operator' && (token.value === '+' || token.value === '-')) {
      this.index += 1;
      const operand = this.parseUnary();
      return {
        value: checkResult(token.value === '-' ? -operand.value : operand.value, token.start),
        lastBinary: null,
      };
    }
    return this.parsePower();
  }

  private parsePower(): EvaluationResult {
    const left = this.parsePostfix();
    const operatorToken = this.peek();
    if (operatorToken?.type !== 'operator' || operatorToken.value !== '^') {
      return left;
    }
    this.index += 1;
    const right = this.parseUnary();
    return {
      value: checkResult(applyBinary('^', left.value, right.value), operatorToken.start),
      lastBinary: { operator: '^', operand: right.value },
    };
  }

  private parsePostfix(): EvaluationResult {
    let value = this.parsePrimary().value;
    while (true) {
      const token = this.peek();
      if (token?.type === 'factorial') {
        this.index += 1;
        value = factorial(value, token.start);
        continue;
      }
      if (token?.type === 'percent') {
        this.index += 1;
        value = checkResult(value / 100, token.start);
        continue;
      }
      return { value, lastBinary: null };
    }
  }

  private parsePrimary(): EvaluationResult {
    const token = this.peek();
    if (!token) {
      throw new CalcError('syntax');
    }

    if (token.type === 'number') {
      this.index += 1;
      return { value: checkResult(token.value, token.start), lastBinary: null };
    }

    if (token.type === 'identifier') {
      this.index += 1;
      const next = this.peek();
      if (next?.type === 'lparen') {
        return this.parseFunctionCall(token.value, token.start);
      }
      const constant = this.resolveIdentifier(token.value, token.start);
      return { value: constant, lastBinary: null };
    }

    if (token.type === 'lparen') {
      this.index += 1;
      this.enterNesting(token.start);
      const result = this.parseAdditive();
      this.expectClosingParen();
      this.nesting -= 1;
      return { value: result.value, lastBinary: null };
    }

    if (token.type === 'rparen') {
      throw new CalcError('unbalanced_parentheses', token.start);
    }

    throw new CalcError('syntax', token.start);
  }

  private parseFunctionCall(name: string, position: number): EvaluationResult {
    this.expectType('lparen');
    this.enterNesting(position);
    const args: number[] = [];
    if (this.peek()?.type !== 'rparen') {
      while (true) {
        args.push(this.parseAdditive().value);
        if (this.peek()?.type === 'comma') {
          this.index += 1;
          continue;
        }
        break;
      }
    }
    this.expectClosingParen();
    this.nesting -= 1;
    return {
      value: applyFunction(name, args, this.angleMode, position),
      lastBinary: null,
    };
  }

  private resolveIdentifier(name: string, position: number): number {
    switch (name) {
      case 'pi':
        return Math.PI;
      case 'e':
        return Math.E;
      case 'ans':
        return checkResult(this.ans, position);
      default:
        throw new CalcError('unknown_function', position);
    }
  }

  private enterNesting(position: number): void {
    this.nesting += 1;
    if (this.nesting > MAX_NESTING) {
      throw new CalcError('nesting_too_deep', position);
    }
  }

  private expectClosingParen(): void {
    const token = this.peek();
    if (token?.type !== 'rparen') {
      throw new CalcError('unbalanced_parentheses', token?.start ?? null);
    }
    this.index += 1;
  }

  private expectType(type: Token['type']): Token {
    const token = this.peek();
    if (token?.type !== type) {
      throw new CalcError('syntax', token?.start ?? null);
    }
    this.index += 1;
    return token;
  }

  private peek(): Token | undefined {
    return this.tokens[this.index];
  }
}

export function evaluate(expression: string, options: EvaluationOptions = {}): EvaluationResult {
  const tokens = tokenize(expression);
  if (tokens.length === 0) {
    throw new CalcError('syntax');
  }
  const parser = new Parser(tokens, options.angleMode ?? 'deg', options.ans ?? 0);
  return parser.parse();
}

export function applyRepeatedEquals(value: number, operation: LastBinaryOperation | null): number {
  if (!operation) {
    return value;
  }
  return checkResult(applyBinary(operation.operator, value, operation.operand));
}
