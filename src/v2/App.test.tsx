import { render, screen, waitFor, within, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  V2ConceptSearchResult,
  V2DatabaseState,
  V2DatabaseStatus,
  V2ExamSummary,
  V2ExamSyllabus,
  V2StudyTotals,
  V2StickyNote,
  V2TimerSnapshot,
} from './api';
import V2App, { acknowledgeTimerFocusRequest } from './App';
import { retainViewAfterStatus } from './viewStatus';
import { SIDEBAR_MODE_STORAGE_KEY } from '../lib/uiPreferences';

vi.mock('./api', () => ({
  getV2DatabaseStatus: vi.fn(),
  initializeV2Database: vi.fn(),
  listV2Exams: vi.fn(),
  checkV2Integrity: vi.fn(),
  getV2Timer: vi.fn(),
  startV2Timer: vi.fn(),
  pauseV2Timer: vi.fn(),
  resumeV2Timer: vi.fn(),
  finishV2Timer: vi.fn(),
  discardV2Timer: vi.fn(),
  heartbeatV2Timer: vi.fn(),
  updateV2TimerSettings: vi.fn(),
  listV2Sessions: vi.fn(),
  setV2SessionAllocations: vi.fn(),
  getV2StudyTotals: vi.fn(),
  searchV2Concepts: vi.fn(),
  getV2ConceptDetail: vi.fn(),
  updateV2ConceptMastery: vi.fn(),
  replayV2ConceptMastery: vi.fn(),
  verifyV2MasteryRun: vi.fn(),
  getV2AppSettings: vi.fn(),
  updateV2AppSettings: vi.fn(),
  listV2ExamProfiles: vi.fn(),
  setV2ExamActive: vi.fn(),
  previewV2ExamDateChange: vi.fn(),
  applyV2ExamDateChange: vi.fn(),
  createV2ManualSession: vi.fn(),
  updateV2StudySession: vi.fn(),
  deleteV2StudySession: vi.fn(),
  getV2PlannerDay: vi.fn(),
  getV2Agenda: vi.fn(),
  saveV2Task: vi.fn(),
  setV2TaskStatus: vi.fn(),
  rescheduleV2Task: vi.fn(),
  deleteV2Task: vi.fn(),
  saveV2Block: vi.fn(),
  setV2BlockStatus: vi.fn(),
  linkV2BlockSession: vi.fn(),
  deleteV2Block: vi.fn(),
  listV2Resources: vi.fn(),
  saveV2Resource: vi.fn(),
  archiveV2Resource: vi.fn(),
  updateV2ResourceProgress: vi.fn(),
  linkV2ResourceExam: vi.fn(),
  importV2ResourceFile: vi.fn(),
  removeV2ResourceFile: vi.fn(),
  getV2BackupOverview: vi.fn(),
  updateV2BackupSettings: vi.fn(),
  createV2ManualBackup: vi.fn(),
  runV2AutomaticBackupIfDue: vi.fn(),
  previewV2RestoreBackup: vi.fn(),
  applyV2RestoreBackup: vi.fn(),
  getV2RestoreRecovery: vi.fn(),
  restoreV2RecoverySafety: vi.fn(),
  getV2ExamSyllabus: vi.fn(),
  updateV2SyllabusNodeProgress: vi.fn(),
  getV2Analytics: vi.fn(),
  getV2AnalyticsCharts: vi.fn(),
  getV2BuiltinConstants: vi.fn(),
  getV2RevisionDashboard: vi.fn(),
  updateV2RevisionSettings: vi.fn(),
  scheduleV2RevisionConcept: vi.fn(),
  rescheduleV2Revision: vi.fn(),
  setV2RevisionStatus: vi.fn(),
  completeV2Revision: vi.fn(),
  deferV2Revision: vi.fn(),
  getV2RevisionDailyPlan: vi.fn(),
  generateV2RevisionDailyPlan: vi.fn(),
  rebalanceV2RevisionDailyPlan: vi.fn(),
  moveV2RevisionPlanItem: vi.fn(),
  linkV2RevisionPlanBlock: vi.fn(),
  getV2DashboardSections: vi.fn(),
  saveV2DashboardSections: vi.fn(),
  resetV2DashboardSections: vi.fn(),
  getV2PracticeAnalytics: vi.fn(),
  globalSearchV2: vi.fn(),
  listV2Flashcards: vi.fn(),
  getV2FlashcardStats: vi.fn(),
  getV2FlashcardDetail: vi.fn(),
  setV2FlashcardArchived: vi.fn(),
  saveV2Flashcard: vi.fn(),
  getV2FlashcardReviewQueue: vi.fn(),
  reviewV2Flashcard: vi.fn(),
  listV2StickyNotes: vi.fn(),
  saveV2StickyNote: vi.fn(),
  deleteV2StickyNote: vi.fn(),
}));

import * as api from './api';

const mockedStatus = vi.mocked(api.getV2DatabaseStatus);
const mockedInitialize = vi.mocked(api.initializeV2Database);
const mockedTimer = vi.mocked(api.getV2Timer);
const mockedPauseTimer = vi.mocked(api.pauseV2Timer);
const mockedHeartbeat = vi.mocked(api.heartbeatV2Timer);
const mockedSessions = vi.mocked(api.listV2Sessions);
const mockedTotals = vi.mocked(api.getV2StudyTotals);
const mockedConceptSearch = vi.mocked(api.searchV2Concepts);
const mockedDashboardSections = vi.mocked(api.getV2DashboardSections);
const mockedSaveDashboardSections = vi.mocked(api.saveV2DashboardSections);
const mockedPlannerDay = vi.mocked(api.getV2PlannerDay);
const mockedRevisionDashboard = vi.mocked(api.getV2RevisionDashboard);
const mockedAppSettings = vi.mocked(api.getV2AppSettings);
const mockedStickyNotes = vi.mocked(api.listV2StickyNotes);
const mockedSyllabus = vi.mocked(api.getV2ExamSyllabus);
const TODAY = '2026-11-04';

function dashboardSections() {
  return [
    { sectionKey: 'overview', title: 'Database overview', visible: true, displayOrder: 0 },
    { sectionKey: 'study', title: 'Study timer and sessions', visible: true, displayOrder: 1 },
    { sectionKey: 'exams', title: 'Examination workspaces', visible: true, displayOrder: 2 },
    { sectionKey: 'database', title: 'V2 database details', visible: true, displayOrder: 3 },
  ];
}

const SMART_SECTION_TITLES: Record<string, string> = {
  quickstart: 'Quick start timer',
  today: "Today's overview",
  plan: "Today's study plan",
  revision: 'Revision overview',
  recent: 'Recently studied',
  performance: 'Performance and weak areas',
  overview: 'Database overview',
  study: 'Study timer and sessions',
  exams: 'Examination workspaces',
  database: 'V2 database details',
};

function smartSections(withPerformance = true): Array<{
  sectionKey: string;
  title: string;
  visible: boolean;
  displayOrder: number;
}> {
  const keys = Object.keys(SMART_SECTION_TITLES);
  return keys
    .filter((key) => withPerformance || key !== 'performance')
    .map((sectionKey, displayOrder) => ({
      sectionKey,
      title: SMART_SECTION_TITLES[sectionKey],
      visible: true,
      displayOrder,
    }));
}

function exam(overrides: Partial<V2ExamSummary> = {}): V2ExamSummary {
  return {
    id: 1,
    code: 'csir-net-physics',
    name: 'CSIR-UGC NET Physical Sciences',
    shortName: 'CSIR-NET',
    authority: 'CSIR-HRDG / NTA',
    officialUrl: 'https://csirnet.nta.nic.in/',
    displayOrder: 1,
    isActive: true,
    examDate: null,
    ...overrides,
  };
}

const EXAMS: V2ExamSummary[] = [
  exam(),
  exam({
    id: 2,
    code: 'gate-physics',
    name: 'GATE Physics (PH)',
    shortName: 'GATE PH',
    authority: 'IIT',
    officialUrl: 'https://gate.iitm.ac.in/',
    displayOrder: 2,
    examDate: '2026-11-06',
  }),
  exam({
    id: 3,
    code: 'tifr-gs-physics',
    name: 'TIFR Graduate School Physics',
    shortName: 'TIFR GS',
    authority: 'TIFR',
    officialUrl: 'https://www.tifr.res.in/academics/',
    displayOrder: 3,
    examDate: TODAY,
  }),
  exam({
    id: 4,
    code: 'jest-physics',
    name: 'JEST Physics',
    shortName: 'JEST',
    authority: 'JEST Consortium',
    officialUrl: 'https://www.jest.org.in/',
    displayOrder: 4,
    examDate: '2026-11-01',
  }),
];

const CSIR_FORMULA_SYLLABUS: V2ExamSyllabus = {
  examId: 1,
  examCode: 'csir-net-physics',
  examName: 'CSIR-UGC NET Physical Sciences',
  examIsActive: true,
  configured: true,
  contentVersionLabel: 'v1',
  contentReviewStatus: 'verified',
  nodes: [
    {
      id: 1,
      examId: 1,
      parentId: null,
      code: 'csir-part-a-core-electromagnetic-theory-t004-maxwell-equations-in-free-space-and-linear-isotrop',
      title: 'Maxwell equations in free space and linear isotropic media',
      nodeKind: 'topic',
      category: 'Electromagnetic Theory',
      displayOrder: 1,
      readinessWeight: 1,
      isArchived: false,
      sourceTitle: 'Official source',
      sourceVerifiedOn: '2026-11-04',
      sourceUrl: 'https://example.com/syllabus.pdf',
      sourceReference: 'Official source reference',
      contentVersionLabel: 'v1',
      contentReviewStatus: 'verified',
      progressStatus: 'not_started',
      priority: 'normal',
      confidence: 3,
      notes: '',
      nodePath: ['Maxwell equations in free space and linear isotropic media'],
    },
  ],
  summary: {
    nodeCount: 1,
    topicCount: 1,
    revisedOrConfidentCount: 0,
    mappedConceptCount: 0,
  },
};

function timerSnapshot(stateOverrides: Partial<V2TimerSnapshot['state']> = {}): V2TimerSnapshot {
  return {
    state: {
      status: 'idle',
      mode: 'focus',
      durationMs: 25 * 60_000,
      accumulatedMs: 0,
      elapsedMs: 0,
      remainingMs: 25 * 60_000,
      segmentStartedAt: null,
      sessionDate: null,
      activeExamId: null,
      recoveryReason: null,
      recoveryExcludedMs: 0,
      clockSkewDetected: false,
      lastEvent: null,
      lastEventAt: null,
      lastEventDurationMs: 0,
      lastCompletedSessionId: null,
      ...stateOverrides,
    },
    settings: {
      focusMinutes: 25,
      shortBreakMinutes: 5,
      longBreakMinutes: 15,
      notificationsEnabled: false,
    },
    allocations: [],
    serverNowMs: Date.parse('2026-11-04T10:00:00Z'),
  };
}

function runningTimerSnapshot(
  stateOverrides: Partial<V2TimerSnapshot['state']> = {},
): V2TimerSnapshot {
  return timerSnapshot({
    status: 'running',
    elapsedMs: 60_000,
    remainingMs: 24 * 60_000,
    segmentStartedAt: '2026-11-04T10:00:00.000Z',
    sessionDate: TODAY,
    ...stateOverrides,
  });
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function studyTotals(): V2StudyTotals {
  return {
    totalSeconds: 0,
    sessionCount: 0,
    allocatedSeconds: 0,
    unallocatedSeconds: 0,
    exams: EXAMS.map((exam) => ({
      examId: exam.id,
      examCode: exam.code,
      examName: exam.name,
      examIsActive: exam.isActive,
      allocatedSeconds: 0,
      sessionCount: 0,
    })),
    topics: [],
  };
}

function conceptSearch(): V2ConceptSearchResult {
  return {
    concepts: [],
    total: 0,
    domains: [],
    examCoverage: EXAMS.map((exam) => ({
      examId: exam.id,
      examCode: exam.code,
      examName: exam.name,
      examShortName: exam.shortName,
      examIsActive: exam.isActive,
      configured: false,
      status: 'not_configured',
      contentVersionLabel: null,
      contentReviewStatus: null,
      contentVerifiedOn: null,
      nodeCount: 0,
      topicNodeCount: 0,
      revisedNodeCount: 0,
      mappedConceptCount: 0,
      verifiedMappedConceptCount: 0,
    })),
  };
}

function status(overrides: Partial<V2DatabaseStatus> = {}): V2DatabaseStatus {
  const exams = overrides.exams ?? EXAMS;
  const active = exams.filter((item) => item.isActive).length;
  return {
    state: 'healthy',
    appIdentifier: 'com.crimsonvisa.physics-tracker',
    dataDir: 'C:\\Users\\test\\AppData\\Roaming\\com.crimsonvisa.physics-tracker',
    databasePath:
      'C:\\Users\\test\\AppData\\Roaming\\com.crimsonvisa.physics-tracker\\physics-tracker.sqlite3',
    schemaVersion: 1,
    expectedSchemaVersion: 1,
    metadataSchemaVersion: 1,
    databaseIdValid: true,
    migrationChecksumValid: true,
    initialized: true,
    foreignKeysEnabled: true,
    journalMode: 'wal',
    integrityCheck: 'ok',
    foreignKeyViolations: 0,
    examCount: exams.length,
    activeExamCount: active,
    exams,
    errorCode: null,
    errorMessage: null,
    ...overrides,
  };
}

function card(code: string): HTMLElement {
  const element = document.querySelector<HTMLElement>(`.v2-exam-card[data-exam-code="${code}"]`);
  if (!element) {
    throw new Error(`Missing exam card: ${code}`);
  }
  return element;
}

async function renderHealthy() {
  mockedStatus.mockResolvedValue(status());
  render(<V2App today={TODAY} />);
  await screen.findByRole('heading', { level: 1, name: 'Combined dashboard' });
  await screen.findByText(/No study sessions yet/);
}

describe('V2 application shell', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedInitialize.mockResolvedValue(status());
    mockedTimer.mockResolvedValue(timerSnapshot());
    mockedHeartbeat.mockResolvedValue(timerSnapshot());
    mockedSessions.mockResolvedValue([]);
    mockedTotals.mockResolvedValue(studyTotals());
    mockedConceptSearch.mockResolvedValue(conceptSearch());
    mockedStickyNotes.mockResolvedValue([]);
    mockedDashboardSections.mockResolvedValue(dashboardSections());
    mockedSaveDashboardSections.mockImplementation(async (input) =>
      input.sections.map((section) => ({
        ...section,
        title:
          dashboardSections().find((candidate) => candidate.sectionKey === section.sectionKey)
            ?.title ?? section.sectionKey,
      })),
    );
  });

  it('toggles and saves dashboard section visibility', async () => {
    const user = userEvent.setup();
    await renderHealthy();

    await user.click(screen.getByText('Customize dashboard'));
    await user.click(screen.getByRole('checkbox', { name: 'Study timer and sessions' }));
    await user.click(screen.getByRole('button', { name: 'Save dashboard layout' }));

    expect(mockedSaveDashboardSections).toHaveBeenCalledTimes(1);
    const saved = mockedSaveDashboardSections.mock.calls[0][0];
    expect(saved.examId).toBeNull();
    expect(saved.sections.find((section) => section.sectionKey === 'study')?.visible).toBe(false);
  });

  it('renders the smart daily dashboard sections with combined daily data', async () => {
    mockedDashboardSections.mockResolvedValue(smartSections(false));
    mockedPlannerDay.mockResolvedValue({ date: TODAY, tasks: [], blocks: [] });
    mockedRevisionDashboard.mockResolvedValue({
      settings: {
        initialAgainDays: 1,
        initialDifficultDays: 2,
        initialGoodDays: 4,
        initialEasyDays: 7,
        growthMultiplier: 2,
        minimumIntervalDays: 1,
        maximumIntervalDays: 180,
        includeWeekends: true,
        dailyRevisionTarget: 5,
        remindersEnabled: false,
      },
      dueToday: [],
      overdue: [],
      upcoming: [],
      recentHistory: [],
      stats: {
        dueToday: 0,
        overdue: 0,
        completedToday: 0,
        completedLast7Days: 0,
        totalCompleted: 0,
      },
      examId: null,
    });
    mockedAppSettings.mockResolvedValue({
      preparationStartDate: null,
      dailyStudyTargetMinutes: 90,
    });

    await renderHealthy();

    for (const title of [
      "Start today's timer",
      "Today's overview",
      "Today's study plan",
      'Revision overview',
      'Recently studied',
    ]) {
      expect(screen.getByRole('heading', { level: 2, name: title })).toBeInTheDocument();
    }
    expect(mockedPlannerDay).toHaveBeenCalledWith(TODAY);
    expect(mockedRevisionDashboard).toHaveBeenCalledWith({ examId: null, today: TODAY });
    expect(screen.getByText(/Daily target: 0 of 90 min/u)).toBeInTheDocument();
    expect(
      screen.getByText('No revisions are due today and none are overdue.'),
    ).toBeInTheDocument();

    await userEvent.setup().click(screen.getByRole('button', { name: 'Open the study timer' }));
  });

  it('persists the full smart-section layout when saving customization', async () => {
    mockedDashboardSections.mockResolvedValue(smartSections(true));
    await renderHealthy();

    const user = userEvent.setup();
    await user.click(screen.getByText('Customize dashboard'));
    await user.click(screen.getByRole('checkbox', { name: 'Performance and weak areas' }));
    await user.click(screen.getByRole('button', { name: 'Save dashboard layout' }));

    expect(mockedSaveDashboardSections).toHaveBeenCalledTimes(1);
    const saved = mockedSaveDashboardSections.mock.calls[0][0];
    expect(saved.examId).toBeNull();
    expect(saved.sections).toHaveLength(10);
    expect(saved.sections.map((section) => section.sectionKey)).toEqual([
      'quickstart',
      'today',
      'plan',
      'revision',
      'recent',
      'performance',
      'overview',
      'study',
      'exams',
      'database',
    ]);
    expect(saved.sections.find((section) => section.sectionKey === 'performance')?.visible).toBe(
      false,
    );
  });

  it('renders the combined dashboard with exactly four exam workspaces', async () => {
    await renderHealthy();

    expect(document.querySelectorAll('.v2-exam-card')).toHaveLength(4);
    expect(document.querySelectorAll('.v2-exam-nav')).toHaveLength(4);
    expect(screen.getByText('CSIR-UGC NET Physical Sciences')).toBeInTheDocument();
    expect(screen.getByText('GATE Physics (PH)')).toBeInTheDocument();
    expect(screen.getByText('TIFR Graduate School Physics')).toBeInTheDocument();
    expect(screen.getByText('JEST Physics')).toBeInTheDocument();
    expect(screen.queryByText(/%/)).not.toBeInTheDocument();
  });

  it('navigates to the shared concept library from the sidebar', async () => {
    const user = userEvent.setup();
    await renderHealthy();

    await user.click(screen.getByRole('button', { name: 'Concept library' }));

    expect(
      await screen.findByRole('heading', { level: 1, name: 'Concept library' }),
    ).toBeInTheDocument();
    expect(mockedConceptSearch).toHaveBeenCalled();
    expect(
      screen.getByText(
        'No concepts match these filters. Clear the search or choose a different examination.',
      ),
    ).toBeInTheDocument();
  });

  it('navigates to the flashcards workspace from the sidebar', async () => {
    const user = userEvent.setup();
    vi.mocked(api.listV2Flashcards).mockResolvedValue({
      cards: [],
      total: 0,
      subjects: [],
    });
    vi.mocked(api.getV2FlashcardStats).mockResolvedValue({
      totalCards: 0,
      dueCards: 0,
      reviewedCards: 0,
      newCards: 0,
      totalReviews: 0,
      reviewsLast30Days: 0,
      successfulLast30Days: 0,
      retentionPercent: null,
    });
    await renderHealthy();

    await user.click(screen.getByRole('button', { name: 'Flashcards' }));

    expect(
      await screen.findByRole('heading', { level: 1, name: 'Flashcards' }),
    ).toBeInTheDocument();
    expect(document.querySelector('.v2-app-frame')).toHaveAttribute('data-workspace', 'flashcards');
  });

  it('navigates to the calculator workspace from the sidebar', async () => {
    const user = userEvent.setup();
    await renderHealthy();

    await user.click(screen.getByRole('button', { name: 'Scientific calculator' }));

    expect(
      await screen.findByRole('heading', { level: 1, name: 'Scientific calculator' }),
    ).toBeInTheDocument();
    expect(document.querySelector('.v2-app-frame')).toHaveAttribute('data-workspace', 'calculator');
    expect(screen.queryByRole('combobox', { name: 'Calculator mode' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sine' })).toBeEnabled();
  });

  it('navigates to the formula reference from the sidebar', async () => {
    const user = userEvent.setup();
    await renderHealthy();

    await user.click(screen.getByRole('button', { name: 'Formula reference' }));

    expect(
      await screen.findByRole(
        'heading',
        { level: 1, name: 'Physics Formula Reference' },
        { timeout: 4000 },
      ),
    ).toBeInTheDocument();
    expect(document.querySelector('.v2-app-frame')).toHaveAttribute('data-workspace', 'formulas');
    expect(screen.getByText('47 formulas')).toBeInTheDocument();
  });

  it('navigates to the terms and units reference from the sidebar', async () => {
    const user = userEvent.setup();
    await renderHealthy();

    await user.click(screen.getByRole('button', { name: 'Physics terms and SI units' }));

    expect(
      await screen.findByRole(
        'heading',
        { level: 1, name: 'Physics Terms & SI Units' },
        { timeout: 4000 },
      ),
    ).toBeInTheDocument();
    expect(document.querySelector('.v2-app-frame')).toHaveAttribute('data-workspace', 'terms');
    expect(screen.getByText('123 terms')).toBeInTheDocument();
  });

  it('opens the terms reference filtered to an examination workspace', async () => {
    const user = userEvent.setup();
    await renderHealthy();

    await user.click(within(card('gate-physics')).getByRole('button', { name: 'Open workspace' }));
    await user.click(
      await screen.findByRole('button', {
        name: 'Browse terms and units for this exam',
      }),
    );

    expect(
      await screen.findByRole(
        'heading',
        { level: 1, name: 'Physics Terms & SI Units' },
        { timeout: 4000 },
      ),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Examination filter')).toHaveValue('gate-physics');
  });

  it('round-trips from a formula to related terms and back', async () => {
    const user = userEvent.setup();
    await renderHealthy();

    await user.click(screen.getByRole('button', { name: 'Formula reference' }));
    await user.click(
      await screen.findByRole('button', {
        name: "Open formula Newton's second law (momentum form)",
      }),
    );
    await user.click(await screen.findByRole('button', { name: 'Browse related terms and units' }));

    expect(await screen.findByText(/Showing terms related to/)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Open term linear momentum' }));
    await user.click(screen.getByRole('button', { name: "Newton's second law (momentum form)" }));

    expect(
      await screen.findByRole('heading', {
        level: 1,
        name: "Newton's second law (momentum form)",
      }),
    ).toBeInTheDocument();
  });

  it('preserves calculator session state across workspace navigation', async () => {
    const user = userEvent.setup();
    await renderHealthy();

    await user.click(screen.getByRole('button', { name: 'Scientific calculator' }));
    await user.click(screen.getByRole('button', { name: 'Radians' }));
    await user.click(screen.getByRole('button', { name: 'Two' }));
    await user.click(screen.getByRole('button', { name: 'Add' }));
    await user.click(screen.getByRole('button', { name: 'Three' }));
    await user.click(screen.getByRole('button', { name: 'Equals' }));
    await user.click(screen.getByRole('button', { name: 'Memory store' }));

    await user.click(screen.getByRole('button', { name: 'Combined dashboard' }));
    await user.click(screen.getByRole('button', { name: 'Scientific calculator' }));

    expect(screen.getByRole('button', { name: 'Radians' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByLabelText('Memory stored')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Reuse expression 2+3' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Previous answer' }));
    await user.click(screen.getByRole('button', { name: 'Equals' }));
    expect(screen.getByLabelText('Result 5')).toBeInTheDocument();
  });

  it('keeps global shortcuts working while the calculator is focused', async () => {
    const user = userEvent.setup();
    await renderHealthy();
    await user.click(screen.getByRole('button', { name: 'Scientific calculator' }));
    const root = document.querySelector('.v2-calculator-page');
    expect(root).toBeInstanceOf(HTMLElement);
    (root as HTMLElement).focus();

    fireEvent.keyDown(root as HTMLElement, { bubbles: true, ctrlKey: true, key: 'k' });
    expect(await screen.findByRole('dialog', { name: 'Global search' })).toBeInTheDocument();

    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: 'Global search' })).not.toBeInTheDocument();

    fireEvent.keyDown(root as HTMLElement, { bubbles: true, ctrlKey: true, key: '2' });
    expect(
      await screen.findByRole('heading', { level: 1, name: 'GATE Physics (PH)' }),
    ).toBeInTheDocument();
  });

  it('keeps the active timer indicator visible across workspaces', async () => {
    const user = userEvent.setup();
    mockedTimer.mockResolvedValue(runningTimerSnapshot({ activeExamId: null }));
    await renderHealthy();

    expect(screen.getByRole('button', { name: /Focus timer running/ })).toBeInTheDocument();

    for (const name of [
      'Concept library',
      'Exam syllabus',
      'GATE Physics (PH)',
      'Practice systems',
      'Flashcards',
      'Scientific calculator',
      'Formula reference',
      'Physics terms and SI units',
    ]) {
      await user.click(screen.getByRole('button', { name }));
      expect(screen.getByRole('button', { name: /Focus timer running/ })).toBeInTheDocument();
    }
  });

  it('pauses the running timer from the single floating dock outside route content', async () => {
    const user = userEvent.setup();
    mockedTimer.mockResolvedValue(runningTimerSnapshot({ activeExamId: null }));
    mockedPauseTimer.mockResolvedValue(
      runningTimerSnapshot({
        status: 'paused',
        segmentStartedAt: null,
        elapsedMs: 60_000,
        remainingMs: 24 * 60_000,
      }),
    );
    await renderHealthy();

    const openButton = screen.getByRole('button', { name: /Focus timer running/ });
    expect(screen.getAllByRole('button', { name: /Focus timer running/ })).toHaveLength(1);
    const dock = openButton.closest('.v2-global-timer-dock');
    expect(dock).not.toBeNull();
    expect(dock?.closest('#main-content')).toBeNull();

    await user.click(screen.getByRole('button', { name: 'Pause study timer' }));

    expect(mockedPauseTimer).toHaveBeenCalledTimes(1);
    expect(await screen.findByRole('button', { name: /Focus timer paused/ })).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: /Focus timer paused/ })).toHaveLength(1);
  });

  it('shows a paused timer across workspace navigation', async () => {
    const user = userEvent.setup();
    mockedTimer.mockResolvedValue(
      runningTimerSnapshot({
        status: 'paused',
        segmentStartedAt: null,
        elapsedMs: 90_000,
        remainingMs: 25 * 60_000 - 90_000,
      }),
    );
    await renderHealthy();

    expect(screen.getByRole('button', { name: /Focus timer paused/ })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Scientific calculator' }));
    expect(screen.getByRole('button', { name: /Focus timer paused/ })).toBeInTheDocument();
  });

  it('opens the sticky notes page and the global quick panel', async () => {
    const user = userEvent.setup();
    await renderHealthy();

    await user.click(screen.getByRole('button', { name: 'Sticky notes' }));
    expect(
      await screen.findByRole('heading', { level: 1, name: 'Sticky notes' }),
    ).toBeInTheDocument();
    expect(await screen.findByText('No sticky notes yet')).toBeInTheDocument();
    expect(mockedStickyNotes).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole('button', { name: 'Open quick notes' }));
    const quickPanel = await screen.findByRole('dialog', { name: 'Sticky notes' });
    expect(within(quickPanel).getByRole('button', { name: 'New note' })).toBeInTheDocument();
    expect(mockedStickyNotes).toHaveBeenCalledTimes(1);
  });

  it('expands quick notes into the full page with the selected note preserved', async () => {
    const user = userEvent.setup();
    const stickyNote: V2StickyNote = {
      id: 8,
      title: 'Expandable note',
      body: 'Keep this selection when opening the full page.',
      color: 'yellow',
      isPinned: false,
      examId: null,
      examCode: null,
      subjectNodeId: null,
      subjectTitle: null,
      topicNodeId: null,
      topicTitle: null,
      createdAt: '2026-11-10T10:00:00Z',
      updatedAt: '2026-11-10T11:00:00Z',
    };
    mockedStickyNotes.mockResolvedValue([stickyNote]);
    await renderHealthy();

    await user.click(screen.getByRole('button', { name: 'Open quick notes' }));
    const quickPanel = await screen.findByRole('dialog', { name: 'Sticky notes' });
    expect(await within(quickPanel).findByLabelText('Title')).toHaveValue('Expandable note');

    await user.click(within(quickPanel).getByRole('button', { name: 'Open full page' }));

    expect(screen.queryByRole('dialog', { name: 'Sticky notes' })).not.toBeInTheDocument();
    expect(
      await screen.findByRole('heading', { level: 1, name: 'Sticky notes' }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Title')).toHaveValue('Expandable note');
  });

  it('keeps Notes active and the selected editor available after status reconciliation', async () => {
    const user = userEvent.setup();
    const stickyNote: V2StickyNote = {
      id: 7,
      title: 'Status note',
      body: 'Keep this editor open.',
      color: 'yellow',
      isPinned: false,
      examId: null,
      examCode: null,
      subjectNodeId: null,
      subjectTitle: null,
      topicNodeId: null,
      topicTitle: null,
      createdAt: '2026-11-10T10:00:00Z',
      updatedAt: '2026-11-10T10:00:00Z',
    };
    mockedStickyNotes.mockResolvedValue([stickyNote]);
    await renderHealthy();

    await user.click(screen.getByRole('button', { name: 'Sticky notes' }));
    await user.click(await screen.findByRole('button', { name: 'Open sticky note Status note' }));
    expect(screen.getByRole('heading', { name: 'Edit note' })).toBeInTheDocument();

    expect(retainViewAfterStatus({ kind: 'notes' }, status().exams)).toEqual({ kind: 'notes' });
    expect(screen.getByLabelText('Title')).toHaveValue('Status note');
  });

  it('does not show the indicator for an idle timer', async () => {
    await renderHealthy();

    expect(screen.queryByRole('button', { name: /timer running/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /timer paused/i })).not.toBeInTheDocument();
  });

  it('does not show the indicator for a completed timer', async () => {
    mockedTimer.mockResolvedValue(
      timerSnapshot({ lastEvent: 'completed', lastCompletedSessionId: 4 }),
    );
    await renderHealthy();

    expect(screen.queryByRole('button', { name: /timer running/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /timer paused/i })).not.toBeInTheDocument();
  });

  it('opens the active exam timer and focuses the timer heading', async () => {
    const user = userEvent.setup();
    const scrollIntoView = vi.fn();
    const originalScrollIntoView = HTMLElement.prototype.scrollIntoView;
    HTMLElement.prototype.scrollIntoView = scrollIntoView;
    mockedTimer.mockResolvedValue(runningTimerSnapshot({ activeExamId: 2 }));

    try {
      await renderHealthy();
      await user.click(screen.getByRole('button', { name: 'Scientific calculator' }));
      await user.click(screen.getByRole('button', { name: /Focus timer running/ }));

      expect(
        await screen.findByRole('heading', { level: 1, name: 'GATE Physics (PH)' }),
      ).toBeInTheDocument();
      const heading = await screen.findByRole('heading', {
        level: 2,
        name: 'Timer and sessions',
      });
      await waitFor(() => expect(scrollIntoView).toHaveBeenCalled());
      await waitFor(() => expect(document.activeElement).toBe(heading));
    } finally {
      HTMLElement.prototype.scrollIntoView = originalScrollIntoView;
    }
  });

  it('does not clear a newer focus request with an old acknowledgement', () => {
    expect(acknowledgeTimerFocusRequest(2, 1)).toBe(2);
    expect(acknowledgeTimerFocusRequest(2, 2)).toBe(0);
  });

  it('cancels a stale focus request when navigating away before readiness', async () => {
    const user = userEvent.setup();
    const preferences = deferred<Awaited<ReturnType<typeof dashboardSections>>>();
    const scrollIntoView = vi.fn();
    const originalScrollIntoView = HTMLElement.prototype.scrollIntoView;
    HTMLElement.prototype.scrollIntoView = scrollIntoView;
    mockedDashboardSections.mockReturnValue(preferences.promise);
    mockedTimer.mockResolvedValue(runningTimerSnapshot({ activeExamId: null }));

    try {
      await renderHealthy();
      await user.click(screen.getByRole('button', { name: 'Scientific calculator' }));
      await user.click(screen.getByRole('button', { name: /Focus timer running/ }));

      await user.click(screen.getByRole('button', { name: 'Scientific calculator' }));
      preferences.resolve(dashboardSections());
      await screen.findByRole('heading', { level: 1, name: 'Scientific calculator' });
      expect(scrollIntoView).not.toHaveBeenCalled();

      await user.click(screen.getByRole('button', { name: 'Combined dashboard' }));
      await screen.findByRole('heading', { level: 1, name: 'Combined dashboard' });
      expect(scrollIntoView).not.toHaveBeenCalled();

      await user.click(screen.getByRole('button', { name: /Focus timer running/ }));
      await waitFor(() => expect(scrollIntoView).toHaveBeenCalled());
    } finally {
      HTMLElement.prototype.scrollIntoView = originalScrollIntoView;
    }
  });

  it('waits for dashboard preferences before focusing a visible timer section', async () => {
    const user = userEvent.setup();
    const preferences = deferred<Awaited<ReturnType<typeof dashboardSections>>>();
    const scrollIntoView = vi.fn();
    const originalScrollIntoView = HTMLElement.prototype.scrollIntoView;
    HTMLElement.prototype.scrollIntoView = scrollIntoView;
    mockedDashboardSections.mockReturnValue(preferences.promise);
    mockedTimer.mockResolvedValue(runningTimerSnapshot({ activeExamId: null }));

    try {
      await renderHealthy();
      await user.click(screen.getByRole('button', { name: /Focus timer running/ }));
      expect(scrollIntoView).not.toHaveBeenCalled();

      preferences.resolve(dashboardSections());
      await waitFor(() => expect(scrollIntoView).toHaveBeenCalled());
    } finally {
      HTMLElement.prototype.scrollIntoView = originalScrollIntoView;
    }
  });

  it('waits for dashboard preferences before reporting a hidden timer section', async () => {
    const user = userEvent.setup();
    const preferences = deferred<Awaited<ReturnType<typeof dashboardSections>>>();
    mockedStatus.mockResolvedValue(status());
    mockedDashboardSections.mockReturnValue(preferences.promise);
    mockedTimer.mockResolvedValue(runningTimerSnapshot({ activeExamId: null }));

    render(<V2App today={TODAY} />);
    await screen.findByRole('heading', { level: 1, name: 'Combined dashboard' });

    await user.click(await screen.findByRole('button', { name: /Focus timer running/ }));
    expect(screen.queryByText(/study timer section is hidden/i)).not.toBeInTheDocument();

    preferences.resolve(dashboardSections().filter((section) => section.sectionKey !== 'study'));

    expect(
      await screen.findByText(
        /The study timer section is hidden\. Enable it under Customize dashboard/,
      ),
    ).toBeInTheDocument();
  });

  it('guides to dashboard customization when the study section is hidden', async () => {
    const user = userEvent.setup();
    mockedStatus.mockResolvedValue(status());
    mockedDashboardSections.mockResolvedValue(
      dashboardSections().filter((section) => section.sectionKey !== 'study'),
    );
    mockedTimer.mockResolvedValue(runningTimerSnapshot({ activeExamId: null }));

    render(<V2App today={TODAY} />);
    await screen.findByRole('heading', { level: 1, name: 'Combined dashboard' });
    await waitFor(() => expect(document.getElementById('study-timer-heading')).toBeNull());

    await user.click(screen.getByRole('button', { name: /Focus timer running/ }));

    expect(
      await screen.findByText(
        /The study timer section is hidden\. Enable it under Customize dashboard/,
      ),
    ).toBeInTheDocument();
  });

  it('falls back to an allocation exam when the active exam id is invalid', async () => {
    const user = userEvent.setup();
    const snapshot = runningTimerSnapshot({ activeExamId: 99 });
    snapshot.allocations = [
      {
        examId: 2,
        examCode: 'gate-physics',
        examName: 'GATE Physics (PH)',
        nodeId: 1,
        nodeTitle: 'Topic',
        subjectTitle: 'Subject',
        allocatedSeconds: 600,
      },
    ];
    mockedTimer.mockResolvedValue(snapshot);

    await renderHealthy();
    await user.click(screen.getByRole('button', { name: 'Scientific calculator' }));
    await user.click(screen.getByRole('button', { name: /Focus timer running/ }));

    expect(
      await screen.findByRole('heading', { level: 1, name: 'GATE Physics (PH)' }),
    ).toBeInTheDocument();
  });

  it('falls back to the dashboard when no exam candidate is valid', async () => {
    const user = userEvent.setup();
    mockedTimer.mockResolvedValue(runningTimerSnapshot({ activeExamId: 99 }));

    await renderHealthy();
    await user.click(screen.getByRole('button', { name: 'Scientific calculator' }));
    await user.click(screen.getByRole('button', { name: /Focus timer running/ }));

    expect(
      await screen.findByRole('heading', { level: 1, name: 'Combined dashboard' }),
    ).toBeInTheDocument();
  });

  it('shows missing, future, today, and passed exam date states', async () => {
    await renderHealthy();

    expect(within(card('csir-net-physics')).getByText('Not set')).toBeInTheDocument();
    expect(within(card('gate-physics')).getByText('2 days remaining')).toBeInTheDocument();
    expect(within(card('tifr-gs-physics')).getByText('Exam day')).toBeInTheDocument();
    expect(within(card('jest-physics')).getByText('Exam date passed')).toBeInTheDocument();
  });

  it('navigates to an exam workspace by stable exam code and back', async () => {
    const user = userEvent.setup();
    await renderHealthy();

    await user.click(within(card('gate-physics')).getByRole('button', { name: 'Open workspace' }));

    expect(
      await screen.findByRole('heading', { level: 1, name: 'GATE Physics (PH)' }),
    ).toBeInTheDocument();
    expect(screen.getByText('gate-physics')).toBeInTheDocument();
    expect(screen.getByText('IIT')).toBeInTheDocument();
    expect(
      screen.getByText(
        /It shows the examination identity, activation state, shared timer, and exam-specific syllabus progress/,
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/workflows will be added in later milestones/),
    ).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: '← Back to combined dashboard' }));
    expect(
      await screen.findByRole('heading', { level: 1, name: 'Combined dashboard' }),
    ).toBeInTheDocument();
  });

  it('opens the formula reference filtered to an examination workspace', async () => {
    const user = userEvent.setup();
    await renderHealthy();

    await user.click(within(card('gate-physics')).getByRole('button', { name: 'Open workspace' }));
    await user.click(
      await screen.findByRole('button', {
        name: 'Browse formula reference for this exam',
      }),
    );

    expect(
      await screen.findByRole(
        'heading',
        { level: 1, name: 'Physics Formula Reference' },
        { timeout: 4000 },
      ),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Examination filter')).toHaveValue('gate-physics');
  });

  it('round-trips from a formula to its syllabus node and back', async () => {
    const user = userEvent.setup();
    mockedSyllabus.mockResolvedValue(CSIR_FORMULA_SYLLABUS);
    await renderHealthy();

    await user.click(screen.getByRole('button', { name: 'Formula reference' }));
    await user.click(
      await screen.findByRole('button', {
        name: 'Open formula Maxwell-Faraday law (differential form)',
      }),
    );
    await user.click(
      await screen.findByRole('button', {
        name: 'Open syllabus topic Maxwell equations in free space and linear isotropic media',
      }),
    );

    expect(
      await screen.findByRole('heading', { level: 1, name: 'Exam syllabus' }),
    ).toBeInTheDocument();
    await user.click(
      await screen.findByRole('button', {
        name: 'Open formulas for Maxwell equations in free space and linear isotropic media',
      }),
    );

    expect(await screen.findByText(/Showing formulas linked to/)).toBeInTheDocument();
    expect(screen.getByText('1 formula')).toBeInTheDocument();
    expect(screen.getByLabelText('Examination filter')).toHaveValue('csir-net-physics');
  });

  it('resolves workspaces by code even when the API order changes', async () => {
    const user = userEvent.setup();
    mockedStatus.mockResolvedValue(status({ exams: [EXAMS[3], EXAMS[0], EXAMS[1], EXAMS[2]] }));
    render(<V2App today={TODAY} />);
    await screen.findByRole('heading', { level: 1, name: 'Combined dashboard' });

    await user.click(within(card('jest-physics')).getByRole('button', { name: 'Open workspace' }));

    expect(
      await screen.findByRole('heading', { level: 1, name: 'JEST Physics' }),
    ).toBeInTheDocument();
    expect(screen.getByText('jest-physics')).toBeInTheDocument();
  });

  it('keeps inactive exams visible and opens a read-only historical workspace', async () => {
    const user = userEvent.setup();
    const exams = EXAMS.map((item) =>
      item.code === 'tifr-gs-physics' ? { ...item, isActive: false } : item,
    );
    mockedStatus.mockResolvedValue(status({ exams }));
    render(<V2App today={TODAY} />);
    await screen.findByRole('heading', { level: 1, name: 'Combined dashboard' });

    expect(within(card('tifr-gs-physics')).getByText('Inactive')).toBeInTheDocument();
    await user.click(
      within(card('tifr-gs-physics')).getByRole('button', {
        name: 'Open historical workspace',
      }),
    );

    expect(
      await screen.findByRole('heading', { name: 'Inactive examination' }),
    ).toBeInTheDocument();
    const workspace = document.querySelector<HTMLElement>('.v2-workspace');
    expect(workspace).not.toBeNull();
    expect(within(workspace as HTMLElement).getByText('Inactive')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /edit|save|set date/i })).not.toBeInTheDocument();
  });

  it('auto-initializes a not-initialized database once', async () => {
    mockedStatus.mockResolvedValue(status({ state: 'notInitialized', initialized: false }));
    mockedInitialize.mockResolvedValue(status());
    render(<V2App today={TODAY} />);

    expect(
      await screen.findByRole('heading', { level: 1, name: 'Combined dashboard' }),
    ).toBeInTheDocument();
    expect(mockedInitialize).toHaveBeenCalledTimes(1);
  });

  it('shows the not-initialized state when initialization still cannot complete', async () => {
    mockedStatus.mockResolvedValue(status({ state: 'notInitialized', initialized: false }));
    mockedInitialize.mockResolvedValue(status({ state: 'notInitialized', initialized: false }));
    render(<V2App today={TODAY} />);

    expect(
      await screen.findByRole('heading', { level: 1, name: 'V2 database not initialized' }),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Initialize database' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Retry status check' })).toBeInTheDocument();
  });

  it('shows an understandable bootstrap error without falling back to V1', async () => {
    mockedStatus.mockRejectedValue(new Error('database unavailable'));
    render(<V2App today={TODAY} />);

    expect(
      await screen.findByRole('heading', {
        level: 1,
        name: 'Physics Tracker could not read its V2 database',
      }),
    ).toBeInTheDocument();
    expect(document.querySelectorAll('.v2-exam-card')).toHaveLength(0);
    expect(screen.queryByText('CSIR-UGC NET Physical Sciences')).not.toBeInTheDocument();
  });

  const unhealthyCases: Array<[V2DatabaseState, string]> = [
    ['unsupportedSchema', 'Database schema is newer than this application'],
    ['metadataMismatch', 'Database metadata mismatch'],
    ['integrityFailure', 'Database integrity check failed'],
    ['foreignKeyFailure', 'Database relationship check failed'],
    ['inaccessible', 'V2 database inaccessible'],
  ];

  it.each(unhealthyCases)('renders the %s database state as unhealthy', async (state, title) => {
    mockedStatus.mockResolvedValue(
      status({
        state,
        initialized: false,
        errorMessage: `database state: ${state}`,
      }),
    );
    render(<V2App today={TODAY} />);

    expect(await screen.findByRole('heading', { level: 1, name: title })).toBeInTheDocument();
    expect(document.querySelectorAll('.v2-exam-card')).toHaveLength(0);
  });

  it('reports the specified database state and schema details', async () => {
    mockedStatus.mockResolvedValue(
      status({
        state: 'integrityFailure',
        initialized: false,
        schemaVersion: 1,
        expectedSchemaVersion: 1,
        errorMessage: 'quick_check failed',
      }),
    );
    render(<V2App today={TODAY} />);

    expect(await screen.findByText('integrityFailure')).toBeInTheDocument();
    expect(screen.getByText('quick_check failed')).toBeInTheDocument();
    expect(screen.getByText(/1 \/ expected 1/)).toBeInTheDocument();
  });

  it('collapses and expands the sidebar while persisting the preference', async () => {
    localStorage.removeItem(SIDEBAR_MODE_STORAGE_KEY);
    const user = userEvent.setup();
    await renderHealthy();

    const frame = document.querySelector<HTMLElement>('.v2-app-frame');
    expect(frame?.getAttribute('data-sidebar-mode')).toBe('expanded');
    expect(localStorage.getItem(SIDEBAR_MODE_STORAGE_KEY)).toBe('expanded');

    await user.click(screen.getByRole('button', { name: 'Collapse sidebar' }));
    expect(frame?.getAttribute('data-sidebar-mode')).toBe('collapsed');
    expect(screen.getByRole('button', { name: 'Expand sidebar' })).toHaveAttribute(
      'aria-expanded',
      'false',
    );
    expect(localStorage.getItem(SIDEBAR_MODE_STORAGE_KEY)).toBe('collapsed');

    await user.click(screen.getByRole('button', { name: 'Expand sidebar' }));
    expect(frame?.getAttribute('data-sidebar-mode')).toBe('expanded');
    expect(localStorage.getItem(SIDEBAR_MODE_STORAGE_KEY)).toBe('expanded');

    localStorage.removeItem(SIDEBAR_MODE_STORAGE_KEY);
  });

  it('restores a collapsed sidebar from the stored preference', async () => {
    localStorage.setItem(SIDEBAR_MODE_STORAGE_KEY, 'collapsed');
    mockedStatus.mockResolvedValue(status());
    render(<V2App today={TODAY} />);
    await screen.findByRole('heading', { level: 1, name: 'Combined dashboard' });

    expect(
      document.querySelector<HTMLElement>('.v2-app-frame')?.getAttribute('data-sidebar-mode'),
    ).toBe('collapsed');
    expect(screen.getByRole('button', { name: 'Expand sidebar' })).toBeInTheDocument();

    localStorage.removeItem(SIDEBAR_MODE_STORAGE_KEY);
  });

  it('treats a stored hidden sidebar preference as expanded', async () => {
    localStorage.setItem(SIDEBAR_MODE_STORAGE_KEY, 'hidden');
    mockedStatus.mockResolvedValue(status());
    render(<V2App today={TODAY} />);
    await screen.findByRole('heading', { level: 1, name: 'Combined dashboard' });

    expect(
      document.querySelector<HTMLElement>('.v2-app-frame')?.getAttribute('data-sidebar-mode'),
    ).toBe('expanded');
    // The V2 shell normalizes the legacy hidden value back to expanded.
    expect(localStorage.getItem(SIDEBAR_MODE_STORAGE_KEY)).toBe('expanded');

    localStorage.removeItem(SIDEBAR_MODE_STORAGE_KEY);
  });
});

describe('V2 keyboard shortcuts', () => {
  it('opens global search with Ctrl+K and the shortcut reference with Ctrl+/', async () => {
    await renderHealthy();

    fireEvent.keyDown(window, { ctrlKey: true, key: 'k' });
    const search = await screen.findByRole('dialog', { name: 'Global search' });
    expect(search).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Search Physics Tracker' })).toBeInTheDocument();

    fireEvent.keyDown(window, { ctrlKey: true, key: '/' });
    expect(screen.queryByRole('dialog', { name: 'Global search' })).not.toBeInTheDocument();
    expect(await screen.findByRole('dialog', { name: 'Keyboard shortcuts' })).toBeInTheDocument();
    expect(screen.getByText('Open global search (works while typing)')).toBeInTheDocument();
  });

  it('opens each exam workspace with Ctrl+1..4', async () => {
    await renderHealthy();
    fireEvent.keyDown(window, { ctrlKey: true, key: '2' });
    expect(
      await screen.findByRole('heading', { level: 1, name: 'GATE Physics (PH)' }),
    ).toBeInTheDocument();
    expect(document.querySelector('.v2-app-frame')).toHaveAttribute('data-workspace', 'exam');
    expect(document.querySelector('.v2-app-frame')).toHaveAttribute('data-exam', 'gate-physics');
  });
});
