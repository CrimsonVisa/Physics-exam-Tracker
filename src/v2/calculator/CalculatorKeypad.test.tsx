import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { CalculatorKeypad } from './CalculatorKeypad';

describe('CalculatorKeypad', () => {
  it('emits append and command actions for scientific keys', async () => {
    const user = userEvent.setup();
    const onAction = vi.fn();
    render(<CalculatorKeypad onAction={onAction} />);

    await user.click(screen.getByRole('button', { name: 'Sine' }));
    expect(onAction).toHaveBeenLastCalledWith({ type: 'append', value: 'sin(' });

    await user.click(screen.getByRole('button', { name: 'Square' }));
    expect(onAction).toHaveBeenLastCalledWith({ type: 'append', value: '^2' });

    await user.click(screen.getByRole('button', { name: 'Arbitrary base logarithm' }));
    expect(onAction).toHaveBeenLastCalledWith({ type: 'append', value: 'logb(' });

    await user.click(screen.getByRole('button', { name: 'Memory store' }));
    expect(onAction).toHaveBeenLastCalledWith({
      type: 'memory',
      operation: 'store',
    });

    await user.click(screen.getByRole('button', { name: 'Equals' }));
    expect(onAction).toHaveBeenLastCalledWith({ type: 'evaluate' });

    await user.click(screen.getByRole('button', { name: 'Backspace' }));
    expect(onAction).toHaveBeenLastCalledWith({ type: 'backspace' });

    await user.click(screen.getByRole('button', { name: 'All clear' }));
    expect(onAction).toHaveBeenLastCalledWith({ type: 'all-clear' });

    await user.click(screen.getByRole('button', { name: 'Change sign' }));
    expect(onAction).toHaveBeenLastCalledWith({ type: 'toggle-sign' });

    await user.click(screen.getByRole('button', { name: 'Comma argument separator' }));
    expect(onAction).toHaveBeenLastCalledWith({ type: 'append', value: ',' });
  });

  it('makes every scientific, memory and numeric function available immediately', () => {
    render(<CalculatorKeypad onAction={() => undefined} />);

    for (const name of [
      'Sine',
      'Hyperbolic sine',
      'Inverse hyperbolic tangent',
      'Arbitrary base logarithm',
      'Arbitrary root',
      'Factorial',
      'Memory recall',
      'Previous answer',
      'Equals',
    ]) {
      expect(screen.getByRole('button', { name })).toBeEnabled();
    }
  });

  it('provides readable accessible names for symbols', () => {
    render(<CalculatorKeypad onAction={() => undefined} />);
    expect(screen.getByRole('button', { name: 'Inverse hyperbolic tangent' })).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Scientific notation exponent' }),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Previous answer' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Decimal point' })).toBeInTheDocument();
  });
});
