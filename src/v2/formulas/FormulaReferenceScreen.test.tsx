import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { V2ExamSummary } from '../api';
import { FormulaReferenceScreen } from './FormulaReferenceScreen';

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
    isActive: false,
    examDate: null,
  },
];

function renderScreen(props: Partial<Parameters<typeof FormulaReferenceScreen>[0]> = {}) {
  return render(<FormulaReferenceScreen exams={EXAMS} {...props} />);
}

describe('FormulaReferenceScreen', () => {
  it('renders the combined formula library with all subjects', async () => {
    renderScreen();

    expect(
      screen.getByRole('heading', { level: 1, name: 'Physics Formula Reference' }),
    ).toBeInTheDocument();
    expect(screen.getByText('47 formulas')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /All subjects/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Quantum Mechanics/ })).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Open formula Maxwell-Faraday law (differential form)' }),
    ).toBeInTheDocument();
  });

  it('searches titles and Unicode symbols', async () => {
    const user = userEvent.setup();
    renderScreen();

    const search = screen.getByLabelText('Search formulas');
    await user.type(search, 'Maxwell-Faraday law (differential form)');
    expect(screen.getByText('1 formula')).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Open formula Maxwell-Faraday law (differential form)' }),
    ).toBeInTheDocument();

    await user.clear(search);
    await user.type(search, 'ω');
    expect(
      screen.getByRole('button', { name: 'Open formula Underdamped oscillator frequency' }),
    ).toBeInTheDocument();
  });

  it('filters by examination and excludes unsupported JEST subjects', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.selectOptions(screen.getByLabelText('Examination filter'), 'jest-physics');
    expect(screen.getByText('38 formulas')).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Open formula Maxwell-Faraday law (differential form)' }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: /Open formula Rydberg formula/ }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: /Open formula Drude electrical conductivity/ }),
    ).not.toBeInTheDocument();
  });

  it('filters by subject and topic, then clears filters', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(screen.getByRole('button', { name: /Quantum Mechanics/ }));
    expect(screen.getByText('4 formulas')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /All subjects/ }));
    await user.selectOptions(screen.getByLabelText('Topic filter'), 'Diffraction');
    expect(screen.getByText('2 formulas')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Clear filters' }));
    expect(screen.getByText('47 formulas')).toBeInTheDocument();
    expect(screen.getByLabelText('Examination filter')).toHaveValue('all');
    expect(screen.getByLabelText('Topic filter')).toHaveValue('all');
  });

  it('opens a formula detail with symbols, conditions, syllabus links and sources', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(
      screen.getByRole('button', { name: 'Open formula Maxwell-Faraday law (differential form)' }),
    );

    expect(
      screen.getByRole('heading', {
        level: 1,
        name: 'Maxwell-Faraday law (differential form)',
      }),
    ).toBeInTheDocument();
    expect(screen.getByText('Physical meaning')).toBeInTheDocument();
    expect(screen.getByText('Symbols, units and dimensions')).toBeInTheDocument();
    expect(screen.getByText('Conditions and assumptions')).toBeInTheDocument();
    expect(screen.getByText('Examination and syllabus links')).toBeInTheDocument();
    expect(screen.getByText('Sources and provenance')).toBeInTheDocument();
    expect(screen.getByText('chapter checked')).toBeInTheDocument();
    expect(screen.getByText('unverified locator')).toBeInTheDocument();
    expect(
      screen.getByText(/not that the exact formula content was independently verified/i),
    ).toBeInTheDocument();
    await waitFor(() =>
      expect(document.querySelector('.v2-formula-equation-display .katex')).not.toBeNull(),
    );

    await user.click(screen.getByRole('button', { name: /Back to formulas/ }));
    expect(
      await screen.findByRole('heading', { level: 1, name: 'Physics Formula Reference' }),
    ).toBeInTheDocument();
  });

  it('opens a formula detail directly from a formula deep link', () => {
    renderScreen({ initialFormulaId: 'optics-malus-law' });

    expect(screen.getByRole('heading', { level: 1, name: 'Malus law' })).toBeInTheDocument();
    expect(screen.getByText('Physical meaning')).toBeInTheDocument();
  });

  it('navigates between related formulas', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(
      screen.getByRole('button', { name: "Open formula Newton's second law (momentum form)" }),
    );
    await user.click(screen.getByRole('button', { name: 'Work-energy theorem' }));
    expect(
      screen.getByRole('heading', { level: 1, name: 'Work-energy theorem' }),
    ).toBeInTheDocument();
  });

  it('opens a linked syllabus node through the stable node code', async () => {
    const user = userEvent.setup();
    const onOpenSyllabus = vi.fn();
    renderScreen({ onOpenSyllabus });

    await user.click(
      screen.getByRole('button', { name: 'Open formula Maxwell-Faraday law (differential form)' }),
    );
    await user.click(
      screen.getByRole('button', {
        name: 'Open syllabus topic Maxwell equations in free space and linear isotropic media',
      }),
    );

    expect(onOpenSyllabus).toHaveBeenCalledWith(
      'csir-net-physics',
      'csir-part-a-core-electromagnetic-theory-t004-maxwell-equations-in-free-space-and-linear-isotrop',
      'Maxwell equations in free space and linear isotropic media',
    );
  });

  it('opens related terminology from a formula detail', async () => {
    const user = userEvent.setup();
    const onOpenTerms = vi.fn();
    renderScreen({ onOpenTerms });

    await user.click(
      screen.getByRole('button', { name: 'Open formula Maxwell-Faraday law (differential form)' }),
    );
    await user.click(screen.getByRole('button', { name: 'Browse related terms and units' }));

    expect(onOpenTerms).toHaveBeenCalledWith(
      'electromagnetism-maxwell-faraday',
      'Maxwell-Faraday law (differential form)',
    );
  });

  it('filters by a syllabus node and clears the syllabus filter', async () => {
    const user = userEvent.setup();
    renderScreen({
      initialNodeCode: 'csir-part-a-core-classical-mechanics-t001-newton-s-laws',
      initialNodeTitle: "Newton's laws",
    });

    expect(screen.getByText(/Showing formulas linked to/)).toBeInTheDocument();
    expect(screen.getByText("Newton's laws")).toBeInTheDocument();
    expect(screen.getByText('1 formula')).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: "Open formula Newton's second law (momentum form)" }),
    ).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Clear syllabus filter' }));
    expect(screen.getByText('47 formulas')).toBeInTheDocument();
  });

  it('preselects an examination and marks it inactive', () => {
    renderScreen({ initialExamCode: 'jest-physics' });

    expect(screen.getByLabelText('Examination filter')).toHaveValue('jest-physics');
    expect(within(screen.getByRole('note')).getByText('JEST')).toBeInTheDocument();
    expect(screen.getByText('Inactive examination')).toBeInTheDocument();
    expect(screen.getByText('38 formulas')).toBeInTheDocument();
  });

  it('shows an empty state for an unlinked syllabus node', () => {
    renderScreen({
      initialNodeCode: 'missing-node',
      initialNodeTitle: 'Missing topic',
    });

    const emptyState = screen
      .getByText('No formulas match these filters')
      .closest('.v2-formula-empty');
    expect(emptyState).toBeInstanceOf(HTMLElement);
    expect(
      within(emptyState as HTMLElement).getByRole('button', { name: 'Clear filters' }),
    ).toBeInTheDocument();
  });

  it('clears a stale syllabus-node filter when the examination changes', async () => {
    const user = userEvent.setup();
    renderScreen({
      initialExamCode: 'csir-net-physics',
      initialNodeCode: 'csir-part-a-core-classical-mechanics-t001-newton-s-laws',
      initialNodeTitle: "Newton's laws",
    });

    expect(screen.getByText('1 formula')).toBeInTheDocument();
    await user.selectOptions(screen.getByLabelText('Examination filter'), 'gate-physics');

    expect(screen.getByText('47 formulas')).toBeInTheDocument();
    expect(screen.queryByText(/Showing formulas linked to/)).not.toBeInTheDocument();
  });

  it('falls back to all examinations for invalid or unavailable exam codes', () => {
    const { unmount } = renderScreen({ initialExamCode: 'not-an-exam' });
    expect(screen.getByLabelText('Examination filter')).toHaveValue('all');
    expect(screen.getByText('47 formulas')).toBeInTheDocument();
    unmount();

    renderScreen({
      exams: EXAMS.filter((exam) => exam.code !== 'gate-physics'),
      initialExamCode: 'gate-physics',
    });
    expect(screen.getByLabelText('Examination filter')).toHaveValue('all');
    expect(screen.getByText('47 formulas')).toBeInTheDocument();
  });

  it('labels uncertain TIFR relevance on formula cards', () => {
    renderScreen();

    const rydbergCard = screen.getByRole('button', {
      name: 'Open formula Rydberg formula for hydrogen-like atoms',
    });
    expect(within(rydbergCard).getByText('TIFR GS · uncertain')).toBeInTheDocument();
    expect(within(rydbergCard).getByText('TIFR GS · uncertain')).toHaveAttribute(
      'data-relevance',
      'uncertain',
    );
  });

  it('exposes accessible filter and result semantics', () => {
    renderScreen();

    expect(screen.getByLabelText('Search formulas')).toBeInTheDocument();
    expect(screen.getByLabelText('Examination filter')).toBeInTheDocument();
    expect(screen.getByLabelText('Topic filter')).toBeInTheDocument();
    expect(screen.getByRole('navigation', { name: 'Filter by subject' })).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Formula results' })).toBeInTheDocument();
    expect(screen.getByText('47 formulas')).toHaveAttribute('aria-live', 'polite');
  });
});
