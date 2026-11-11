import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2CustomUnit } from '../api';
import { CustomUnitLibrary } from './CustomUnitLibrary';

vi.mock('../api', () => ({
  listV2CustomUnits: vi.fn(),
  saveV2CustomUnit: vi.fn(),
  deleteV2CustomUnit: vi.fn(),
}));

import * as api from '../api';

const mockedList = vi.mocked(api.listV2CustomUnits);
const mockedDelete = vi.mocked(api.deleteV2CustomUnit);

function unit(overrides: Partial<V2CustomUnit> = {}): V2CustomUnit {
  return {
    id: 1,
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

describe('CustomUnitLibrary', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedList.mockResolvedValue([]);
    mockedDelete.mockResolvedValue(undefined);
  });

  it('shows an empty state and an add action when no units exist', async () => {
    render(<CustomUnitLibrary />);

    expect(await screen.findByText('No custom units yet')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add your first unit' })).toBeInTheDocument();
  });

  it('filters by search, physical quantity and SI classification', async () => {
    const user = userEvent.setup();
    mockedList.mockResolvedValue([
      unit(),
      unit({
        id: 2,
        name: 'Smoot',
        symbol: 'smoot',
        quantity: 'Length',
        dimension: '',
        conversionKind: 'contextual',
        conversionFactor: '',
        conversionReferenceUnit: '',
        conversionNote: 'Defined relative to the Harvard Bridge.',
        description: 'A humorous non-SI length unit.',
      }),
      unit({
        id: 3,
        name: 'Joule-second',
        symbol: 'J s',
        quantity: 'Action',
        dimension: 'M L^2 T^-1',
        siClassification: 'si_derived',
        conversionKind: 'none',
        conversionFactor: '',
        conversionReferenceUnit: '',
        description: 'SI derived unit of action.',
      }),
    ]);
    render(<CustomUnitLibrary />);

    await screen.findByText('Foot');
    expect(screen.getByText('3 units')).toBeInTheDocument();

    await user.type(screen.getByLabelText('Search my units'), 'smoot');
    expect(screen.getByText('1 unit')).toBeInTheDocument();
    expect(screen.queryByText('Foot')).not.toBeInTheDocument();

    await user.clear(screen.getByLabelText('Search my units'));
    await user.selectOptions(screen.getByLabelText('Unit SI classification filter'), 'si_derived');
    expect(screen.getByText('Joule-second')).toBeInTheDocument();
    expect(screen.queryByText('Foot')).not.toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText('Unit quantity filter'), 'Length');
    expect(screen.getByText('No custom units match these filters')).toBeInTheDocument();
  });

  it('opens a detail view with conversion information', async () => {
    const user = userEvent.setup();
    mockedList.mockResolvedValue([unit()]);
    render(<CustomUnitLibrary />);

    await user.click(await screen.findByRole('button', { name: 'Open custom unit Foot' }));

    const detail = document.querySelector('.v2-reference-detail');
    expect(detail).not.toBeNull();
    expect(
      within(detail as HTMLElement).getByRole('heading', { level: 2, name: 'Foot' }),
    ).toBeInTheDocument();
    expect(within(detail as HTMLElement).getByText('0.3048')).toBeInTheDocument();
    expect(within(detail as HTMLElement).getByText('1 ft = 0.3048 m')).toBeInTheDocument();
    expect(
      within(detail as HTMLElement).getByText(/no automatic unit conversion is performed/i),
    ).toBeInTheDocument();
  });

  it('confirms deletion before removing a custom unit', async () => {
    const user = userEvent.setup();
    mockedList.mockResolvedValue([unit()]);
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);
    try {
      render(<CustomUnitLibrary />);
      await user.click(await screen.findByRole('button', { name: 'Open custom unit Foot' }));
      await user.click(screen.getByRole('button', { name: 'Delete unit' }));

      await waitFor(() => expect(mockedDelete).toHaveBeenCalledWith(1));
      expect(confirm).toHaveBeenCalledWith('Delete the custom unit "Foot"? This cannot be undone.');
      expect(await screen.findByText('No custom units yet')).toBeInTheDocument();
    } finally {
      confirm.mockRestore();
    }
  });

  it('reports a load failure', async () => {
    mockedList.mockRejectedValue({ code: 'database_error', message: 'Database unavailable.' });
    render(<CustomUnitLibrary />);

    expect(await screen.findByRole('alert')).toHaveTextContent('Database unavailable.');
  });
});
