import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2CustomConstant } from '../api';
import { ConstantEditor } from './ConstantEditor';

vi.mock('../api', () => ({
  saveV2CustomConstant: vi.fn(),
}));

import * as api from '../api';

const mockedSave = vi.mocked(api.saveV2CustomConstant);

function savedConstant(overrides: Partial<V2CustomConstant> = {}): V2CustomConstant {
  return {
    id: 3,
    isBuiltin: false,
    name: 'Planck constant (local)',
    symbol: 'h',
    valueText: '6.62607015e-34',
    unit: 'J s',
    category: 'Fundamental',
    uncertaintyText: 'exact',
    precisionText: '9 significant figures',
    source: 'User note',
    description: 'Locally recorded constant.',
    notes: '',
    createdAt: '2026-11-10T00:00:00.000Z',
    updatedAt: '2026-11-10T00:00:00.000Z',
    ...overrides,
  };
}

async function fillRequired() {
  const user = userEvent.setup();
  await user.type(screen.getByLabelText('Full name'), 'Planck constant (local)');
  await user.type(screen.getByLabelText('Symbol'), 'h');
  await user.clear(screen.getByLabelText('Category'));
  await user.type(screen.getByLabelText('Category'), 'Fundamental');
  await user.type(screen.getByLabelText(/Numerical value/), '6.62607015e-34');
  await user.type(screen.getByLabelText('Description'), 'Locally recorded constant.');
  return user;
}

describe('ConstantEditor', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('submits verbatim value text with optional uncertainty and precision', async () => {
    const onSaved = vi.fn();
    mockedSave.mockResolvedValue(savedConstant());
    render(<ConstantEditor onClose={vi.fn()} onSaved={onSaved} />);

    const user = await fillRequired();
    await user.type(screen.getByLabelText('Unit'), 'J s');
    await user.type(screen.getByLabelText('Uncertainty'), 'exact');
    await user.type(
      screen.getByLabelText('Precision or significant figures'),
      '9 significant figures',
    );
    await user.type(screen.getByLabelText('Source or reference'), 'User note');
    await user.click(screen.getByRole('button', { name: 'Add constant' }));

    expect(mockedSave).toHaveBeenCalledWith({
      id: null,
      name: 'Planck constant (local)',
      symbol: 'h',
      valueText: '6.62607015e-34',
      unit: 'J s',
      category: 'Fundamental',
      uncertaintyText: 'exact',
      precisionText: '9 significant figures',
      source: 'User note',
      description: 'Locally recorded constant.',
      notes: '',
    });
    expect(onSaved).toHaveBeenCalledWith(savedConstant());
  });

  it('maps a backend value error to the numerical value field', async () => {
    const user = userEvent.setup();
    mockedSave.mockRejectedValue({
      code: 'validation_error',
      field: 'valueText',
      message: 'Enter a finite number, for example 1.602e-19.',
    });
    render(<ConstantEditor onClose={vi.fn()} onSaved={vi.fn()} />);

    await fillRequired();
    await user.click(screen.getByRole('button', { name: 'Add constant' }));

    expect(
      await screen.findByText('Enter a finite number, for example 1.602e-19.'),
    ).toBeInTheDocument();
    expect(screen.getByLabelText(/Numerical value/)).toHaveAttribute('aria-invalid', 'true');
  });

  it('keeps uncertainty and precision optional', async () => {
    const user = userEvent.setup();
    mockedSave.mockResolvedValue(
      savedConstant({ uncertaintyText: '', precisionText: '', source: '', unit: '' }),
    );
    render(<ConstantEditor onClose={vi.fn()} onSaved={vi.fn()} />);

    await fillRequired();
    await user.click(screen.getByRole('button', { name: 'Add constant' }));

    expect(mockedSave).toHaveBeenCalledWith(
      expect.objectContaining({
        uncertaintyText: '',
        precisionText: '',
        source: '',
        unit: '',
      }),
    );
  });

  it('previews LaTeX symbols and units while preserving the exact source', async () => {
    const user = userEvent.setup();
    const symbol = String.raw`\hbar`;
    const unit = String.raw`\mathrm{J\,s}`;
    mockedSave.mockResolvedValue(savedConstant({ symbol, unit }));
    render(<ConstantEditor onClose={vi.fn()} onSaved={vi.fn()} />);

    await user.type(screen.getByLabelText('Full name'), 'Reduced Planck constant');
    fireEvent.change(screen.getByLabelText('Symbol'), { target: { value: symbol } });
    fireEvent.change(screen.getByLabelText('Unit'), { target: { value: unit } });
    await user.clear(screen.getByLabelText('Category'));
    await user.type(screen.getByLabelText('Category'), 'Fundamental');
    await user.type(screen.getByLabelText(/Numerical value/), '1.054571817e-34');
    await user.type(
      screen.getByLabelText('Description'),
      String.raw`Reduced Planck constant; $E=\hbar\omega$.`,
    );

    await waitFor(() =>
      expect(
        [...document.querySelectorAll('[data-math-preview] [data-latex-source]')].some(
          (element) => element.getAttribute('data-latex-source') === symbol,
        ),
      ).toBe(true),
    );
    expect(document.querySelectorAll('[data-math-preview] .katex').length).toBeGreaterThan(0);
    expect(document.querySelector('[data-math-preview] .v2-math-text .katex')).not.toBeNull();

    await user.click(screen.getByRole('button', { name: 'Add constant' }));
    expect(mockedSave).toHaveBeenCalledWith(
      expect.objectContaining({
        symbol,
        unit,
        description: String.raw`Reduced Planck constant; $E=\hbar\omega$.`,
      }),
    );
  });

  it('reopens an existing LaTeX symbol in source form with a preview', async () => {
    const symbol = String.raw`\mu_0`;
    render(
      <ConstantEditor
        constant={savedConstant({ symbol, unit: String.raw`\mathrm{N\,A^{-2}}` })}
        onClose={vi.fn()}
        onSaved={vi.fn()}
      />,
    );

    expect(screen.getByLabelText('Symbol')).toHaveValue(symbol);
    expect(screen.getByLabelText('Unit')).toHaveValue(String.raw`\mathrm{N\,A^{-2}}`);
    await waitFor(() =>
      expect(
        [...document.querySelectorAll('[data-latex-source]')].some(
          (element) => element.getAttribute('data-latex-source') === symbol,
        ),
      ).toBe(true),
    );
  });
});
