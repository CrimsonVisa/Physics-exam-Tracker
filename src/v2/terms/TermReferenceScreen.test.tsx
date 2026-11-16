import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { V2ExamSummary } from '../api';
import { TermReferenceScreen } from './TermReferenceScreen';

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

function renderScreen(props: Partial<Parameters<typeof TermReferenceScreen>[0]> = {}) {
  return render(<TermReferenceScreen exams={EXAMS} {...props} />);
}

describe('TermReferenceScreen', () => {
  it('renders the combined 123-term library with all subjects', () => {
    renderScreen();

    expect(
      screen.getByRole('heading', { level: 1, name: 'Physics Terms & SI Units' }),
    ).toBeInTheDocument();
    expect(screen.getByText('123 terms')).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: /Units, Dimensions & Measurement/ }),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open term newton' })).toBeInTheDocument();
  });

  it('keeps constants available under Terms & Units', () => {
    renderScreen();

    expect(screen.getByRole('tab', { name: 'Constants' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'My Units' })).toBeInTheDocument();
  });

  it('searches exact terms, aliases, Unicode symbols, units and dimensions', () => {
    renderScreen();

    const search = screen.getByLabelText('Search terms');
    fireEvent.change(search, { target: { value: 'newton' } });
    expect(screen.getByRole('button', { name: 'Open term newton' })).toBeInTheDocument();

    fireEvent.change(search, { target: { value: 'meter' } });
    expect(screen.getByRole('button', { name: 'Open term metre' })).toBeInTheDocument();

    fireEvent.change(search, { target: { value: '∇' } });
    expect(screen.getByRole('button', { name: 'Open term gradient' })).toBeInTheDocument();

    fireEvent.change(search, { target: { value: 'kg m s^-1' } });
    expect(screen.getByRole('button', { name: 'Open term linear momentum' })).toBeInTheDocument();

    fireEvent.change(search, { target: { value: 'M L T^-2' } });
    expect(screen.getByRole('button', { name: 'Open term newton' })).toBeInTheDocument();

    fireEvent.change(search, { target: { value: 'grad f' } });
    expect(screen.getByRole('button', { name: 'Open term gradient' })).toBeInTheDocument();
  });

  it('keeps subject counts query-aware', () => {
    renderScreen();

    fireEvent.change(screen.getByLabelText('Search terms'), {
      target: { value: 'candela' },
    });

    const unitsButton = screen.getByRole('button', {
      name: /Units, Dimensions & Measurement/,
    });
    expect(within(unitsButton).getByText('1')).toBeInTheDocument();
    expect(screen.getByText('1 term')).toBeInTheDocument();
  });

  it('filters by examination, subject, topic and kind', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.selectOptions(screen.getByLabelText('Examination filter'), 'gate-physics');
    await user.click(screen.getByRole('button', { name: /Units, Dimensions & Measurement/ }));
    await user.selectOptions(screen.getByLabelText('Kind filter'), 'base-unit');
    expect(screen.getByText('7 terms')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open term metre' })).toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText('Topic filter'), 'SI base units');
    expect(screen.getByText('7 terms')).toBeInTheDocument();
  });

  it('opens a detail with definition, symbol, unit, dimensions, related terms and sources', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(screen.getByRole('button', { name: 'Open term linear momentum' }));

    expect(screen.getByRole('heading', { level: 1, name: 'linear momentum' })).toBeInTheDocument();
    expect(screen.getByText('Definition')).toBeInTheDocument();
    expect(screen.getByText('Symbol, unit and dimensions')).toBeInTheDocument();
    expect(screen.getByText('SI unit')).toBeInTheDocument();
    expect(document.querySelectorAll('.v2-terms-symbol').length).toBeGreaterThanOrEqual(2);
    expect(screen.getAllByText('M L T^-1').length).toBeGreaterThan(0);
    expect(screen.getByText('Related terms')).toBeInTheDocument();
    expect(screen.getByText('Related formulas')).toBeInTheDocument();
    expect(screen.getByText('Sources and provenance')).toBeInTheDocument();
    await waitFor(() => expect(document.querySelector('.v2-terms-symbol .katex')).not.toBeNull());
  });

  it('renders notation variants through the mathematical rendering path', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(screen.getByRole('button', { name: 'Open term vector' }));
    await waitFor(() =>
      expect(document.querySelector('.v2-terms-notation-list .katex')).not.toBeNull(),
    );
  });

  it('navigates to a related term and back to the list', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(screen.getByRole('button', { name: 'Open term linear momentum' }));
    await user.click(screen.getByRole('button', { name: 'angular momentum' }));
    expect(screen.getByRole('heading', { level: 1, name: 'angular momentum' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /Back to terms/ }));
    expect(
      await screen.findByRole('heading', { level: 1, name: 'Physics Terms & SI Units' }),
    ).toBeInTheDocument();
  });

  it('opens related formulas and syllabus topics through callbacks', async () => {
    const user = userEvent.setup();
    const onOpenFormula = vi.fn();
    const onOpenSyllabus = vi.fn();
    renderScreen({ onOpenFormula, onOpenSyllabus });

    await user.click(screen.getByRole('button', { name: 'Open term linear momentum' }));
    await user.click(screen.getByRole('button', { name: "Newton's second law (momentum form)" }));
    expect(onOpenFormula).toHaveBeenCalledWith(
      'classical-mechanics-newton-second-law',
      "Newton's second law (momentum form)",
    );

    await user.click(screen.getByRole('button', { name: "Open syllabus topic Newton's laws" }));
    expect(onOpenSyllabus).toHaveBeenCalledWith(
      'csir-net-physics',
      'csir-part-a-core-classical-mechanics-t001-newton-s-laws',
      "Newton's laws",
    );
  });

  it('shows a general-reference entry without invented syllabus links', async () => {
    const user = userEvent.setup();
    renderScreen();

    await user.click(screen.getByRole('button', { name: 'Open term order of magnitude' }));
    expect(screen.getByText('General reference')).toBeInTheDocument();
    expect(screen.getByText(/No defensible examination-syllabus mapping/)).toBeInTheDocument();
  });

  it('clears a stale syllabus-node filter when the examination changes', async () => {
    const user = userEvent.setup();
    renderScreen({
      initialExamCode: 'csir-net-physics',
      initialNodeCode: 'csir-part-a-core-mathematical-methods-of-physics-t001-dimensional-analysis',
      initialNodeTitle: 'Dimensional analysis',
    });

    expect(screen.getByText(/Showing terms linked to/)).toBeInTheDocument();
    await user.selectOptions(screen.getByLabelText('Examination filter'), 'gate-physics');
    expect(screen.queryByText(/Showing terms linked to/)).not.toBeInTheDocument();
    expect(screen.getByLabelText('Examination filter')).toHaveValue('gate-physics');
  });

  it('falls back safely for invalid or unavailable exam codes', () => {
    const { unmount } = renderScreen({ initialExamCode: 'not-an-exam' });
    expect(screen.getByLabelText('Examination filter')).toHaveValue('all');
    expect(screen.getByText('123 terms')).toBeInTheDocument();
    unmount();

    renderScreen({
      exams: EXAMS.filter((exam) => exam.code !== 'gate-physics'),
      initialExamCode: 'gate-physics',
    });
    expect(screen.getByLabelText('Examination filter')).toHaveValue('all');
  });

  it('labels uncertain TIFR relevance and inactive examinations', () => {
    renderScreen();

    const bindingCard = screen.getByRole('button', {
      name: 'Open term nuclear binding energy',
    });
    expect(within(bindingCard).getByText('TIFR GS · uncertain')).toBeInTheDocument();
    expect(within(bindingCard).getByText('TIFR GS · uncertain')).toHaveAttribute(
      'data-relevance',
      'uncertain',
    );

    const momentumCard = screen.getByRole('button', { name: 'Open term linear momentum' });
    expect(within(momentumCard).getByText('JEST')).toBeInTheDocument();
  });

  it('filters by a related formula deep link', () => {
    renderScreen({
      initialFormulaId: 'classical-mechanics-newton-second-law',
      initialFormulaTitle: "Newton's second law (momentum form)",
    });

    expect(screen.getByText(/Showing terms related to/)).toBeInTheDocument();
    expect(screen.getByText('2 terms')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open term linear momentum' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open term newton' })).toBeInTheDocument();
  });

  it('exposes accessible filter and result semantics', () => {
    renderScreen();

    expect(screen.getByLabelText('Search terms')).toBeInTheDocument();
    expect(screen.getByLabelText('Examination filter')).toBeInTheDocument();
    expect(screen.getByLabelText('Topic filter')).toBeInTheDocument();
    expect(screen.getByLabelText('Kind filter')).toBeInTheDocument();
    expect(screen.getByRole('navigation', { name: 'Filter by subject' })).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Term results' })).toBeInTheDocument();
    expect(screen.getByText('123 terms')).toHaveAttribute('aria-live', 'polite');
  });
});
