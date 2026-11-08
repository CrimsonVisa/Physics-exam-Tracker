export type AngleMode = 'deg' | 'rad';

export type CalculatorModeId =
  'practice' | 'csir-net-physics' | 'gate-physics' | 'tifr-gs-physics' | 'jest-physics';

export type CalculatorModeType = 'practice' | 'exam-calculator' | 'exam-policy';

export type EvidenceStatus =
  | 'confirmed-permitted'
  | 'confirmed-prohibited'
  | 'confirmed-provided'
  | 'not-documented'
  | 'not-provided'
  | 'year-dependent';

export type FunctionAvailability =
  'available' | 'unavailable' | 'not-documented' | 'practice-fallback' | 'mock-observed';

export type CalculatorFunctionId =
  | 'add'
  | 'subtract'
  | 'multiply'
  | 'divide'
  | 'parentheses'
  | 'percentage'
  | 'sign'
  | 'reciprocal'
  | 'factorial'
  | 'abs'
  | 'mod'
  | 'square'
  | 'cube'
  | 'power'
  | 'sqrt'
  | 'cbrt'
  | 'root'
  | 'scientific-notation'
  | 'sin'
  | 'cos'
  | 'tan'
  | 'asin'
  | 'acos'
  | 'atan'
  | 'sinh'
  | 'cosh'
  | 'tanh'
  | 'asinh'
  | 'acosh'
  | 'atanh'
  | 'angle-mode'
  | 'ln'
  | 'log10'
  | 'log2'
  | 'logb'
  | 'exp'
  | 'pow10'
  | 'pi'
  | 'e'
  | 'memory'
  | 'ans'
  | 'repeat-equals';

export type BinaryOperator = '+' | '-' | '*' | '/' | '^' | 'mod';

export type LastBinaryOperation = {
  operator: BinaryOperator;
  operand: number;
};

export type PolicySource = {
  title: string;
  url: string;
  year: string;
  reference: string;
  kind: 'policy' | 'mock-reference';
};

export type CalculatorProfile = {
  id: CalculatorModeId;
  label: string;
  shortLabel: string;
  modeType: CalculatorModeType;
  physicalCalculator: EvidenceStatus;
  onScreenCalculator: EvidenceStatus;
  functionSet: EvidenceStatus;
  applicableYear: string;
  lastVerified: string;
  summary: string;
  caveats: string[];
  sources: PolicySource[];
  functions: Record<CalculatorFunctionId, FunctionAvailability>;
  hasKeypad: boolean;
  keyboardNote?: string;
  fallbackMode?: CalculatorModeId;
};

export type HistoryEntry = {
  id: string;
  expression: string;
  result: string;
  value: number;
  mode: CalculatorModeId;
  timestamp: number;
};
