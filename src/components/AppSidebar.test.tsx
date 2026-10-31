import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { NAV_ITEMS } from '../lib/navigation';
import { AppSidebar } from './AppSidebar';

function renderSidebar(mode: 'expanded' | 'collapsed' = 'expanded') {
  const onNavigate = vi.fn();
  const onToggleCollapse = vi.fn();
  const onHide = vi.fn();
  const onThemeChange = vi.fn();
  const collapseButtonRef = { current: null as HTMLButtonElement | null };

  render(
    <AppSidebar
      activeView="timer"
      collapseButtonRef={collapseButtonRef}
      mode={mode}
      onHide={onHide}
      onNavigate={onNavigate}
      onThemeChange={onThemeChange}
      onToggleCollapse={onToggleCollapse}
      theme="dark"
    />,
  );

  return { collapseButtonRef, onHide, onNavigate, onThemeChange, onToggleCollapse };
}

describe('AppSidebar', () => {
  it('preserves every navigation label and the active state', async () => {
    const { onNavigate } = renderSidebar();

    for (const item of NAV_ITEMS) {
      expect(screen.getByRole('button', { name: item.label })).toBeInTheDocument();
    }
    expect(screen.getByRole('button', { name: 'Timer' })).toHaveAttribute('aria-current', 'page');

    await userEvent.click(screen.getByRole('button', { name: 'Syllabus' }));
    expect(onNavigate).toHaveBeenCalledWith('syllabus');
  });

  it('exposes collapse, hide, and theme controls', async () => {
    const { onHide, onThemeChange, onToggleCollapse } = renderSidebar();

    expect(screen.getByRole('button', { name: 'Collapse sidebar' })).toHaveAttribute(
      'aria-expanded',
      'true',
    );
    await userEvent.click(screen.getByRole('button', { name: 'Collapse sidebar' }));
    expect(onToggleCollapse).toHaveBeenCalledTimes(1);

    await userEvent.click(screen.getByRole('button', { name: 'Hide sidebar' }));
    expect(onHide).toHaveBeenCalledTimes(1);

    await userEvent.click(screen.getByRole('button', { name: 'Light theme' }));
    expect(onThemeChange).toHaveBeenCalledWith('light');
  });

  it('shows a tooltip on hover and keyboard focus in collapsed mode', () => {
    renderSidebar('collapsed');

    const timer = screen.getByRole('button', { name: 'Timer' });
    expect(timer).toHaveAttribute('title', 'Timer');
    expect(screen.getByRole('button', { name: 'Expand sidebar' })).toHaveAttribute(
      'aria-expanded',
      'false',
    );

    fireEvent.mouseEnter(timer);
    expect(screen.getByRole('tooltip')).toHaveTextContent('Timer');

    fireEvent.mouseLeave(timer);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();

    fireEvent.focus(timer);
    expect(screen.getByRole('tooltip')).toHaveTextContent('Timer');

    fireEvent(window, new Event('resize'));
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();

    fireEvent.focus(timer);
    expect(screen.getByRole('tooltip')).toHaveTextContent('Timer');

    fireEvent.scroll(window);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();

    fireEvent.focus(timer);
    fireEvent.blur(timer);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });
});
