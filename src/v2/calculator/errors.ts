export type CalculationErrorCode =
  | 'syntax'
  | 'unknown_function'
  | 'unbalanced_parentheses'
  | 'division_by_zero'
  | 'domain_error'
  | 'overflow'
  | 'underflow'
  | 'invalid_number'
  | 'factorial_domain'
  | 'angle_domain'
  | 'too_long'
  | 'too_many_tokens'
  | 'nesting_too_deep';

const MESSAGES: Record<CalculationErrorCode, string> = {
  syntax: 'Check the expression syntax and try again.',
  unknown_function: 'That function is not supported by this calculator.',
  unbalanced_parentheses: 'Parentheses are not balanced.',
  division_by_zero: 'Division by zero is not allowed.',
  domain_error: 'The result is outside the supported domain.',
  overflow: 'The result is too large to display.',
  underflow: 'The result is too small to display.',
  invalid_number: 'That number is not valid.',
  factorial_domain: 'Factorial requires a non-negative integer from 0 to 170.',
  angle_domain: 'That angle is not valid for this function.',
  too_long: 'The expression is too long. Keep it under 512 characters.',
  too_many_tokens: 'The expression has too many parts.',
  nesting_too_deep: 'The expression is nested too deeply.',
};

export class CalcError extends Error {
  readonly code: CalculationErrorCode;
  readonly position: number | null;

  constructor(code: CalculationErrorCode, position: number | null = null, message?: string) {
    super(message ?? MESSAGES[code]);
    this.name = 'CalcError';
    this.code = code;
    this.position = position;
  }
}

export function isCalcError(error: unknown): error is CalcError {
  return error instanceof CalcError;
}

export function calculationErrorMessage(error: unknown): string {
  return isCalcError(error) ? error.message : 'The calculation could not be completed.';
}
