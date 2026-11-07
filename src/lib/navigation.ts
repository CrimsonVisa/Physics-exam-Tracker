export type AppView =
  | 'overview'
  | 'exams'
  | 'timer'
  | 'sessions'
  | 'checklist'
  | 'calendar'
  | 'revision'
  | 'pyq'
  | 'mock-tests'
  | 'analytics'
  | 'backup'
  | 'syllabus'
  | 'resources';

export type NavIconName =
  | AppView
  | 'concepts'
  | 'search'
  | 'keyboard'
  | 'sun'
  | 'moon'
  | 'check'
  | 'chevron-left'
  | 'chevron-right'
  | 'panel-hide'
  | 'menu';

export type NavItem = {
  view: AppView;
  label: string;
  icon: NavIconName;
};

export const NAV_ITEMS: NavItem[] = [
  { view: 'overview', label: 'Overview', icon: 'overview' },
  { view: 'exams', label: 'Exams', icon: 'exams' },
  { view: 'timer', label: 'Timer', icon: 'timer' },
  { view: 'sessions', label: 'Sessions', icon: 'sessions' },
  { view: 'checklist', label: 'Checklist', icon: 'checklist' },
  { view: 'calendar', label: 'Calendar', icon: 'calendar' },
  { view: 'revision', label: 'Revision', icon: 'revision' },
  { view: 'pyq', label: 'PYQs', icon: 'pyq' },
  { view: 'mock-tests', label: 'Mock tests', icon: 'mock-tests' },
  { view: 'analytics', label: 'Analytics', icon: 'analytics' },
  { view: 'backup', label: 'Backup', icon: 'backup' },
  { view: 'syllabus', label: 'Syllabus', icon: 'syllabus' },
  { view: 'resources', label: 'Resources', icon: 'resources' },
];

export const KNOWN_VIEWS: AppView[] = NAV_ITEMS.map((item) => item.view);
