import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2CustomUnit } from '../api';
import { CustomUnitEditor } from './CustomUnitEditor';

vi.mock('../api', () => ({
  saveV2CustomUnit: vi.fn(),
}));

import * as api from '../api';

const mockedSave = vi.mocked(api.saveV2CustomUnit);

function savedUnit(overrides: Partial<V2CustomUnit> = {}): V2CustomUnit {
  return {
    id: 9,
    isBuiltin: false,
    name: 'Foot',
    symbol: 'ft',
    quantity: 'Length',
    dimension: 'L',
    siClassification: 'non_si',
    conversionKind: 'scalar',
    conversionFactor: '0.3048',
    conversionReferenceUnit: 'm',
    conversionNote: '',
    description: 'Imperial unit of length.',
    notes: '',
    source: '',
    createdAt: '2026-11-10T00:00:00.000Z',
    updatedAt: '2026-11-10T00:00:00.000Z',
    ...overrides,
  };
}

async function fillRequired() {
  const user = userEvent.setup();
  await user.type(screen.getByLabelText('Unit name'), 'Foot');
  await user.type(screen.getByLabelText(/^Symbol/), 'ft');
  await user.type(screen.getByLabelText('Physical quantity'), 'Length');
  await user.type(screen.getByLabelText('Description'), 'Imperial unit of length.');
  return user;
}

describe('CustomUnitEditor', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('submits a scalar conversion with factor and reference unit', async () => {
    const onSaved = vi.fn();
    mockedSave.mockResolvedValue(savedUnit());
    render(<CustomUnitEditor onClose={vi.fn()} onSaved={onSaved} />);

    const user = await fillRequired();
    await user.selectOptions(screen.getByLabelText(/Conversion kind/), 'scalar');
    await user.type(screen.getByLabelText(/Conversion factor/), '0.3048');
    await user.type(screen.getByLabelText('Reference unit'), 'm');
    await user.click(screen.getByRole('button', { name: 'Add unit' }));

    expect(mockedSave).toHaveBeenCalledWith({
      id: null,
      name: 'Foot',
      symbol: 'ft',
      quantity: 'Length',
      dimension: '',
      siClassification: 'non_si',
      conversionKind: 'scalar',
      conversionFactor: '0.3048',
      conversionReferenceUnit: 'm',
      conversionNote: '',
      description: 'Imperial unit of length.',
      notes: '',
      source: '',
    });
    expect(onSaved).toHaveBeenCalledWith(savedUnit());
  });

  it('requires contextual conversion text and clears the scalar factor', async () => {
    const user = userEvent.setup();
    mockedSave.mockResolvedValue(
      savedUnit({
        conversionKind: 'contextual',
        conversionFactor: '',
        conversionReferenceUnit: '',
        conversionNote: 'Defined relative to the Harvard Bridge.',
      }),
    );
    render(<CustomUnitEditor onClose={vi.fn()} onSaved={vi.fn()} />);

    await fillRequired();
    await user.selectOptions(screen.getByLabelText(/Conversion kind/), 'scalar');
    await user.type(screen.getByLabelText(/Conversion factor/), '1.7');
    await user.selectOptions(screen.getByLabelText(/Conversion kind/), 'contextual');

    expect(screen.queryByLabelText('Conversion factor')).not.toBeInTheDocument();
    expect(screen.getByLabelText('Conversion context')).toBeRequired();
    await user.type(
      screen.getByLabelText('Conversion context'),
      'Defined relative to the Harvard Bridge.',
    );
    await user.click(screen.getByRole('button', { name: 'Add unit' }));

    expect(mockedSave).toHaveBeenCalledWith(
      expect.objectContaining({
        conversionKind: 'contextual',
        conversionFactor: '',
        conversionNote: 'Defined relative to the Harvard Bridge.',
      }),
    );
  });

  it('maps a backend conversion error to the factor field', async () => {
    const user = userEvent.setup();
    mockedSave.mockRejectedValue({
      code: 'validation_error',
      field: 'conversionFactor',
      message: 'Enter a finite number, for example 1.602e-19.',
    });
    render(<CustomUnitEditor onClose={vi.fn()} onSaved={vi.fn()} />);

    await fillRequired();
    await user.selectOptions(screen.getByLabelText(/Conversion kind/), 'scalar');
    await user.type(screen.getByLabelText(/Conversion factor/), '1e-3');
    await user.type(screen.getByLabelText('Reference unit'), 'm');
    await user.click(screen.getByRole('button', { name: 'Add unit' }));

    expect(
      await screen.findByText('Enter a finite number, for example 1.602e-19.'),
    ).toBeInTheDocument();
    expect(screen.getByLabelText(/Conversion factor/)).toHaveAttribute('aria-invalid', 'true');
  });

  it('allows a custom unit name that also exists in the built-in catalogue', async () => {
    const user = userEvent.setup();
    mockedSave.mockResolvedValue(savedUnit({ name: 'Metre', symbol: 'ft' }));
    render(<CustomUnitEditor onClose={vi.fn()} onSaved={vi.fn()} />);

    await fillRequired();
    await user.clear(screen.getByLabelText('Unit name'));
    await user.type(screen.getByLabelText('Unit name'), 'Metre');

    expect(await screen.findByText(/A built-in unit already uses this name/)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Add unit' }));

    expect(mockedSave).toHaveBeenCalledWith(expect.objectContaining({ name: 'Metre' }));
  });

  it('allows a custom unit symbol that also exists in the built-in catalogue', async () => {
    const user = userEvent.setup();
    mockedSave.mockResolvedValue(savedUnit({ symbol: 'm' }));
    render(<CustomUnitEditor onClose={vi.fn()} onSaved={vi.fn()} />);

    await fillRequired();
    await user.clear(screen.getByLabelText(/^Symbol/));
    await user.type(screen.getByLabelText(/^Symbol/), 'm');

    expect(await screen.findByText(/A built-in unit already uses this symbol/)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Add unit' }));

    expect(mockedSave).toHaveBeenCalledWith(expect.objectContaining({ symbol: 'm' }));
  });

  it('shows a general error for a duplicate symbol', async () => {
    const user = userEvent.setup();
    mockedSave.mockRejectedValue({
      code: 'validation_error',
      field: 'symbol',
      message: 'A custom unit with this symbol already exists.',
    });
    render(<CustomUnitEditor onClose={vi.fn()} onSaved={vi.fn()} />);

    await fillRequired();
    await user.click(screen.getByRole('button', { name: 'Add unit' }));

    expect(
      await screen.findByText('A custom unit with this symbol already exists.'),
    ).toBeInTheDocument();
    expect(screen.getByLabelText(/^Symbol/)).toHaveAttribute('aria-invalid', 'true');
  });

  it('previews LaTeX unit symbols, dimensions and reference units while preserving source', async () => {
    const user = userEvent.setup();
    const symbol = String.raw`\mu_0`;
    const dimension = String.raw`\mathsf{M}\mathsf{L}\mathsf{T}^{-2}\mathsf{I}^{-2}`;
    const referenceUnit = String.raw`\mathrm{N\,A^{-2}}`;
    mockedSave.mockResolvedValue(savedUnit({ symbol }));
    render(<CustomUnitEditor onClose={vi.fn()} onSaved={vi.fn()} />);

    await fillRequired();
    fireEvent.change(screen.getByLabelText(/^Symbol/), { target: { value: symbol } });
    fireEvent.change(screen.getByLabelText('Physical dimension'), {
      target: { value: dimension },
    });
    await user.selectOptions(screen.getByLabelText(/Conversion kind/), 'scalar');
    fireEvent.change(screen.getByLabelText(/Conversion factor/), {
      target: { value: '4.0e-7' },
    });
    fireEvent.change(screen.getByLabelText('Reference unit'), {
      target: { value: referenceUnit },
    });

    await waitFor(() =>
      expect(
        [...document.querySelectorAll('[data-math-preview] [data-latex-source]')].map((element) =>
          element.getAttribute('data-latex-source'),
        ),
      ).toEqual(expect.arrayContaining([symbol, dimension, referenceUnit])),
    );

    await user.click(screen.getByRole('button', { name: 'Add unit' }));
    expect(mockedSave).toHaveBeenCalledWith(
      expect.objectContaining({
        symbol,
        dimension,
        conversionReferenceUnit: referenceUnit,
      }),
    );
  });

  it('reopens saved LaTeX unit fields in source form with previews', async () => {
    const unit = savedUnit({
      symbol: String.raw`\Omega`,
      dimension: String.raw`\mathsf{M}\mathsf{L}^{2}\mathsf{T}^{-3}\mathsf{I}^{-2}`,
      conversionReferenceUnit: String.raw`\mathrm{V\,A^{-1}}`,
    });
    render(<CustomUnitEditor unit={unit} onClose={vi.fn()} onSaved={vi.fn()} />);

    expect(screen.getByLabelText(/^Symbol/)).toHaveValue(unit.symbol);
    expect(screen.getByLabelText('Physical dimension')).toHaveValue(unit.dimension);
    expect(screen.getByLabelText('Reference unit')).toHaveValue(unit.conversionReferenceUnit);
    await waitFor(() =>
      expect(
        [...document.querySelectorAll('[data-latex-source]')].some(
          (element) => element.getAttribute('data-latex-source') === unit.symbol,
        ),
      ).toBe(true),
    );
  });
});
