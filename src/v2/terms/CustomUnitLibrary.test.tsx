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
const mockedSave = vi.mocked(api.saveV2CustomUnit);
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

  it('creates a unit with conversion metadata and confirms the save', async () => {
    const user = userEvent.setup();
    const saved = unit({
      id: 9,
      name: 'Pace',
      symbol: 'pace',
      conversionFactor: '0.762',
      description: 'A walking pace unit.',
    });
    mockedSave.mockResolvedValue(saved);
    render(<CustomUnitLibrary />);

    await user.click(await screen.findByRole('button', { name: 'Add your first unit' }));
    const dialog = screen.getByRole('dialog');
    await user.type(within(dialog).getByLabelText('Unit name'), 'Pace');
    await user.type(within(dialog).getByLabelText(/^Symbol/), 'pace');
    await user.type(within(dialog).getByLabelText('Physical quantity'), 'Length');
    await user.type(within(dialog).getByLabelText('Description'), 'A walking pace unit.');
    await user.selectOptions(within(dialog).getByLabelText(/Conversion kind/), 'scalar');
    await user.type(within(dialog).getByLabelText(/Conversion factor/), '0.762');
    await user.type(within(dialog).getByLabelText('Reference unit'), 'm');
    await user.click(within(dialog).getByRole('button', { name: 'Add unit' }));

    await waitFor(() =>
      expect(mockedSave).toHaveBeenCalledWith(
        expect.objectContaining({
          id: null,
          name: 'Pace',
          symbol: 'pace',
          quantity: 'Length',
          siClassification: 'non_si',
          conversionKind: 'scalar',
          conversionFactor: '0.762',
          conversionReferenceUnit: 'm',
          description: 'A walking pace unit.',
        }),
      ),
    );
    expect(await screen.findByText('Unit "Pace" saved.')).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 2, name: 'Pace' })).toBeInTheDocument();
  });

  it('reloads a persisted unit from storage on a fresh mount', async () => {
    mockedList.mockResolvedValue([unit({ id: 10, name: 'Persisted unit', symbol: 'pu' })]);
    render(<CustomUnitLibrary />);

    expect(
      await screen.findByRole('button', { name: 'Open custom unit Persisted unit' }),
    ).toBeInTheDocument();
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

  it('retrieves a custom unit whose name and symbol match the built-in catalogue', async () => {
    const user = userEvent.setup();
    mockedList.mockResolvedValue([unit({ id: 16, name: 'Metre', symbol: 'm' })]);
    render(<CustomUnitLibrary />);

    await user.click(await screen.findByRole('button', { name: 'Open custom unit Metre' }));

    const detail = document.querySelector('.v2-reference-detail') as HTMLElement;
    expect(within(detail).getByRole('heading', { level: 2, name: 'Metre' })).toBeInTheDocument();
    expect(within(detail).getByText('User-defined')).toBeInTheDocument();
    expect(within(detail).getAllByText('m').length).toBeGreaterThan(0);
  });

  it('edits the selected custom unit without changing its record id', async () => {
    const user = userEvent.setup();
    mockedList.mockResolvedValue([unit()]);
    mockedSave.mockResolvedValue(unit({ name: 'Edited foot', conversionFactor: '0.3049' }));
    render(<CustomUnitLibrary />);

    await user.click(await screen.findByRole('button', { name: 'Open custom unit Foot' }));
    await user.click(screen.getByRole('button', { name: 'Edit unit' }));
    const dialog = screen.getByRole('dialog');
    await user.clear(within(dialog).getByLabelText('Unit name'));
    await user.type(within(dialog).getByLabelText('Unit name'), 'Edited foot');
    await user.clear(within(dialog).getByLabelText(/Conversion factor/));
    await user.type(within(dialog).getByLabelText(/Conversion factor/), '0.3049');
    await user.click(within(dialog).getByRole('button', { name: 'Save changes' }));

    await waitFor(() =>
      expect(mockedSave).toHaveBeenCalledWith(
        expect.objectContaining({ id: 1, name: 'Edited foot', conversionFactor: '0.3049' }),
      ),
    );
    expect(await screen.findByText('Unit "Edited foot" saved.')).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 2, name: 'Edited foot' })).toBeInTheDocument();
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
    const conversion = (detail as HTMLElement).querySelector('[data-unit-conversion="scalar"]');
    expect(conversion).not.toBeNull();
    expect(conversion).toHaveTextContent('0.3048');
    expect(conversion?.querySelector('.katex')).not.toBeNull();
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
