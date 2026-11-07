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
    },
  ],
  overdue: [],
  upcoming: [],
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
});
