import { useEffect, useState } from 'react';
import { BrandMark } from '../components/BrandMark';
import { Icon } from '../components/NavIcon';
import { ThemeSwitch } from '../components/ThemeSwitch';
import type { NavIconName } from '../lib/navigation';
import type { Theme } from '../lib/uiPreferences';
import type { V2ExamSummary } from './api';
import type { V2View } from './navigation';

export type V2SidebarMode = 'expanded' | 'collapsed';

type V2NavItem = {
  key: string;
  label: string;
  ariaLabel: string;
  icon: NavIconName;
  view: V2View;
};

type TooltipState = {
  label: string;
  left: number;
  top: number;
};

const EXAM_INITIALS: Record<string, string> = {
  'csir-net-physics': 'C',
  'gate-physics': 'G',
  'tifr-gs-physics': 'T',
  'jest-physics': 'J',
};

const TOP_NAV_ITEMS: V2NavItem[] = [
  {
    key: 'dashboard',
    label: 'Dashboard',
    ariaLabel: 'Combined dashboard',
    icon: 'overview',
    view: { kind: 'dashboard' },
  },
  {
    key: 'concepts',
    label: 'Concepts',
    ariaLabel: 'Concept library',
    icon: 'concepts',
    view: { kind: 'concepts', examCode: null },
  },
  {
    key: 'syllabus',
    label: 'Syllabus',
    ariaLabel: 'Exam syllabus',
    icon: 'syllabus',
    view: { kind: 'syllabus', examCode: null },
  },
];

const STUDY_NAV_ITEMS: V2NavItem[] = [
  {
    key: 'practice',
    label: 'Practice',
    ariaLabel: 'Practice systems',
    icon: 'pyq',
    view: { kind: 'practice', examCode: null },
  },
  {
    key: 'flashcards',
    label: 'Flashcards',
    ariaLabel: 'Flashcards',
    icon: 'flashcards',
    view: { kind: 'flashcards' },
  },
  {
    key: 'calculator',
    label: 'Calculator',
    ariaLabel: 'Scientific calculator',
    icon: 'calculator',
    view: { kind: 'calculator' },
  },
  {
    key: 'formulas',
    label: 'Formulas',
    ariaLabel: 'Formula reference',
    icon: 'formula',
    view: { kind: 'formulas' },
  },
  {
    key: 'terms',
    label: 'Terms & Units',
    ariaLabel: 'Physics terms and SI units',
    icon: 'terms',
    view: { kind: 'terms' },
  },
  {
    key: 'planner',
    label: 'Planner',
    ariaLabel: 'Planner and calendar',
    icon: 'calendar',
    view: { kind: 'planner' },
  },
  {
    key: 'revision',
    label: 'Revision',
    ariaLabel: 'Revision planning',
    icon: 'revision',
    view: { kind: 'revision' },
  },
  {
    key: 'notes',
    label: 'Notes',
    ariaLabel: 'Sticky notes',
    icon: 'notes',
    view: { kind: 'notes' },
  },
];

const INSIGHT_NAV_ITEMS: V2NavItem[] = [
  {
    key: 'analytics',
    label: 'Analytics',
    ariaLabel: 'Analytics',
    icon: 'analytics',
    view: { kind: 'analytics' },
  },
  {
    key: 'resources',
    label: 'Resources',
    ariaLabel: 'Resources',
    icon: 'resources',
    view: { kind: 'resources' },
  },
];

const SYSTEM_NAV_ITEMS: V2NavItem[] = [
  {
    key: 'backup',
    label: 'Backup',
    ariaLabel: 'Backup and restore',
    icon: 'backup',
    view: { kind: 'backup' },
  },
  {
    key: 'settings',
    label: 'Settings',
    ariaLabel: 'Settings',
    icon: 'checklist',
    view: { kind: 'settings' },
  },
];

type V2SidebarProps = {
  exams: V2ExamSummary[];
  view: V2View;
  theme: Theme;
  mode: V2SidebarMode;
  onNavigate: (view: V2View) => void;
  onOpenSearch: () => void;
  onOpenNotes: () => void;
  onOpenShortcuts: () => void;
  onToggleCollapse: () => void;
  onThemeChange: (theme: Theme) => void;
};

export function V2Sidebar({
  exams,
  view,
  theme,
  mode,
  onNavigate,
  onOpenSearch,
  onOpenNotes,
  onOpenShortcuts,
  onToggleCollapse,
  onThemeChange,
}: V2SidebarProps) {
  const [tooltip, setTooltip] = useState<TooltipState | null>(null);

  useEffect(() => {
    if (!tooltip) {
      return;
    }

    const dismiss = () => setTooltip(null);
    window.addEventListener('resize', dismiss);
    window.addEventListener('scroll', dismiss, true);
    return () => {
      window.removeEventListener('resize', dismiss);
      window.removeEventListener('scroll', dismiss, true);
    };
  }, [tooltip]);

  const collapseControlLabel = mode === 'expanded' ? 'Collapse sidebar' : 'Expand sidebar';

  function showTooltip(label: string, element: HTMLElement, always = false) {
    if (!always && mode !== 'collapsed') {
      return;
    }

    const rect = element.getBoundingClientRect();
    setTooltip({
      label,
      left: Math.min(rect.right + 10, window.innerWidth - 180),
      top: rect.top + rect.height / 2,
    });
  }

  function renderNavItem(item: V2NavItem) {
    const active = view.kind === item.view.kind;
    return (
      <button
        aria-current={active ? 'page' : undefined}
        aria-label={item.ariaLabel}
        className={`nav-item nav-button${active ? ' nav-item-active' : ''}`}
        key={item.key}
        onClick={() => onNavigate(item.view)}
        onBlur={() => setTooltip(null)}
        onFocus={(event) => showTooltip(item.label, event.currentTarget)}
        onMouseEnter={(event) => showTooltip(item.label, event.currentTarget)}
        onMouseLeave={() => setTooltip(null)}
        title={mode === 'collapsed' ? item.label : undefined}
        type="button"
      >
        <Icon className="nav-icon" name={item.icon} size={19} />
        <span className="nav-label">{item.label}</span>
      </button>
    );
  }

  function renderExamItem(exam: V2ExamSummary) {
    const active = view.kind === 'exam' && view.examCode === exam.code;
    const tooltipLabel = exam.isActive ? exam.name : `${exam.name} (inactive)`;
    return (
      <button
        aria-current={active ? 'page' : undefined}
        aria-label={exam.name}
        className={`nav-item nav-button v2-exam-nav${active ? ' nav-item-active' : ''}`}
        data-exam-code={exam.code}
        key={exam.code}
        onClick={() => onNavigate({ kind: 'exam', examCode: exam.code })}
        onBlur={() => setTooltip(null)}
        onFocus={(event) => showTooltip(tooltipLabel, event.currentTarget)}
        onMouseEnter={(event) => showTooltip(tooltipLabel, event.currentTarget)}
        onMouseLeave={() => setTooltip(null)}
        title={exam.name}
        type="button"
      >
        <span aria-hidden="true" className="nav-icon v2-exam-monogram">
          {EXAM_INITIALS[exam.code] ?? exam.shortName.slice(0, 4)}
        </span>
        <span className="nav-label">{exam.shortName}</span>
        {!exam.isActive && <span className="nav-tag v2-inactive-tag">Inactive</span>}
      </button>
    );
  }

  return (
    <aside aria-label="Physics Tracker navigation" className={`sidebar sidebar-${mode}`}>
      <div className="sidebar-header">
        <div className="brand">
          <BrandMark size={mode === 'collapsed' ? 32 : 38} />
          <div className="brand-copy">
            <p className="brand-title">Physics Tracker</p>
            <p className="brand-subtitle">Four-exam foundation</p>
          </div>
        </div>

        <div className="sidebar-controls">
          <button
            aria-label="Open global search"
            className="sidebar-control"
            onClick={onOpenSearch}
            onFocus={(event) => showTooltip('Search (Ctrl+K)', event.currentTarget)}
            onMouseEnter={(event) => showTooltip('Search (Ctrl+K)', event.currentTarget)}
            onMouseLeave={() => setTooltip(null)}
            onBlur={() => setTooltip(null)}
            title="Search (Ctrl+K)"
            type="button"
          >
            <Icon name="search" size={16} />
          </button>
          <button
            aria-label="Open quick notes"
            className="sidebar-control"
            onClick={onOpenNotes}
            onBlur={() => setTooltip(null)}
            onFocus={(event) => showTooltip('Quick notes (Ctrl+Shift+N)', event.currentTarget)}
            onMouseEnter={(event) => showTooltip('Quick notes (Ctrl+Shift+N)', event.currentTarget)}
            onMouseLeave={() => setTooltip(null)}
            title="Quick notes (Ctrl+Shift+N)"
            type="button"
          >
            <Icon name="notes" size={16} />
          </button>
          <button
            aria-controls="primary-navigation"
            aria-expanded={mode === 'expanded'}
            aria-label={collapseControlLabel}
            className="sidebar-control"
            onClick={onToggleCollapse}
            onBlur={() => setTooltip(null)}
            onFocus={(event) => showTooltip(collapseControlLabel, event.currentTarget, true)}
            onMouseEnter={(event) => showTooltip(collapseControlLabel, event.currentTarget, true)}
            onMouseLeave={() => setTooltip(null)}
            title={collapseControlLabel}
            type="button"
          >
            <Icon name={mode === 'expanded' ? 'chevron-left' : 'chevron-right'} size={17} />
          </button>
        </div>
      </div>

      <nav aria-label="Primary navigation" className="nav" id="primary-navigation">
        <p className="v2-nav-group-label">Workspace</p>
        {TOP_NAV_ITEMS.map(renderNavItem)}
        <p className="v2-nav-group-label">Examinations</p>
        {exams.map(renderExamItem)}
        <p className="v2-nav-group-label">Study &amp; practice</p>
        {STUDY_NAV_ITEMS.map(renderNavItem)}
        <p className="v2-nav-group-label">Insights &amp; library</p>
        {INSIGHT_NAV_ITEMS.map(renderNavItem)}
        <p className="v2-nav-group-label">System</p>
        {SYSTEM_NAV_ITEMS.map(renderNavItem)}
      </nav>

      {mode === 'expanded' && (
        <p className="sidebar-note">
          One local database. Legacy data is untouched and is never opened by this application.
        </p>
      )}

      <div className="sidebar-footer">
        <ThemeSwitch compact={mode === 'collapsed'} onChange={onThemeChange} theme={theme} />
        <button
          aria-label="Keyboard shortcuts"
          className="sidebar-control v2-shortcuts-trigger"
          onClick={onOpenShortcuts}
          onBlur={() => setTooltip(null)}
          onFocus={(event) => showTooltip('Keyboard shortcuts (Ctrl+/)', event.currentTarget)}
          onMouseEnter={(event) => showTooltip('Keyboard shortcuts (Ctrl+/)', event.currentTarget)}
          onMouseLeave={() => setTooltip(null)}
          title="Keyboard shortcuts (Ctrl+/)"
          type="button"
        >
          <Icon name="keyboard" size={16} />
        </button>
      </div>

      {tooltip && (
        <div
          className="sidebar-tooltip"
          role="tooltip"
          style={{ left: tooltip.left, top: tooltip.top }}
        >
          {tooltip.label}
        </div>
      )}
    </aside>
  );
}
