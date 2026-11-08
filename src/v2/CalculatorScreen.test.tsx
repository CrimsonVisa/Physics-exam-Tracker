import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { CalculatorScreen } from './CalculatorScreen';
import { CalculatorSessionProvider } from './calculator/CalculatorSessionProvider';

const writeText = vi.fn().mockResolvedValue(undefined);

function key(name: string) {
  return screen.getByRole('button', { name });
}

function renderScreen() {
  return render(
    <CalculatorSessionProvider>
      <CalculatorScreen />
    </CalculatorSessionProvider>,
  );
}

function calculatorRoot(): HTMLElement {
  const root = document.querySelector('.v2-calculator-page');
  if (!(root instanceof HTMLElement)) {
    throw new Error('Calculator root not found');
  }
  root.focus();
  return root;
}

describe('CalculatorScreen', () => {
  beforeEach(() => {
    writeText.mockClear();
    if (navigator.clipboard) {
      Object.defineProperty(navigator.clipboard, 'writeText', {
        configurable: true,
        value: writeText,
      });
    } else {
      Object.defineProperty(navigator, 'clipboard', {
        configurable: true,
        value: { writeText },
      });
    }
  });

  it('opens in Practice mode with the combined-policy notice', () => {
    renderScreen();
    expect(
      screen.getByRole('heading', { level: 1, name: 'Scientific calculator' }),
    ).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Calculator mode' })).toHaveValue('practice');
    expect(screen.getByText('Practice — not an examination simulation')).toBeInTheDocument();
    expect(
      screen.getByText(/Combined view has no single examination calculator policy/),
    ).toBeInTheDocument();
    expect(key('Sine')).toBeEnabled();
    expect(screen.getByLabelText('No result')).toHaveAttribute('aria-live', 'polite');
  });

  it('evaluates keypad input and records history', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(key('Two'));
    await user.click(key('Add'));
    await user.click(key('Three'));
    await user.click(key('Equals'));

    expect(screen.getByLabelText('Result 5')).toBeInTheDocument();
    expect(screen.getByLabelText('Current expression: 2+3')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Reuse expression 2+3' })).toBeInTheDocument();
  });

  it('supports keyboard entry, repeated equals and Ans', async () => {
    const user = userEvent.setup();
    renderScreen();
    calculatorRoot();

    await user.keyboard('2+3{Enter}');
    expect(screen.getByLabelText('Result 5')).toBeInTheDocument();

    await user.keyboard('=');
    expect(screen.getByLabelText('Result 8')).toBeInTheDocument();
    await user.keyboard('=');
    expect(screen.getByLabelText('Result 11')).toBeInTheDocument();

    await user.keyboard('+2{Enter}');
    expect(screen.getByLabelText('Result 13')).toBeInTheDocument();
  });

  it('supports backspace, clear entry and all clear', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(key('One'));
    await user.click(key('Two'));
    expect(screen.getByLabelText('Current expression: 12')).toBeInTheDocument();

    await user.click(key('Backspace'));
    expect(screen.getByLabelText('Current expression: 1')).toBeInTheDocument();

    await user.click(key('Clear entry'));
    expect(screen.getByLabelText('Current expression: empty')).toBeInTheDocument();

    await user.click(key('One'));
    await user.click(key('Add'));
    await user.click(key('Two'));
    await user.click(key('Equals'));
    await user.click(key('All clear'));
    expect(screen.getByLabelText('Current expression: empty')).toBeInTheDocument();
    expect(screen.getByLabelText('No result')).toBeInTheDocument();
  });

  it('supports Escape, Delete and Shift+Escape', async () => {
    const user = userEvent.setup();
    renderScreen();
    const root = calculatorRoot();

    await user.keyboard('1+2');
    await user.keyboard('{Escape}');
    expect(screen.getByLabelText('Current expression: empty')).toBeInTheDocument();

    await user.keyboard('3+4');
    await user.keyboard('{Delete}');
    expect(screen.getByLabelText('Current expression: empty')).toBeInTheDocument();

    await user.keyboard('5+6');
    fireEvent.keyDown(root, { key: 'Escape', shiftKey: true });
    expect(screen.getByLabelText('Current expression: empty')).toBeInTheDocument();
    expect(screen.getByLabelText('No result')).toBeInTheDocument();
  });

  it('keeps the expression when evaluation fails', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(key('One'));
    await user.click(key('Divide'));
    await user.click(key('Zero'));
    await user.click(key('Equals'));

    expect(screen.getByRole('alert')).toHaveTextContent('Division by zero is not allowed.');
    expect(screen.getByLabelText('Current expression: 1/0')).toBeInTheDocument();
    expect(key('Copy result')).toBeDisabled();
  });

  it('copies the displayed rounded result, not hidden precision', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(key('Zero'));
    await user.click(key('Decimal point'));
    await user.click(key('One'));
    await user.click(key('Add'));
    await user.click(key('Zero'));
    await user.click(key('Decimal point'));
    await user.click(key('Two'));
    await user.click(key('Equals'));

    expect(screen.getByLabelText('Approximately 0.3')).toBeInTheDocument();
    await user.click(key('Copy result'));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith('0.3'));
    expect(await screen.findByText('Copied result.')).toBeInTheDocument();
  });

  it('reports unavailable clipboard access without losing the result', async () => {
    const user = userEvent.setup();
    writeText.mockRejectedValueOnce(new Error('denied'));
    renderScreen();

    await user.click(key('Two'));
    await user.click(key('Add'));
    await user.click(key('Three'));
    await user.click(key('Equals'));
    await user.click(key('Copy result'));

    expect(await screen.findByText(/Clipboard access is unavailable/)).toBeInTheDocument();
    expect(screen.getByLabelText('Result 5')).toBeInTheDocument();
  });

  it('supports memory store, recall, add and subtract', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(key('Five'));
    await user.click(key('Memory store'));
    expect(screen.getByLabelText('Memory stored')).toBeInTheDocument();

    await user.click(key('All clear'));
    await user.click(key('Memory recall'));
    await user.click(key('Equals'));
    expect(screen.getByLabelText('Result 5')).toBeInTheDocument();

    await user.click(key('All clear'));
    await user.click(key('Two'));
    await user.click(key('Memory add'));
    await user.click(key('All clear'));
    await user.click(key('Memory recall'));
    await user.click(key('Equals'));
    expect(screen.getByLabelText('Result 7')).toBeInTheDocument();

    await user.click(key('All clear'));
    await user.click(key('One'));
    await user.click(key('Memory subtract'));
    await user.click(key('All clear'));
    await user.click(key('Memory recall'));
    await user.click(key('Equals'));
    expect(screen.getByLabelText('Result 6')).toBeInTheDocument();

    await user.click(key('Memory clear'));
    expect(screen.queryByLabelText('Memory stored')).not.toBeInTheDocument();
  });

  it('uses the full repeated-equals result for memory operations', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(key('Two'));
    await user.click(key('Add'));
    await user.click(key('Three'));
    await user.click(key('Equals'));
    await user.click(key('Equals'));
    await user.click(key('Equals'));
    expect(screen.getByLabelText('Result 11')).toBeInTheDocument();

    await user.click(key('Memory add'));
    await user.click(key('All clear'));
    await user.click(key('Memory recall'));
    await user.click(key('Equals'));
    expect(screen.getByLabelText('Result 11')).toBeInTheDocument();
  });

  it('rejects M+ overflow and preserves the previous memory value', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(key('One'));
    await user.click(key('Scientific notation exponent'));
    await user.click(key('Three'));
    await user.click(key('Zero'));
    await user.click(key('Eight'));
    await user.click(key('Memory store'));
    expect(screen.getByLabelText('Memory stored')).toBeInTheDocument();

    await user.click(key('Memory add'));
    expect(screen.getByRole('alert')).toHaveTextContent('Memory can only store finite numbers.');
    expect(screen.getByLabelText('Memory stored')).toBeInTheDocument();

    await user.click(key('Clear entry'));
    await user.click(key('Memory recall'));
    await user.click(key('Equals'));
    expect(screen.getByLabelText('Result 1E308')).toBeInTheDocument();
  });

  it('rejects M− overflow and preserves the previous memory value', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(key('One'));
    await user.click(key('Scientific notation exponent'));
    await user.click(key('Three'));
    await user.click(key('Zero'));
    await user.click(key('Eight'));
    await user.click(key('Memory store'));
    expect(screen.getByLabelText('Memory stored')).toBeInTheDocument();

    await user.click(key('Change sign'));
    await user.click(key('Memory subtract'));
    expect(screen.getByRole('alert')).toHaveTextContent('Memory can only store finite numbers.');
    expect(screen.getByLabelText('Memory stored')).toBeInTheDocument();

    await user.click(key('Clear entry'));
    await user.click(key('Memory recall'));
    await user.click(key('Equals'));
    expect(screen.getByLabelText('Result 1E308')).toBeInTheDocument();
  });

  it('evaluates two-argument logarithms and roots with the comma key', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(key('Arbitrary base logarithm'));
    await user.click(key('Eight'));
    await user.click(key('Comma argument separator'));
    await user.click(key('Two'));
    await user.click(key('Close parenthesis'));
    await user.click(key('Equals'));
    expect(screen.getByLabelText('Result 3')).toBeInTheDocument();

    await user.click(key('All clear'));
    await user.click(key('Arbitrary root'));
    calculatorRoot();
    await user.keyboard('27,3){Enter}');
    expect(screen.getByLabelText(/^(Approximately|Result) 3$/)).toBeInTheDocument();
  });

  it('switches to policy-only modes without discarding the expression', async () => {
    const user = userEvent.setup();
    renderScreen();
    const mode = screen.getByRole('combobox', { name: 'Calculator mode' });

    await user.click(key('Two'));
    await user.click(key('Add'));
    await user.click(key('Three'));

    await user.selectOptions(mode, 'csir-net-physics');
    expect(
      screen.getByRole('heading', { name: 'No examination calculator keypad' }),
    ).toBeInTheDocument();
    expect(screen.getAllByText(/June 2025/).length).toBeGreaterThan(0);
    expect(screen.queryByRole('button', { name: 'Sine' })).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Display angle mode')).not.toBeInTheDocument();
    expect(screen.getByLabelText('Current expression: 2+3')).toBeInTheDocument();

    await user.click(key('Use Practice Calculator'));
    expect(screen.getByRole('combobox', { name: 'Calculator mode' })).toHaveValue('practice');
    expect(screen.getByLabelText('Current expression: 2+3')).toBeInTheDocument();
    await user.click(key('Equals'));
    expect(screen.getByLabelText('Result 5')).toBeInTheDocument();
  });

  it('supports keyboard arithmetic in GATE mode', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.selectOptions(
      screen.getByRole('combobox', { name: 'Calculator mode' }),
      'gate-physics',
    );
    calculatorRoot();
    await user.keyboard('2+3{Enter}');
    expect(screen.getByLabelText('Result 5')).toBeInTheDocument();
  });

  it.each(['csir-net-physics', 'tifr-gs-physics', 'jest-physics'])(
    'ignores keyboard calculation in %s mode',
    async (mode) => {
      const user = userEvent.setup();
      renderScreen();

      await user.selectOptions(screen.getByRole('combobox', { name: 'Calculator mode' }), mode);
      calculatorRoot();
      await user.keyboard('2+3{Enter}');
      expect(screen.getByLabelText('Current expression: empty')).toBeInTheDocument();
      expect(screen.getByLabelText('No result')).toBeInTheDocument();
    },
  );

  it('does not evaluate an existing expression with Enter in policy-only mode', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(key('Two'));
    await user.click(key('Add'));
    await user.click(key('Three'));
    await user.selectOptions(
      screen.getByRole('combobox', { name: 'Calculator mode' }),
      'csir-net-physics',
    );
    calculatorRoot();
    await user.keyboard('{Enter}');
    expect(screen.getByLabelText('Current expression: 2+3')).toBeInTheDocument();
    expect(screen.getByLabelText('No result')).toBeInTheDocument();
  });

  it('restores keyboard calculation after returning to Practice', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(key('Two'));
    await user.click(key('Add'));
    await user.click(key('Three'));
    await user.selectOptions(
      screen.getByRole('combobox', { name: 'Calculator mode' }),
      'csir-net-physics',
    );
    calculatorRoot();
    await user.keyboard('9{Enter}');
    expect(screen.getByLabelText('Current expression: 2+3')).toBeInTheDocument();

    await user.click(key('Use Practice Calculator'));
    expect(screen.getByRole('combobox', { name: 'Calculator mode' })).toHaveValue('practice');
    expect(screen.getByLabelText('Current expression: 2+3')).toBeInTheDocument();
    calculatorRoot();
    await user.keyboard('+4{Enter}');
    expect(screen.getByLabelText('Result 9')).toBeInTheDocument();
  });

  it('renders GATE scientific mode with the qualification banner', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.selectOptions(
      screen.getByRole('combobox', { name: 'Calculator mode' }),
      'gate-physics',
    );
    expect(screen.getByText('GATE 2027 qualification')).toBeInTheDocument();
    expect(screen.getAllByText(/not an exact official GATE replica/).length).toBeGreaterThan(0);
    expect(screen.getByText(/keyboard operation is disabled/)).toBeInTheDocument();
    expect(key('Hyperbolic sine')).toBeEnabled();
    expect(key('Arbitrary base logarithm')).toBeEnabled();
  });

  it('renders TIFR and JEST policy-only cards', async () => {
    const user = userEvent.setup();
    renderScreen();
    const mode = screen.getByRole('combobox', { name: 'Calculator mode' });

    await user.selectOptions(mode, 'tifr-gs-physics');
    expect(screen.getAllByText(/GS 2024/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/GS 2027/).length).toBeGreaterThan(0);
    expect(screen.queryByRole('button', { name: 'Sine' })).not.toBeInTheDocument();

    await user.selectOptions(mode, 'jest-physics');
    expect(screen.getAllByText(/JEST 2026/).length).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: 'Use Practice Calculator' })).toBeInTheDocument();
  });

  it('switches between degree and radian modes', async () => {
    const user = userEvent.setup();
    renderScreen();

    expect(screen.getByRole('button', { name: 'Degrees' })).toHaveAttribute('aria-pressed', 'true');
    await user.click(key('Sine'));
    await user.click(key('Nine'));
    await user.click(key('Zero'));
    await user.click(key('Close parenthesis'));
    await user.click(key('Equals'));
    expect(screen.getByLabelText('Result 1')).toBeInTheDocument();

    await user.click(key('All clear'));
    await user.click(screen.getByRole('button', { name: 'Radians' }));
    expect(screen.getByRole('button', { name: 'Radians' })).toHaveAttribute('aria-pressed', 'true');
    await user.click(key('Sine'));
    await user.click(key('Pi'));
    await user.click(key('Divide'));
    await user.click(key('Two'));
    await user.click(key('Close parenthesis'));
    await user.click(key('Equals'));
    expect(screen.getByLabelText('Result 1')).toBeInTheDocument();
  });

  it('reuses and copies history entries and clears history', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(key('Two'));
    await user.click(key('Add'));
    await user.click(key('Three'));
    await user.click(key('Equals'));

    await user.click(screen.getByRole('button', { name: 'Reuse expression 2+3' }));
    expect(screen.getByLabelText('Current expression: 2+3')).toBeInTheDocument();
    await user.click(key('Equals'));
    expect(screen.getByLabelText('Result 5')).toBeInTheDocument();

    const copyButtons = screen.getAllByRole('button', { name: 'Copy result 5' });
    await user.click(copyButtons[0]);
    await waitFor(() => expect(writeText).toHaveBeenCalledWith('5'));

    await user.click(key('Clear history'));
    expect(screen.getByText('No calculations yet.')).toBeInTheDocument();
  });

  it('provides accessible labels, alerts and a live result region', async () => {
    const user = userEvent.setup();
    renderScreen();
    expect(key('Inverse hyperbolic tangent')).toHaveAttribute(
      'aria-label',
      'Inverse hyperbolic tangent',
    );
    expect(key('Decimal point')).toHaveAttribute('aria-label', 'Decimal point');
    expect(screen.getByLabelText('No result')).toHaveAttribute('aria-live', 'polite');

    await user.click(key('One'));
    await user.click(key('Divide'));
    await user.click(key('Zero'));
    await user.click(key('Equals'));
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });
});
