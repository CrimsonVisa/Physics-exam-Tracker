import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2CustomFormula } from '../api';
import { CustomFormulaEditor } from './CustomFormulaEditor';

vi.mock('../api', () => ({
  saveV2CustomFormula: vi.fn(),
}));

import * as api from '../api';

const mockedSave = vi.mocked(api.saveV2CustomFormula);

function savedFormula(): V2CustomFormula {
  return {
    id: 7,
    isBuiltin: false,
    name: 'Local work-energy relation',
    latex: 'W = Delta K',
    plainText: 'W = Delta K',
    category: 'Classical mechanics',
    description: 'Net work equals the change in kinetic energy.',
    notes: '',
    source: '',
    createdAt: '2026-11-10T00:00:00.000Z',
    updatedAt: '2026-11-10T00:00:00.000Z',
    variables: [{ id: 1, position: 0, symbol: 'W', meaning: 'net work', unit: 'J' }],
  };
}

async function fillRequired() {
  const user = userEvent.setup();
  await user.type(screen.getByLabelText(/Formula name/), 'Local work-energy relation');
  await user.type(screen.getByLabelText('Category'), 'Classical mechanics');
  await user.type(screen.getByLabelText('LaTeX expression'), 'W = Delta K');
  await user.type(screen.getByLabelText('Plain-text expression'), 'W = Delta K');
  await user.type(
    screen.getByLabelText('Description'),
    'Net work equals the change in kinetic energy.',
  );
  return user;
}

describe('CustomFormulaEditor', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('submits name, both expression forms and variable rows', async () => {
    const onSaved = vi.fn();
    mockedSave.mockResolvedValue(savedFormula());
    render(<CustomFormulaEditor onClose={vi.fn()} onSaved={onSaved} />);

    const user = await fillRequired();
    await user.click(screen.getByRole('button', { name: 'Add variable' }));
    await user.type(screen.getByLabelText('Variable 1 symbol'), 'W');
    await user.type(screen.getByLabelText('Variable 1 meaning'), 'net work');
    await user.type(screen.getByLabelText('Variable 1 unit'), 'J');
    await user.click(screen.getByRole('button', { name: 'Add formula' }));

    expect(mockedSave).toHaveBeenCalledTimes(1);
    expect(mockedSave).toHaveBeenCalledWith({
      id: null,
      name: 'Local work-energy relation',
      latex: 'W = Delta K',
      plainText: 'W = Delta K',
      category: 'Classical mechanics',
      description: 'Net work equals the change in kinetic energy.',
      notes: '',
      source: '',
      variables: [{ symbol: 'W', meaning: 'net work', unit: 'J' }],
    });
    expect(onSaved).toHaveBeenCalledWith(savedFormula());
  });

  it('shows a backend field error without closing the editor', async () => {
    const onSaved = vi.fn();
    mockedSave.mockRejectedValue({
      code: 'validation_error',
      field: 'name',
      message: 'A custom formula with this name already exists.',
    });
    render(<CustomFormulaEditor onClose={vi.fn()} onSaved={onSaved} />);

    const user = await fillRequired();
    await user.click(screen.getByRole('button', { name: 'Add formula' }));

    expect(
      await screen.findByText('A custom formula with this name already exists.'),
    ).toBeInTheDocument();
    expect(screen.getByLabelText(/Formula name/)).toHaveAttribute('aria-invalid', 'true');
    expect(onSaved).not.toHaveBeenCalled();
  });

  it('blocks a name that duplicates a built-in formula', async () => {
    const user = userEvent.setup();
    const onSaved = vi.fn();
    render(<CustomFormulaEditor onClose={vi.fn()} onSaved={onSaved} />);

    await fillRequired();
    await user.clear(screen.getByLabelText(/Formula name/));
    await user.type(
      screen.getByLabelText(/Formula name/),
      'Maxwell-Faraday law (differential form)',
    );
    await user.click(screen.getByRole('button', { name: 'Add formula' }));

    expect(
      await screen.findByText(/The built-in formula "Maxwell-Faraday law/),
    ).toBeInTheDocument();
    expect(mockedSave).not.toHaveBeenCalled();
    expect(onSaved).not.toHaveBeenCalled();
  });

  it('adds and removes variable rows', async () => {
    const user = userEvent.setup();
    render(<CustomFormulaEditor onClose={vi.fn()} onSaved={vi.fn()} />);

    await user.click(screen.getByRole('button', { name: 'Add variable' }));
    expect(screen.getByLabelText('Variable 1 symbol')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Remove variable 1' }));
    expect(screen.queryByLabelText('Variable 1 symbol')).not.toBeInTheDocument();
    expect(screen.getByText('No variables recorded yet.')).toBeInTheDocument();
  });
});
