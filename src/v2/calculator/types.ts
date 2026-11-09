export type AngleMode = 'deg' | 'rad';

export type BinaryOperator = '+' | '-' | '*' | '/' | '^' | 'mod';

export type LastBinaryOperation = {
  operator: BinaryOperator;
  operand: number;
};

export type HistoryEntry = {
  id: string;
  expression: string;
  result: string;
  value: number;
  timestamp: number;
};
