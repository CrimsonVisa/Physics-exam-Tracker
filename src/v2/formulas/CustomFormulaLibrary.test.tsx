import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2CustomFormula } from '../api';
import { CustomFormulaLibrary } from './CustomFormulaLibrary';

vi.mock('../api', () => ({
  listV2CustomFormulas: vi.fn(),
  saveV2CustomFormula: vi.fn(),
  deleteV2CustomFormula: vi.fn(),
}));

import * as api from '../api';

const mockedList = vi.mocked(api.listV2CustomFormulas);
const mockedDelete = vi.mocked(api.deleteV2CustomFormula);

function formula(overrides: Partial<V2CustomFormula> = {}): V2CustomFormula {
  return {
    id: 1,
    isBuiltin: false,
    name: 'Mass-energy relation',
    latex: String.raw`E = mc^2`,
    plainText: 'E = m c^2',
    category: 'Special relativity',
    description: 'Rest energy of a body with mass m.',
    notes: '',
    source: '',
    createdAt: '2026-11-10T00:00:00.000Z',
    updatedAt: '2026-11-10T00:00:00.000Z',
    variables: [
      { id: 1, position: 0, symbol: 'E', meaning: 'rest energy', unit: 'J' },
      { id: 2, position: 1, symbol: 'm', meaning: 'mass', unit: 'kg' },
    ],
    ...overrides,
  };
}

describe('CustomFormulaLibrary', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedList.mockResolvedValue([]);
    mockedDelete.mockResolvedValue(undefined);
  });

  it('shows an empty state and an add action when no formulas exist', async () => {
    render(<CustomFormulaLibrary />);

    expect(await screen.findByText('No custom formulas yet')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add your first formula' })).toBeInTheDocument();
    expect(mockedList).toHaveBeenCalledTimes(1);
  });

  it('searches and filters custom formulas by category', async () => {
    const user = userEvent.setup();
    mockedList.mockResolvedValue([
      formula(),
      formula({
        id: 2,
        name: 'Photon energy',
        latex: 'E = h nu',
        plainText: 'E = h nu',
        category: 'Quantum mechanics',
        description: 'Energy of a photon.',
        variables: [],
      }),
    ]);
    render(<CustomFormulaLibrary />);

    await screen.findByText('Mass-energy relation');
    expect(screen.getByText('2 formulas')).toBeInTheDocument();

    await user.type(screen.getByLabelText('Search my formulas'), 'photon');
    expect(screen.getByText('1 formula')).toBeInTheDocument();
    expect(screen.queryByText('Mass-energy relation')).not.toBeInTheDocument();

    await user.clear(screen.getByLabelText('Search my formulas'));
    await user.selectOptions(
      screen.getByLabelText('Formula category filter'),
      'Special relativity',
    );
    expect(screen.getByText('Mass-energy relation')).toBeInTheDocument();
    expect(screen.queryByText('Photon energy')).not.toBeInTheDocument();
  });

  it('opens a custom formula detail with variables and does not expose edit controls for built-ins', async () => {
    const user = userEvent.setup();
    mockedList.mockResolvedValue([formula()]);
    render(<CustomFormulaLibrary />);

    await user.click(
      await screen.findByRole('button', { name: 'Open custom formula Mass-energy relation' }),
    );

    const detail = document.querySelector('.v2-reference-detail');
    expect(detail).not.toBeNull();
    expect(
      within(detail as HTMLElement).getByRole('heading', {
        level: 2,
        name: 'Mass-energy relation',
      }),
    ).toBeInTheDocument();
    expect(within(detail as HTMLElement).getByText('rest energy')).toBeInTheDocument();
    expect(within(detail as HTMLElement).getByText('kg')).toBeInTheDocument();
    expect(within(detail as HTMLElement).getByText('User-defined')).toBeInTheDocument();
  });

  it('confirms deletion before removing a custom formula', async () => {
    const user = userEvent.setup();
    mockedList.mockResolvedValue([formula()]);
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);
    try {
      render(<CustomFormulaLibrary />);
      await user.click(
        await screen.findByRole('button', { name: 'Open custom formula Mass-energy relation' }),
      );
      await user.click(screen.getByRole('button', { name: 'Delete formula' }));

      await waitFor(() => expect(mockedDelete).toHaveBeenCalledWith(1));
      expect(confirm).toHaveBeenCalledWith(
        'Delete the custom formula "Mass-energy relation"? This cannot be undone.',
      );
      expect(await screen.findByText('No custom formulas yet')).toBeInTheDocument();
    } finally {
      confirm.mockRestore();
    }
  });

  it('reports a load failure without hiding the add action', async () => {
    mockedList.mockRejectedValue({ code: 'database_error', message: 'Database unavailable.' });
    render(<CustomFormulaLibrary />);

    expect(await screen.findByRole('alert')).toHaveTextContent('Database unavailable.');
    expect(screen.getByRole('button', { name: 'Add formula' })).toBeInTheDocument();
  });
});
