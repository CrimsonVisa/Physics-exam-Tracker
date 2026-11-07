import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2AppSettings, V2ExamProfile } from './api';
import { SettingsScreen } from './SettingsScreen';

vi.mock('./api', () => ({
  getV2AppSettings: vi.fn(),
  updateV2AppSettings: vi.fn(),
  listV2ExamProfiles: vi.fn(),
  setV2ExamActive: vi.fn(),
  previewV2ExamDateChange: vi.fn(),
  applyV2ExamDateChange: vi.fn(),
}));

import * as api from './api';

const mockedSettings = vi.mocked(api.getV2AppSettings);
const mockedUpdateSettings = vi.mocked(api.updateV2AppSettings);
const mockedProfiles = vi.mocked(api.listV2ExamProfiles);
const mockedSetActive = vi.mocked(api.setV2ExamActive);
const mockedPreviewDate = vi.mocked(api.previewV2ExamDateChange);
const mockedApplyDate = vi.mocked(api.applyV2ExamDateChange);

const SETTINGS: V2AppSettings = {
  preparationStartDate: '2026-10-01',
  dailyStudyTargetMinutes: 120,
};

const PROFILES: V2ExamProfile[] = [
  {
    id: 1,
    code: 'csir-net-physics',
    name: 'CSIR-UGC NET Physical Sciences',
    shortName: 'CSIR-NET',
    authority: 'CSIR-HRDG / NTA',
    officialUrl: 'https://csirnet.nta.nic.in/',
    displayOrder: 1,
    isActive: true,
    examDate: null,
    dateUpdatedAt: null,
  },
  {
    id: 2,
    code: 'gate-physics',
    name: 'GATE Physics (PH)',
    shortName: 'GATE PH',
    authority: 'IIT',
    officialUrl: 'https://gate.iitm.ac.in/',
    displayOrder: 2,
    isActive: true,
    examDate: null,
    dateUpdatedAt: null,
  },
];

describe('V2 settings and exam management', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    document.documentElement.removeAttribute('data-font-family');
    document.documentElement.removeAttribute('data-font-size');
    mockedSettings.mockResolvedValue(SETTINGS);
    mockedProfiles.mockResolvedValue(PROFILES);
  });

  it('applies and persists typography preferences from the appearance section', async () => {
    const user = userEvent.setup();
    render(<SettingsScreen onChanged={vi.fn()} />);
    await screen.findByDisplayValue('2026-10-01');

    await user.click(screen.getByRole('button', { name: /Manrope/u }));
    expect(document.documentElement).toHaveAttribute('data-font-family', 'manrope');
    expect(window.localStorage.getItem('csir-net-tracker.ui.font-family.v1')).toBe('manrope');

    await user.click(screen.getByRole('button', { name: 'Comfortable' }));
    expect(document.documentElement).toHaveAttribute('data-font-size', 'comfortable');
    expect(window.localStorage.getItem('csir-net-tracker.ui.font-size.v1')).toBe('comfortable');

    expect(screen.getByRole('button', { name: /Manrope/u })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(screen.getByRole('button', { name: 'Comfortable' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
  });

  it('loads and saves application settings', async () => {
    const user = userEvent.setup();
    const onChanged = vi.fn();
    mockedUpdateSettings.mockResolvedValue({
      preparationStartDate: '2026-10-02',
      dailyStudyTargetMinutes: 180,
    });
    render(<SettingsScreen onChanged={onChanged} />);

    expect(await screen.findByDisplayValue('2026-10-01')).toBeInTheDocument();
    const target = screen.getByLabelText('Daily study target (minutes)');
    await user.clear(target);
    await user.type(target, '180');
    await user.click(screen.getByRole('button', { name: 'Save application settings' }));

    await waitFor(() =>
      expect(mockedUpdateSettings).toHaveBeenCalledWith({
        preparationStartDate: '2026-10-01',
        dailyStudyTargetMinutes: 180,
      }),
    );
    expect(onChanged).toHaveBeenCalled();
  });

  it('saves exam dates and restores affected anchored tasks', async () => {
    const user = userEvent.setup();
    mockedPreviewDate.mockResolvedValue({
      examId: 1,
      oldDate: null,
      newDate: '2026-12-20',
      affectedTasks: [
        { id: 3, title: 'Revision', currentDate: '2026-12-15', newDate: '2026-12-20' },
      ],
    });
    mockedApplyDate.mockResolvedValue({
      ...PROFILES[0],
      examDate: '2026-12-20',
      dateUpdatedAt: '2026-11-04T10:00:00.000Z',
    });
    render(<SettingsScreen onChanged={vi.fn()} />);
    await screen.findByText('Exam profiles');

    const dateInputs = screen.getAllByLabelText('Exam date');
    await user.type(dateInputs[0], '2026-12-20');
    await user.click(screen.getAllByRole('button', { name: 'Save date' })[0]);

    await waitFor(() =>
      expect(mockedApplyDate).toHaveBeenCalledWith({
        examId: 1,
        examDate: '2026-12-20',
        reason: 'Settings',
      }),
    );
    expect(
      await screen.findByText('CSIR-NET date saved. 1 exam-anchored task(s) were rescheduled.'),
    ).toBeInTheDocument();
  });

  it('deactivates and reactivates an examination', async () => {
    const user = userEvent.setup();
    mockedSetActive
      .mockResolvedValueOnce({ ...PROFILES[0], isActive: false })
      .mockResolvedValueOnce({ ...PROFILES[0], isActive: true });
    render(<SettingsScreen onChanged={vi.fn()} />);
    await screen.findByText('Exam profiles');

    await user.click(screen.getAllByRole('button', { name: 'Deactivate' })[0]);
    await waitFor(() =>
      expect(mockedSetActive).toHaveBeenLastCalledWith({ examId: 1, isActive: false }),
    );
    expect(await screen.findByText('CSIR-NET is now inactive.')).toBeInTheDocument();
    await user.click(await screen.findByRole('button', { name: 'Reactivate' }));
    await waitFor(() =>
      expect(mockedSetActive).toHaveBeenLastCalledWith({ examId: 1, isActive: true }),
    );
  });
});
