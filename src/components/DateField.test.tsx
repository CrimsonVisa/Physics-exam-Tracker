import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { DateField } from './DateField';

describe('DateField', () => {
  it('associates its label, hint, and native date input', () => {
    render(
      <DateField
        id="exam-date"
        label="Exam date"
        value="2026-12-19"
        onChange={() => {}}
        hint="The provisional default is 19 December 2026."
      />,
    );

    const input = screen.getByLabelText('Exam date');
    expect(input).toHaveAttribute('type', 'date');
    expect(input).toHaveValue('2026-12-19');
    expect(input).toHaveAccessibleDescription('The provisional default is 19 December 2026.');
    expect(screen.getByText('The provisional default is 19 December 2026.')).toHaveAttribute(
      'id',
      'exam-date-hint',
    );
  });

  it('announces an error and marks the input invalid', () => {
    render(
      <DateField
        id="exam-date"
        label="Exam date"
        value=""
        onChange={() => {}}
        hint="Use the date picker or type a date."
        error="Choose a real calendar date."
      />,
    );

    const input = screen.getByLabelText('Exam date');
    expect(input).toHaveAttribute('aria-invalid', 'true');
    expect(input).toHaveAccessibleDescription(
      'Use the date picker or type a date. Choose a real calendar date.',
    );
    expect(screen.getByRole('alert')).toHaveTextContent('Choose a real calendar date.');
  });

  it('reports manual and picker changes through one callback', () => {
    const onChange = vi.fn();
    render(<DateField id="exam-date" label="Exam date" value="2026-12-19" onChange={onChange} />);

    fireEvent.change(screen.getByLabelText('Exam date'), {
      target: { value: '2027-01-10' },
    });

    expect(onChange).toHaveBeenCalledWith('2027-01-10');
  });

  it('is reachable from the keyboard and can be disabled during saves', async () => {
    const user = userEvent.setup();
    const { rerender } = render(
      <DateField id="exam-date" label="Exam date" value="2026-12-19" onChange={() => {}} />,
    );

    await user.tab();
    expect(screen.getByLabelText('Exam date')).toHaveFocus();

    rerender(
      <DateField
        id="exam-date"
        label="Exam date"
        value="2026-12-19"
        onChange={() => {}}
        disabled
      />,
    );

    expect(screen.getByLabelText('Exam date')).toBeDisabled();
  });

  it('exposes an optional valid range to the native picker', () => {
    render(
      <DateField
        id="preparation-start-date"
        label="Preparation start date"
        value="2026-10-29"
        onChange={() => {}}
        min="2020-01-01"
        max="2026-12-19"
      />,
    );

    const input = screen.getByLabelText('Preparation start date');
    expect(input).toHaveAttribute('min', '2020-01-01');
    expect(input).toHaveAttribute('max', '2026-12-19');
  });
});
