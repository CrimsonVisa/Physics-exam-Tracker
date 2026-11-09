export type MemoryOperation = 'clear' | 'recall' | 'store' | 'add' | 'subtract';

export type CalculatorKeyAction =
  | { type: 'append'; value: string }
  | { type: 'evaluate' }
  | { type: 'backspace' }
  | { type: 'clear-entry' }
  | { type: 'all-clear' }
  | { type: 'toggle-sign' }
  | { type: 'memory'; operation: MemoryOperation };

type KeyVariant = 'memory' | 'function' | 'numeric' | 'operator' | 'command' | 'equals';

type CalculatorKeyDefinition = {
  id: string;
  label: string;
  ariaLabel: string;
  action: CalculatorKeyAction;
  variant: KeyVariant;
};

const MEMORY_KEYS: CalculatorKeyDefinition[] = [
  {
    id: 'memory-clear',
    label: 'MC',
    ariaLabel: 'Memory clear',
    action: { type: 'memory', operation: 'clear' },
    variant: 'memory',
  },
  {
    id: 'memory-recall',
    label: 'MR',
    ariaLabel: 'Memory recall',
    action: { type: 'memory', operation: 'recall' },
    variant: 'memory',
  },
  {
    id: 'memory-store',
    label: 'MS',
    ariaLabel: 'Memory store',
    action: { type: 'memory', operation: 'store' },
    variant: 'memory',
  },
  {
    id: 'memory-add',
    label: 'M+',
    ariaLabel: 'Memory add',
    action: { type: 'memory', operation: 'add' },
    variant: 'memory',
  },
];

const FUNCTION_ROWS: CalculatorKeyDefinition[][] = [
  [
    {
      id: 'memory-subtract',
      label: 'M−',
      ariaLabel: 'Memory subtract',
      action: { type: 'memory', operation: 'subtract' },
      variant: 'memory',
    },
    {
      id: 'sin',
      label: 'sin',
      ariaLabel: 'Sine',
      action: { type: 'append', value: 'sin(' },
      variant: 'function',
    },
    {
      id: 'cos',
      label: 'cos',
      ariaLabel: 'Cosine',
      action: { type: 'append', value: 'cos(' },
      variant: 'function',
    },
    {
      id: 'tan',
      label: 'tan',
      ariaLabel: 'Tangent',
      action: { type: 'append', value: 'tan(' },
      variant: 'function',
    },
  ],
  [
    {
      id: 'asin',
      label: 'sin⁻¹',
      ariaLabel: 'Inverse sine',
      action: { type: 'append', value: 'asin(' },
      variant: 'function',
    },
    {
      id: 'acos',
      label: 'cos⁻¹',
      ariaLabel: 'Inverse cosine',
      action: { type: 'append', value: 'acos(' },
      variant: 'function',
    },
    {
      id: 'atan',
      label: 'tan⁻¹',
      ariaLabel: 'Inverse tangent',
      action: { type: 'append', value: 'atan(' },
      variant: 'function',
    },
    {
      id: 'sinh',
      label: 'sinh',
      ariaLabel: 'Hyperbolic sine',
      action: { type: 'append', value: 'sinh(' },
      variant: 'function',
    },
  ],
  [
    {
      id: 'cosh',
      label: 'cosh',
      ariaLabel: 'Hyperbolic cosine',
      action: { type: 'append', value: 'cosh(' },
      variant: 'function',
    },
    {
      id: 'tanh',
      label: 'tanh',
      ariaLabel: 'Hyperbolic tangent',
      action: { type: 'append', value: 'tanh(' },
      variant: 'function',
    },
    {
      id: 'asinh',
      label: 'sinh⁻¹',
      ariaLabel: 'Inverse hyperbolic sine',
      action: { type: 'append', value: 'asinh(' },
      variant: 'function',
    },
    {
      id: 'acosh',
      label: 'cosh⁻¹',
      ariaLabel: 'Inverse hyperbolic cosine',
      action: { type: 'append', value: 'acosh(' },
      variant: 'function',
    },
  ],
  [
    {
      id: 'atanh',
      label: 'tanh⁻¹',
      ariaLabel: 'Inverse hyperbolic tangent',
      action: { type: 'append', value: 'atanh(' },
      variant: 'function',
    },
    {
      id: 'ln',
      label: 'ln',
      ariaLabel: 'Natural logarithm',
      action: { type: 'append', value: 'ln(' },
      variant: 'function',
    },
    {
      id: 'log10',
      label: 'log',
      ariaLabel: 'Base ten logarithm',
      action: { type: 'append', value: 'log(' },
      variant: 'function',
    },
    {
      id: 'log2',
      label: 'log₂',
      ariaLabel: 'Base two logarithm',
      action: { type: 'append', value: 'log2(' },
      variant: 'function',
    },
  ],
  [
    {
      id: 'logb',
      label: 'log_b',
      ariaLabel: 'Arbitrary base logarithm',
      action: { type: 'append', value: 'logb(' },
      variant: 'function',
    },
    {
      id: 'exp',
      label: 'e^x',
      ariaLabel: 'e raised to the power x',
      action: { type: 'append', value: 'exp(' },
      variant: 'function',
    },
    {
      id: 'pow10',
      label: '10^x',
      ariaLabel: 'Ten raised to the power x',
      action: { type: 'append', value: 'pow10(' },
      variant: 'function',
    },
    {
      id: 'square',
      label: 'x²',
      ariaLabel: 'Square',
      action: { type: 'append', value: '^2' },
      variant: 'function',
    },
  ],
  [
    {
      id: 'cube',
      label: 'x³',
      ariaLabel: 'Cube',
      action: { type: 'append', value: '^3' },
      variant: 'function',
    },
    {
      id: 'power',
      label: 'x^y',
      ariaLabel: 'Raise to a power',
      action: { type: 'append', value: '^' },
      variant: 'operator',
    },
    {
      id: 'sqrt',
      label: '√',
      ariaLabel: 'Square root',
      action: { type: 'append', value: 'sqrt(' },
      variant: 'function',
    },
    {
      id: 'cbrt',
      label: '∛',
      ariaLabel: 'Cube root',
      action: { type: 'append', value: 'cbrt(' },
      variant: 'function',
    },
  ],
  [
    {
      id: 'root',
      label: 'ʸ√x',
      ariaLabel: 'Arbitrary root',
      action: { type: 'append', value: 'root(' },
      variant: 'function',
    },
    {
      id: 'pi',
      label: 'π',
      ariaLabel: 'Pi',
      action: { type: 'append', value: 'π' },
      variant: 'function',
    },
    {
      id: 'e',
      label: 'e',
      ariaLabel: 'Euler number',
      action: { type: 'append', value: 'e' },
      variant: 'function',
    },
    {
      id: 'abs',
      label: '|x|',
      ariaLabel: 'Absolute value',
      action: { type: 'append', value: 'abs(' },
      variant: 'function',
    },
  ],
  [
    {
      id: 'factorial',
      label: 'n!',
      ariaLabel: 'Factorial',
      action: { type: 'append', value: '!' },
      variant: 'function',
    },
    {
      id: 'mod',
      label: 'mod',
      ariaLabel: 'Modulus remainder',
      action: { type: 'append', value: ' mod ' },
      variant: 'operator',
    },
    {
      id: 'exp-entry',
      label: 'Exp',
      ariaLabel: 'Scientific notation exponent',
      action: { type: 'append', value: 'E' },
      variant: 'function',
    },
    {
      id: 'reciprocal',
      label: '1/x',
      ariaLabel: 'Reciprocal',
      action: { type: 'append', value: 'recip(' },
      variant: 'function',
    },
  ],
  [
    {
      id: 'lparen',
      label: '(',
      ariaLabel: 'Open parenthesis',
      action: { type: 'append', value: '(' },
      variant: 'operator',
    },
    {
      id: 'rparen',
      label: ')',
      ariaLabel: 'Close parenthesis',
      action: { type: 'append', value: ')' },
      variant: 'operator',
    },
    {
      id: 'percent',
      label: '%',
      ariaLabel: 'Percentage',
      action: { type: 'append', value: '%' },
      variant: 'operator',
    },
    {
      id: 'ans',
      label: 'Ans',
      ariaLabel: 'Previous answer',
      action: { type: 'append', value: 'Ans' },
      variant: 'function',
    },
  ],
];

const NUMERIC_ROWS: CalculatorKeyDefinition[][] = [
  [
    {
      id: 'all-clear',
      label: 'AC',
      ariaLabel: 'All clear',
      action: { type: 'all-clear' },
      variant: 'command',
    },
    {
      id: 'clear-entry',
      label: 'CE',
      ariaLabel: 'Clear entry',
      action: { type: 'clear-entry' },
      variant: 'command',
    },
    {
      id: 'backspace',
      label: '⌫',
      ariaLabel: 'Backspace',
      action: { type: 'backspace' },
      variant: 'command',
    },
    {
      id: 'divide',
      label: '÷',
      ariaLabel: 'Divide',
      action: { type: 'append', value: '/' },
      variant: 'operator',
    },
  ],
  [
    {
      id: 'seven',
      label: '7',
      ariaLabel: 'Seven',
      action: { type: 'append', value: '7' },
      variant: 'numeric',
    },
    {
      id: 'eight',
      label: '8',
      ariaLabel: 'Eight',
      action: { type: 'append', value: '8' },
      variant: 'numeric',
    },
    {
      id: 'nine',
      label: '9',
      ariaLabel: 'Nine',
      action: { type: 'append', value: '9' },
      variant: 'numeric',
    },
    {
      id: 'multiply',
      label: '×',
      ariaLabel: 'Multiply',
      action: { type: 'append', value: '*' },
      variant: 'operator',
    },
  ],
  [
    {
      id: 'four',
      label: '4',
      ariaLabel: 'Four',
      action: { type: 'append', value: '4' },
      variant: 'numeric',
    },
    {
      id: 'five',
      label: '5',
      ariaLabel: 'Five',
      action: { type: 'append', value: '5' },
      variant: 'numeric',
    },
    {
      id: 'six',
      label: '6',
      ariaLabel: 'Six',
      action: { type: 'append', value: '6' },
      variant: 'numeric',
    },
    {
      id: 'subtract',
      label: '−',
      ariaLabel: 'Subtract',
      action: { type: 'append', value: '-' },
      variant: 'operator',
    },
  ],
  [
    {
      id: 'one',
      label: '1',
      ariaLabel: 'One',
      action: { type: 'append', value: '1' },
      variant: 'numeric',
    },
    {
      id: 'two',
      label: '2',
      ariaLabel: 'Two',
      action: { type: 'append', value: '2' },
      variant: 'numeric',
    },
    {
      id: 'three',
      label: '3',
      ariaLabel: 'Three',
      action: { type: 'append', value: '3' },
      variant: 'numeric',
    },
    {
      id: 'add',
      label: '+',
      ariaLabel: 'Add',
      action: { type: 'append', value: '+' },
      variant: 'operator',
    },
  ],
  [
    {
      id: 'toggle-sign',
      label: '±',
      ariaLabel: 'Change sign',
      action: { type: 'toggle-sign' },
      variant: 'command',
    },
    {
      id: 'zero',
      label: '0',
      ariaLabel: 'Zero',
      action: { type: 'append', value: '0' },
      variant: 'numeric',
    },
    {
      id: 'decimal',
      label: '.',
      ariaLabel: 'Decimal point',
      action: { type: 'append', value: '.' },
      variant: 'numeric',
    },
    {
      id: 'equals',
      label: '=',
      ariaLabel: 'Equals',
      action: { type: 'evaluate' },
      variant: 'equals',
    },
  ],
];

type CalculatorKeypadProps = {
  onAction: (action: CalculatorKeyAction) => void;
};

function CalculatorKey({
  keyDefinition,
  onAction,
}: {
  keyDefinition: CalculatorKeyDefinition;
  onAction: (action: CalculatorKeyAction) => void;
}) {
  return (
    <button
      aria-label={keyDefinition.ariaLabel}
      className={`v2-calculator-key is-${keyDefinition.variant}`}
      onClick={() => onAction(keyDefinition.action)}
      title={keyDefinition.ariaLabel}
      type="button"
    >
      {keyDefinition.label}
    </button>
  );
}

export function CalculatorKeypad({ onAction }: CalculatorKeypadProps) {
  return (
    <section aria-label="Calculator keypad" className="v2-calculator-keypad">
      <div
        aria-label="Scientific functions"
        className="v2-calculator-key-grid is-functions"
        role="group"
      >
        {MEMORY_KEYS.map((key) => (
          <CalculatorKey key={key.id} keyDefinition={key} onAction={onAction} />
        ))}
        {FUNCTION_ROWS.flat().map((key) => (
          <CalculatorKey key={key.id} keyDefinition={key} onAction={onAction} />
        ))}
      </div>
      <div aria-label="Number pad" className="v2-calculator-key-grid is-numeric" role="group">
        {NUMERIC_ROWS.flat().map((key) => (
          <CalculatorKey key={key.id} keyDefinition={key} onAction={onAction} />
        ))}
      </div>
      <div className="v2-calculator-keypad-utility">
        <button
          aria-label="Comma argument separator"
          className="v2-calculator-key is-command v2-calculator-comma-key"
          onClick={() => onAction({ type: 'append', value: ',' })}
          type="button"
        >
          ,
        </button>
        <span>Use commas in log_b(x, b) and root(x, n).</span>
      </div>
    </section>
  );
}
