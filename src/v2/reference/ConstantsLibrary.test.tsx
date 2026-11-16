import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2BuiltinConstantCatalog, V2CustomConstant } from '../api';
import { ConstantsLibrary } from './ConstantsLibrary';

vi.mock('../api', () => ({
  getV2BuiltinConstants: vi.fn(),
  listV2CustomConstants: vi.fn(),
  saveV2CustomConstant: vi.fn(),
  deleteV2CustomConstant: vi.fn(),
}));

import * as api from '../api';

const mockedBuiltin = vi.mocked(api.getV2BuiltinConstants);
const mockedList = vi.mocked(api.listV2CustomConstants);
const mockedSave = vi.mocked(api.saveV2CustomConstant);
const mockedDelete = vi.mocked(api.deleteV2CustomConstant);

function catalog(): V2BuiltinConstantCatalog {
  return {
    versionLabel: 'CODATA 2022',
    editionYear: 2022,
    publishedOn: '2024-05-20',
    retrievedOn: '2026-11-10',
    publisher: 'CODATA / NIST',
    sourceUrl: 'https://physics.nist.gov/cuu/Constants/',
    citation: 'CODATA 2022 citation',
    seedSha256: 'a'.repeat(64),
    constants: [],
  };
}

function constant(overrides: Partial<V2CustomConstant> = {}): V2CustomConstant {
  return {
    id: 1,
    isBuiltin: false,
    name: 'Planck constant (local)',
    symbol: 'h',
    valueText: '6.62607015e-34',
    unit: 'J s',
    category: 'Fundamental',
    uncertaintyText: 'exact',
    precisionText: '9 significant figures',
    source: '',
    description: 'A locally recorded value for reference.',
    notes: '',
    createdAt: '2026-11-10T00:00:00.000Z',
    updatedAt: '2026-11-10T00:00:00.000Z',
    ...overrides,
  };
}

describe('ConstantsLibrary', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedBuiltin.mockResolvedValue(catalog());
    mockedList.mockResolvedValue([]);
    mockedDelete.mockResolvedValue(undefined);
  });

  it('shows an empty state with an add action', async () => {
    const user = userEvent.setup();
    render(<ConstantsLibrary />);
    await user.click(screen.getByRole('tab', { name: 'My Constants' }));

    expect(await screen.findByText('No custom constants yet')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add your first constant' })).toBeInTheDocument();
  });

  it('creates a constant with its exact value text and confirms the save', async () => {
    const user = userEvent.setup();
    const valueText = '6.62607015×10⁻³⁴';
    const saved = constant({
      id: 8,
      name: 'Local Planck',
      symbol: 'h_local',
      valueText,
    });
    mockedSave.mockResolvedValue(saved);
    render(<ConstantsLibrary />);
    await user.click(screen.getByRole('tab', { name: 'My Constants' }));
    await user.click(await screen.findByRole('button', { name: 'Add your first constant' }));

    const dialog = screen.getByRole('dialog');
    await user.type(within(dialog).getByLabelText('Full name'), 'Local Planck');
    await user.type(within(dialog).getByLabelText('Symbol'), 'h_local');
    await user.clear(within(dialog).getByLabelText('Category'));
    await user.type(within(dialog).getByLabelText('Category'), 'Fundamental');
    await user.type(within(dialog).getByLabelText(/Numerical value/), valueText);
    await user.type(within(dialog).getByLabelText('Description'), 'A local Planck constant.');
    await user.click(within(dialog).getByRole('button', { name: 'Add constant' }));

    await waitFor(() =>
      expect(mockedSave).toHaveBeenCalledWith(
        expect.objectContaining({
          id: null,
          name: 'Local Planck',
          symbol: 'h_local',
          valueText,
          category: 'Fundamental',
          description: 'A local Planck constant.',
        }),
      ),
    );
    expect(await screen.findByText('Constant "Local Planck" saved.')).toBeInTheDocument();
    const detail = document.querySelector('.v2-reference-detail') as HTMLElement;
    expect(within(detail).getAllByText(valueText).length).toBeGreaterThan(0);
  });

  it('reloads a persisted constant from storage on a fresh mount', async () => {
    const user = userEvent.setup();
    mockedList.mockResolvedValue([constant({ id: 12, name: 'Persisted constant' })]);
    render(<ConstantsLibrary />);
    await user.click(screen.getByRole('tab', { name: 'My Constants' }));

    expect(
      await screen.findByRole('button', { name: 'Open constant Persisted constant' }),
    ).toBeInTheDocument();
  });

  it('searches and filters constants by category', async () => {
    const user = userEvent.setup();
    mockedList.mockResolvedValue([
      constant(),
      constant({
        id: 2,
        name: 'Elementary charge (local)',
        symbol: 'e',
        valueText: '1.602176634e-19',
        unit: 'C',
        category: 'Electromagnetic',
        description: 'A locally recorded charge value.',
      }),
    ]);
    render(<ConstantsLibrary />);
    await user.click(screen.getByRole('tab', { name: 'My Constants' }));

    await screen.findByText('Planck constant (local)');
    expect(screen.getByText('2 constants')).toBeInTheDocument();

    await user.type(screen.getByLabelText('Search constants'), 'charge');
    expect(screen.getByText('1 constant')).toBeInTheDocument();

    await user.clear(screen.getByLabelText('Search constants'));
    await user.selectOptions(screen.getByLabelText('Constant category filter'), 'Fundamental');
    expect(screen.getByText('Planck constant (local)')).toBeInTheDocument();
    expect(screen.queryByText('Elementary charge (local)')).not.toBeInTheDocument();
  });

  it('searches raw LaTeX source and renders mathematical card content', async () => {
    const user = userEvent.setup();
    mockedList.mockResolvedValue([
      constant({
        name: 'Reduced Planck',
        symbol: String.raw`\hbar`,
        description: String.raw`Energy relations $E=mc^2$ and $E=\hbar\omega$.`,
      }),
    ]);
    render(<ConstantsLibrary />);
    await user.click(screen.getByRole('tab', { name: 'My Constants' }));

    expect(
      await screen.findByRole('button', { name: 'Open constant Reduced Planck' }),
    ).toBeInTheDocument();
    await waitFor(() => expect(document.querySelector('.v2-reference-card .katex')).not.toBeNull());

    await user.type(screen.getByLabelText('Search constants'), 'hbar');
    expect(
      screen.getByRole('button', { name: 'Open constant Reduced Planck' }),
    ).toBeInTheDocument();

    await user.clear(screen.getByLabelText('Search constants'));
    await user.type(screen.getByLabelText('Search constants'), 'E=mc');
    expect(
      screen.getByRole('button', { name: 'Open constant Reduced Planck' }),
    ).toBeInTheDocument();
  });

  it('edits the selected custom constant and preserves its exact value', async () => {
    const user = userEvent.setup();
    const valueText = '6.62607015×10⁻³⁴';
    mockedList.mockResolvedValue([constant({ valueText })]);
    mockedSave.mockResolvedValue(constant({ name: 'Edited Planck', valueText }));
    render(<ConstantsLibrary />);
    await user.click(screen.getByRole('tab', { name: 'My Constants' }));
    await user.click(
      await screen.findByRole('button', { name: 'Open constant Planck constant (local)' }),
    );
    await user.click(screen.getByRole('button', { name: 'Edit constant' }));

    const dialog = screen.getByRole('dialog');
    await user.clear(within(dialog).getByLabelText('Full name'));
    await user.type(within(dialog).getByLabelText('Full name'), 'Edited Planck');
    await user.click(within(dialog).getByRole('button', { name: 'Save changes' }));

    await waitFor(() =>
      expect(mockedSave).toHaveBeenCalledWith(
        expect.objectContaining({ id: 1, name: 'Edited Planck', valueText }),
      ),
    );
    expect(await screen.findByText('Constant "Edited Planck" saved.')).toBeInTheDocument();
    const detail = document.querySelector('.v2-reference-detail') as HTMLElement;
    expect(within(detail).getAllByText(valueText).length).toBeGreaterThan(0);
  });

  it('opens a detail view that preserves the entered value and precision', async () => {
    const user = userEvent.setup();
    mockedList.mockResolvedValue([
      constant({
        valueText: '6.62607015×10^-34',
        uncertaintyText: '0.00000001',
        precisionText: '9 significant figures',
      }),
    ]);
    render(<ConstantsLibrary />);
    await user.click(screen.getByRole('tab', { name: 'My Constants' }));

    await user.click(
      await screen.findByRole('button', {
        name: 'Open constant Planck constant (local)',
      }),
    );

    const detail = document.querySelector('.v2-reference-detail');
    expect(detail).not.toBeNull();
    expect(
      within(detail as HTMLElement).getByRole('heading', {
        level: 2,
        name: 'Planck constant (local)',
      }),
    ).toBeInTheDocument();
    expect(within(detail as HTMLElement).getAllByText('6.62607015×10^-34').length).toBeGreaterThan(
      0,
    );
    expect(within(detail as HTMLElement).getByText('9 significant figures')).toBeInTheDocument();
    expect(within(detail as HTMLElement).getByText('0.00000001')).toBeInTheDocument();
    expect(within(detail as HTMLElement).getByText('User-defined')).toBeInTheDocument();
  });

  it('confirms deletion before removing a custom constant', async () => {
    const user = userEvent.setup();
    mockedList.mockResolvedValue([constant()]);
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);
    try {
      render(<ConstantsLibrary />);
      await user.click(screen.getByRole('tab', { name: 'My Constants' }));
      await user.click(
        await screen.findByRole('button', {
          name: 'Open constant Planck constant (local)',
        }),
      );
      await user.click(screen.getByRole('button', { name: 'Delete constant' }));

      await waitFor(() => expect(mockedDelete).toHaveBeenCalledWith(1));
      expect(confirm).toHaveBeenCalledWith(
        'Delete the custom constant "Planck constant (local)"? This cannot be undone.',
      );
      expect(await screen.findByText('No custom constants yet')).toBeInTheDocument();
    } finally {
      confirm.mockRestore();
    }
  });

  it('reports a load failure', async () => {
    const user = userEvent.setup();
    mockedList.mockRejectedValue({ code: 'database_error', message: 'Database unavailable.' });
    render(<ConstantsLibrary />);
    await user.click(screen.getByRole('tab', { name: 'My Constants' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Database unavailable.');
  });
});
