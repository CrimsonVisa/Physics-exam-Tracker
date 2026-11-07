import { type FormEvent, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import { onTimerUpdated } from '../lib/timerEvents';
import {
  createV2ManualSession,
  deleteV2StudySession,
  discardV2Timer,
  finishV2Timer,
  getV2ExamSyllabus,
  getV2StudyTotals,
  getV2Timer,
  heartbeatV2Timer,
  listV2Sessions,
  pauseV2Timer,
  resumeV2Timer,
  setV2SessionAllocations,
  setV2TimerAllocations,
  startV2Timer,
  updateV2StudySession,
  updateV2TimerSettings,
  type V2ExamSyllabus,
  type V2ExamSummary,
  type V2StudySession,
  type V2StudyTotals,
  type V2TimerMode,
  type V2TimerSettings,
  type V2TimerSnapshot,
  type V2TopicAllocationInput,
} from './api';

type StudySectionProps = {
  exams: V2ExamSummary[];
  today: string;
  focusExamCode?: string;
  readOnly?: boolean;
};

const MODE_LABELS: Record<V2TimerMode, string> = {
  focus: 'Focus session',
  shortBreak: 'Short break',
  longBreak: 'Long break',
};

const STATUS_LABELS = {
  idle: 'Idle',
  running: 'Running',
  paused: 'Paused',
} as const;

function pad(value: number): string {
  return String(value).padStart(2, '0');
}

function formatClock(milliseconds: number): string {
  const totalSeconds = Math.max(0, Math.floor(milliseconds / 1_000));
  const hours = Math.floor(totalSeconds / 3_600);
  const minutes = Math.floor((totalSeconds % 3_600) / 60);
  const seconds = totalSeconds % 60;
  return hours > 0 ? `${hours}:${pad(minutes)}:${pad(seconds)}` : `${pad(minutes)}:${pad(seconds)}`;
}

function formatDuration(seconds: number): string {
  if (seconds <= 0) {
    return '0 min';
  }
  const hours = Math.floor(seconds / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  const remainingSeconds = seconds % 60;
  if (hours > 0) {
    return minutes > 0 ? `${hours} h ${minutes} min` : `${hours} h`;
  }
  if (minutes > 0) {
    return remainingSeconds > 0 ? `${minutes} min ${remainingSeconds} s` : `${minutes} min`;
  }
  return `${remainingSeconds} s`;
}

function modeDurationMinutes(mode: V2TimerMode, settings: V2TimerSettings): number {
  if (mode === 'focus') {
    return settings.focusMinutes;
  }
  return mode === 'shortBreak' ? settings.shortBreakMinutes : settings.longBreakMinutes;
}

const PROGRESS_LABELS: Record<string, string> = {
  not_started: 'Not started',
  learning: 'Learning',
  revised: 'Revised',
  confident: 'Confident',
};

type AllocationDraft = {
  key: number;
  examId: string;
  nodeId: string;
  minutes: string;
  search: string;
};

export function StudySection({ exams, today, focusExamCode, readOnly = false }: StudySectionProps) {
  const focusExam = useMemo(
    () => exams.find((exam) => exam.code === focusExamCode) ?? null,
    [exams, focusExamCode],
  );
  const locked = readOnly || (focusExam !== null && !focusExam.isActive);

  const [timer, setTimer] = useState<V2TimerSnapshot | null>(null);
  const [sessions, setSessions] = useState<V2StudySession[]>([]);
  const [totals, setTotals] = useState<V2StudyTotals | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [mode, setMode] = useState<V2TimerMode>('focus');
  const [selectedExamId, setSelectedExamId] = useState(() =>
    focusExam && focusExam.isActive ? String(focusExam.id) : '',
  );
  const [settingsDraft, setSettingsDraft] = useState<V2TimerSettings | null>(null);
  const [receivedAt, setReceivedAt] = useState(() => Date.now());
  const [clock, setClock] = useState(() => Date.now());
  const [editingSessionId, setEditingSessionId] = useState<number | null>(null);
  const [allocationDraft, setAllocationDraft] = useState<Record<number, string>>({});
  const [allocationError, setAllocationError] = useState<string | null>(null);
  const [syllabi, setSyllabi] = useState<Record<number, V2ExamSyllabus>>({});
  const [allocationRows, setAllocationRows] = useState<AllocationDraft[]>([]);
  const [timerAllocationError, setTimerAllocationError] = useState<string | null>(null);
  const allocationKey = useRef(1);
  const [manualDate, setManualDate] = useState(today);
  const [manualMinutes, setManualMinutes] = useState('30');
  const [manualExamId, setManualExamId] = useState('');
  const [manualNotes, setManualNotes] = useState('');
  const [editingNoteId, setEditingNoteId] = useState<number | null>(null);
  const [noteDraft, setNoteDraft] = useState('');
  const [deletingSessionId, setDeletingSessionId] = useState<number | null>(null);

  const applySnapshot = useCallback((snapshot: V2TimerSnapshot) => {
    setTimer(snapshot);
    setSettingsDraft(snapshot.settings);
    if (snapshot.state.status === 'idle') {
      setMode(snapshot.state.mode);
    }
    setReceivedAt(Date.now());
    setClock(Date.now());
  }, []);

  const refreshRecords = useCallback(async () => {
    const [nextSessions, nextTotals] = await Promise.all([listV2Sessions(), getV2StudyTotals()]);
    setSessions(nextSessions);
    setTotals(nextTotals);
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [snapshot, nextSessions, nextTotals] = await Promise.all([
        getV2Timer(),
        listV2Sessions(),
        getV2StudyTotals(),
      ]);
      applySnapshot(snapshot);
      setSessions(nextSessions);
      setTotals(nextTotals);
    } catch (loadError) {
      setError(toCommandError(loadError).message);
    } finally {
      setLoading(false);
    }
  }, [applySnapshot]);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const [snapshot, nextSessions, nextTotals] = await Promise.all([
          getV2Timer(),
          listV2Sessions(),
          getV2StudyTotals(),
        ]);
        if (cancelled) {
          return;
        }
        applySnapshot(snapshot);
        setSessions(nextSessions);
        setTotals(nextTotals);
        setError(null);
      } catch (loadError) {
        if (!cancelled) {
          setError(toCommandError(loadError).message);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [applySnapshot]);

  useEffect(() => {
    const interval = window.setInterval(() => setClock(Date.now()), 1_000);
    return () => window.clearInterval(interval);
  }, []);

  // Quick starts and resumes from other surfaces (dashboard Quick Start) change
  // the same persisted timer; re-read the backend snapshot so this card never
  // drifts out of sync. The database remains the single source of truth.
  useEffect(
    () =>
      onTimerUpdated(() => {
        void (async () => {
          try {
            const snapshot = await getV2Timer();
            applySnapshot(snapshot);
          } catch {
            // The next heartbeat or user action surfaces backend errors.
          }
        })();
      }),
    [applySnapshot],
  );

  useEffect(() => {
    if (timer?.state.status !== 'running') {
      return;
    }
    const interval = window.setInterval(() => {
      void (async () => {
        try {
          const snapshot = await heartbeatV2Timer();
          applySnapshot(snapshot);
          if (snapshot.state.status !== 'running') {
            await refreshRecords();
          }
        } catch (heartbeatError) {
          setError(toCommandError(heartbeatError).message);
        }
      })();
    }, 5_000);
    return () => window.clearInterval(interval);
  }, [applySnapshot, refreshRecords, timer?.state.status]);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const loaded: Record<number, V2ExamSyllabus> = {};
      await Promise.all(
        exams
          .filter((exam) => exam.isActive)
          .map(async (exam) => {
            try {
              const value = await getV2ExamSyllabus(exam.id);
              if (value?.configured) {
                loaded[exam.id] = value;
              }
            } catch {
              // A syllabus can be unavailable without making the timer unusable.
            }
          }),
      );
      if (!cancelled) {
        setSyllabi(loaded);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [exams]);

  useEffect(() => {
    if (!timer) {
      return;
    }
    if (timer.allocations.length > 0) {
      setAllocationRows(
        timer.allocations.map((allocation) => ({
          key: allocationKey.current++,
          examId: String(allocation.examId),
          nodeId: String(allocation.nodeId),
          minutes: String(Math.max(1, Math.round(allocation.allocatedSeconds / 60))),
          search: '',
        })),
      );
    } else if (timer.state.status === 'idle') {
      setAllocationRows([]);
    }
    // Re-sync only when the persisted timer lifecycle changes, not on every
    // heartbeat, so an in-progress allocation edit is not overwritten.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [timer?.state.status, timer?.state.lastCompletedSessionId]);

  const elapsedMs = useMemo(() => {
    if (!timer) {
      return 0;
    }
    if (timer.state.status === 'running') {
      return Math.min(
        timer.state.durationMs,
        timer.state.elapsedMs + Math.max(0, clock - receivedAt),
      );
    }
    return timer.state.elapsedMs;
  }, [clock, receivedAt, timer]);

  const remainingMs = timer ? Math.max(0, timer.state.durationMs - elapsedMs) : 0;
  const progress =
    timer && timer.state.durationMs > 0
      ? Math.min(100, (elapsedMs / timer.state.durationMs) * 100)
      : 0;

  const visibleSessions = focusExam
    ? sessions.filter((session) =>
        session.allocations.some((allocation) => allocation.examId === focusExam.id),
      )
    : sessions;

  const editingSession = visibleSessions.find((session) => session.id === editingSessionId) ?? null;

  const draftAllocatedSeconds = useMemo(() => {
    if (!editingSession) {
      return 0;
    }
    return exams.reduce((total, exam) => {
      const value = Number(allocationDraft[exam.id] ?? '0');
      return total + (Number.isFinite(value) ? Math.round(Math.max(0, value) * 60) : 0);
    }, 0);
  }, [allocationDraft, editingSession, exams]);

  const timerAllocationInputs = useMemo<V2TopicAllocationInput[]>(
    () =>
      allocationRows
        .map((row) => ({
          examId: Number(row.examId),
          nodeId: Number(row.nodeId),
          allocatedSeconds: Math.max(0, Number(row.minutes) || 0) * 60,
        }))
        .filter(
          (allocation) =>
            allocation.examId > 0 && allocation.nodeId > 0 && allocation.allocatedSeconds > 0,
        ),
    [allocationRows],
  );
  const subjectTotals = useMemo(() => {
    const grouped = new Map<
      string,
      { examCode: string; subjectTitle: string; allocatedSeconds: number }
    >();
    for (const topic of totals?.topics ?? []) {
      const key = `${topic.examId}:${topic.subjectTitle}`;
      const current = grouped.get(key) ?? {
        examCode: topic.examCode,
        subjectTitle: topic.subjectTitle,
        allocatedSeconds: 0,
      };
      current.allocatedSeconds += topic.allocatedSeconds;
      grouped.set(key, current);
    }
    return [...grouped.values()].sort(
      (left, right) => right.allocatedSeconds - left.allocatedSeconds,
    );
  }, [totals]);
  const timerAllocationSeconds = timerAllocationInputs.reduce(
    (total, allocation) => total + allocation.allocatedSeconds,
    0,
  );
  const timerAllocationLimitSeconds = timer
    ? timer.state.status === 'idle'
      ? modeDurationMinutes(mode, timer.settings) * 60
      : Math.floor(timer.state.durationMs / 1_000)
    : 0;

  function addAllocation() {
    const firstExam = exams.find((exam) => exam.isActive);
    setAllocationRows((rows) => [
      ...rows,
      {
        key: allocationKey.current++,
        examId: firstExam ? String(firstExam.id) : '',
        nodeId: '',
        minutes: '30',
        search: '',
      },
    ]);
    setTimerAllocationError(null);
  }

  function updateAllocation(key: number, patch: Partial<AllocationDraft>) {
    setAllocationRows((rows) => rows.map((row) => (row.key === key ? { ...row, ...patch } : row)));
    setTimerAllocationError(null);
  }

  function removeAllocation(key: number) {
    setAllocationRows((rows) => rows.filter((row) => row.key !== key));
    setTimerAllocationError(null);
  }

  function validateTimerAllocations(): boolean {
    if (timerAllocationSeconds > timerAllocationLimitSeconds) {
      setTimerAllocationError(
        `Allocations total ${formatDuration(timerAllocationSeconds)}, but the timer is ${formatDuration(
          timerAllocationLimitSeconds,
        )}. Reduce the allocations before starting or saving.`,
      );
      return false;
    }
    const seen = new Set<string>();
    for (const allocation of timerAllocationInputs) {
      const key = `${allocation.examId}:${allocation.nodeId}`;
      if (seen.has(key)) {
        setTimerAllocationError('Each examination topic can appear only once.');
        return false;
      }
      seen.add(key);
    }
    setTimerAllocationError(null);
    return true;
  }

  async function saveRunningAllocations() {
    if (!validateTimerAllocations()) {
      return;
    }
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const snapshot = await setV2TimerAllocations({
        allocations: timerAllocationInputs,
      });
      applySnapshot(snapshot);
      setNotice('Timer study allocations saved.');
    } catch (allocationFailure) {
      setTimerAllocationError(toCommandError(allocationFailure).message);
    } finally {
      setBusy(false);
    }
  }

  async function runTimerAction(
    action: () => Promise<V2TimerSnapshot>,
    message?: (snapshot: V2TimerSnapshot) => string,
  ) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const snapshot = await action();
      applySnapshot(snapshot);
      await refreshRecords();
      if (message) {
        setNotice(message(snapshot));
      }
    } catch (actionError) {
      setError(toCommandError(actionError).message);
    } finally {
      setBusy(false);
    }
  }

  function handleStart() {
    if (mode === 'focus' && !validateTimerAllocations()) {
      return;
    }
    const activeExamId = mode === 'focus' && selectedExamId ? Number(selectedExamId) : null;
    void runTimerAction(
      () =>
        startV2Timer({
          mode,
          sessionDate: today,
          activeExamId,
          allocations: mode === 'focus' ? timerAllocationInputs : [],
        }),
      () => 'Timer started.',
    );
  }

  function handlePause() {
    void runTimerAction(pauseV2Timer, () => 'Timer paused.');
  }

  function handleResume() {
    void runTimerAction(resumeV2Timer, () => 'Timer resumed.');
  }

  function handleFinish() {
    void runTimerAction(finishV2Timer, (snapshot) =>
      snapshot.state.lastCompletedSessionId !== null
        ? `Session saved: ${formatClock(snapshot.state.lastEventDurationMs)}.`
        : `Timer completed: ${formatClock(snapshot.state.lastEventDurationMs)}.`,
    );
  }

  function handleDiscard() {
    if (!window.confirm('Discard this timer? No study session will be saved.')) {
      return;
    }
    void runTimerAction(discardV2Timer, () => 'Timer discarded. No session was saved.');
  }

  async function handleSaveSettings() {
    if (!settingsDraft) {
      return;
    }
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const snapshot = await updateV2TimerSettings(settingsDraft);
      applySnapshot(snapshot);
      setNotice('Timer lengths saved for the next timer.');
    } catch (settingsError) {
      setError(toCommandError(settingsError).message);
    } finally {
      setBusy(false);
    }
  }

  function openAllocationEditor(session: V2StudySession) {
    const draft: Record<number, string> = {};
    for (const exam of exams) {
      const existing = session.allocations.find((allocation) => allocation.examId === exam.id);
      draft[exam.id] = existing ? String(existing.allocatedSeconds / 60) : '';
    }
    setAllocationDraft(draft);
    setAllocationError(null);
    setEditingSessionId(session.id);
  }

  async function handleSaveAllocations(session: V2StudySession) {
    setAllocationError(null);
    if (draftAllocatedSeconds > session.durationSeconds) {
      setAllocationError(
        `Allocated time is ${formatDuration(draftAllocatedSeconds)}, but the session is ${formatDuration(
          session.durationSeconds,
        )}. Reduce the allocations before saving.`,
      );
      return;
    }

    const allocations = exams
      .map((exam) => ({
        examId: exam.id,
        allocatedSeconds: Math.round(Math.max(0, Number(allocationDraft[exam.id] ?? '0')) * 60),
      }))
      .filter((allocation) => allocation.allocatedSeconds > 0);

    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await setV2SessionAllocations({ sessionId: session.id, allocations });
      await refreshRecords();
      setEditingSessionId(null);
      setNotice('Session allocations saved.');
    } catch (allocationFailure) {
      setAllocationError(toCommandError(allocationFailure).message);
    } finally {
      setBusy(false);
    }
  }

  async function handleAddManualSession(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const minutes = Number(manualMinutes);
    if (!Number.isFinite(minutes) || minutes <= 0 || minutes > 1_440) {
      setError('Manual session duration must be between 1 minute and 24 hours.');
      return;
    }
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await createV2ManualSession({
        sessionDate: manualDate,
        durationSeconds: Math.round(minutes * 60),
        notes: manualNotes.trim(),
        primaryExamId: manualExamId ? Number(manualExamId) : null,
      });
      setManualNotes('');
      await refreshRecords();
      setNotice('Manual study session saved.');
    } catch (manualError) {
      setError(toCommandError(manualError).message);
    } finally {
      setBusy(false);
    }
  }

  function beginNoteEdit(session: V2StudySession) {
    setEditingNoteId(session.id);
    setNoteDraft(session.notes);
    setDeletingSessionId(null);
  }

  async function saveSessionNote(session: V2StudySession) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await updateV2StudySession({
        sessionId: session.id,
        sessionDate: session.sessionDate,
        durationSeconds: session.durationSeconds,
        notes: noteDraft,
        primaryExamId: session.primaryExamId,
      });
      setEditingNoteId(null);
      await refreshRecords();
      setNotice('Session note saved.');
    } catch (noteError) {
      setError(toCommandError(noteError).message);
    } finally {
      setBusy(false);
    }
  }

  async function deleteSession(session: V2StudySession) {
    if (deletingSessionId !== session.id) {
      setDeletingSessionId(session.id);
      setEditingNoteId(null);
      return;
    }
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await deleteV2StudySession(session.id);
      setDeletingSessionId(null);
      await refreshRecords();
      setNotice('Study session deleted.');
    } catch (deleteError) {
      setError(toCommandError(deleteError).message);
    } finally {
      setBusy(false);
    }
  }

  const lastEventNotice =
    timer?.state.lastEvent === 'completed'
      ? timer.state.lastCompletedSessionId !== null
        ? `Previous timer saved as session #${timer.state.lastCompletedSessionId} (${formatClock(
            timer.state.lastEventDurationMs,
          )}).`
        : `Previous break completed after ${formatClock(timer.state.lastEventDurationMs)}. No study session was saved.`
      : timer?.state.lastEvent === 'discarded'
        ? `Previous timer was discarded after ${formatClock(timer.state.lastEventDurationMs)}.`
        : null;

  const recoveryNotice = timer?.state.recoveryReason
    ? timer.state.recoveryReason === 'clockChange'
      ? 'A clock change was detected. Unverified time was not counted.'
      : `The timer was paused after ${timer.state.recoveryReason === 'restart' ? 'an application restart' : 'missing checkpoints'}. ${formatDuration(
          Math.round(timer.state.recoveryExcludedMs / 1_000),
        )} of unverified time was excluded.`
    : null;

  if (loading && !timer) {
    return (
      <section className="v2-study" aria-busy="true">
        <div className="v2-section-heading">
          <div>
            <p className="eyebrow">Study</p>
            <h2>Timer and sessions</h2>
          </div>
        </div>
        <p className="v2-empty-copy">Loading the persisted study timer and session history…</p>
      </section>
    );
  }

  if (!timer) {
    return (
      <section className="v2-study">
        <div className="v2-section-heading">
          <div>
            <p className="eyebrow">Study</p>
            <h2>Timer and sessions</h2>
          </div>
        </div>
        <p className="v2-study-error" role="alert">
          {error ?? 'The study timer could not be loaded.'}
        </p>
        <button
          className="secondary-button"
          disabled={busy}
          onClick={() => void load()}
          type="button"
        >
          Retry study data
        </button>
      </section>
    );
  }

  const status = timer.state.status;
  const completed = status === 'idle' && timer.state.lastEvent === 'completed';
  const statusLabel = completed ? 'Completed' : STATUS_LABELS[status];
  const selectedModeMinutes = modeDurationMinutes(mode, timer.settings);
  const activeExams = exams.filter((exam) => exam.isActive);

  function renderAllocationBuilder(editable: boolean) {
    return (
      <div className="v2-timer-allocation-builder">
        <div className="v2-section-heading">
          <div>
            <p className="eyebrow">Shared allocation</p>
            <h4>Exam and topic allocations</h4>
          </div>
          <p>{formatDuration(timerAllocationSeconds)} planned</p>
        </div>
        {allocationRows.map((row) => {
          const syllabus = syllabi[Number(row.examId)];
          const topics = (syllabus?.nodes ?? [])
            .filter((node) => node.nodeKind === 'topic' && !node.isArchived)
            .filter((node) => {
              const query = row.search.trim().toLowerCase();
              return (
                !query ||
                [node.title, node.code, ...node.nodePath].join(' ').toLowerCase().includes(query)
              );
            });
          return (
            <div className="v2-timer-allocation-row" key={row.key}>
              <label className="v2-field">
                <span>Examination</span>
                <select
                  disabled={!editable || busy}
                  onChange={(event) =>
                    updateAllocation(row.key, {
                      examId: event.target.value,
                      nodeId: '',
                      search: '',
                    })
                  }
                  value={row.examId}
                >
                  <option value="">Choose exam</option>
                  {activeExams.map((exam) => (
                    <option key={exam.id} value={exam.id}>
                      {exam.shortName} — {exam.code}
                    </option>
                  ))}
                </select>
              </label>
              <label className="v2-field">
                <span>Find topic</span>
                <input
                  disabled={!editable || busy}
                  onChange={(event) => updateAllocation(row.key, { search: event.target.value })}
                  placeholder="Search subject or topic"
                  type="search"
                  value={row.search}
                />
              </label>
              <label className="v2-field">
                <span>Subject / topic</span>
                <select
                  disabled={!editable || busy || topics.length === 0}
                  onChange={(event) => updateAllocation(row.key, { nodeId: event.target.value })}
                  value={row.nodeId}
                >
                  <option value="">{syllabus ? 'Choose topic' : 'Syllabus not loaded'}</option>
                  {topics.map((topic) => (
                    <option key={topic.id} value={topic.id}>
                      {topic.nodePath.slice(-2).join(' › ')}
                    </option>
                  ))}
                </select>
              </label>
              <label className="v2-field">
                <span>Minutes</span>
                <input
                  disabled={!editable || busy}
                  min={1}
                  onChange={(event) => updateAllocation(row.key, { minutes: event.target.value })}
                  type="number"
                  value={row.minutes}
                />
              </label>
              {editable && (
                <button
                  aria-label="Remove allocation"
                  className="secondary-button v2-danger-button"
                  disabled={busy}
                  onClick={() => removeAllocation(row.key)}
                  type="button"
                >
                  Remove
                </button>
              )}
            </div>
          );
        })}
        <div className="v2-timer-allocation-actions">
          <button
            className="secondary-button"
            disabled={!editable || busy}
            onClick={addAllocation}
            type="button"
          >
            Add exam/topic allocation
          </button>
          {editable && timer?.state.status !== 'idle' && (
            <button
              className="primary-button"
              disabled={busy}
              onClick={() => void saveRunningAllocations()}
              type="button"
            >
              Save timer allocations
            </button>
          )}
        </div>
        <p className="v2-empty-copy">
          {timerAllocationSeconds === 0
            ? 'No allocation selected. The session will still be recorded as combined study time.'
            : `${formatDuration(timerAllocationSeconds)} of ${formatDuration(
                timerAllocationLimitSeconds,
              )} allocated. Unallocated time remains counted once in combined totals.`}
        </p>
        {timerAllocationError && (
          <p className="v2-study-error" role="alert">
            {timerAllocationError}
          </p>
        )}
      </div>
    );
  }

  return (
    <section aria-labelledby="study-timer-heading" className="v2-study">
      <div className="v2-section-heading">
        <div>
          <p className="eyebrow">Study</p>
          <h2 id="study-timer-heading">Timer and sessions</h2>
        </div>
        <p>
          {focusExam
            ? `Exam context: ${focusExam.shortName}`
            : 'Combined study tracking across all four examinations'}
        </p>
      </div>

      {error && (
        <p className="v2-study-error" role="alert">
          {error}
        </p>
      )}
      {notice && (
        <p className="v2-study-notice" role="status">
          {notice}
        </p>
      )}

      <div className="v2-study-grid">
        <article className="v2-timer-card" aria-labelledby="timer-readout-heading">
          <div className="v2-timer-topline">
            <div>
              <p className="eyebrow">Study timer</p>
              <h3 id="timer-readout-heading">{MODE_LABELS[timer.state.mode]}</h3>
            </div>
            <span className={`v2-timer-status v2-timer-status-${completed ? 'completed' : status}`}>
              {statusLabel}
            </span>
          </div>

          <div
            aria-label={`${formatClock(elapsedMs)} elapsed, ${formatClock(remainingMs)} remaining`}
            className="v2-timer-readout"
            role="timer"
          >
            <span className="v2-timer-elapsed">{formatClock(elapsedMs)}</span>
            <span className="v2-timer-caption">elapsed · {formatClock(remainingMs)} remaining</span>
          </div>
          <div
            aria-hidden="true"
            className="v2-timer-progress"
            data-progress={Math.round(progress)}
          >
            <span style={{ width: `${progress}%` }} />
          </div>

          {recoveryNotice && <p className="v2-timer-recovery">{recoveryNotice}</p>}

          {locked ? (
            <p className="v2-empty-copy">
              This examination is inactive. Its historical study sessions remain visible, but new
              timers and allocation changes are disabled.
            </p>
          ) : (
            <>
              {status === 'idle' && (
                <div className="v2-timer-start">
                  <div className="v2-timer-start-grid">
                    <label className="v2-field">
                      <span>Timer mode</span>
                      <select
                        onChange={(event) => setMode(event.target.value as V2TimerMode)}
                        value={mode}
                      >
                        {(Object.keys(MODE_LABELS) as V2TimerMode[]).map((value) => (
                          <option key={value} value={value}>
                            {MODE_LABELS[value]} ({modeDurationMinutes(value, timer.settings)} min)
                          </option>
                        ))}
                      </select>
                    </label>
                    <label className="v2-field">
                      <span>Study context</span>
                      <select
                        disabled={mode !== 'focus'}
                        onChange={(event) => setSelectedExamId(event.target.value)}
                        value={mode === 'focus' ? selectedExamId : ''}
                      >
                        <option value="">No exam (combined)</option>
                        {activeExams.map((exam) => (
                          <option key={exam.code} value={exam.id}>
                            {exam.shortName} — {exam.code}
                          </option>
                        ))}
                      </select>
                    </label>
                  </div>
                  <p className="v2-empty-copy">
                    {mode === 'focus'
                      ? `A ${selectedModeMinutes}-minute focus timer will start. Study time is allocated to the selected examination topics when the session is saved.`
                      : `A ${selectedModeMinutes}-minute break timer will start. Breaks do not create study sessions.`}
                  </p>
                  {mode === 'focus' && renderAllocationBuilder(true)}
                  <button
                    className="primary-button"
                    disabled={busy}
                    onClick={handleStart}
                    type="button"
                  >
                    {busy ? 'Starting…' : 'Start timer'}
                  </button>
                </div>
              )}

              {status !== 'idle' && (
                <div className="v2-timer-actions">
                  {status === 'running' ? (
                    <button
                      className="secondary-button"
                      disabled={busy}
                      onClick={handlePause}
                      type="button"
                    >
                      Pause timer
                    </button>
                  ) : (
                    <button
                      className="primary-button"
                      disabled={busy}
                      onClick={handleResume}
                      type="button"
                    >
                      Resume timer
                    </button>
                  )}
                  <button
                    className="primary-button"
                    disabled={busy}
                    onClick={handleFinish}
                    type="button"
                  >
                    Stop &amp; save session
                  </button>
                  <button
                    className="secondary-button v2-danger-button"
                    disabled={busy}
                    onClick={handleDiscard}
                    type="button"
                  >
                    Discard timer
                  </button>
                </div>
              )}

              {status !== 'idle' && timer.state.mode === 'focus' && renderAllocationBuilder(true)}

              {settingsDraft && (
                <details className="v2-timer-settings">
                  <summary>Timer lengths</summary>
                  <div className="v2-timer-settings-grid">
                    <label className="v2-field">
                      <span>Focus minutes</span>
                      <input
                        max={240}
                        min={1}
                        onChange={(event) =>
                          setSettingsDraft({
                            ...settingsDraft,
                            focusMinutes: Number(event.target.value),
                          })
                        }
                        type="number"
                        value={settingsDraft.focusMinutes}
                      />
                    </label>
                    <label className="v2-field">
                      <span>Short break minutes</span>
                      <input
                        max={60}
                        min={1}
                        onChange={(event) =>
                          setSettingsDraft({
                            ...settingsDraft,
                            shortBreakMinutes: Number(event.target.value),
                          })
                        }
                        type="number"
                        value={settingsDraft.shortBreakMinutes}
                      />
                    </label>
                    <label className="v2-field">
                      <span>Long break minutes</span>
                      <input
                        max={120}
                        min={1}
                        onChange={(event) =>
                          setSettingsDraft({
                            ...settingsDraft,
                            longBreakMinutes: Number(event.target.value),
                          })
                        }
                        type="number"
                        value={settingsDraft.longBreakMinutes}
                      />
                    </label>
                  </div>
                  <button
                    className="secondary-button"
                    disabled={busy}
                    onClick={() => void handleSaveSettings()}
                    type="button"
                  >
                    Save timer lengths
                  </button>
                  <p className="v2-empty-copy">
                    Length changes apply to the next timer. A running timer keeps its original
                    length.
                  </p>
                </details>
              )}
            </>
          )}

          {lastEventNotice && <p className="v2-timer-last-event">{lastEventNotice}</p>}
        </article>

        <aside aria-labelledby="study-totals-heading" className="v2-study-totals">
          <p className="eyebrow">Recorded time</p>
          <h3 id="study-totals-heading">Study totals</h3>
          {totals ? (
            <>
              <dl className="v2-totals-facts">
                <div>
                  <dt>Combined study time</dt>
                  <dd>{formatDuration(totals.totalSeconds)}</dd>
                </div>
                <div>
                  <dt>Allocated to exams</dt>
                  <dd>{formatDuration(totals.allocatedSeconds)}</dd>
                </div>
                <div>
                  <dt>Unallocated time</dt>
                  <dd>{formatDuration(totals.unallocatedSeconds)}</dd>
                </div>
                <div>
                  <dt>Sessions</dt>
                  <dd>{totals.sessionCount}</dd>
                </div>
              </dl>
              <h4>Exam allocations</h4>
              <ul className="v2-total-exams">
                {totals.exams.map((examTotal) => {
                  const exam = exams.find((item) => item.id === examTotal.examId);
                  return (
                    <li key={examTotal.examId}>
                      <span>
                        {exam?.shortName ?? examTotal.examCode}
                        {!examTotal.examIsActive && ' (inactive)'}
                      </span>
                      <span>{formatDuration(examTotal.allocatedSeconds)}</span>
                    </li>
                  );
                })}
              </ul>
              {subjectTotals.length > 0 && (
                <>
                  <h4>Subject study time</h4>
                  <ul className="v2-total-exams">
                    {subjectTotals.slice(0, 8).map((subject) => (
                      <li key={`${subject.examCode}-${subject.subjectTitle}`}>
                        <span>
                          {exams.find((exam) => exam.code === subject.examCode)?.shortName ??
                            subject.examCode}{' '}
                          · {subject.subjectTitle}
                        </span>
                        <span>{formatDuration(subject.allocatedSeconds)}</span>
                      </li>
                    ))}
                  </ul>
                </>
              )}
              {totals.topics?.length > 0 && (
                <>
                  <h4>Topic study time</h4>
                  <ul className="v2-total-exams">
                    {totals.topics.slice(0, 12).map((topic) => {
                      const syllabusNode = syllabi[topic.examId]?.nodes.find(
                        (node) => node.id === topic.nodeId,
                      );
                      const status = syllabusNode?.progressStatus ?? 'not_started';
                      const examShortName =
                        exams.find((exam) => exam.code === topic.examCode)?.shortName ??
                        topic.examCode;
                      return (
                        <li key={`${topic.examId}-${topic.nodeId}`}>
                          <span>
                            {examShortName} · {topic.subjectTitle} · {topic.nodeTitle}{' '}
                            <span className="v2-status-badge v2-status-inactive">
                              {PROGRESS_LABELS[status] ?? status}
                            </span>
                          </span>
                          <span>
                            {formatDuration(topic.allocatedSeconds)} · {topic.sessionCount} session
                            {topic.sessionCount === 1 ? '' : 's'}
                            {topic.lastStudiedDate ? ` · last ${topic.lastStudiedDate}` : ''}
                          </span>
                        </li>
                      );
                    })}
                  </ul>
                </>
              )}
              <p className="v2-empty-copy">
                Combined time counts every session once. Exam totals use allocation records only;
                topic totals are a breakdown of those allocations.
              </p>
            </>
          ) : (
            <p className="v2-empty-copy">Study totals are not available yet.</p>
          )}
        </aside>
      </div>

      <section aria-labelledby="session-history-heading" className="v2-session-history">
        <div className="v2-section-heading">
          <div>
            <p className="eyebrow">History</p>
            <h3 id="session-history-heading">
              {focusExam ? `Sessions allocated to ${focusExam.shortName}` : 'Study sessions'}
            </h3>
          </div>
          <p>
            {visibleSessions.length} recorded session
            {visibleSessions.length === 1 ? '' : 's'} · newest first
          </p>
        </div>

        {!locked && (
          <form className="v2-manual-session-form" onSubmit={handleAddManualSession}>
            <p className="eyebrow">Manual entry</p>
            <h4>Record a study session</h4>
            <div className="v2-manual-session-grid">
              <label className="v2-field">
                <span>Session date</span>
                <input
                  onChange={(event) => setManualDate(event.target.value)}
                  type="date"
                  value={manualDate}
                />
              </label>
              <label className="v2-field">
                <span>Duration (minutes)</span>
                <input
                  max={1440}
                  min={1}
                  onChange={(event) => setManualMinutes(event.target.value)}
                  type="number"
                  value={manualMinutes}
                />
              </label>
              <label className="v2-field">
                <span>Examination context</span>
                <select
                  onChange={(event) => setManualExamId(event.target.value)}
                  value={manualExamId}
                >
                  <option value="">Unallocated</option>
                  {activeExams.map((exam) => (
                    <option key={exam.id} value={exam.id}>
                      {exam.shortName}
                    </option>
                  ))}
                </select>
              </label>
              <label className="v2-field v2-manual-session-notes">
                <span>Notes</span>
                <input
                  maxLength={4000}
                  onChange={(event) => setManualNotes(event.target.value)}
                  value={manualNotes}
                />
              </label>
            </div>
            <button className="secondary-button" disabled={busy} type="submit">
              Save manual session
            </button>
          </form>
        )}

        {visibleSessions.length === 0 ? (
          <p className="v2-empty-panel-copy">
            {focusExam
              ? 'No study sessions are allocated to this examination yet.'
              : 'No study sessions yet. Start the timer and stop it to save your first session.'}
          </p>
        ) : (
          <ul className="v2-session-list">
            {visibleSessions.map((session) => (
              <li className="v2-session-row" data-session-id={session.id} key={session.id}>
                <div className="v2-session-main">
                  <div className="v2-session-heading">
                    <strong>{formatDuration(session.durationSeconds)}</strong>
                    <span className="v2-session-date">{session.sessionDate}</span>
                    <span className="v2-status-badge v2-status-neutral">
                      {session.source === 'timer' ? 'Timer' : 'Manual'}
                    </span>
                  </div>
                  <div className="v2-allocation-chips">
                    {session.allocations.map((allocation) => (
                      <span
                        className={`v2-allocation-chip${
                          allocation.examIsActive ? '' : ' v2-allocation-chip-inactive'
                        }`}
                        key={allocation.examId}
                      >
                        {allocation.examCode}: {formatDuration(allocation.allocatedSeconds)}
                        {!allocation.examIsActive && ' (inactive)'}
                      </span>
                    ))}
                    {session.topicAllocations?.map((allocation) => (
                      <span
                        className="v2-allocation-chip"
                        key={`${allocation.examId}-${allocation.nodeId}`}
                      >
                        {allocation.examCode}: {allocation.nodeTitle} ·{' '}
                        {formatDuration(allocation.allocatedSeconds)}
                      </span>
                    ))}
                    {session.unallocatedSeconds > 0 && (
                      <span className="v2-allocation-chip v2-allocation-unallocated">
                        Unallocated: {formatDuration(session.unallocatedSeconds)}
                      </span>
                    )}
                    {session.allocations.length === 0 && (
                      <span className="v2-allocation-chip v2-allocation-unallocated">
                        Unallocated: full session
                      </span>
                    )}
                  </div>
                  {session.notes && <p className="v2-session-notes">{session.notes}</p>}
                  {editingNoteId === session.id && !locked && (
                    <form
                      className="v2-allocation-form"
                      onSubmit={(event) => {
                        event.preventDefault();
                        void saveSessionNote(session);
                      }}
                    >
                      <label className="v2-field">
                        <span>Session notes</span>
                        <textarea
                          maxLength={4000}
                          onChange={(event) => setNoteDraft(event.target.value)}
                          rows={3}
                          value={noteDraft}
                        />
                      </label>
                      <div className="v2-form-actions">
                        <button className="primary-button" disabled={busy} type="submit">
                          Save note
                        </button>
                        <button
                          className="secondary-button"
                          onClick={() => setEditingNoteId(null)}
                          type="button"
                        >
                          Cancel
                        </button>
                      </div>
                    </form>
                  )}
                </div>
                {!locked && (
                  <div className="v2-form-actions">
                    <button
                      className="secondary-button v2-allocate-button"
                      onClick={() => openAllocationEditor(session)}
                      type="button"
                    >
                      {editingSessionId === session.id ? 'Close allocation' : 'Allocate time'}
                    </button>
                    <button
                      className="secondary-button v2-allocate-button"
                      onClick={() => beginNoteEdit(session)}
                      type="button"
                    >
                      Edit notes
                    </button>
                    <button
                      className="secondary-button v2-danger-button v2-allocate-button"
                      disabled={busy}
                      onClick={() => void deleteSession(session)}
                      type="button"
                    >
                      {deletingSessionId === session.id ? 'Confirm delete' : 'Delete'}
                    </button>
                  </div>
                )}

                {editingSessionId === session.id && !locked && (
                  <form
                    className="v2-allocation-form"
                    onSubmit={(event) => {
                      event.preventDefault();
                      void handleSaveAllocations(session);
                    }}
                  >
                    <h4>Allocate session time</h4>
                    <p className="v2-empty-copy">
                      Session duration: {formatDuration(session.durationSeconds)}. Enter time in
                      minutes for one or more examinations. Existing inactive allocations are
                      preserved and cannot be changed.
                    </p>
                    <div className="v2-allocation-grid">
                      {exams.map((exam) => {
                        const existing = session.allocations.find(
                          (allocation) => allocation.examId === exam.id,
                        );
                        const unavailable = !exam.isActive && !existing;
                        return (
                          <label className="v2-field" key={exam.id}>
                            <span>
                              {exam.shortName}
                              {!exam.isActive && ' (inactive)'}
                            </span>
                            <input
                              disabled={unavailable || (!exam.isActive && Boolean(existing))}
                              min={0}
                              onChange={(event) =>
                                setAllocationDraft((current) => ({
                                  ...current,
                                  [exam.id]: event.target.value,
                                }))
                              }
                              placeholder={unavailable ? 'Not available' : '0'}
                              step={0.5}
                              type="number"
                              value={allocationDraft[exam.id] ?? ''}
                            />
                          </label>
                        );
                      })}
                    </div>
                    <p
                      className={
                        draftAllocatedSeconds > session.durationSeconds
                          ? 'v2-study-error'
                          : 'v2-empty-copy'
                      }
                    >
                      Allocated: {formatDuration(draftAllocatedSeconds)} · Unallocated:{' '}
                      {formatDuration(Math.max(0, session.durationSeconds - draftAllocatedSeconds))}
                    </p>
                    {allocationError && (
                      <p className="v2-study-error" role="alert">
                        {allocationError}
                      </p>
                    )}
                    <div className="v2-form-actions">
                      <button className="primary-button" disabled={busy} type="submit">
                        Save allocations
                      </button>
                      <button
                        className="secondary-button"
                        onClick={() => {
                          setEditingSessionId(null);
                          setAllocationError(null);
                        }}
                        type="button"
                      >
                        Cancel
                      </button>
                    </div>
                  </form>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>
    </section>
  );
}
