import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import type { V2ExamSummary } from './api';
import type { V2View } from './navigation';
import { V2Sidebar, type V2SidebarMode } from './V2Sidebar';

function exam(overrides: Partial<V2ExamSummary> = {}): V2ExamSummary {
  return {
    id: 1,
    code: 'csir-net-physics',
    name: 'CSIR-UGC NET Physical Sciences',
    shortName: 'CSIR-NET',
    authority: 'CSIR-HRDG / NTA',
    officialUrl: 'https://csirnet.nta.nic.in/',
    displayOrder: 1,
    isActive: true,
    examDate: null,
    ...overrides,
  };
}

const EXAMS: V2ExamSummary[] = [
  exam(),
  exam({
    id: 2,
    code: 'gate-physics',
    name: 'GATE Physics (PH)',
    shortName: 'GATE PH',
    authority: 'IIT',
    officialUrl: 'https://gate.iitm.ac.in/',
    displayOrder: 2,
    examDate: '2026-11-06',
  }),
  exam({
    id: 3,
    code: 'jest-physics',
    name: 'JEST Physics',
    shortName: 'JEST',
    authority: 'JEST Consortium',
    officialUrl: 'https://www.jest.org.in/',
    displayOrder: 3,
    isActive: false,
  }),
  exam({
    id: 4,
    code: 'tifr-gs-physics',
    name: 'TIFR Graduate School Physics',
    shortName: 'TIFR GS',
    authority: 'TIFR',
    officialUrl: 'https://www.tifr.res.in/',
    displayOrder: 4,
    isActive: true,
  }),
];

function renderSidebar(mode: V2SidebarMode, view: V2View = { kind: 'dashboard' }) {
  const onNavigate = vi.fn();
  const onToggleCollapse = vi.fn();
  const onThemeChange = vi.fn();
  const onOpenSearch = vi.fn();
  const onOpenNotes = vi.fn();
  const onOpenShortcuts = vi.fn();

  render(
    <V2Sidebar
      exams={EXAMS}
      mode={mode}
      onNavigate={onNavigate}
      onOpenNotes={onOpenNotes}
      onOpenSearch={onOpenSearch}
      onOpenShortcuts={onOpenShortcuts}
      onThemeChange={onThemeChange}
      onToggleCollapse={onToggleCollapse}
      theme="dark"
      view={view}
    />,
  );

  return {
    onNavigate,
    onToggleCollapse,
    onThemeChange,
    onOpenSearch,
    onOpenNotes,
    onOpenShortcuts,
  };
}

describe('V2Sidebar', () => {
  it('renders an expanded sidebar with icons, labels, and a collapse control', () => {
    renderSidebar('expanded');

    const aside = document.querySelector('aside.sidebar');
    expect(aside).toHaveClass('sidebar-expanded');
    expect(aside).not.toHaveClass('sidebar-collapsed');

    for (const label of [
      'Dashboard',
      'Concepts',
      'Syllabus',
      'Practice',
      'Flashcards',
      'Calculator',
      'Formulas',
      'Terms & Units',
      'Planner',
      'Revision',
      'Analytics',
      'Resources',
      'Backup',
      'Settings',
    ]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }

    expect(screen.getByText('CSIR-NET')).toBeInTheDocument();
    expect(screen.getByText('GATE PH')).toBeInTheDocument();
    expect(screen.getByText('JEST')).toBeInTheDocument();
    expect(screen.getByText('TIFR GS')).toBeInTheDocument();
    expect(screen.getByText('Inactive')).toBeInTheDocument();
    expect(screen.getByText(/One local V2 database/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Collapse sidebar' })).toHaveAttribute(
      'aria-expanded',
      'true',
    );
    // The shared stylesheet hides icons by default; the V2 shell renders them.
    expect(document.querySelectorAll('.nav-icon')).toHaveLength(19);

    // Expanded navigation relies on the visible label, not a title tooltip.
    expect(screen.getByRole('button', { name: 'Practice systems' })).not.toHaveAttribute('title');
  });

  it('keeps the brand copy and header controls in separate regions', () => {
    renderSidebar('expanded');

    const header = document.querySelector('.sidebar-header');
    expect(header).not.toBeNull();
    expect(header).toContainElement(screen.getByText('Physics Tracker'));
    expect(header).toContainElement(screen.getByText('Four-exam foundation'));
    expect(header).toContainElement(screen.getByRole('button', { name: 'Open global search' }));
    expect(header).toContainElement(screen.getByRole('button', { name: 'Collapse sidebar' }));

    const brand = header?.querySelector(':scope > .brand');
    const controls = header?.querySelector(':scope > .sidebar-controls');
    expect(brand).toContainElement(screen.getByText('Physics Tracker'));
    expect(controls).toContainElement(screen.getByRole('button', { name: 'Open global search' }));
    expect(controls).toContainElement(screen.getByRole('button', { name: 'Collapse sidebar' }));
    expect(brand).not.toContainElement(screen.getByRole('button', { name: 'Open global search' }));
  });

  it('renders distinct C, G, T and J monograms for all four exams', () => {
    renderSidebar('expanded');

    const expected: Record<string, string> = {
      'csir-net-physics': 'C',
      'gate-physics': 'G',
      'tifr-gs-physics': 'T',
      'jest-physics': 'J',
    };

    for (const [code, initial] of Object.entries(expected)) {
      const button = document.querySelector(`button[data-exam-code="${code}"]`);
      expect(button).toBeInstanceOf(HTMLElement);
      const monogram = button?.querySelector('.v2-exam-monogram');
      expect(monogram).toHaveTextContent(initial);
      expect(monogram).toHaveClass('nav-icon');
    }
  });

  it('marks the active exam monogram with the active navigation state', () => {
    renderSidebar('expanded', { kind: 'exam', examCode: 'gate-physics' });

    const activeButton = screen.getByRole('button', { name: 'GATE Physics (PH)' });
    expect(activeButton).toHaveClass('nav-item-active');
    expect(activeButton.querySelector('.v2-exam-monogram')).toHaveTextContent('G');
  });

  it('keeps the sidebar spacing and monogram CSS contracts', () => {
    const css = readFileSync(join(process.cwd(), 'src/v2/design-system.css'), 'utf8');

    expect(css).toContain('.v2-app-frame .sidebar-header .brand-copy');
    expect(css).toContain('white-space: normal');
    expect(css).toContain('overflow-wrap: break-word');
    expect(css).toContain('.v2-app-frame .nav-item .v2-exam-monogram');
    expect(css).toContain('display: inline-grid');
    expect(css).toContain('place-items: center');
    expect(css).toContain('.v2-app-frame .nav-item.nav-item-active .v2-exam-monogram');
    expect(css).toContain('color: var(--ws-on-accent)');
  });

  it('marks the active navigation item', () => {
    renderSidebar('expanded', { kind: 'exam', examCode: 'gate-physics' });

    expect(screen.getByRole('button', { name: 'GATE Physics (PH)' })).toHaveAttribute(
      'aria-current',
      'page',
    );
    expect(screen.getByRole('button', { name: 'Combined dashboard' })).not.toHaveAttribute(
      'aria-current',
    );
  });

  it('navigates with the expected V2 views', async () => {
    const { onNavigate } = renderSidebar('expanded');

    await userEvent.click(screen.getByRole('button', { name: 'Concept library' }));
    expect(onNavigate).toHaveBeenCalledWith({ kind: 'concepts', examCode: null });

    await userEvent.click(screen.getByRole('button', { name: 'Flashcards' }));
    expect(onNavigate).toHaveBeenCalledWith({ kind: 'flashcards' });

    await userEvent.click(screen.getByRole('button', { name: 'Scientific calculator' }));
    expect(onNavigate).toHaveBeenCalledWith({ kind: 'calculator' });

    await userEvent.click(screen.getByRole('button', { name: 'Formula reference' }));
    expect(onNavigate).toHaveBeenCalledWith({ kind: 'formulas' });

    await userEvent.click(screen.getByRole('button', { name: 'Physics terms and SI units' }));
    expect(onNavigate).toHaveBeenCalledWith({ kind: 'terms' });

    await userEvent.click(screen.getByRole('button', { name: 'GATE Physics (PH)' }));
    expect(onNavigate).toHaveBeenCalledWith({ kind: 'exam', examCode: 'gate-physics' });
  });

  it('exposes the collapse and theme controls without navigating', async () => {
    const { onNavigate, onThemeChange, onToggleCollapse } = renderSidebar('expanded');

    await userEvent.click(screen.getByRole('button', { name: 'Collapse sidebar' }));
    expect(onToggleCollapse).toHaveBeenCalledTimes(1);
    expect(onNavigate).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole('button', { name: 'Light theme' }));
    expect(onThemeChange).toHaveBeenCalledWith('light');
  });

  it('opens global search, quick notes and the shortcut reference from the sidebar', async () => {
    const { onOpenSearch, onOpenNotes, onOpenShortcuts, onNavigate } = renderSidebar('expanded');
    await userEvent.click(screen.getByRole('button', { name: 'Open global search' }));
    expect(onOpenSearch).toHaveBeenCalledTimes(1);
    await userEvent.click(screen.getByRole('button', { name: 'Open quick notes' }));
    expect(onOpenNotes).toHaveBeenCalledTimes(1);
    await userEvent.click(screen.getByRole('button', { name: 'Keyboard shortcuts' }));
    expect(onOpenShortcuts).toHaveBeenCalledTimes(1);
    expect(onNavigate).not.toHaveBeenCalled();
  });

  it('navigates to the sticky notes page from the sidebar', async () => {
    const { onNavigate } = renderSidebar('expanded');
    await userEvent.click(screen.getByRole('button', { name: 'Sticky notes' }));
    expect(onNavigate).toHaveBeenCalledWith({ kind: 'notes' });
  });

  it('shows a collapsed icon-only sidebar with tooltips on hover and focus', () => {
    renderSidebar('collapsed');

    const aside = document.querySelector('aside.sidebar');
    expect(aside).toHaveClass('sidebar-collapsed');
    expect(aside).not.toHaveClass('sidebar-expanded');

    const practice = screen.getByRole('button', { name: 'Practice systems' });
    expect(practice).toHaveAttribute('title', 'Practice');
    expect(screen.getByRole('button', { name: 'Expand sidebar' })).toHaveAttribute(
      'aria-expanded',
      'false',
    );
    expect(screen.queryByText(/One local V2 database/)).not.toBeInTheDocument();

    fireEvent.mouseEnter(practice);
    expect(screen.getByRole('tooltip')).toHaveTextContent('Practice');

    fireEvent.mouseLeave(practice);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();

    fireEvent.focus(practice);
    expect(screen.getByRole('tooltip')).toHaveTextContent('Practice');

    fireEvent.blur(practice);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('suppresses navigation tooltips while expanded but keeps the control tooltip', () => {
    renderSidebar('expanded');

    const practice = screen.getByRole('button', { name: 'Practice systems' });
    fireEvent.mouseEnter(practice);
    fireEvent.focus(practice);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();

    const control = screen.getByRole('button', { name: 'Collapse sidebar' });
    fireEvent.mouseEnter(control);
    expect(screen.getByRole('tooltip')).toHaveTextContent('Collapse sidebar');
    fireEvent.mouseLeave(control);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('dismisses the tooltip on window resize and scroll', () => {
    renderSidebar('collapsed');

    const practice = screen.getByRole('button', { name: 'Practice systems' });

    fireEvent.focus(practice);
    expect(screen.getByRole('tooltip')).toHaveTextContent('Practice');

    fireEvent(window, new Event('resize'));
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();

    fireEvent.focus(practice);
    expect(screen.getByRole('tooltip')).toHaveTextContent('Practice');

    fireEvent.scroll(window);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('annotates inactive examinations in the collapsed tooltip', () => {
    renderSidebar('collapsed');

    const inactiveExam = screen.getByRole('button', { name: 'JEST Physics' });
    fireEvent.mouseEnter(inactiveExam);
    expect(screen.getByRole('tooltip')).toHaveTextContent('JEST Physics (inactive)');

    fireEvent.mouseLeave(inactiveExam);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });
});
