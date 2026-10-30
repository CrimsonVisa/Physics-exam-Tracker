import { type FormEvent, useEffect, useState } from 'react';
import { DateField } from './components/DateField';
import { StudyTodayCard } from './components/StudyTodayCard';
import { toCommandError } from './lib/commandError';
import { countdownFor, formatDateLong, parseDateOnly, type Countdown } from './lib/dateOnly';
import { getSettings, saveSettings, type Settings } from './lib/settingsApi';
import { useTodayDateOnly } from './lib/useTodayDateOnly';
import SessionsScreen from './screens/SessionsScreen';
import SyllabusScreen from './screens/SyllabusScreen';
import TimerScreen from './screens/TimerScreen';

type AppView = 'overview' | 'timer' | 'sessions' | 'syllabus';

type FormState = {
  examDate: string;
  preparationStartDate: string;
  dailyMinutes: string;
};

type FormField = keyof FormState;

type FieldErrors = Partial<Record<FormField, string>>;

type SaveFeedback = {
  tone: 'success' | 'error';
  message: string;
} | null;

type AppProps = {
  today?: string;
};

const FORM_FIELDS: FormField[] = ['examDate', 'preparationStartDate', 'dailyMinutes'];

const FIELD_IDS: Record<FormField, string> = {
  examDate: 'exam-date',
  preparationStartDate: 'preparation-start-date',
  dailyMinutes: 'daily-target-minutes',
};

const BACKEND_FIELD_MAP: Record<string, FormField> = {
  examDate: 'examDate',
  preparationStartDate: 'preparationStartDate',
  dailyStudyTargetMinutes: 'dailyMinutes',
};

function firstErrorField(errors: FieldErrors): FormField | null {
  return FORM_FIELDS.find((candidate) => errors[candidate]) ?? null;
}

function formFromSettings(settings: Settings): FormState {
  return {
    examDate: settings.examDate,
    preparationStartDate: settings.preparationStartDate,
    dailyMinutes: String(settings.dailyStudyTargetMinutes),
  };
}

function parseDailyMinutes(value: string): number | null {
  const trimmed = value.trim();
  if (!/^\d+$/.test(trimmed)) {
    return null;
  }

  const minutes = Number(trimmed);
  return minutes >= 1 && minutes <= 1440 ? minutes : null;
}

function formatHoursPerDay(minutes: number): string {
  const hours = minutes / 60;
  const display = Number.isInteger(hours)
    ? String(hours)
    : hours.toFixed(2).replace(/0+$/, '').replace(/\.$/, '');
  const unit = Number(display) === 1 ? 'hour' : 'hours';

  return `${display} ${unit} per day`;
}

function validateForm(form: FormState): {
  errors: FieldErrors;
  settings: Settings | null;
} {
  const errors: FieldErrors = {};
  const examDate = parseDateOnly(form.examDate);
  const preparationStartDate = parseDateOnly(form.preparationStartDate);
  const dailyMinutes = parseDailyMinutes(form.dailyMinutes);

  if (!examDate) {
    errors.examDate = 'Choose a real calendar date.';
  }

  if (!preparationStartDate) {
    errors.preparationStartDate = 'Choose a real calendar date.';
  } else if (examDate && form.preparationStartDate > form.examDate) {
    errors.preparationStartDate = 'Preparation start date cannot be after the exam date.';
  }

  if (dailyMinutes === null) {
    errors.dailyMinutes = 'Enter a whole number of minutes from 1 to 1440.';
  }

  if (Object.keys(errors).length > 0 || dailyMinutes === null) {
    return { errors, settings: null };
  }

  return {
    errors,
    settings: {
      examDate: form.examDate,
      preparationStartDate: form.preparationStartDate,
      dailyStudyTargetMinutes: dailyMinutes,
    },
  };
}

function isDirty(form: FormState, settings: Settings): boolean {
  return (
    form.examDate !== settings.examDate ||
    form.preparationStartDate !== settings.preparationStartDate ||
    parseDailyMinutes(form.dailyMinutes) !== settings.dailyStudyTargetMinutes
  );
}

function Sidebar({
  activeView,
  onNavigate,
}: {
  activeView: AppView;
  onNavigate: (view: AppView) => void;
}) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <span className="brand-mark" aria-hidden="true">
          CN
        </span>
        <div>
          <p className="brand-title">CSIR-NET</p>
          <p className="brand-subtitle">Physics Tracker</p>
        </div>
      </div>

      <nav className="nav" aria-label="Primary navigation">
        <button
          type="button"
          className={`nav-item nav-button${activeView === 'overview' ? ' nav-item-active' : ''}`}
          aria-current={activeView === 'overview' ? 'page' : undefined}
          onClick={() => onNavigate('overview')}
        >
          Overview
        </button>
        <button
          type="button"
          className={`nav-item nav-button${activeView === 'timer' ? ' nav-item-active' : ''}`}
          aria-current={activeView === 'timer' ? 'page' : undefined}
          onClick={() => onNavigate('timer')}
        >
          Timer
        </button>
        <button
          type="button"
          className={`nav-item nav-button${activeView === 'sessions' ? ' nav-item-active' : ''}`}
          aria-current={activeView === 'sessions' ? 'page' : undefined}
          onClick={() => onNavigate('sessions')}
        >
          Sessions
        </button>
        <button
          type="button"
          className={`nav-item nav-button${activeView === 'syllabus' ? ' nav-item-active' : ''}`}
          aria-current={activeView === 'syllabus' ? 'page' : undefined}
          onClick={() => onNavigate('syllabus')}
        >
          Syllabus
        </button>
      </nav>

      <p className="sidebar-note">
        Local-first. Settings, syllabus progress, and study sessions are stored on this PC and never
        sent to a server.
      </p>
    </aside>
  );
}

function CountdownBody({ countdown }: { countdown: Countdown | null }) {
  if (!countdown) {
    return (
      <div className="countdown-empty">
        <p className="countdown-empty-title">Choose a valid exam date</p>
        <p className="countdown-caption">The countdown updates as you edit the date.</p>
      </div>
    );
  }

  if (countdown.state === 'past') {
    return (
      <>
        <p className="countdown-past-title">Exam date has passed</p>
        <p className="countdown-past-days">
          {countdown.daysSinceExam} {countdown.daysSinceExam === 1 ? 'day' : 'days'} ago
        </p>
        <p className="countdown-caption">
          Update the exam date when you know the date for the next attempt.
        </p>
      </>
    );
  }

  const days = countdown.daysRemaining;
  return (
    <>
      <p className="countdown-value">{days}</p>
      <p className="countdown-label">{days === 1 ? 'day remaining' : 'days remaining'}</p>
      <p className="countdown-caption">
        {countdown.state === 'exam-day'
          ? 'Today is exam day. Focus on the paper.'
          : 'Calendar days on this device. Take it one focused block at a time.'}
      </p>
    </>
  );
}

export default function App({ today }: AppProps) {
  const currentToday = useTodayDateOnly(today);
  const [view, setView] = useState<AppView>('overview');
  const [settings, setSettings] = useState<Settings | null>(null);
  const [form, setForm] = useState<FormState | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [isSaving, setIsSaving] = useState(false);
  const [saveFeedback, setSaveFeedback] = useState<SaveFeedback>(null);
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [focusField, setFocusField] = useState<FormField | null>(null);

  useEffect(() => {
    let cancelled = false;

    getSettings()
      .then((loadedSettings) => {
        if (cancelled) {
          return;
        }

        setSettings(loadedSettings);
        setForm(formFromSettings(loadedSettings));
        setFieldErrors({});
        setSaveFeedback(null);
      })
      .catch((error: unknown) => {
        if (cancelled) {
          return;
        }

        setLoadError(toCommandError(error).message);
      })
      .finally(() => {
        if (!cancelled) {
          setIsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [reloadKey]);

  useEffect(() => {
    if (!focusField || isSaving) {
      return;
    }

    document.getElementById(FIELD_IDS[focusField])?.focus();
  }, [focusField, isSaving]);

  function handleRetry() {
    setIsLoading(true);
    setLoadError(null);
    setReloadKey((key) => key + 1);
  }

  function handleFieldChange(field: FormField, value: string) {
    setForm((current) => (current ? { ...current, [field]: value } : current));
    setFieldErrors((current) => {
      if (!current[field]) {
        return current;
      }

      const next = { ...current };
      delete next[field];
      return next;
    });
    setFocusField(null);
    setSaveFeedback(null);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!form || !settings || isSaving) {
      return;
    }

    setSaveFeedback(null);
    const validation = validateForm(form);
    setFieldErrors(validation.errors);

    if (!validation.settings) {
      setFocusField(firstErrorField(validation.errors));
      return;
    }

    setIsSaving(true);

    try {
      const saved = await saveSettings(validation.settings);
      setSettings(saved);
      setForm(formFromSettings(saved));
      setFieldErrors({});
      setFocusField(null);
      setSaveFeedback({
        tone: 'success',
        message: 'Settings saved to the local database.',
      });
    } catch (error: unknown) {
      const commandError = toCommandError(error);
      const field = commandError.field ? BACKEND_FIELD_MAP[commandError.field] : undefined;

      if (field) {
        setFieldErrors({ [field]: commandError.message });
        setFocusField(field);
      }

      setSaveFeedback({ tone: 'error', message: commandError.message });
    } finally {
      setIsSaving(false);
    }
  }

  const countdown = form ? countdownFor(form.examDate, currentToday) : null;
  const examDateLabel = form ? formatDateLong(form.examDate) : null;
  const preparationStartLabel = form ? formatDateLong(form.preparationStartDate) : null;
  const formMinutes = form ? parseDailyMinutes(form.dailyMinutes) : null;
  const dirty = Boolean(form && settings && isDirty(form, settings));

  return (
    <div className="app-frame">
      <Sidebar activeView={view} onNavigate={setView} />

      <main className="main-area">
        {view === 'syllabus' ? (
          <SyllabusScreen />
        ) : view === 'timer' ? (
          <TimerScreen />
        ) : view === 'sessions' ? (
          <SessionsScreen today={currentToday} />
        ) : (
          <div className="page-container">
            <header className="page-header">
              <p className="eyebrow">Dashboard</p>
              <h1>Exam countdown</h1>
              <p className="page-lede">
                Set the date you are working toward, see the days that remain, and keep your daily
                target in view. Settings stay in a local SQLite database.
              </p>
            </header>

            {isLoading && (
              <section className="state-card" role="status" aria-live="polite">
                <p className="state-title">Loading local settings…</p>
                <p className="state-copy">Opening the SQLite database on this device.</p>
              </section>
            )}

            {!isLoading && loadError && (
              <section className="state-card state-card-error" role="alert">
                <p className="state-title">Settings could not be loaded</p>
                <p className="state-copy">{loadError}</p>
                <button type="button" className="primary-button" onClick={handleRetry}>
                  Retry
                </button>
              </section>
            )}

            {!isLoading && !loadError && form && settings && (
              <div className="dashboard-grid">
                <section className="countdown-card" aria-labelledby="countdown-heading">
                  <h2 className="sr-only" id="countdown-heading">
                    Countdown details
                  </h2>

                  <div className="countdown-topline">
                    <p className="eyebrow eyebrow-light">Target exam</p>
                    <p className="exam-date-line">
                      {examDateLabel ? (
                        <>
                          <time dateTime={form.examDate}>{examDateLabel}</time>
                        </>
                      ) : (
                        'Choose a valid date'
                      )}
                    </p>
                  </div>

                  <div className="countdown-body" aria-live="polite">
                    <CountdownBody countdown={countdown} />
                  </div>

                  <dl className="countdown-meta">
                    <div>
                      <dt>Preparation start</dt>
                      <dd>{preparationStartLabel ?? 'Not set'}</dd>
                    </div>
                    <div>
                      <dt>Daily target</dt>
                      <dd>{formMinutes === null ? 'Not set' : formatHoursPerDay(formMinutes)}</dd>
                    </div>
                  </dl>

                  <p className="provisional-note">
                    <strong>Verify the exam date.</strong> 19 December is a provisional default, not
                    an official schedule. Confirm the actual subject date on your admit card before
                    relying on this countdown.
                  </p>
                </section>

                <section className="settings-card" aria-labelledby="settings-heading">
                  <div className="card-heading">
                    <div>
                      <p className="eyebrow">Planning</p>
                      <h2 id="settings-heading">Settings</h2>
                    </div>
                    <span className="local-badge">SQLite · local</span>
                  </div>

                  <p className="settings-lede">
                    Changes are saved only when you choose <strong>Save settings</strong>. Past
                    dates are allowed so completed study cycles remain visible.
                  </p>

                  <form className="settings-form" onSubmit={handleSubmit} noValidate>
                    <DateField
                      id={FIELD_IDS.examDate}
                      label="Exam date"
                      value={form.examDate}
                      onChange={(value) => handleFieldChange('examDate', value)}
                      hint="The provisional default is 19 December 2026."
                      error={fieldErrors.examDate}
                      disabled={isSaving}
                    />

                    <DateField
                      id={FIELD_IDS.preparationStartDate}
                      label="Preparation start date"
                      value={form.preparationStartDate}
                      onChange={(value) => handleFieldChange('preparationStartDate', value)}
                      hint="First run uses today's date; adjust it if your preparation began earlier."
                      error={fieldErrors.preparationStartDate}
                      max={parseDateOnly(form.examDate) ? form.examDate : undefined}
                      disabled={isSaving}
                    />

                    <div
                      className={`form-field${fieldErrors.dailyMinutes ? ' form-field-invalid' : ''}`}
                    >
                      <label htmlFor={FIELD_IDS.dailyMinutes}>Daily study target</label>
                      <div className="input-with-suffix">
                        <input
                          id={FIELD_IDS.dailyMinutes}
                          type="number"
                          inputMode="numeric"
                          min={1}
                          max={1440}
                          step={1}
                          value={form.dailyMinutes}
                          onChange={(event) =>
                            handleFieldChange('dailyMinutes', event.target.value)
                          }
                          aria-invalid={fieldErrors.dailyMinutes ? true : undefined}
                          aria-describedby={
                            fieldErrors.dailyMinutes
                              ? 'daily-target-hint daily-target-error'
                              : 'daily-target-hint'
                          }
                          disabled={isSaving}
                          required
                        />
                        <span aria-hidden="true">minutes</span>
                      </div>
                      <p className="field-hint" id="daily-target-hint">
                        {formMinutes === null
                          ? 'Enter a whole number of minutes from 1 to 1440.'
                          : `${formMinutes} minutes is ${formatHoursPerDay(formMinutes)}.`}
                      </p>
                      {fieldErrors.dailyMinutes && (
                        <p className="field-error" id="daily-target-error" role="alert">
                          {fieldErrors.dailyMinutes}
                        </p>
                      )}
                    </div>

                    <div className="form-footer">
                      <button
                        type="submit"
                        className="primary-button"
                        disabled={!dirty || isSaving}
                      >
                        {isSaving ? 'Saving…' : 'Save settings'}
                      </button>
                      <span className="dirty-hint" aria-live="polite">
                        {isSaving
                          ? 'Writing to SQLite…'
                          : dirty
                            ? 'Unsaved changes'
                            : 'All changes saved'}
                      </span>
                    </div>

                    {saveFeedback && (
                      <p
                        className={`save-feedback save-feedback-${saveFeedback.tone}`}
                        role={saveFeedback.tone === 'error' ? 'alert' : 'status'}
                      >
                        {saveFeedback.message}
                      </p>
                    )}
                  </form>
                </section>

                <StudyTodayCard
                  today={currentToday}
                  targetMinutes={formMinutes ?? settings.dailyStudyTargetMinutes}
                  onOpenSessions={() => setView('sessions')}
                />
              </div>
            )}
          </div>
        )}
      </main>
    </div>
  );
}
