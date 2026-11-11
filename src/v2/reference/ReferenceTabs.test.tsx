import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2ExamSummary } from '../api';
import { FormulaReferenceScreen } from '../formulas/FormulaReferenceScreen';
import { TermReferenceScreen } from '../terms/TermReferenceScreen';

vi.mock('../api', () => ({
  getV2BuiltinConstants: vi.fn(),
  listV2CustomFormulas: vi.fn(),
  saveV2CustomFormula: vi.fn(),
  deleteV2CustomFormula: vi.fn(),
  listV2CustomUnits: vi.fn(),
  saveV2CustomUnit: vi.fn(),
  deleteV2CustomUnit: vi.fn(),
  listV2CustomConstants: vi.fn(),
  saveV2CustomConstant: vi.fn(),
  deleteV2CustomConstant: vi.fn(),
}));

import * as api from '../api';

const mockedFormulas = vi.mocked(api.listV2CustomFormulas);
const mockedUnits = vi.mocked(api.listV2CustomUnits);
const mockedConstants = vi.mocked(api.listV2CustomConstants);
const mockedBuiltin = vi.mocked(api.getV2BuiltinConstants);

const EXAMS: V2ExamSummary[] = [
  {
    id: 1,
    code: 'csir-net-physics',
    name: 'CSIR-UGC NET Physical Sciences',
    shortName: 'CSIR-NET',
    authority: 'CSIR-HRDG',
    officialUrl: 'https://example.com/csir',
    displayOrder: 1,
    isActive: true,
    examDate: null,
  },
  {
    id: 2,
    code: 'gate-physics',
    name: 'GATE Physics (PH)',
    shortName: 'GATE PH',
    authority: 'IIT Madras',
    officialUrl: 'https://example.com/gate',
    displayOrder: 2,
    isActive: true,
    examDate: null,
  },
  {
    id: 3,
    code: 'tifr-gs-physics',
    name: 'TIFR Graduate School Physics',
    shortName: 'TIFR GS',
    authority: 'TIFR',
    officialUrl: 'https://example.com/tifr',
    displayOrder: 3,
    isActive: true,
    examDate: null,
  },
  {
    id: 4,
    code: 'jest-physics',
    name: 'JEST Physics',
    shortName: 'JEST',
    authority: 'JEST Consortium',
    officialUrl: 'https://example.com/jest',
    displayOrder: 4,
    isActive: true,
    examDate: null,
  },
];

describe('custom reference tabs', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedFormulas.mockResolvedValue([]);
    mockedUnits.mockResolvedValue([]);
    mockedConstants.mockResolvedValue([]);
    mockedBuiltin.mockResolvedValue({
      versionLabel: 'CODATA 2022',
      editionYear: 2022,
      publishedOn: '2024-05-20',
      retrievedOn: '2026-11-10',
      publisher: 'CODATA / NIST',
      sourceUrl: 'https://physics.nist.gov/cuu/Constants/',
      citation: 'CODATA 2022 citation',
      seedSha256: 'a'.repeat(64),
      constants: [],
    });
  });

  it('loads formula libraries only when their tab is opened', async () => {
    const user = userEvent.setup();
    render(<FormulaReferenceScreen exams={EXAMS} />);

    expect(screen.getByText('47 formulas')).toBeInTheDocument();
    expect(mockedFormulas).not.toHaveBeenCalled();
    expect(mockedConstants).not.toHaveBeenCalled();

    await user.click(screen.getByRole('tab', { name: 'My formulas' }));

    expect(mockedFormulas).toHaveBeenCalledTimes(1);
    expect(await screen.findByText('No custom formulas yet')).toBeInTheDocument();

    await user.click(screen.getByRole('tab', { name: 'Constants' }));
    await user.click(await screen.findByRole('tab', { name: 'My Constants' }));

    expect(mockedConstants).toHaveBeenCalledTimes(1);
    expect(await screen.findByText('No custom constants yet')).toBeInTheDocument();

    await user.click(screen.getByRole('tab', { name: /Built-in formulas/ }));

    expect(screen.getByText('47 formulas')).toBeInTheDocument();
  });

  it('loads unit and constant libraries only when their tab is opened', async () => {
    const user = userEvent.setup();
    render(<TermReferenceScreen exams={EXAMS} />);

    expect(screen.getByText('123 terms')).toBeInTheDocument();
    expect(mockedUnits).not.toHaveBeenCalled();
    expect(mockedConstants).not.toHaveBeenCalled();

    await user.click(screen.getByRole('tab', { name: 'My Units' }));

    expect(mockedUnits).toHaveBeenCalledTimes(1);
    expect(await screen.findByText('No custom units yet')).toBeInTheDocument();

    await user.click(screen.getByRole('tab', { name: 'Constants' }));
    await user.click(await screen.findByRole('tab', { name: 'My Constants' }));

    expect(mockedConstants).toHaveBeenCalledTimes(1);
    expect(await screen.findByText('No custom constants yet')).toBeInTheDocument();

    await user.click(screen.getByRole('tab', { name: /Terms & Units/ }));

    expect(screen.getByText('123 terms')).toBeInTheDocument();
  });

  it('supports arrow-key tab navigation', async () => {
    const user = userEvent.setup();
    render(<FormulaReferenceScreen exams={EXAMS} />);

    const builtinTab = screen.getByRole('tab', { name: /Built-in formulas/ });
    builtinTab.focus();
    expect(builtinTab).toHaveAttribute('aria-selected', 'true');

    await user.keyboard('{ArrowRight}');

    const customTab = screen.getByRole('tab', { name: 'My formulas' });
    expect(customTab).toHaveAttribute('aria-selected', 'true');
    expect(document.activeElement).toBe(customTab);

    await user.keyboard('{ArrowLeft}');

    expect(screen.getByRole('tab', { name: /Built-in formulas/ })).toHaveAttribute(
      'aria-selected',
      'true',
    );
  });
});
