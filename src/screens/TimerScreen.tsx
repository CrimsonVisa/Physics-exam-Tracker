import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification';
import { type FormEvent, useCallback, useEffect, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import { searchSyllabusTopics, type SyllabusTopic } from '../lib/syllabusApi';
import { formatStudyDuration, formatTimerClock } from '../lib/studyTime';
import {
  discardStudyTimer,
  finishStudyTimer,
  getTimerSnapshot,
  pauseStudyTimer,
  resumeStudyTimer,
  startStudyTimer,
  updateTimerSettings,
  type TimerMode,
  type TimerSettings,
  type TimerSettingsInput,
  type TimerSnapshot,
} from '../lib/timerApi';

type SettingsForm = {
  focusMinutes: string;
  shortBreakMinutes: string;
  longBreakMinutes: string;
  notificationsEnabled: boolean;
};

type SettingsField = 'focusMinutes' | 'shortBreakMinutes' | 'longBreakMinutes';

type Feedback = {
  tone: 'success' | 'error';
  message: string;
} | null;

const MODE_LABELS: Record<TimerMode, string> = {
  focus: 'Focus',
  short_break: 'Short break',
  long_break: 'Long break',
};

const MODE_OPTIONS: { mode: TimerMode; label: string; description: string }[] = [
  { mode: 'focus', label: 'Focus', description: 'Counts toward study time' },
  { mode: 'short_break', label: 'Short break', description: 'Never counted as study' },
  { mode: 'long_break', label: 'Long break', description: 'Never counted as study' },
];

function settingsToForm(settings: TimerSettings): SettingsForm {
  return {
    focusMinutes: String(settings.focusMinutes),
    shortBreakMinutes: String(settings.shortBreakMinutes),
    longBreakMinutes: String(settings.longBreakMinutes),
    notificationsEnabled: settings.notificationsEnabled,
  };
}

function parseWholeNumber(value: string, minimum: number, maximum: number): number | null {
  const trimmed = value.trim();
  if (!/^\d+$/.test(trimmed)) {
    return null;
  }

  const parsed = Number(trimmed);
  return parsed >= minimum && parsed <= maximum ? parsed : null;
}

function validateSettings(form: SettingsForm): {
  values: TimerSettingsInput | null;
  errors: Partial<Record<SettingsField, string>>;
} {
  const errors: Partial<Record<SettingsField, string>> = {};
  const focusMinutes = parseWholeNumber(form.focusMinutes, 1, 240);
  const shortBreakMinutes = parseWholeNumber(form.shortBreakMinutes, 1, 60);
  const longBreakMinutes = parseWholeNumber(form.longBreakMinutes, 1, 120);

  if (focusMinutes === null) {
    errors.focusMinutes = 'Enter a whole number of minutes from 1 to 240.';
  }
  if (shortBreakMinutes === null) {
    errors.shortBreakMinutes = 'Enter a whole number of minutes from 1 to 60.';
  }
  if (longBreakMinutes === null) {
    errors.longBreakMinutes = 'Enter a whole number of minutes from 1 to 120.';
  }

  if (focusMinutes === null || shortBreakMinutes === null || longBreakMinutes === null) {
    return { values: null, errors };
  }

  return {
    values: {
      focusMinutes,
      shortBreakMinutes,
      longBreakMinutes,
      notificationsEnabled: form.notificationsEnabled,
    },
    errors,
  };
}

function settingsAreDirty(form: SettingsForm, settings: TimerSettings): boolean {
  return (
    parseWholeNumber(form.focusMinutes, 1, 240) !== settings.focusMinutes ||
    parseWholeNumber(form.shortBreakMinutes, 1, 60) !== settings.shortBreakMinutes ||
    parseWholeNumber(form.longBreakMinutes, 1, 120) !== settings.longBreakMinutes ||
    form.notificationsEnabled !== settings.notificationsEnabled
  );
}

async function ensureNotificationPermission(): Promise<boolean> {
  try {
    if (await isPermissionGranted()) {
      return true;
    }

    return (await requestPermission()) === 'granted';
  } catch {
    return false;
  }
}

function FieldError({ id, message }: { id: string; message?: string }) {
  if (!message) {
    return null;
  }

  return (
    <p className="field-error" id={id} role="alert">
      {message}
    </p>
  );
}

export default function TimerScreen() {
  const [snapshot, setSnapshot] = useState<TimerSnapshot | null>(null);
  const [anchor, setAnchor] = useState({ serverNowMs: 0, clientNow: 0 });
  const [clockNow, setClockNow] = useState(() => Date.now());
  const [topics, setTopics] = useState<SyllabusTopic[]>([]);
  const [topicsError, setTopicsError] = useState<string | null>(null);
  const [topicsReloadKey, setTopicsReloadKey] = useState(0);
  const [selectedMode, setSelectedMode] = useState<TimerMode>('focus');
  const [selectedTopicId, setSelectedTopicId] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const [settingsForm, setSettingsForm] = useState<SettingsForm | null>(null);
  const [settingsErrors, setSettingsErrors] = useState<Partial<Record<SettingsField, string>>>({});
  const [settingsFeedback, setSettingsFeedback] = useState<Feedback>(null);
  const [isSavingSettings, setIsSavingSettings] = useState(false);

  const acceptSnapshot = useCallback((next: TimerSnapshot) => {
    setSnapshot(next);
    setAnchor({ serverNowMs: next.serverNowMs, clientNow: Date.now() });
    setClockNow(Date.now());
  }, []);

  const refreshSnapshot = useCallback(async () => {
    const next = await getTimerSnapshot();
    acceptSnapshot(next);
    return next;
  }, [acceptSnapshot]);

  useEffect(() => {
    let cancelled = false;

    getTimerSnapshot()
      .then((next) => {
        if (cancelled) {
          return;
        }

        acceptSnapshot(next);
        setSettingsForm(settingsToForm(next.settings));
        setSelectedMode(next.state.mode);
        setSelectedTopicId(next.state.topic ? String(next.state.topic.id) : '');
        setLoadError(null);
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setLoadError(toCommandError(error).message);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setIsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [reloadKey, acceptSnapshot]);

  useEffect(() => {
    let cancelled = false;

    searchSyllabusTopics({})
      .then((catalogTopics) => {
        if (!cancelled) {
          setTopics(catalogTopics);
          setTopicsError(null);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setTopicsError(toCommandError(error).message);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [topicsReloadKey]);

  useEffect(() => {
    if (snapshot?.state.status !== 'running') {
      return;
    }

    const id = window.setInterval(() => {
      setClockNow(Date.now());

      if (document.visibilityState === 'visible') {
        void refreshSnapshot().catch((error: unknown) => {
          setActionError(toCommandError(error).message);
        });
      }
    }, 5_000);

    return () => window.clearInterval(id);
  }, [snapshot?.state.status, refreshSnapshot]);

  useEffect(() => {
    if (snapshot?.state.status !== 'running') {
      return;
    }

    const id = window.setInterval(() => setClockNow(Date.now()), 250);
    return () => window.clearInterval(id);
  }, [snapshot?.state.status]);

  useEffect(() => {
    function handleWake() {
      if (document.visibilityState !== 'visible') {
        return;
      }

      void refreshSnapshot().catch((error: unknown) => {
        setActionError(toCommandError(error).message);
      });
    }

    window.addEventListener('focus', handleWake);
    document.addEventListener('visibilitychange', handleWake);
    return () => {
      window.removeEventListener('focus', handleWake);
      document.removeEventListener('visibilitychange', handleWake);
    };
  }, [refreshSnapshot]);

  const displayRemainingMs = (() => {
    if (!snapshot) {
      return 0;
    }

    if (snapshot.state.status === 'idle') {
      const idleMinutes =
        selectedMode === 'focus'
          ? snapshot.settings.focusMinutes
          : selectedMode === 'short_break'
            ? snapshot.settings.shortBreakMinutes
            : snapshot.settings.longBreakMinutes;
      return Math.max(0, idleMinutes * 60_000);
    }

    if (snapshot.state.status !== 'running') {
      return snapshot.state.remainingMs;
    }

    const elapsed = Math.max(0, clockNow - anchor.clientNow);
    return Math.max(0, snapshot.state.remainingMs - elapsed);
  })();

  const activeMode: TimerMode =
    snapshot && snapshot.state.status !== 'idle' ? snapshot.state.mode : selectedMode;
  const modeLabel = MODE_LABELS[activeMode];
  const statusLabel =
    snapshot?.state.status === 'running'
      ? 'Running'
      : snapshot?.state.status === 'paused'
        ? 'Paused'
        : 'Ready';
  const canUseTopic = activeMode === 'focus';
  const isActive = snapshot ? snapshot.state.status !== 'idle' : false;
  const isRunning = snapshot?.state.status === 'running';
  const isPaused = snapshot?.state.status === 'paused';

  async function handleStart() {
    if (!snapshot || snapshot.state.status !== 'idle' || busyAction) {
      return;
    }

    const topicId = canUseTopic && selectedTopicId ? Number(selectedTopicId) : null;
    if (topicId !== null && (!Number.isSafeInteger(topicId) || topicId <= 0)) {
      setActionError('Choose a topic from the syllabus.');
      return;
    }

    setBusyAction('start');
    setActionError(null);
    setConfirmDiscard(false);

    try {
      const next = await startStudyTimer(activeMode, topicId);
      acceptSnapshot(next);
    } catch (error: unknown) {
      setActionError(toCommandError(error).message);
      void refreshSnapshot().catch(() => undefined);
    } finally {
      setBusyAction(null);
    }
  }

  async function handlePause() {
    if (!isRunning || busyAction) {
      return;
    }

    setBusyAction('pause');
    setActionError(null);

    try {
      acceptSnapshot(await pauseStudyTimer());
    } catch (error: unknown) {
      setActionError(toCommandError(error).message);
      void refreshSnapshot().catch(() => undefined);
    } finally {
      setBusyAction(null);
    }
  }

  async function handleResume() {
    if (!isPaused || busyAction) {
      return;
    }

    setBusyAction('resume');
    setActionError(null);

    try {
      acceptSnapshot(await resumeStudyTimer());
    } catch (error: unknown) {
      setActionError(toCommandError(error).message);
      void refreshSnapshot().catch(() => undefined);
    } finally {
      setBusyAction(null);
    }
  }

  async function handleFinish() {
    if (!isActive || busyAction) {
      return;
    }

    setBusyAction('finish');
    setActionError(null);

    try {
      acceptSnapshot(await finishStudyTimer());
    } catch (error: unknown) {
      setActionError(toCommandError(error).message);
      void refreshSnapshot().catch(() => undefined);
    } finally {
      setBusyAction(null);
    }
  }

  async function handleDiscard() {
    if (!isActive || busyAction) {
      return;
    }

    setBusyAction('discard');
    setActionError(null);

    try {
      acceptSnapshot(await discardStudyTimer());
      setConfirmDiscard(false);
    } catch (error: unknown) {
      setActionError(toCommandError(error).message);
    } finally {
      setBusyAction(null);
    }
  }

  function handleModeChange(mode: TimerMode) {
    setSelectedMode(mode);
    if (mode !== 'focus') {
      setSelectedTopicId('');
    }
    setActionError(null);
  }

  async function handleSaveSettings(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!settingsForm || !snapshot || isSavingSettings) {
      return;
    }

    setSettingsFeedback(null);
    const validation = validateSettings(settingsForm);
    setSettingsErrors(validation.errors);
    if (!validation.values) {
      return;
    }

    setIsSavingSettings(true);

    try {
      if (validation.values.notificationsEnabled) {
        const granted = await ensureNotificationPermission();
        if (!granted) {
          setSettingsFeedback({
            tone: 'error',
            message:
              'Windows did not grant notification permission. Timer settings were not saved. You can still use the timer without notifications.',
          });
          return;
        }
      }

      const next = await updateTimerSettings(validation.values);
      acceptSnapshot(next);
      setSettingsForm(settingsToForm(next.settings));
      setSettingsErrors({});
      setSettingsFeedback({
        tone: 'success',
        message: 'Timer settings saved. They apply to the next timer you start.',
      });
    } catch (error: unknown) {
      setSettingsFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setIsSavingSettings(false);
    }
  }

  function handleSettingsField(field: SettingsField, value: string) {
    setSettingsForm((current) => (current ? { ...current, [field]: value } : current));
    setSettingsErrors((current) => {
      if (!current[field]) {
        return current;
      }

      const next = { ...current };
      delete next[field];
      return next;
    });
    setSettingsFeedback(null);
  }

  const settingsDirty =
    settingsForm && snapshot ? settingsAreDirty(settingsForm, snapshot.settings) : false;

  return (
    <div className="page-container timer-page">
      <header className="page-header">
        <p className="eyebrow">Study timer</p>
        <h1>Focused study blocks</h1>
        <p className="page-lede">
          The timer uses saved timestamps rather than a browser countdown. Only focus time is
          recorded; breaks and paused time never enter study totals.
        </p>
      </header>

      {isLoading && (
        <section className="state-card" role="status" aria-live="polite">
          <p className="state-title">Loading the study timer.</p>
          <p className="state-copy">Restoring saved timer state and the syllabus topic list.</p>
        </section>
      )}

      {!isLoading && loadError && (
        <section className="state-card state-card-error" role="alert">
          <p className="state-title">The study timer could not be loaded</p>
          <p className="state-copy">{loadError}</p>
          <button
            type="button"
            className="primary-button"
            onClick={() => {
              setIsLoading(true);
              setLoadError(null);
              setReloadKey((key) => key + 1);
            }}
          >
            Retry
          </button>
        </section>
      )}

      {!isLoading && !loadError && snapshot && settingsForm && (
        <div className="timer-layout">
          <section
            className={`timer-card timer-mode-${activeMode}`}
            aria-labelledby="timer-heading"
          >
            <div className="timer-topline">
              <div>
                <p className="eyebrow eyebrow-light">Current block</p>
                <h2 id="timer-heading">{modeLabel}</h2>
              </div>
              <span className={`timer-status timer-status-${snapshot.state.status}`}>
                {statusLabel}
              </span>
            </div>

            <p
              className="timer-clock"
              role="timer"
              aria-live="off"
              aria-label={`${modeLabel} time remaining`}
            >
              {formatTimerClock(displayRemainingMs)}
            </p>

            <p className="timer-caption">
              {isRunning
                ? displayRemainingMs === 0
                  ? 'Completing the block and recording focus time…'
                  : 'Running from persisted timestamps, even while this window is minimized.'
                : isPaused
                  ? 'Paused. Only active focus time is counted when you resume.'
                  : activeMode === 'focus'
                    ? 'A completed focus block is saved to Study sessions.'
                    : 'Breaks are never recorded as study time.'}
            </p>

            {snapshot.state.recovery && (
              <div className="timer-recovery" role="status">
                <p className="timer-recovery-title">
                  {snapshot.state.recovery.reason === 'restart'
                    ? 'Timer recovered after restart'
                    : 'Timer paused after inactivity'}
                </p>
                <p>
                  {formatStudyDuration(snapshot.state.recovery.excludedMs / 1000)} was excluded
                  because no timer heartbeat was received. Saved focus time:{' '}
                  {formatStudyDuration(snapshot.state.accumulatedMs / 1000)}. Resume when you are
                  ready.
                </p>
              </div>
            )}

            {snapshot.state.status === 'idle' && snapshot.state.lastEvent && (
              <div className="timer-last-event" role="status">
                <p className="timer-last-event-title">
                  {snapshot.state.lastEvent === 'completed'
                    ? snapshot.state.mode === 'focus'
                      ? 'Focus session saved'
                      : 'Break completed'
                    : 'Timer discarded'}
                </p>
                <p>
                  {snapshot.state.lastEvent === 'discarded'
                    ? `${formatStudyDuration(snapshot.state.lastEventDurationMs / 1000)} was not added to study totals.`
                    : snapshot.state.mode === 'focus'
                      ? `${formatStudyDuration(snapshot.state.lastEventDurationMs / 1000)} is now in Study sessions.`
                      : 'Break time is not part of study totals.'}
                </p>
              </div>
            )}

            {snapshot.state.topic && (
              <p className="timer-topic-line">
                <span>Topic</span>
                <strong>
                  {snapshot.state.topic.code} · {snapshot.state.topic.title}
                </strong>
              </p>
            )}

            {actionError && (
              <p className="save-feedback save-feedback-error" role="alert">
                {actionError}
              </p>
            )}

            {!isActive ? (
              <fieldset className="timer-mode-picker">
                <legend>Choose the next block</legend>
                <div className="timer-mode-options">
                  {MODE_OPTIONS.map((option) => (
                    <label
                      className={`timer-mode-option${
                        selectedMode === option.mode ? ' timer-mode-option-selected' : ''
                      }`}
                      key={option.mode}
                    >
                      <input
                        type="radio"
                        name="timer-mode"
                        value={option.mode}
                        checked={selectedMode === option.mode}
                        onChange={() => handleModeChange(option.mode)}
                      />
                      <span>{option.label}</span>
                      <small>{option.description}</small>
                    </label>
                  ))}
                </div>

                {canUseTopic && (
                  <div className="form-field">
                    <label htmlFor="timer-topic">Syllabus topic (optional)</label>
                    <select
                      id="timer-topic"
                      value={selectedTopicId}
                      onChange={(event) => setSelectedTopicId(event.target.value)}
                      disabled={topics.length === 0}
                    >
                      <option value="">
                        {topics.length === 0 ? 'No active topics available' : 'No specific topic'}
                      </option>
                      {topics.map((topic) => (
                        <option value={topic.id} key={topic.id}>
                          {topic.code} · {topic.title}
                        </option>
                      ))}
                    </select>
                    {topicsError && (
                      <p className="field-error" role="alert">
                        Topics could not be loaded: {topicsError}{' '}
                        <button
                          type="button"
                          className="link-button"
                          onClick={() => {
                            setTopicsError(null);
                            setTopicsReloadKey((key) => key + 1);
                          }}
                        >
                          Retry topics
                        </button>
                      </p>
                    )}
                  </div>
                )}

                <button
                  type="button"
                  className="primary-button timer-primary-action"
                  onClick={() => void handleStart()}
                  disabled={busyAction !== null}
                >
                  {busyAction === 'start' ? 'Starting…' : `Start ${modeLabel.toLowerCase()}`}
                </button>
              </fieldset>
            ) : (
              <div className="timer-controls">
                {snapshot.state.status === 'running' ? (
                  <button
                    type="button"
                    className="secondary-button"
                    onClick={() => void handlePause()}
                    disabled={busyAction !== null}
                  >
                    {busyAction === 'pause' ? 'Pausing…' : 'Pause'}
                  </button>
                ) : (
                  <button
                    type="button"
                    className="primary-button"
                    onClick={() => void handleResume()}
                    disabled={busyAction !== null}
                  >
                    {busyAction === 'resume' ? 'Resuming…' : 'Resume'}
                  </button>
                )}

                <button
                  type="button"
                  className="primary-button"
                  onClick={() => void handleFinish()}
                  disabled={busyAction !== null}
                >
                  {busyAction === 'finish'
                    ? 'Finishing…'
                    : activeMode === 'focus'
                      ? 'Finish & save'
                      : 'End break'}
                </button>

                <button
                  type="button"
                  className="danger-button"
                  onClick={() => setConfirmDiscard(true)}
                  disabled={busyAction !== null}
                >
                  Discard
                </button>
              </div>
            )}

            {confirmDiscard && (
              <div className="timer-confirm" role="alertdialog" aria-labelledby="discard-title">
                <p className="timer-confirm-title" id="discard-title">
                  Discard this timer?
                </p>
                <p>No study session will be created, and the active focus time will be lost.</p>
                <div className="form-footer">
                  <button
                    type="button"
                    className="danger-button"
                    onClick={() => void handleDiscard()}
                    disabled={busyAction !== null}
                  >
                    {busyAction === 'discard' ? 'Discarding…' : 'Confirm discard'}
                  </button>
                  <button
                    type="button"
                    className="secondary-button"
                    onClick={() => setConfirmDiscard(false)}
                    disabled={busyAction !== null}
                  >
                    Keep timer
                  </button>
                </div>
              </div>
            )}

            <div className="timer-policy">
              <p>
                <strong>Recovery policy:</strong> a background heartbeat saves progress every five
                seconds. If the app closes, the timer resumes paused at the last heartbeat; time
                while the app was closed is excluded. If heartbeats stop for more than twenty
                seconds, such as during computer sleep, the timer pauses and excludes that gap.
              </p>
              <p>
                A focus session is filed under the local calendar date when the timer started. A
                block crossing midnight stays on its start date.
              </p>
            </div>
          </section>

          <div className="timer-side">
            <section className="timer-settings-card" aria-labelledby="timer-settings-heading">
              <div className="card-heading">
                <div>
                  <p className="eyebrow">Durations</p>
                  <h2 id="timer-settings-heading">Timer settings</h2>
                </div>
                <span className="local-badge">Local</span>
              </div>

              <form className="timer-settings-form" onSubmit={handleSaveSettings} noValidate>
                <div className="timer-duration-grid">
                  <div
                    className={`form-field${settingsErrors.focusMinutes ? ' form-field-invalid' : ''}`}
                  >
                    <label htmlFor="focus-minutes">Focus (minutes)</label>
                    <input
                      id="focus-minutes"
                      type="number"
                      min={1}
                      max={240}
                      step={1}
                      value={settingsForm.focusMinutes}
                      onChange={(event) => handleSettingsField('focusMinutes', event.target.value)}
                      aria-invalid={settingsErrors.focusMinutes ? true : undefined}
                      aria-describedby={
                        settingsErrors.focusMinutes ? 'focus-minutes-error' : 'focus-minutes-hint'
                      }
                      disabled={isSavingSettings}
                    />
                    <p className="field-hint" id="focus-minutes-hint">
                      Default 25. Range 1–240.
                    </p>
                    <FieldError id="focus-minutes-error" message={settingsErrors.focusMinutes} />
                  </div>

                  <div
                    className={`form-field${
                      settingsErrors.shortBreakMinutes ? ' form-field-invalid' : ''
                    }`}
                  >
                    <label htmlFor="short-break-minutes">Short break (minutes)</label>
                    <input
                      id="short-break-minutes"
                      type="number"
                      min={1}
                      max={60}
                      step={1}
                      value={settingsForm.shortBreakMinutes}
                      onChange={(event) =>
                        handleSettingsField('shortBreakMinutes', event.target.value)
                      }
                      aria-invalid={settingsErrors.shortBreakMinutes ? true : undefined}
                      aria-describedby={
                        settingsErrors.shortBreakMinutes
                          ? 'short-break-minutes-error'
                          : 'short-break-minutes-hint'
                      }
                      disabled={isSavingSettings}
                    />
                    <p className="field-hint" id="short-break-minutes-hint">
                      Default 5. Range 1–60.
                    </p>
                    <FieldError
                      id="short-break-minutes-error"
                      message={settingsErrors.shortBreakMinutes}
                    />
                  </div>

                  <div
                    className={`form-field${
                      settingsErrors.longBreakMinutes ? ' form-field-invalid' : ''
                    }`}
                  >
                    <label htmlFor="long-break-minutes">Long break (minutes)</label>
                    <input
                      id="long-break-minutes"
                      type="number"
                      min={1}
                      max={120}
                      step={1}
                      value={settingsForm.longBreakMinutes}
                      onChange={(event) =>
                        handleSettingsField('longBreakMinutes', event.target.value)
                      }
                      aria-invalid={settingsErrors.longBreakMinutes ? true : undefined}
                      aria-describedby={
                        settingsErrors.longBreakMinutes
                          ? 'long-break-minutes-error'
                          : 'long-break-minutes-hint'
                      }
                      disabled={isSavingSettings}
                    />
                    <p className="field-hint" id="long-break-minutes-hint">
                      Default 15. Range 1–120.
                    </p>
                    <FieldError
                      id="long-break-minutes-error"
                      message={settingsErrors.longBreakMinutes}
                    />
                  </div>
                </div>

                <label className="timer-checkbox">
                  <input
                    type="checkbox"
                    checked={settingsForm.notificationsEnabled}
                    onChange={(event) => {
                      setSettingsForm((current) =>
                        current
                          ? { ...current, notificationsEnabled: event.target.checked }
                          : current,
                      );
                      setSettingsFeedback(null);
                    }}
                    disabled={isSavingSettings}
                  />
                  <span>
                    Desktop notifications
                    <small>
                      Off by default. Windows permission is requested when you enable this.
                    </small>
                  </span>
                </label>

                <p className="field-hint">
                  Setting changes apply to the next timer you start. A running timer keeps its
                  original duration.
                </p>

                <div className="form-footer">
                  <button
                    type="submit"
                    className="primary-button"
                    disabled={!settingsDirty || isSavingSettings}
                  >
                    {isSavingSettings ? 'Saving…' : 'Save timer settings'}
                  </button>
                  <span className="dirty-hint" aria-live="polite">
                    {isSavingSettings
                      ? 'Writing to SQLite…'
                      : settingsDirty
                        ? 'Unsaved changes'
                        : 'All changes saved'}
                  </span>
                </div>

                {settingsFeedback && (
                  <p
                    className={`save-feedback save-feedback-${settingsFeedback.tone}`}
                    role={settingsFeedback.tone === 'error' ? 'alert' : 'status'}
                  >
                    {settingsFeedback.message}
                  </p>
                )}
              </form>
            </section>

            <section className="timer-help-card" aria-labelledby="timer-help-heading">
              <h2 id="timer-help-heading">How time is counted</h2>
              <ul>
                <li>Focus, pause and resume all use persisted UTC timestamps.</li>
                <li>Paused time and break time never enter study totals.</li>
                <li>Only one timer can be active at a time.</li>
                <li>Discarding a timer never creates a completed session.</li>
                <li>
                  Notifications are sent by the Rust heartbeat, so they can appear while the window
                  is minimized. Windows notification settings can still suppress them.
                </li>
              </ul>
            </section>
          </div>
        </div>
      )}
    </div>
  );
}
