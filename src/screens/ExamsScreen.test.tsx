import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ExamProfile } from '../lib/examsApi';
import ExamsScreen from './ExamsScreen';
import {
  confirmLegacyExamDate,
  dismissLegacyExamDateNotice,
  getExamProfiles,
  setExamActive,
  updateExamDate,
} from '../lib/examsApi';

vi.mock('../lib/examsApi', () => ({
  getExamProfiles: vi.fn(),
  updateExamDate: vi.fn(),
  setExamActive: vi.fn(),
  confirmLegacyExamDate: vi.fn(),
  dismissLegacyExamDateNotice: vi.fn(),
}));

const today = '2026-11-02';

function profile(overrides: Partial<ExamProfile> = {}): ExamProfile {
  return {
    examId: 1,
    code: 'csir-net-physics',
    name: 'CSIR-UGC NET Physical Sciences',
    shortName: 'CSIR-NET',
    authority: 'CSIR-HRDG / National Testing Agency',
    officialUrl: 'https://csirnet.nta.nic.in/',
    displayOrder: 1,
    isBuiltin: true,
    isActive: true,
    examDate: null,
    legacyDateNoticeState: 'none',
    legacyExamDate: null,
    createdAt: '2026-11-02T00:00:00Z',
    updatedAt: '2026-11-02T00:00:00Z',
    ...overrides,
  };
}

const profiles: ExamProfile[] = [
  profile(),
  profile({
    examId: 2,
    code: 'gate-physics',
    name: 'GATE Physics (PH)',
    shortName: 'GATE PH',
    authority: 'Indian Institute of Technology Madras',
    displayOrder: 2,
    examDate: '2027-02-14',
  }),
  profile({
    examId: 3,
    code: 'tifr-gs-physics',
    name: 'TIFR Graduate School Physics',
    shortName: 'TIFR GS',
    authority: 'Tata Institute of Fundamental Research',
    displayOrder: 3,
    examDate: today,
  }),
  profile({
    examId: 4,
    code: 'jest-physics',
    name: 'JEST Physics',
    shortName: 'JEST',
    authority: 'JEST Consortium',
    displayOrder: 4,
    examDate: '2026-04-12',
  }),
];

function renderScreen(selectedExamId: number | null = null) {
  const onSelectExam = vi.fn();
  render(<ExamsScreen today={today} selectedExamId={selectedExamId} onSelectExam={onSelectExam} />);
  return { onSelectExam };
}

describe('ExamsScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getExamProfiles).mockResolvedValue(profiles);
    vi.mocked(updateExamDate).mockImplementation(async (examId, examDate) => {
      const current = profiles.find((item) => item.examId === examId);
      if (!current) {
        throw new Error('missing profile');
      }
      return { ...current, examDate };
    });
    vi.mocked(setExamActive).mockImplementation(async (examId, isActive) => {
      const current = profiles.find((item) => item.examId === examId);
      if (!current) {
        throw new Error('missing profile');
      }
      return { ...current, isActive };
    });
  });

  it('shows all four active profiles and the required date states', async () => {
    renderScreen();

    expect(await screen.findByRole('heading', { name: 'Exam profiles' })).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: 'CSIR-UGC NET Physical Sciences' }),
    ).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'GATE Physics (PH)' })).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: 'TIFR Graduate School Physics' }),
    ).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'JEST Physics' })).toBeInTheDocument();
    const csirCard = screen
      .getByRole('heading', { name: 'CSIR-UGC NET Physical Sciences' })
      .closest('article');
    const gateCard = screen.getByRole('heading', { name: 'GATE Physics (PH)' }).closest('article');
    const tifrCard = screen
      .getByRole('heading', { name: 'TIFR Graduate School Physics' })
      .closest('article');
    const jestCard = screen.getByRole('heading', { name: 'JEST Physics' }).closest('article');
    expect(csirCard).not.toBeNull();
    expect(gateCard).not.toBeNull();
    expect(tifrCard).not.toBeNull();
    expect(jestCard).not.toBeNull();
    expect(within(csirCard!).getByText('Not set')).toBeInTheDocument();
    expect(within(gateCard!).getByText(/104 days remaining/)).toBeInTheDocument();
    expect(within(tifrCard!).getByText('Exam day')).toBeInTheDocument();
    expect(within(jestCard!).getByText('Exam date passed')).toBeInTheDocument();
    expect(screen.getAllByText('Active')).toHaveLength(4);
  });

  it('saves and clears a manually entered date', async () => {
    renderScreen();
    await screen.findByRole('heading', { name: 'Exam profiles' });

    const csirCard = screen
      .getByRole('heading', { name: 'CSIR-UGC NET Physical Sciences' })
      .closest('article');
    expect(csirCard).not.toBeNull();

    const input = screen.getByLabelText('CSIR-NET exam date');
    await userEvent.clear(input);
    await userEvent.type(input, '2027-06-15');
    await userEvent.click(within(csirCard!).getByRole('button', { name: 'Save date' }));

    expect(updateExamDate).toHaveBeenCalledWith(1, '2027-06-15');
    expect(await screen.findByText('CSIR-NET exam date saved.')).toBeInTheDocument();
    expect(screen.getByText(/225 days remaining/)).toBeInTheDocument();

    await userEvent.click(within(csirCard!).getByRole('button', { name: 'Clear date' }));
    expect(updateExamDate).toHaveBeenCalledWith(1, null);
    expect(await screen.findByText('CSIR-NET exam date cleared.')).toBeInTheDocument();
  });

  it('shows the legacy date notice and can confirm it', async () => {
    vi.mocked(getExamProfiles).mockResolvedValue([
      profile({
        legacyDateNoticeState: 'pending',
        legacyExamDate: '2026-12-19',
      }),
      ...profiles.slice(1),
    ]);
    vi.mocked(confirmLegacyExamDate).mockResolvedValue(
      profile({
        examDate: '2026-12-19',
        legacyDateNoticeState: 'transferred',
        legacyExamDate: null,
      }),
    );

    renderScreen();
    expect(await screen.findByText('Previous CSIR-NET date found')).toBeInTheDocument();
    expect(screen.getByText(/19 December 2026/)).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Use this legacy date' }));
    expect(confirmLegacyExamDate).toHaveBeenCalledWith(1);
    expect(
      await screen.findByText('The legacy CSIR-NET date was confirmed as a user-entered date.'),
    ).toBeInTheDocument();
  });

  it('dismisses the legacy notice without changing the date', async () => {
    vi.mocked(getExamProfiles).mockResolvedValue([
      profile({
        legacyDateNoticeState: 'pending',
        legacyExamDate: '2026-12-19',
      }),
      ...profiles.slice(1),
    ]);
    vi.mocked(dismissLegacyExamDateNotice).mockResolvedValue(
      profile({
        legacyDateNoticeState: 'dismissed',
        legacyExamDate: '2026-12-19',
      }),
    );

    renderScreen();
    await userEvent.click(await screen.findByRole('button', { name: 'Dismiss notice' }));
    expect(dismissLegacyExamDateNotice).toHaveBeenCalledWith(1);
    expect(
      await screen.findByText(
        'The legacy-date notice was dismissed. The original value remains preserved.',
      ),
    ).toBeInTheDocument();
    expect(screen.queryByText('Previous CSIR-NET date found')).not.toBeInTheDocument();
  });

  it('deactivates a profile and clears an inactive selected context', async () => {
    const { onSelectExam } = renderScreen(1);
    await screen.findByRole('heading', { name: 'Exam profiles' });

    const csirCard = screen
      .getByRole('heading', { name: 'CSIR-UGC NET Physical Sciences' })
      .closest('article');
    expect(csirCard).not.toBeNull();
    await userEvent.click(within(csirCard!).getByRole('button', { name: 'Deactivate' }));

    expect(setExamActive).toHaveBeenCalledWith(1, false);
    expect(onSelectExam).toHaveBeenCalledWith(null);
    expect(
      await screen.findByText('CSIR-NET deactivated. Historical records are preserved.'),
    ).toBeInTheDocument();
  });
});
