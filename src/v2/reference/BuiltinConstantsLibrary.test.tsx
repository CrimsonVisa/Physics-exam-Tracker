import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2BuiltinConstant, V2BuiltinConstantCatalog } from '../api';
import { BuiltinConstantsLibrary } from './BuiltinConstantsLibrary';

vi.mock('../api', () => ({
  getV2BuiltinConstants: vi.fn(),
}));

import * as api from '../api';

const mockedGet = vi.mocked(api.getV2BuiltinConstants);

function entry(overrides: Partial<V2BuiltinConstant> = {}): V2BuiltinConstant {
  return {
    id: 1,
    stableKey: 'codata-2022-speed-of-light',
    name: 'Speed of light in vacuum',
    symbol: 'c',
    valueText: '299 792 458',
    unit: 'm s^-1',
    uncertaintyText: '(exact)',
    precisionText: 'exact',
    isExact: true,
    category: 'Fundamental',
    description: 'The speed of electromagnetic radiation in vacuum.',
    notes: '',
    sourceLocator: 'NIST CODATA 2022, speed of light in vacuum',
    sourceReference: 'CODATA 2022, NIST SP 961 (May 2024), NIST database version 9.0',
    sourceUrl: 'https://physics.nist.gov/cuu/Constants/Table/allascii.txt',
    displayOrder: 1,
    ...overrides,
  };
}

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
    constants: [entry()],
  };
}

describe('BuiltinConstantsLibrary', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedGet.mockResolvedValue(catalog());
  });

  it('renders the immutable catalogue and source edition', async () => {
    render(<BuiltinConstantsLibrary />);

    expect(await screen.findByText('Speed of light in vacuum')).toBeInTheDocument();
    expect(screen.getByText('1 constant')).toBeInTheDocument();
    expect(screen.getAllByText(/CODATA 2022/).length).toBeGreaterThan(0);
    expect(screen.getByText('Built-in')).toBeInTheDocument();
  });

  it('opens a read-only detail with exact value, uncertainty and source locator', async () => {
    const user = userEvent.setup();
    render(<BuiltinConstantsLibrary />);

    await user.click(
      await screen.findByRole('button', {
        name: 'Open built-in constant Speed of light in vacuum',
      }),
    );

    expect(
      screen.getByRole('heading', { level: 2, name: 'Speed of light in vacuum' }),
    ).toBeInTheDocument();
    expect(screen.getAllByText('299 792 458').length).toBeGreaterThan(0);
    expect(screen.getAllByText('(exact)').length).toBeGreaterThan(0);
    expect(screen.getByText('NIST CODATA 2022, speed of light in vacuum')).toBeInTheDocument();
    expect(screen.getByText(/CODATA 2022, NIST SP 961/)).toBeInTheDocument();
    expect(screen.getByText('Read-only')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Edit/ })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Delete/ })).not.toBeInTheDocument();
  });

  it('filters by category and search', async () => {
    const user = userEvent.setup();
    mockedGet.mockResolvedValue({
      ...catalog(),
      constants: [
        entry(),
        entry({
          id: 2,
          stableKey: 'codata-2022-newtonian-gravitation',
          name: 'Newtonian constant of gravitation',
          symbol: 'G',
          valueText: '6.674 30 e-11',
          unit: 'm^3 kg^-1 s^-2',
          uncertaintyText: '0.000 15 e-11',
          precisionText: 'CODATA 2022 published precision',
          isExact: false,
          displayOrder: 2,
        }),
      ],
    });
    render(<BuiltinConstantsLibrary />);

    await screen.findByText('Speed of light in vacuum');
    await user.type(screen.getByLabelText('Search built-in constants'), 'gravitation');
    expect(screen.getByText('Newtonian constant of gravitation')).toBeInTheDocument();
    expect(screen.queryByText('Speed of light in vacuum')).not.toBeInTheDocument();

    await user.clear(screen.getByLabelText('Search built-in constants'));
    await user.selectOptions(
      screen.getByLabelText('Built-in constant category filter'),
      'Fundamental',
    );
    expect(screen.getByText('2 constants')).toBeInTheDocument();
  });

  it('reports a catalogue load failure', async () => {
    mockedGet.mockRejectedValue({ code: 'schema_integrity_error', message: 'Catalogue invalid.' });
    render(<BuiltinConstantsLibrary />);

    expect(await screen.findByRole('alert')).toHaveTextContent('Catalogue invalid.');
  });
});
