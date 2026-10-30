import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { RevisionSettings } from '../lib/revisionApi';
import { RevisionSettingsPanel } from './RevisionSettingsPanel';

const { invokeMock, isPermissionGrantedMock, requestPermissionMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  isPermissionGrantedMock: vi.fn(),
  requestPermissionMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: isPermissionGrantedMock,
  requestPermission: requestPermissionMock,
}));

const SETTINGS: RevisionSettings = {
  initialAgainDays: 1,
  initialDifficultDays: 2,
  initialGoodDays: 5,
  initialEasyDays: 10,
  growthMultiplier: 1.5,
  minimumIntervalDays: 1,
  maximumIntervalDays: 3650,
  includeWeekends: true,
  dailyRevisionTarget: 5,
  remindersEnabled: false,
  updatedAt: '2026-10-30T10:00:00.000Z',
};

describe('revision settings panel', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isPermissionGrantedMock.mockReset();
    requestPermissionMock.mockReset();
  });

  it('saves validated interval, weekend, target, and reminder configuration', async () => {
    invokeMock.mockResolvedValue({
      ...SETTINGS,
      initialGoodDays: 7,
      growthMultiplier: 2,
      dailyRevisionTarget: 6,
      includeWeekends: false,
    });
    const onSaved = vi.fn();

    render(<RevisionSettingsPanel settings={SETTINGS} onSaved={onSaved} />);

    fireEvent.change(screen.getByLabelText('Good initial days'), { target: { value: '7' } });
    fireEvent.change(screen.getByLabelText('Growth multiplier'), { target: { value: '2' } });
    fireEvent.change(screen.getByLabelText('Daily revision target'), {
      target: { value: '6' },
    });
    await userEvent.click(screen.getByLabelText(/Include Saturdays and Sundays/));
    await userEvent.click(screen.getByRole('button', { name: 'Save revision settings' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('update_revision_settings', {
        settings: {
          initialAgainDays: 1,
          initialDifficultDays: 2,
          initialGoodDays: 7,
          initialEasyDays: 10,
          growthMultiplier: 2,
          minimumIntervalDays: 1,
          maximumIntervalDays: 3650,
          includeWeekends: false,
          dailyRevisionTarget: 6,
          remindersEnabled: false,
        },
      }),
    );
    expect(onSaved).toHaveBeenCalledTimes(1);
    expect(await screen.findByText(/Revision settings saved/)).toBeInTheDocument();
  });

  it('does not enable reminders when Windows denies permission', async () => {
    isPermissionGrantedMock.mockResolvedValue(false);
    requestPermissionMock.mockResolvedValue('denied');

    render(<RevisionSettingsPanel settings={SETTINGS} onSaved={vi.fn()} />);

    await userEvent.click(screen.getByLabelText(/Desktop review reminders/));
    await userEvent.click(screen.getByRole('button', { name: 'Save revision settings' }));

    expect(await screen.findByText(/notification permission/)).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('maps backend interval errors to the responsible field', async () => {
    invokeMock.mockRejectedValue({
      code: 'validation_error',
      message: 'Initial outcome intervals must stay between the minimum and maximum intervals.',
      field: 'initialGoodDays',
    });

    render(<RevisionSettingsPanel settings={SETTINGS} onSaved={vi.fn()} />);

    await userEvent.click(screen.getByRole('button', { name: 'Save revision settings' }));

    expect(
      (
        await screen.findAllByText(
          'Initial outcome intervals must stay between the minimum and maximum intervals.',
        )
      ).length,
    ).toBeGreaterThan(0);
    expect(screen.getByLabelText('Good initial days')).toHaveAttribute('aria-invalid', 'true');
  });
});
