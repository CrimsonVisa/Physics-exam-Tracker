import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2ExamSummary, V2RevisionDashboard } from './api';
import { RevisionScreen } from './RevisionScreen';

vi.mock('./api', () => ({
  getV2RevisionDashboard: vi.fn(),
  getV2RevisionDailyPlan: vi.fn(),
  generateV2RevisionDailyPlan: vi.fn(),
  rebalanceV2RevisionDailyPlan: vi.fn(),
  scheduleV2RevisionConcept: vi.fn(),
  rescheduleV2Revision: vi.fn(),
  completeV2Revision: vi.fn(),
  deferV2Revision: vi.fn(),
  moveV2RevisionPlanItem: vi.fn(),
  linkV2RevisionPlanBlock: vi.fn(),
  searchV2Concepts: vi.fn(),
}));

import * as api from './api';

const mockedDashboard = vi.mocked(api.getV2RevisionDashboard);
const mockedPlan = vi.mocked(api.getV2RevisionDailyPlan);
const mockedSchedule = vi.mocked(api.scheduleV2RevisionConcept);
const mockedReschedule = vi.mocked(api.rescheduleV2Revision);
const mockedGenerate = vi.mocked(api.generateV2RevisionDailyPlan);
const mockedComplete = vi.mocked(api.completeV2Revision);
const mockedSearch = vi.mocked(api.searchV2Concepts);

const EXAMS: V2ExamSummary[] = [
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
  },
];

const TODAY = '2026-11-04';

const DASHBOARD: V2RevisionDashboard = {
  settings: {
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
  },
  dueToday: [
    {
      conceptId: 7,
      conceptSlug: 'kinematics',
      conceptTitle: 'Kinematics',
      conceptDomain: 'Classical Mechanics',
      status: 'active',
      lastRevisionDate: null,
      nextRevisionDate: TODAY,
      currentIntervalDays: 5,
      completedRevisions: 0,
      previousOutcome: null,
      notes: '',
      overdueDays: 0,
      masteryScore: null,
      selectionReason: 'Due today; no mastery evidence; approved concept-to-syllabus mapping',
      eligible: true,
      approvedExamCodes: ['csir-net-physics'],
      pendingExamCodes: [],
      alreadyScheduled: false,
    },
  ],
  overdue: [],
  upcoming: [],
  awaitingApproval: [],
  recentHistory: [],
  stats: { dueToday: 1, overdue: 0, completedToday: 0, completedLast7Days: 0, totalCompleted: 0 },
  examId: null,
};

describe('V2 revision planning', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedDashboard.mockResolvedValue(DASHBOARD);
    mockedPlan.mockResolvedValue(null);
    mockedSearch.mockResolvedValue({
      concepts: [
        {
          id: 7,
          slug: 'kinematics',
          title: 'Kinematics',
          description: '',
          domain: 'Classical Mechanics',
          parentConceptId: null,
          parentSlug: null,
          parentTitle: null,
          difficulty: 1,
          isArchived: false,
          mastery: null,
          examCodes: [],
          verifiedExamCodes: [],
        },
      ],
      total: 1,
      domains: [],
      examCoverage: [],
    });
  });

  it('loads the due queue and completes a revision', async () => {
    const user = userEvent.setup();
    mockedComplete.mockResolvedValue(DASHBOARD);
    render(<RevisionScreen exams={EXAMS} today={TODAY} />);

    expect((await screen.findAllByText('Kinematics')).length).toBeGreaterThan(0);
    await user.click(screen.getByRole('button', { name: 'Complete' }));
    await waitFor(() =>
      expect(mockedComplete).toHaveBeenCalledWith({
        conceptId: 7,
        outcome: 'good',
        revisionDate: TODAY,
        triggerExamId: null,
      }),
    );
  });

  it('schedules a selected concept', async () => {
    const user = userEvent.setup();
    mockedSchedule.mockResolvedValue(DASHBOARD.dueToday[0]);
    render(<RevisionScreen exams={EXAMS} today={TODAY} />);
    expect((await screen.findAllByText('Kinematics')).length).toBeGreaterThan(0);

    await user.selectOptions(screen.getByLabelText('Schedule a shared concept'), '7');
    await user.click(screen.getByRole('button', { name: 'Schedule concept' }));
    await waitFor(() => expect(mockedSchedule).toHaveBeenCalledWith({ conceptId: 7 }));
  });

  it('reports the real next revision date and eligible mapping after scheduling', async () => {
    const user = userEvent.setup();
    mockedSchedule.mockResolvedValue({
      ...DASHBOARD.dueToday[0],
      nextRevisionDate: '2026-11-09',
      alreadyScheduled: false,
    });
    render(<RevisionScreen exams={EXAMS} today={TODAY} />);
    expect((await screen.findAllByText('Kinematics')).length).toBeGreaterThan(0);

    await user.selectOptions(screen.getByLabelText('Schedule a shared concept'), '7');
    await user.click(screen.getByRole('button', { name: 'Schedule concept' }));

    expect(
      await screen.findByText(
        'Concept added to the revision schedule. Next revision 2026-11-09. It is eligible for revision queues because its mapping is approved.',
      ),
    ).toBeInTheDocument();
  });

  it('explains that a proposed mapping must be approved before queues include the concept', async () => {
    const user = userEvent.setup();
    const pending = {
      ...DASHBOARD.dueToday[0],
      nextRevisionDate: '2026-11-09',
      eligible: false,
      approvedExamCodes: [],
      pendingExamCodes: ['csir-net-physics'],
    };
    mockedSchedule.mockResolvedValue(pending);
    mockedDashboard.mockResolvedValue({
      ...DASHBOARD,
      dueToday: [],
      awaitingApproval: [pending],
      stats: { ...DASHBOARD.stats, dueToday: 0 },
    });
    render(<RevisionScreen exams={EXAMS} today={TODAY} />);
    expect((await screen.findAllByText('Kinematics')).length).toBeGreaterThan(0);

    await user.selectOptions(screen.getByLabelText('Schedule a shared concept'), '7');
    await user.click(screen.getByRole('button', { name: 'Schedule concept' }));

    expect(
      await screen.findByText(/awaiting approval of its csir-net-physics syllabus mapping/u),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: 'Scheduled concepts without an approved mapping' }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Pending mapping: csir-net-physics/u)).toBeInTheDocument();
  });

  it('shows upcoming revisions and can move one into today plan', async () => {
    const user = userEvent.setup();
    const upcoming = {
      ...DASHBOARD.dueToday[0],
      conceptId: 8,
      conceptTitle: 'Wave optics',
      nextRevisionDate: '2026-11-09',
      selectionReason: 'Scheduled for 2026-11-09',
    };
    mockedDashboard.mockResolvedValue({
      ...DASHBOARD,
      dueToday: [],
      upcoming: [upcoming],
      stats: { ...DASHBOARD.stats, dueToday: 0 },
    });
    mockedReschedule.mockResolvedValue({ ...upcoming, nextRevisionDate: TODAY });
    mockedGenerate.mockResolvedValue({
      id: 1,
      planDate: TODAY,
      examId: null,
      version: 1,
      effectiveBudgetMinutes: 150,
      maxTopics: 5,
      plannedMinutes: 30,
      completedMinutes: 0,
      status: 'active',
      items: [
        {
          planItemId: 9,
          conceptId: 8,
          conceptTitle: 'Wave optics',
          conceptDomain: 'Optics',
          membershipStatus: 'planned',
          lane: 'maintain',
          triageScore: 40,
          estimatedMinutes: 30,
          sourceDueDate: TODAY,
          position: 0,
          plannedBlockId: null,
          plannedBlockTitle: null,
          selectionReason: 'Due today',
        },
      ],
    });
    render(<RevisionScreen exams={EXAMS} today={TODAY} />);

    expect(await screen.findByText('Wave optics')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Move to today' }));

    await waitFor(() =>
      expect(mockedReschedule).toHaveBeenCalledWith({
        conceptId: 8,
        nextRevisionDate: TODAY,
      }),
    );
    expect(mockedGenerate).toHaveBeenCalledWith({ planDate: TODAY, examId: null });
    expect(
      await screen.findByText('Revision moved to today and added to the daily plan.'),
    ).toBeInTheDocument();
  });

  it('reports a completed move when only the plan refresh fails', async () => {
    const user = userEvent.setup();
    const upcoming = {
      ...DASHBOARD.dueToday[0],
      conceptId: 8,
      conceptTitle: 'Wave optics',
      nextRevisionDate: '2026-11-09',
      selectionReason: 'Scheduled for 2026-11-09',
    };
    mockedDashboard.mockResolvedValue({
      ...DASHBOARD,
      dueToday: [],
      upcoming: [upcoming],
      stats: { ...DASHBOARD.stats, dueToday: 0 },
    });
    mockedReschedule.mockResolvedValue({ ...upcoming, nextRevisionDate: TODAY });
    mockedGenerate.mockRejectedValue(
      Object.assign(new Error('Revision plan could not be created.'), {
        code: 'command_failed',
      }),
    );
    render(<RevisionScreen exams={EXAMS} today={TODAY} />);

    expect(await screen.findByText('Wave optics')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Move to today' }));

    expect(
      await screen.findByText(
        'Revision moved to today, but the daily plan could not be refreshed: Revision plan could not be created.',
      ),
    ).toBeInTheDocument();
  });

  it('keeps a scheduling failure visible and does not claim success', async () => {
    const user = userEvent.setup();
    mockedSchedule.mockRejectedValue(
      Object.assign(new Error('Revision schedule could not be saved.'), {
        code: 'command_failed',
      }),
    );
    render(<RevisionScreen exams={EXAMS} today={TODAY} />);
    expect((await screen.findAllByText('Kinematics')).length).toBeGreaterThan(0);

    await user.selectOptions(screen.getByLabelText('Schedule a shared concept'), '7');
    await user.click(screen.getByRole('button', { name: 'Schedule concept' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Revision schedule could not be saved.',
    );
    expect(screen.queryByText(/Concept added to the revision schedule/u)).not.toBeInTheDocument();
  });
});
