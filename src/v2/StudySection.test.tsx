import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  V2ExamSummary,
  V2StudySession,
  V2ExamSyllabus,
  V2StudyTotals,
  V2TimerSettings,
  V2TimerSnapshot,
  V2TimerState,
} from './api';
import { TimerProvider } from './timer/TimerProvider';
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
    <TimerProvider>
      <StudySection
        exams={overrides.exams ?? EXAMS}
        focusExamCode={overrides.focusExamCode}
        readOnly={overrides.readOnly}
        today={TODAY}
      />
    </TimerProvider>,
  );
  await screen.findByRole('heading', { name: 'Timer and sessions' });
  await screen.findByRole('timer');
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

  afterEach(() => {
    vi.useRealTimers();
  });

  it('waits for the real timer section before handling a focus request', async () => {
    let resolveTimer!: (value: V2TimerSnapshot) => void;
    mockedTimer.mockReturnValue(
      new Promise<V2TimerSnapshot>((resolve) => {
        resolveTimer = resolve;
      }),
    );
    const onTimerFocused = vi.fn();

    render(
      <TimerProvider>
        <StudySection
          exams={EXAMS}
          focusRequest={1}
          onTimerFocused={onTimerFocused}
          today={TODAY}
        />
      </TimerProvider>,
    );

    expect(await screen.findByText(/Loading the persisted study timer/)).toBeInTheDocument();
    expect(onTimerFocused).not.toHaveBeenCalled();

    await act(async () => {
      resolveTimer(snapshot({ status: 'running', sessionDate: TODAY }));
    });

    expect(await screen.findByRole('heading', { name: 'Timer and sessions' })).toBeInTheDocument();
    await waitFor(() => expect(onTimerFocused).toHaveBeenCalledWith(1));
    expect(onTimerFocused).toHaveBeenCalledTimes(1);
  });

  it('handles a fresh focus request after the previous request clears', async () => {
    const onTimerFocused = vi.fn();
    const view = render(
      <TimerProvider>
        <StudySection
          exams={EXAMS}
          focusRequest={1}
          onTimerFocused={onTimerFocused}
          today={TODAY}
        />
      </TimerProvider>,
    );

    await waitFor(() => expect(onTimerFocused).toHaveBeenCalledWith(1));
    view.rerender(
      <TimerProvider>
        <StudySection
          exams={EXAMS}
          focusRequest={0}
          onTimerFocused={onTimerFocused}
          today={TODAY}
        />
      </TimerProvider>,
    );
    await act(async () => {
      await Promise.resolve();
    });
    view.rerender(
      <TimerProvider>
        <StudySection
          exams={EXAMS}
          focusRequest={2}
          onTimerFocused={onTimerFocused}
          today={TODAY}
        />
      </TimerProvider>,
    );

    await waitFor(() => expect(onTimerFocused).toHaveBeenCalledWith(2));
    expect(onTimerFocused).toHaveBeenCalledTimes(2);
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

  it('preserves unsaved timer length edits through unchanged heartbeat snapshots', async () => {
    vi.useFakeTimers();
    try {
      mockedTimer.mockResolvedValue(snapshot({ status: 'running', sessionDate: TODAY }));
      mockedHeartbeat.mockResolvedValue(snapshot({ status: 'running', sessionDate: TODAY }));

      render(
        <TimerProvider>
          <StudySection exams={EXAMS} today={TODAY} />
        </TimerProvider>,
      );
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(screen.getByRole('timer')).toBeInTheDocument();

      fireEvent.click(screen.getByText('Timer lengths'));
      const focus = screen.getByLabelText('Focus minutes');
      fireEvent.change(focus, { target: { value: '40' } });
      expect(focus).toHaveValue(40);

      await act(async () => {
        await vi.advanceTimersByTimeAsync(5_000);
      });
      expect(screen.getByLabelText('Focus minutes')).toHaveValue(40);
    } finally {
      vi.useRealTimers();
    }
  });

  it('updates the draft when persisted timer settings genuinely change', async () => {
    vi.useFakeTimers();
    try {
      mockedTimer.mockResolvedValue(snapshot({ status: 'running', sessionDate: TODAY }));
      mockedHeartbeat.mockResolvedValue(
        snapshot({ status: 'running', sessionDate: TODAY }, { focusMinutes: 40 }),
      );

      render(
        <TimerProvider>
          <StudySection exams={EXAMS} today={TODAY} />
        </TimerProvider>,
      );
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      fireEvent.click(screen.getByText('Timer lengths'));
      expect(screen.getByLabelText('Focus minutes')).toHaveValue(25);

      await act(async () => {
        await vi.advanceTimersByTimeAsync(5_000);
      });
      expect(screen.getByLabelText('Focus minutes')).toHaveValue(40);
    } finally {
      vi.useRealTimers();
    }
  });

  it('synchronizes the timer mode from an idle snapshot', async () => {
    mockedTimer.mockResolvedValue(snapshot({ status: 'idle', mode: 'shortBreak' }));
    await renderStudy();

    await waitFor(() => expect(screen.getByLabelText('Timer mode')).toHaveValue('shortBreak'));
  });

  it('renders the full focus session label in the timer card', async () => {
    await renderStudy();
    expect(screen.getByRole('heading', { name: 'Focus session' })).toBeInTheDocument();
  });

  it('shows a completion-aware message when pause loses to heartbeat completion', async () => {
    const user = userEvent.setup();
    mockedTimer.mockResolvedValue(snapshot({ status: 'running', sessionDate: TODAY }));
    mockedPause.mockResolvedValue(
      snapshot({
        status: 'idle',
        lastEvent: 'completed',
        lastCompletedSessionId: 1,
        lastEventDurationMs: 1_500_000,
      }),
    );

    await renderStudy();
    await user.click(screen.getByRole('button', { name: 'Pause timer' }));

    expect(
      await screen.findByText('Timer completed before pause. The session was saved.'),
    ).toBeInTheDocument();
  });

  it('surfaces a completion refresh error without an unhandled rejection', async () => {
    vi.useFakeTimers();
    try {
      mockedTimer.mockResolvedValue(snapshot({ status: 'running', sessionDate: TODAY }));
      mockedHeartbeat.mockResolvedValue(
        snapshot({
          status: 'idle',
          lastEvent: 'completed',
          lastCompletedSessionId: 1,
          lastEventDurationMs: 1_500_000,
        }),
      );
      mockedSessions.mockResolvedValueOnce([]).mockRejectedValueOnce(new Error('sessions down'));
      mockedTotals.mockResolvedValue(totals());

      render(
        <TimerProvider>
          <StudySection exams={EXAMS} today={TODAY} />
        </TimerProvider>,
      );
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(screen.getByRole('timer')).toBeInTheDocument();

      await act(async () => {
        await vi.advanceTimersByTimeAsync(5_000);
      });

      expect(screen.getByRole('alert')).toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
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

  it('refreshes session history when a heartbeat completes the timer', async () => {
    vi.useFakeTimers();
    try {
      const running = snapshot({
        status: 'running',
        elapsedMs: 1_500_000,
        remainingMs: 0,
        durationMs: 1_500_000,
        sessionDate: TODAY,
        activeExamId: 1,
      });
      const completed = snapshot({
        status: 'idle',
        lastEvent: 'completed',
        lastEventDurationMs: 1_500_000,
        lastCompletedSessionId: 1,
      });
      mockedTimer.mockResolvedValue(running);
      mockedHeartbeat.mockResolvedValue(completed);
      mockedSessions.mockResolvedValueOnce([]).mockResolvedValue([
        studySession({
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
        }),
      ]);
      mockedTotals.mockResolvedValueOnce(totals()).mockResolvedValue(
        totals({
          totalSeconds: 1_500,
          sessionCount: 1,
          allocatedSeconds: 1_500,
        }),
      );

      render(
        <TimerProvider>
          <StudySection exams={EXAMS} today={TODAY} />
        </TimerProvider>,
      );

      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(screen.getByRole('timer')).toBeInTheDocument();

      await act(async () => {
        await vi.advanceTimersByTimeAsync(5_000);
      });

      expect(mockedHeartbeat).toHaveBeenCalledTimes(1);
      expect(screen.getByText(/1 recorded session/)).toBeInTheDocument();
      expect(screen.getByText('csir-net-physics: 25 min')).toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
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

  it('uses the dedicated session action grid without the shared form-actions class', async () => {
    mockedSessions.mockResolvedValue([
      studySession({
        notes: `Long session note ${'that must wrap safely '.repeat(12)}`,
      }),
    ]);
    await renderStudy();

    const allocate = await screen.findByRole('button', { name: 'Allocate time' });
    const actions = allocate.closest('.v2-session-actions');
    expect(actions).not.toBeNull();
    expect(actions).not.toHaveClass('v2-form-actions');
    const row = actions?.closest('.v2-session-row');
    expect(row).not.toBeNull();
    expect(row?.querySelector('.v2-session-main')).not.toBeNull();
    expect(row?.querySelector('.v2-session-notes')).toHaveTextContent(
      'Long session note that must wrap safely',
    );
  });

  it('defines an inline-size container query for earlier session-card stacking', () => {
    const css = readFileSync(join(process.cwd(), 'src/v2/v2.css'), 'utf8');
    expect(css).toContain('container-type: inline-size');
    expect(css).toMatch(
      /@container\s*\(max-width:\s*620px\)\s*\{[\s\S]*?\.v2-session-row[\s\S]*?\.v2-session-actions[\s\S]*?justify-content:\s*flex-start/,
    );
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

    render(
      <TimerProvider>
        <StudySection exams={EXAMS} today={TODAY} />
      </TimerProvider>,
    );

    expect(await screen.findByText('The V2 study timer is unavailable.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Retry study data' })).toBeInTheDocument();
  });

  it('saves configured timer lengths without changing unrelated settings', async () => {
    const user = userEvent.setup();
    mockedSettings.mockResolvedValue(snapshot({}, { focusMinutes: 40 }));
    await renderStudy();

    await user.click(await screen.findByText('Timer lengths'));
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
