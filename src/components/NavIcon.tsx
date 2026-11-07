import type { ReactNode } from 'react';
import type { NavIconName } from '../lib/navigation';

const ICON_PATHS: Record<NavIconName, ReactNode> = {
  overview: (
    <>
      <rect x="3.5" y="3.5" width="7" height="7" rx="1.5" />
      <rect x="13.5" y="3.5" width="7" height="7" rx="1.5" />
      <rect x="3.5" y="13.5" width="7" height="7" rx="1.5" />
      <rect x="13.5" y="13.5" width="7" height="7" rx="1.5" />
    </>
  ),
  exams: (
    <>
      <path d="M4 5.5A2.5 2.5 0 0 1 6.5 3H20v16H6.5A2.5 2.5 0 0 0 4 21.5z" />
      <path d="M4 18.5A2.5 2.5 0 0 1 6.5 16H20" />
      <path d="M8 7h8M8 10.5h6" />
    </>
  ),
  timer: (
    <>
      <circle cx="12" cy="13" r="8" />
      <path d="M12 9v4l2.5 1.5" />
      <path d="M9 2h6" />
    </>
  ),
  sessions: (
    <>
      <path d="M4 5h16" />
      <path d="M4 12h16" />
      <path d="M4 19h10" />
      <circle cx="18" cy="19" r="2" />
    </>
  ),
  checklist: (
    <>
      <rect x="4" y="3.5" width="16" height="17" rx="2.5" />
      <path d="m8 9 2 2 4-4" />
      <path d="M8 16h8" />
    </>
  ),
  calendar: (
    <>
      <rect x="3.5" y="5" width="17" height="15" rx="2.5" />
      <path d="M7.5 3v4M16.5 3v4M3.5 9.5h17" />
      <path d="M8 13h.01M12 13h.01M16 13h.01M8 17h.01M12 17h.01" />
    </>
  ),
  revision: (
    <>
      <path d="M20 11a8 8 0 0 0-14.8-4L3 10" />
      <path d="M3 4v6h6" />
      <path d="M4 13a8 8 0 0 0 14.8 4L21 14" />
      <path d="M21 20v-6h-6" />
    </>
  ),
  pyq: (
    <>
      <path d="M4 4.5A2.5 2.5 0 0 1 6.5 2H20v17H6.5A2.5 2.5 0 0 0 4 21.5z" />
      <path d="M4 18.5A2.5 2.5 0 0 1 6.5 16H20" />
      <path d="M8 6h8M8 10h6" />
    </>
  ),
  'mock-tests': (
    <>
      <rect x="4" y="3.5" width="16" height="17" rx="2.5" />
      <path d="M8 8h8M8 12h8M8 16h5" />
      <path d="m17 16 .8 1.5L19.5 18l-1.7.5L17 20l-.8-1.5L14.5 18l1.7-.5z" />
    </>
  ),
  analytics: (
    <>
      <path d="M4 20V10M10 20V4M16 20v-7M22 20H2" />
    </>
  ),
  backup: (
    <>
      <path d="M4 7.5 12 3l8 4.5v9L12 21l-8-4.5z" />
      <path d="M4 7.5 12 12l8-4.5M12 12v9" />
      <path d="M8 5.2 16 9.7" />
    </>
  ),
  syllabus: (
    <>
      <path d="M5 4.5v15" />
      <path d="M5 8h5M5 14.5h8" />
      <rect height="4.5" rx="1.2" width="8" x="10" y="5.75" />
      <rect height="4.5" rx="1.2" width="6" x="13" y="12.25" />
    </>
  ),
  concepts: (
    <>
      <circle cx="6" cy="6.5" r="2.4" />
      <circle cx="18" cy="7.5" r="2.4" />
      <circle cx="12" cy="18" r="2.4" />
      <path d="M7.9 8.1 10.6 15.6M16.2 9.4 13.4 15.9M8.4 6.2l7.2.5" />
    </>
  ),
  resources: (
    <>
      <path d="M3.5 7.5A2.5 2.5 0 0 1 6 5h4l2 2h6a2.5 2.5 0 0 1 2.5 2.5v8A2.5 2.5 0 0 1 18 20H6a2.5 2.5 0 0 1-2.5-2.5z" />
      <path d="M3.5 9.5h17" />
    </>
  ),
  sun: (
    <>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
    </>
  ),
  moon: (
    <>
      <path d="M20.5 14.5A8.5 8.5 0 0 1 9.5 3.5 8.5 8.5 0 1 0 20.5 14.5z" />
    </>
  ),
  check: <path d="m5 12 4.5 4.5L19 7" />,
  'chevron-left': <path d="m14.5 5-7 7 7 7" />,
  'chevron-right': <path d="m9.5 5 7 7-7 7" />,
  'panel-hide': (
    <>
      <rect x="3.5" y="4" width="17" height="16" rx="2.5" />
      <path d="M9 4v16M15 9l-3 3 3 3" />
    </>
  ),
  menu: (
    <>
      <path d="M4 6h16M4 12h16M4 18h16" />
    </>
  ),
  search: (
    <>
      <circle cx="10.5" cy="10.5" r="6.5" />
      <path d="m15.5 15.5 5 5" />
    </>
  ),
  keyboard: (
    <>
      <rect height="10" rx="2" width="18" x="3" y="7" />
      <path d="M6.5 10.5h.01M9.5 10.5h.01M12.5 10.5h.01M15.5 10.5h.01M18 10.5h.01M6.5 13.5h.01M9.5 13.5h.01M12.5 13.5h5" />
    </>
  ),
};

export function Icon({
  name,
  size = 20,
  className,
}: {
  name: NavIconName;
  size?: number;
  className?: string;
}) {
  return (
    <svg
      aria-hidden="true"
      className={className}
      fill="none"
      focusable="false"
      height={size}
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth="1.8"
      viewBox="0 0 24 24"
      width={size}
    >
      {ICON_PATHS[name]}
    </svg>
  );
}
