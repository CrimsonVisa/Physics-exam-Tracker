import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { CalculatorKeypad } from './CalculatorKeypad';
import { getCalculatorProfile } from './profiles';

describe('CalculatorKeypad', () => {
  it('emits append and command actions for scientific keys', async () => {
    const user = userEvent.setup();
    const onAction = vi.fn();
    render(<CalculatorKeypad onAction={onAction} profile={getCalculatorProfile('practice')} />);

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

  it('enables all GATE mock-observed functions', () => {
    render(
      <CalculatorKeypad
        onAction={() => undefined}
        profile={getCalculatorProfile('gate-physics')}
      />,
    );
    expect(screen.getByRole('button', { name: 'Sine' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Hyperbolic sine' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Arbitrary root' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Memory recall' })).toBeEnabled();
  });

  it('disables functions that a policy profile does not document', () => {
    render(
      <CalculatorKeypad
        onAction={() => undefined}
        profile={getCalculatorProfile('csir-net-physics')}
      />,
    );
    expect(screen.getByRole('button', { name: 'Sine' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Equals' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'All clear' })).toBeEnabled();
  });

  it('provides readable accessible names for symbols', () => {
    render(
      <CalculatorKeypad onAction={() => undefined} profile={getCalculatorProfile('practice')} />,
    );
    expect(screen.getByRole('button', { name: 'Inverse hyperbolic tangent' })).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Scientific notation exponent' }),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Previous answer' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Decimal point' })).toBeInTheDocument();
  });
});
