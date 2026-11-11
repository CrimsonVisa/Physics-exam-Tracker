import { render, screen } from '@testing-library/react';
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

  it('blocks a unit name that duplicates the built-in catalogue', async () => {
    const user = userEvent.setup();
    render(<CustomUnitEditor onClose={vi.fn()} onSaved={vi.fn()} />);

    await fillRequired();
    await user.clear(screen.getByLabelText('Unit name'));
    await user.type(screen.getByLabelText('Unit name'), 'Metre');
    await user.click(screen.getByRole('button', { name: 'Add unit' }));

    expect(
      await screen.findByText(/The built-in unit "metre" already uses this name/),
    ).toBeInTheDocument();
    expect(mockedSave).not.toHaveBeenCalled();
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
});
