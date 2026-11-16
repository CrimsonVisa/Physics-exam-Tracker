import { type FormEvent, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification';
import { toCommandError } from '../lib/commandError';
import { emitStudySessionsChanged } from '../lib/studyEvents';
import {
  createV2ManualSession,
  deleteV2StudySession,
  discardV2Timer,
  finishV2Timer,
  getV2ExamSyllabus,
  getV2StudyTotals,
  listV2Sessions,
  pauseV2Timer,
  resumeV2Timer,
  setV2SessionExams,
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
import { formatTimerClock, TIMER_CARD_MODE_LABELS } from './timer/format';
import { useV2Timer } from './timer/timerContext';

type StudySectionProps = {
  exams: V2ExamSummary[];
  today: string;
  focusExamCode?: string;
  readOnly?: boolean;
  focusRequest?: number;
  onTimerFocused?: (requestId: number) => void;
};

const STATUS_LABELS = {
  idle: 'Idle',
  running: 'Running',
  paused: 'Paused',
} as const;

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

async function ensureNotificationPermission(): Promise<boolean> {
  if (await isPermissionGranted()) {
    return true;
  }
  return (await requestPermission()) === 'granted';
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

export function StudySection({
  exams,
  today,
  focusExamCode,
  readOnly = false,
  focusRequest = 0,
  onTimerFocused,
}: StudySectionProps) {
  const focusExam = useMemo(
    () => exams.find((exam) => exam.code === focusExamCode) ?? null,
    [exams, focusExamCode],
  );
  const locked = readOnly || (focusExam !== null && !focusExam.isActive);

  const {
    snapshot: timer,
    loading: timerLoading,
    error: timerError,
    elapsedMs,
    remainingMs,
    progress,
    runMutation,
    refresh: refreshTimer,
  } = useV2Timer();
  const [sessions, setSessions] = useState<V2StudySession[]>([]);
  const [totals, setTotals] = useState<V2StudyTotals | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [mode, setMode] = useState<V2TimerMode>('focus');
  const [selectedExamIds, setSelectedExamIds] = useState<number[]>(() =>
    focusExam && focusExam.isActive ? [focusExam.id] : [],
  );
  const [settingsDraft, setSettingsDraft] = useState<V2TimerSettings | null>(null);
  const [editingSessionId, setEditingSessionId] = useState<number | null>(null);
  const [sessionExamDraft, setSessionExamDraft] = useState<number[]>([]);
  const [allocationError, setAllocationError] = useState<string | null>(null);
  const [syllabi, setSyllabi] = useState<Record<number, V2ExamSyllabus>>({});
  const [allocationRows, setAllocationRows] = useState<AllocationDraft[]>([]);
  const [timerAllocationError, setTimerAllocationError] = useState<string | null>(null);
  const allocationKey = useRef(1);
  const [manualDate, setManualDate] = useState(today);
  const [manualMinutes, setManualMinutes] = useState('30');
  const [manualExamIds, setManualExamIds] = useState<number[]>([]);
  const [manualNotes, setManualNotes] = useState('');
  const [editingNoteId, setEditingNoteId] = useState<number | null>(null);
  const [noteDraft, setNoteDraft] = useState('');
  const [deletingSessionId, setDeletingSessionId] = useState<number | null>(null);
  const previousTimerStatus = useRef(timer?.state.status ?? null);
  const lastPersistedSettings = useRef<V2TimerSettings | null>(null);
  const lastHandledFocusRequest = useRef(0);

  const refreshRecords = useCallback(async () => {
    const [nextSessions, nextTotals] = await Promise.all([listV2Sessions(), getV2StudyTotals()]);
    setSessions(nextSessions);
    setTotals(nextTotals);
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      await refreshTimer(true);
      const [nextSessions, nextTotals] = await Promise.all([listV2Sessions(), getV2StudyTotals()]);
      setSessions(nextSessions);
      setTotals(nextTotals);
    } catch (loadError) {
      setError(toCommandError(loadError).message);
    } finally {
      setLoading(false);
    }
  }, [refreshTimer]);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const [nextSessions, nextTotals] = await Promise.all([
          listV2Sessions(),
          getV2StudyTotals(),
        ]);
        if (cancelled) {
          return;
        }
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
  }, []);

  useEffect(() => {
    if (!timer) {
      return;
    }
    const next = timer.settings;
    const previous = lastPersistedSettings.current;
    if (
      !previous ||
      previous.focusMinutes !== next.focusMinutes ||
      previous.shortBreakMinutes !== next.shortBreakMinutes ||
      previous.longBreakMinutes !== next.longBreakMinutes ||
      previous.notificationsEnabled !== next.notificationsEnabled
    ) {
      lastPersistedSettings.current = next;
      // Synchronize the editable draft only when persisted settings change.
      setSettingsDraft(next);
    }
  }, [timer]);

  useEffect(() => {
    if (timer?.state.status === 'idle') {
      // Reset the selected mode only when the persisted timer returns to idle.
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setMode(timer.state.mode);
    }
  }, [timer?.state.status, timer?.state.mode]);

  useEffect(() => {
    if (!focusRequest) {
      lastHandledFocusRequest.current = 0;
      return;
    }
    if (focusRequest === lastHandledFocusRequest.current || !timer) {
      return;
    }
    const heading = document.getElementById('study-timer-heading');
    if (!heading) {
      return;
    }
    lastHandledFocusRequest.current = focusRequest;
    heading.scrollIntoView?.({ behavior: 'smooth', block: 'start' });
    heading.focus({ preventScroll: true });
    onTimerFocused?.(focusRequest);
  }, [focusRequest, onTimerFocused, timer]);

  useEffect(() => {
    const previous = previousTimerStatus.current;
    previousTimerStatus.current = timer?.state.status ?? null;
    if ((previous === 'running' || previous === 'paused') && timer?.state.status === 'idle') {
      void refreshRecords().catch((refreshError) => setError(toCommandError(refreshError).message));
    }
  }, [timer?.state.status, refreshRecords]);

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

  const visibleSessions = focusExam
    ? sessions.filter((session) =>
        session.allocations.some((allocation) => allocation.examId === focusExam.id),
      )
    : sessions;

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
    const firstExam = allocationExamOptions[0];
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

  function toggleSelectedExam(examId: number, selected: boolean) {
    setSelectedExamIds((current) =>
      selected
        ? current.includes(examId)
          ? current
          : [...current, examId]
        : current.filter((value) => value !== examId),
    );
  }

  function toggleManualExam(examId: number, selected: boolean) {
    setManualExamIds((current) =>
      selected
        ? current.includes(examId)
          ? current
          : [...current, examId]
        : current.filter((value) => value !== examId),
    );
  }

  function toggleSessionExam(examId: number, selected: boolean) {
    setSessionExamDraft((current) =>
      selected
        ? current.includes(examId)
          ? current
          : [...current, examId]
        : current.filter((value) => value !== examId),
    );
  }

  function validateTimerAllocations(): boolean {
    const running = timer?.state.status !== 'idle';
    const allowedExamIds = running ? (timer?.examIds ?? []) : selectedExamIds;
    if (
      (running || allowedExamIds.length > 0) &&
      timerAllocationInputs.some((allocation) => !allowedExamIds.includes(allocation.examId))
    ) {
      setTimerAllocationError(
        'Remove topic allocations for examinations that are not selected for this timer.',
      );
      return false;
    }
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
      await runMutation(() =>
        setV2TimerAllocations({
          allocations: timerAllocationInputs,
        }),
      );
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
      const snapshot = await runMutation(action);
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
    const activeExamId = mode === 'focus' && selectedExamIds.length > 0 ? selectedExamIds[0] : null;
    void runTimerAction(
      () =>
        startV2Timer({
          mode,
          sessionDate: today,
          activeExamId,
          examIds: mode === 'focus' ? selectedExamIds : [],
          allocations: mode === 'focus' ? timerAllocationInputs : [],
        }),
      () => 'Timer started.',
    );
  }

  function handlePause() {
    void runTimerAction(pauseV2Timer, (snapshot) => {
      if (snapshot.state.lastEvent === 'completed') {
        return snapshot.state.lastCompletedSessionId !== null
          ? 'Timer completed before pause. The session was saved.'
          : 'Timer completed before pause. The break had already ended.';
      }
      return 'Timer paused.';
    });
  }

  function handleResume() {
    void runTimerAction(resumeV2Timer, () => 'Timer resumed.');
  }

  function handleFinish() {
    void runTimerAction(finishV2Timer, (snapshot) =>
      snapshot.state.lastCompletedSessionId !== null
        ? `Session saved: ${formatTimerClock(snapshot.state.lastEventDurationMs)}.`
        : `Timer completed: ${formatTimerClock(snapshot.state.lastEventDurationMs)}.`,
    );
  }

  function handleDiscard() {
    if (!window.confirm('Discard this timer? No study session will be saved.')) {
      return;
    }
    void runTimerAction(discardV2Timer, (snapshot) => {
      if (snapshot.state.lastEvent === 'completed') {
        return snapshot.state.lastCompletedSessionId !== null
          ? 'Timer had already completed. The session was saved.'
          : 'Timer had already completed. The break had ended.';
      }
      return 'Timer discarded. No session was saved.';
    });
  }

  async function handleSaveSettings() {
    if (!settingsDraft) {
      return;
    }
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      if (settingsDraft.notificationsEnabled && !(await ensureNotificationPermission())) {
        setError(
          'Windows did not grant notification permission. Timer settings were not saved. You can still use the timer without notifications.',
        );
        return;
      }
      await runMutation(() => updateV2TimerSettings(settingsDraft));
      setNotice(
        settingsDraft.notificationsEnabled
          ? 'Timer lengths and desktop notification preference saved for the next timer.'
          : 'Timer lengths saved for the next timer.',
      );
    } catch (settingsError) {
      setError(toCommandError(settingsError).message);
    } finally {
      setBusy(false);
    }
  }

  function openExamEditor(session: V2StudySession) {
    setSessionExamDraft(session.allocations.map((allocation) => allocation.examId));
    setAllocationError(null);
    setEditingSessionId(session.id);
  }

  async function handleSaveSessionExams(session: V2StudySession) {
    setAllocationError(null);
    if (sessionExamDraft.length === 0) {
      setAllocationError('Choose at least one examination for this session.');
      return;
    }

    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await setV2SessionExams({ sessionId: session.id, examIds: sessionExamDraft });
      emitStudySessionsChanged();
      await refreshRecords();
      setEditingSessionId(null);
      setNotice('Session exam associations saved.');
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
        primaryExamId: manualExamIds[0] ?? null,
        examIds: manualExamIds,
      });
      setManualNotes('');
      setManualExamIds([]);
      emitStudySessionsChanged();
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
      emitStudySessionsChanged();
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
      emitStudySessionsChanged();
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
        ? `Previous timer saved as session #${timer.state.lastCompletedSessionId} (${formatTimerClock(
            timer.state.lastEventDurationMs,
          )}).`
        : `Previous break completed after ${formatTimerClock(timer.state.lastEventDurationMs)}. No study session was saved.`
      : timer?.state.lastEvent === 'discarded'
        ? `Previous timer was discarded after ${formatTimerClock(timer.state.lastEventDurationMs)}.`
        : null;

  const recoveryNotice = timer?.state.recoveryReason
    ? timer.state.recoveryReason === 'clockChange'
      ? 'A clock change was detected. Unverified time was not counted.'
      : `The timer was paused after ${timer.state.recoveryReason === 'restart' ? 'an application restart' : 'missing checkpoints'}. ${formatDuration(
          Math.round(timer.state.recoveryExcludedMs / 1_000),
        )} of unverified time was excluded.`
    : null;

  if ((loading || timerLoading) && !timer) {
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
          {timerError ?? error ?? 'The study timer could not be loaded.'}
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
  const allocationExamOptions =
    timer?.state.status !== 'idle'
      ? activeExams.filter((exam) => timer.examIds.includes(exam.id))
      : selectedExamIds.length > 0
        ? activeExams.filter((exam) => selectedExamIds.includes(exam.id))
        : activeExams;

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
                  {allocationExamOptions.map((exam) => (
                    <option key={exam.id} value={exam.id}>
                      {exam.shortName} - {exam.code}
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
            disabled={!editable || busy || allocationExamOptions.length === 0}
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
        {editable && timer?.state.status !== 'idle' && (
          <p className="v2-empty-copy">
            Only the examinations selected when this timer started can receive topic allocations.
            Exam associations are locked for the running timer.
          </p>
        )}
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
          <h2 id="study-timer-heading" tabIndex={-1}>
            Timer and sessions
          </h2>
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
      {timerError && (
        <p className="v2-study-error" role="alert">
          {timerError}
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
              <h3 id="timer-readout-heading">{TIMER_CARD_MODE_LABELS[timer.state.mode]}</h3>
            </div>
            <span className={`v2-timer-status v2-timer-status-${completed ? 'completed' : status}`}>
              {statusLabel}
            </span>
          </div>

          <div
            aria-label={`${formatTimerClock(elapsedMs)} elapsed, ${formatTimerClock(remainingMs)} remaining`}
            className="v2-timer-readout"
            role="timer"
          >
            <span className="v2-timer-elapsed">{formatTimerClock(elapsedMs)}</span>
            <span className="v2-timer-caption">
              elapsed · {formatTimerClock(remainingMs)} remaining
            </span>
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
                        {(Object.keys(TIMER_CARD_MODE_LABELS) as V2TimerMode[]).map((value) => (
                          <option key={value} value={value}>
                            {TIMER_CARD_MODE_LABELS[value]} (
                            {modeDurationMinutes(value, timer.settings)} min)
                          </option>
                        ))}
                      </select>
                    </label>
                    <fieldset className="v2-exam-selector" disabled={mode !== 'focus'}>
                      <legend>Examinations</legend>
                      <div className="v2-exam-choice-grid">
                        {activeExams.map((exam) => (
                          <label className="v2-checkbox-field" key={exam.code}>
                            <input
                              checked={selectedExamIds.includes(exam.id)}
                              onChange={(event) =>
                                toggleSelectedExam(exam.id, event.target.checked)
                              }
                              type="checkbox"
                            />
                            <span>
                              {exam.shortName} — {exam.code}
                            </span>
                          </label>
                        ))}
                      </div>
                    </fieldset>
                  </div>
                  <p className="v2-empty-copy">
                    {mode === 'focus'
                      ? `A ${selectedModeMinutes}-minute focus timer will start. Every selected examination receives the full session duration; combined study time still counts the session once.${
                          selectedExamIds.length === 0
                            ? ' No examination is selected, so the session will be recorded as combined study time only.'
                            : ''
                        }`
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

              {status !== 'idle' && timer.state.mode === 'focus' && (
                <div className="v2-timer-exam-context">
                  <p className="eyebrow">Exam associations</p>
                  <div className="v2-allocation-chips">
                    {timer.examIds.length === 0 ? (
                      <span className="v2-allocation-chip v2-allocation-unallocated">
                        Combined study time only
                      </span>
                    ) : (
                      timer.examIds.map((examId) => {
                        const exam = exams.find((item) => item.id === examId);
                        return (
                          <span className="v2-allocation-chip" key={examId}>
                            {exam?.shortName ?? `Exam ${examId}`}: full session duration
                          </span>
                        );
                      })
                    )}
                  </div>
                  <p className="v2-empty-copy">
                    Exam associations are locked for this running timer. Each selected exam will
                    receive the full session duration; combined time is counted once.
                  </p>
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
                    <label className="v2-checkbox-field">
                      <input
                        checked={settingsDraft.notificationsEnabled}
                        onChange={(event) =>
                          setSettingsDraft({
                            ...settingsDraft,
                            notificationsEnabled: event.target.checked,
                          })
                        }
                        type="checkbox"
                      />
                      <span>Desktop notifications</span>
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
                    length. Desktop notifications appear when a focus session completes, including
                    while the window is minimized. If they do not appear, check Windows Settings →
                    System → Notifications.
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
                  <dt>Time credited to an exam</dt>
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
              <h4>Exam credit (may overlap)</h4>
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
                Combined time counts every session once. A session associated with several
                examinations credits its full duration to each of them, so exam totals may overlap.
                Topic totals are a separate breakdown of recorded topic detail.
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
              <fieldset className="v2-exam-selector v2-manual-session-exams">
                <legend>Examinations</legend>
                <div className="v2-exam-choice-grid">
                  {activeExams.map((exam) => (
                    <label className="v2-checkbox-field" key={exam.id}>
                      <input
                        checked={manualExamIds.includes(exam.id)}
                        onChange={(event) => toggleManualExam(exam.id, event.target.checked)}
                        type="checkbox"
                      />
                      <span>{exam.shortName}</span>
                    </label>
                  ))}
                </div>
              </fieldset>
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
                  <div className="v2-session-actions">
                    <button
                      className="secondary-button v2-allocate-button"
                      onClick={() => openExamEditor(session)}
                      type="button"
                    >
                      {editingSessionId === session.id ? 'Close exams' : 'Edit exams'}
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
                      void handleSaveSessionExams(session);
                    }}
                  >
                    <h4>Edit session examinations</h4>
                    <p className="v2-empty-copy">
                      Session duration: {formatDuration(session.durationSeconds)}. Select one or
                      more examinations. Each selected examination receives the full session
                      duration; combined study time still counts this session once. Inactive
                      examinations with an existing association are preserved.
                    </p>
                    <div className="v2-exam-choice-grid">
                      {exams.map((exam) => {
                        const existing = session.allocations.some(
                          (allocation) => allocation.examId === exam.id,
                        );
                        const unavailable = !exam.isActive && !existing;
                        return (
                          <label className="v2-checkbox-field" key={exam.id}>
                            <input
                              checked={sessionExamDraft.includes(exam.id)}
                              disabled={unavailable || (!exam.isActive && existing)}
                              onChange={(event) => toggleSessionExam(exam.id, event.target.checked)}
                              type="checkbox"
                            />
                            <span>
                              {exam.shortName}
                              {!exam.isActive && ' (inactive)'}
                              {unavailable && ' — not available'}
                            </span>
                          </label>
                        );
                      })}
                    </div>
                    <p className="v2-empty-copy">
                      {sessionExamDraft.length} examination
                      {sessionExamDraft.length === 1 ? '' : 's'} selected ·{' '}
                      {formatDuration(session.durationSeconds)} credited to each.
                    </p>
                    {allocationError && (
                      <p className="v2-study-error" role="alert">
                        {allocationError}
                      </p>
                    )}
                    <div className="v2-form-actions">
                      <button className="primary-button" disabled={busy} type="submit">
                        Save exams
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
