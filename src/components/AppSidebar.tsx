import { useEffect, useState, type RefObject } from 'react';
import { NAV_ITEMS, type AppView } from '../lib/navigation';
import type { SidebarMode, Theme } from '../lib/uiPreferences';
import { Icon } from './NavIcon';
import { ThemeSwitch } from './ThemeSwitch';

type VisibleSidebarMode = Exclude<SidebarMode, 'hidden'>;

type TooltipState = {
  label: string;
  left: number;
  top: number;
};

export function AppSidebar({
  activeView,
  mode,
  theme,
  collapseButtonRef,
  onNavigate,
  onToggleCollapse,
  onHide,
  onThemeChange,
}: {
  activeView: AppView;
  mode: VisibleSidebarMode;
  theme: Theme;
  collapseButtonRef: RefObject<HTMLButtonElement | null>;
  onNavigate: (view: AppView) => void;
  onToggleCollapse: () => void;
  onHide: () => void;
  onThemeChange: (theme: Theme) => void;
}) {
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

  return (
    <aside aria-label="Application sidebar" className={`sidebar sidebar-${mode}`}>
      <div className="sidebar-header">
        <div className="brand">
          <span aria-hidden="true" className="brand-mark">
            CN
          </span>
          <div className="brand-copy">
            <p className="brand-title">CSIR-NET</p>
            <p className="brand-subtitle">Physics Tracker</p>
          </div>
        </div>

        <div className="sidebar-controls">
          <button
            aria-controls="primary-navigation"
            aria-expanded={mode === 'expanded'}
            aria-label={mode === 'expanded' ? 'Collapse sidebar' : 'Expand sidebar'}
            className="sidebar-control"
            onClick={onToggleCollapse}
            onBlur={() => setTooltip(null)}
            onFocus={(event) =>
              showTooltip(
                mode === 'expanded' ? 'Collapse sidebar' : 'Expand sidebar',
                event.currentTarget,
                true,
              )
            }
            onMouseEnter={(event) =>
              showTooltip(
                mode === 'expanded' ? 'Collapse sidebar' : 'Expand sidebar',
                event.currentTarget,
                true,
              )
            }
            onMouseLeave={() => setTooltip(null)}
            ref={collapseButtonRef}
            title={mode === 'expanded' ? 'Collapse sidebar' : 'Expand sidebar'}
            type="button"
          >
            <Icon name={mode === 'expanded' ? 'chevron-left' : 'chevron-right'} size={17} />
          </button>
          <button
            aria-label="Hide sidebar"
            className="sidebar-control"
            onClick={onHide}
            onBlur={() => setTooltip(null)}
            onFocus={(event) => showTooltip('Hide sidebar', event.currentTarget, true)}
            onMouseEnter={(event) => showTooltip('Hide sidebar', event.currentTarget, true)}
            onMouseLeave={() => setTooltip(null)}
            title="Hide sidebar"
            type="button"
          >
            <Icon name="panel-hide" size={17} />
          </button>
        </div>
      </div>

      <nav aria-label="Primary navigation" className="nav" id="primary-navigation">
        {NAV_ITEMS.map((item) => {
          const active = activeView === item.view;
          return (
            <button
              aria-current={active ? 'page' : undefined}
              aria-label={item.label}
              className={`nav-item nav-button${active ? ' nav-item-active' : ''}`}
              key={item.view}
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
        })}
      </nav>

      {mode === 'expanded' && (
        <p className="sidebar-note">
          Local-first. Settings, study records, backups, and exports stay on this PC and are never
          sent to a server. Use Backup and data management to protect a verified snapshot.
        </p>
      )}

      <div className="sidebar-footer">
        <ThemeSwitch compact={mode === 'collapsed'} onChange={onThemeChange} theme={theme} />
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
