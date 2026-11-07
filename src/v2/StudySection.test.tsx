import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  V2ExamSummary,
  V2StudySession,
  V2ExamSyllabus,
  V2StudyTotals,
  V2TimerSettings,
  V2TimerSnapshot,
  V2TimerState,
} from './api';
import { StudySection } from './StudySection';

vi.mock('./api', () => ({
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
  setV2TimerAllocations: vi.fn(),
  getV2StudyTotals: vi.fn(),
  getV2ExamSyllabus: vi.fn(),
  createV2ManualSession: vi.fn(),
  updateV2StudySession: vi.fn(),
  deleteV2StudySession: vi.fn(),
}));

import * as api from './api';

const mockedTimer = vi.mocked(api.getV2Timer);
const mockedStart = vi.mocked(api.startV2Timer);
const mockedPause = vi.mocked(api.pauseV2Timer);
const mockedResume = vi.mocked(api.resumeV2Timer);
const mockedFinish = vi.mocked(api.finishV2Timer);
const mockedDiscard = vi.mocked(api.discardV2Timer);
const mockedHeartbeat = vi.mocked(api.heartbeatV2Timer);
const mockedSettings = vi.mocked(api.updateV2TimerSettings);
const mockedSessions = vi.mocked(api.listV2Sessions);
const mockedSetAllocations = vi.mocked(api.setV2SessionAllocations);
const mockedTotals = vi.mocked(api.getV2StudyTotals);
const mockedSyllabus = vi.mocked(api.getV2ExamSyllabus);

const TODAY = '2026-11-04';

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
  },
  {
    id: 3,
    code: 'tifr-gs-physics',
    name: 'TIFR Graduate School Physics',
    shortName: 'TIFR GS',
    authority: 'TIFR',
    officialUrl: 'https://www.tifr.res.in/academics/',
    displayOrder: 3,
    isActive: true,
    examDate: null,
  },
  {
    id: 4,
    code: 'jest-physics',
    name: 'JEST Physics',
    shortName: 'JEST',
    authority: 'JEST Consortium',
    officialUrl: 'https://www.jest.org.in/',
    displayOrder: 4,
    isActive: true,
    examDate: null,
  },
];

function syllabus(exam: V2ExamSummary, topicId: number): V2ExamSyllabus {
  return {
    examId: exam.id,
    examCode: exam.code,
    examName: exam.name,
    examIsActive: true,
    configured: true,
    contentVersionLabel: 'official test syllabus',
    contentReviewStatus: 'verified',
    summary: { nodeCount: 2, topicCount: 1, revisedOrConfidentCount: 0, mappedConceptCount: 0 },
    nodes: [
      {
        id: topicId - 1,
        examId: exam.id,
        parentId: null,
        code: `${exam.code}-unit`,
        title: `${exam.shortName} unit`,
        nodeKind: 'unit',
        category: exam.shortName,
        displayOrder: 1,
        readinessWeight: 1,
        isArchived: false,
        sourceTitle: 'Official source',
        sourceVerifiedOn: '2026-11-05',
        sourceUrl: 'https://example.com/syllabus.pdf',
        sourceReference: 'Official source',
        contentVersionLabel: 'official test syllabus',
        contentReviewStatus: 'verified',
        progressStatus: 'not_started',
        priority: 'normal',
        confidence: 3,
        notes: '',
        nodePath: [`${exam.shortName} unit`],
      },
      {
        id: topicId,
        examId: exam.id,
        parentId: topicId - 1,
        code: `${exam.code}-topic`,
        title: `${exam.shortName} topic`,
        nodeKind: 'topic',
        category: exam.shortName,
        displayOrder: 1,
        readinessWeight: 1,
        isArchived: false,
        sourceTitle: 'Official source',
        sourceVerifiedOn: '2026-11-05',
        sourceUrl: 'https://example.com/syllabus.pdf',
        sourceReference: 'Official source',
        contentVersionLabel: 'official test syllabus',
        contentReviewStatus: 'verified',
        progressStatus: 'not_started',
        priority: 'normal',
        confidence: 3,
        notes: '',
        nodePath: [`${exam.shortName} unit`, `${exam.shortName} topic`],
      },
    ],
  };
}

function snapshot(
  state: Partial<V2TimerState> = {},
  settings: Partial<V2TimerSettings> = {},
): V2TimerSnapshot {
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
      ...state,
    },
    settings: {
      focusMinutes: 25,
      shortBreakMinutes: 5,
      longBreakMinutes: 15,
      notificationsEnabled: false,
      ...settings,
    },
    allocations: [],
    serverNowMs: Date.parse('2026-11-04T10:00:00Z'),
  };
}

function totals(overrides: Partial<V2StudyTotals> = {}): V2StudyTotals {
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
    ...overrides,
  };
}

function studySession(overrides: Partial<V2StudySession> = {}): V2StudySession {
  return {
    id: 1,
    sessionDate: TODAY,
    startedAt: '2026-11-04T09:35:00.000Z',
    endedAt: '2026-11-04T10:00:00.000Z',
    durationSeconds: 1_500,
    source: 'timer',
    notes: '',
    primaryExamId: 1,
    allocatedSeconds: 0,
    unallocatedSeconds: 1_500,
    allocations: [],
    topicAllocations: [],
    createdAt: '2026-11-04T10:00:00.000Z',
    updatedAt: '2026-11-04T10:00:00.000Z',
    ...overrides,
  };
}

async function renderStudy(
  overrides: {
    exams?: V2ExamSummary[];
    focusExamCode?: string;
    readOnly?: boolean;
  } = {},
) {
  render(
    <StudySection
      exams={overrides.exams ?? EXAMS}
      focusExamCode={overrides.focusExamCode}
      readOnly={overrides.readOnly}
      today={TODAY}
    />,
  );
  await screen.findByRole('heading', { name: 'Timer and sessions' });
}

describe('V2 study timer and sessions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedTimer.mockResolvedValue(snapshot());
    mockedSessions.mockResolvedValue([]);
    mockedTotals.mockResolvedValue(totals());
    mockedHeartbeat.mockResolvedValue(snapshot());
    mockedSyllabus.mockImplementation(async (examId) => {
      const exam = EXAMS.find((candidate) => candidate.id === examId) ?? EXAMS[0];
      return syllabus(exam, exam.id * 10 + 1);
    });
  });

  it('starts a focus timer with the selected examination context', async () => {
    const user = userEvent.setup();
    mockedStart.mockResolvedValue(snapshot({ status: 'running' }));
    await renderStudy();

    expect(screen.getByRole('timer')).toHaveTextContent('00:00');
    expect(screen.getByRole('timer')).toHaveTextContent('25:00 remaining');
    expect(
      screen.getByText(
        'No study sessions yet. Start the timer and stop it to save your first session.',
      ),
    ).toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText('Study context'), '2');
    await user.click(screen.getByRole('button', { name: 'Start timer' }));

    await waitFor(() =>
      expect(mockedStart).toHaveBeenCalledWith({
        mode: 'focus',
        sessionDate: TODAY,
        activeExamId: 2,
        allocations: [],
      }),
    );
  });

  it('starts one shared timer with multiple exam and topic allocations', async () => {
    const user = userEvent.setup();
    mockedStart.mockResolvedValue(snapshot({ status: 'running' }));
    await renderStudy();

    await user.click(screen.getByRole('button', { name: 'Add exam/topic allocation' }));
    await waitFor(() => expect(screen.getByLabelText('Subject / topic')).not.toBeDisabled());
    await user.selectOptions(screen.getByLabelText('Subject / topic'), '11');
    await user.click(screen.getByRole('button', { name: 'Add exam/topic allocation' }));
    const exams = screen.getAllByLabelText('Examination');
    await user.selectOptions(exams[1], '2');
    await waitFor(() => expect(screen.getAllByLabelText('Subject / topic')[1]).not.toBeDisabled());
    const topics = screen.getAllByLabelText('Subject / topic');
    await user.selectOptions(topics[1], '21');
    const minutes = screen.getAllByLabelText('Minutes');
    for (const input of minutes) {
      await user.clear(input);
      await user.type(input, '2');
    }
    await user.click(screen.getByRole('button', { name: 'Start timer' }));

    await waitFor(() =>
      expect(mockedStart).toHaveBeenCalledWith({
        mode: 'focus',
        sessionDate: TODAY,
        activeExamId: null,
        allocations: [
          { examId: 1, nodeId: 11, allocatedSeconds: 120 },
          { examId: 2, nodeId: 21, allocatedSeconds: 120 },
        ],
      }),
    );
  });

  it('pauses and resumes a running timer', async () => {
    const user = userEvent.setup();
    const running = snapshot({
      status: 'running',
      accumulatedMs: 60_000,
      elapsedMs: 60_000,
      remainingMs: 24 * 60_000,
      segmentStartedAt: '2026-11-04T10:00:00.000Z',
      sessionDate: TODAY,
      activeExamId: 1,
    });
    const paused = snapshot({
      status: 'paused',
      accumulatedMs: 60_000,
      elapsedMs: 60_000,
      remainingMs: 24 * 60_000,
      sessionDate: TODAY,
      activeExamId: 1,
    });
    mockedTimer.mockResolvedValue(running);
    mockedPause.mockResolvedValue(paused);
    mockedResume.mockResolvedValue(running);

    await renderStudy();

    await user.click(await screen.findByRole('button', { name: 'Pause timer' }));
    await waitFor(() => expect(mockedPause).toHaveBeenCalledTimes(1));
    expect(await screen.findByText('Paused')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Resume timer' }));
    await waitFor(() => expect(mockedResume).toHaveBeenCalledTimes(1));
    expect(await screen.findByText('Running')).toBeInTheDocument();
  });

  it('requires confirmation before discarding a timer', async () => {
    const user = userEvent.setup();
    mockedTimer.mockResolvedValue(
      snapshot({ status: 'running', sessionDate: TODAY, activeExamId: 1 }),
    );
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
    await renderStudy();

    await user.click(await screen.findByRole('button', { name: 'Discard timer' }));
    expect(confirm).toHaveBeenCalledTimes(1);
    expect(mockedDiscard).not.toHaveBeenCalled();

    confirm.mockReturnValue(true);
    mockedDiscard.mockResolvedValue(snapshot({ status: 'idle', lastEvent: 'discarded' }));
    await user.click(screen.getByRole('button', { name: 'Discard timer' }));
    await waitFor(() => expect(mockedDiscard).toHaveBeenCalledTimes(1));
    confirm.mockRestore();
  });

  it('shows subject and topic totals with session counts and last studied date', async () => {
    mockedTotals.mockResolvedValue(
      totals({
        topics: [
          {
            examId: 1,
            examCode: 'csir-net-physics',
            nodeId: 11,
            nodeTitle: 'CSIR-NET topic',
            subjectTitle: 'CSIR-NET unit',
            allocatedSeconds: 90,
            sessionCount: 2,
            lastStudiedDate: '2026-11-04',
          },
        ],
      }),
    );
    await renderStudy();

    expect(await screen.findByText('Subject study time')).toBeInTheDocument();
    expect(screen.getByText('CSIR-NET · CSIR-NET unit', { selector: 'span' })).toBeInTheDocument();
    expect(screen.getByText(/CSIR-NET · CSIR-NET unit · CSIR-NET topic/)).toBeInTheDocument();
    expect(screen.getByText(/2 sessions · last 2026-11-04/)).toBeInTheDocument();
  });

  it('stops and saves a running session, then shows it in history', async () => {
    const user = userEvent.setup();
    const saved = studySession({
      allocatedSeconds: 1_500,
      unallocatedSeconds: 0,
      allocations: [
        {
          examId: 1,
          examCode: 'csir-net-physics',
          examName: 'CSIR-UGC NET Physical Sciences',
          examIsActive: true,
          allocatedSeconds: 1_500,
        },
      ],
    });
    mockedTimer.mockResolvedValue(
      snapshot({
        status: 'running',
        elapsedMs: 1_500_000,
        remainingMs: 0,
        durationMs: 1_500_000,
        sessionDate: TODAY,
        activeExamId: 1,
      }),
    );
    mockedFinish.mockResolvedValue(
      snapshot({
        status: 'idle',
        lastEvent: 'completed',
        lastEventDurationMs: 1_500_000,
        lastCompletedSessionId: 1,
      }),
    );
    mockedSessions.mockResolvedValueOnce([]).mockResolvedValue([saved]);
    mockedTotals.mockResolvedValue(
      totals({ totalSeconds: 1_500, sessionCount: 1, allocatedSeconds: 1_500 }),
    );

    await renderStudy();
    await user.click(await screen.findByRole('button', { name: 'Stop & save session' }));

    await waitFor(() => expect(mockedFinish).toHaveBeenCalledTimes(1));
    expect(await screen.findByText('Session saved: 25:00.')).toBeInTheDocument();
    expect(await screen.findByText('csir-net-physics: 25 min')).toBeInTheDocument();
  });

  it('validates allocations before calling the backend', async () => {
    const user = userEvent.setup();
    mockedSessions.mockResolvedValue([studySession()]);
    await renderStudy();

    await user.click(await screen.findByRole('button', { name: 'Allocate time' }));
    const input = screen.getByLabelText('CSIR-NET');
    await user.clear(input);
    await user.type(input, '30');
    await user.click(screen.getByRole('button', { name: 'Save allocations' }));

    expect(
      screen.getByText(
        'Allocated time is 30 min, but the session is 25 min. Reduce the allocations before saving.',
      ),
    ).toBeInTheDocument();
    expect(mockedSetAllocations).not.toHaveBeenCalled();
  });

  it('saves a split allocation across multiple active exams', async () => {
    const user = userEvent.setup();
    mockedSessions.mockResolvedValue([studySession({ primaryExamId: null })]);
    mockedSetAllocations.mockResolvedValue(studySession());
    await renderStudy();

    await user.click(await screen.findByRole('button', { name: 'Allocate time' }));
    const csir = screen.getByLabelText('CSIR-NET');
    const gate = screen.getByLabelText('GATE PH');
    await user.clear(csir);
    await user.type(csir, '10');
    await user.clear(gate);
    await user.type(gate, '5');
    await user.click(screen.getByRole('button', { name: 'Save allocations' }));

    await waitFor(() =>
      expect(mockedSetAllocations).toHaveBeenCalledWith({
        sessionId: 1,
        allocations: [
          { examId: 1, allocatedSeconds: 600 },
          { examId: 2, allocatedSeconds: 300 },
        ],
      }),
    );
  });

  it('keeps an inactive exam workspace read-only while showing its history', async () => {
    const exams = EXAMS.map((exam) =>
      exam.code === 'gate-physics' ? { ...exam, isActive: false } : exam,
    );
    mockedSessions.mockResolvedValue([
      studySession({
        id: 7,
        allocations: [
          {
            examId: 2,
            examCode: 'gate-physics',
            examName: 'GATE Physics (PH)',
            examIsActive: false,
            allocatedSeconds: 1_200,
          },
        ],
        allocatedSeconds: 1_200,
        unallocatedSeconds: 300,
      }),
    ]);

    await renderStudy({ exams, focusExamCode: 'gate-physics' });

    expect(
      screen.getByText(
        'This examination is inactive. Its historical study sessions remain visible, but new timers and allocation changes are disabled.',
      ),
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Start timer' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Allocate time' })).not.toBeInTheDocument();
    expect(screen.getByText('gate-physics: 20 min (inactive)')).toBeInTheDocument();
  });

  it('shows a clear error when the timer cannot be read', async () => {
    mockedTimer.mockRejectedValue({
      code: 'database_error',
      message: 'The V2 study timer is unavailable.',
    });

    render(<StudySection exams={EXAMS} today={TODAY} />);

    expect(await screen.findByText('The V2 study timer is unavailable.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Retry study data' })).toBeInTheDocument();
  });

  it('saves configured timer lengths without changing unrelated settings', async () => {
    const user = userEvent.setup();
    mockedSettings.mockResolvedValue(snapshot({}, { focusMinutes: 40 }));
    await renderStudy();

    await user.click(screen.getByText('Timer lengths'));
    const focus = screen.getByLabelText('Focus minutes');
    await user.clear(focus);
    await user.type(focus, '40');
    await user.click(screen.getByRole('button', { name: 'Save timer lengths' }));

    await waitFor(() =>
      expect(mockedSettings).toHaveBeenCalledWith({
        focusMinutes: 40,
        shortBreakMinutes: 5,
        longBreakMinutes: 15,
        notificationsEnabled: false,
      }),
    );
  });
});
