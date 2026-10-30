import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DashboardLayout, DashboardSection } from '../lib/preparationApi';
import { DashboardCustomizer } from './DashboardCustomizer';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const DEFAULT_LAYOUT: DashboardLayout = {
  sections: [
    'exam-countdown',
    'syllabus-progress',
    'study-totals',
    'today-study',
    'revision-status',
    'deadlines-blocks',
    'recent-pyq',
    'latest-mock',
    'insights',
  ].map((key, displayOrder) => ({
    key: key as DashboardSection['key'],
    visible: true,
    displayOrder,
  })),
};

describe('dashboard customizer', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('reorders and hides sections, then saves the complete SQLite layout payload', async () => {
    const onChanged = vi.fn();
    invokeMock.mockImplementation((command: string, args?: { sections?: DashboardSection[] }) => {
      if (command === 'update_dashboard_layout') {
        return Promise.resolve({ sections: args?.sections ?? [] });
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(
      <DashboardCustomizer
        sections={DEFAULT_LAYOUT.sections}
        onChanged={onChanged}
        onClose={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole('checkbox', { name: 'Syllabus progress' }));
    await userEvent.click(screen.getByRole('button', { name: 'Move Syllabus progress up' }));
    await userEvent.click(screen.getByRole('button', { name: 'Save layout' }));

    await waitFor(() => expect(invokeMock).toHaveBeenCalled());
    const call = invokeMock.mock.calls.find(([name]) => name === 'update_dashboard_layout');
    expect(call?.[1]?.sections).toHaveLength(9);
    expect(call?.[1]?.sections[0]).toEqual({
      key: 'syllabus-progress',
      visible: false,
      displayOrder: 0,
    });
    expect(call?.[1]?.sections[1]).toEqual({
      key: 'exam-countdown',
      visible: true,
      displayOrder: 1,
    });
    expect(onChanged).toHaveBeenCalledTimes(1);
    expect(await screen.findByText('Dashboard layout saved to SQLite.')).toBeInTheDocument();
  });

  it('restores the default layout through the backend', async () => {
    const onChanged = vi.fn();
    invokeMock.mockImplementation((command: string) => {
      if (command === 'reset_dashboard_layout') {
        return Promise.resolve(DEFAULT_LAYOUT);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(
      <DashboardCustomizer
        sections={DEFAULT_LAYOUT.sections.map((section) => ({
          ...section,
          visible: false,
        }))}
        onChanged={onChanged}
        onClose={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole('button', { name: 'Restore default layout' }));

    await waitFor(() => expect(onChanged).toHaveBeenCalledWith(DEFAULT_LAYOUT));
    expect(
      await screen.findByText('Default dashboard layout restored. All sections are visible.'),
    ).toBeInTheDocument();
  });

  it('shows a backend validation error without discarding the draft', async () => {
    invokeMock.mockRejectedValue({
      code: 'validation_error',
      message: 'Dashboard section order values must be unique.',
      field: 'sections',
    });

    render(
      <DashboardCustomizer
        sections={DEFAULT_LAYOUT.sections}
        onChanged={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole('button', { name: 'Save layout' }));

    expect(
      await screen.findByText('Dashboard section order values must be unique.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: 'Syllabus progress' })).toBeInTheDocument();
  });
});
