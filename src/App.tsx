import { type FormEvent, type RefObject, useEffect, useRef, useState } from 'react';
import { AppSidebar } from './components/AppSidebar';
import { DateField } from './components/DateField';
import { Icon } from './components/NavIcon';
import { PreparationDashboard } from './components/PreparationDashboard';
import { ThemeSwitch } from './components/ThemeSwitch';
import { toCommandError } from './lib/commandError';
import { formatDateLong, parseDateOnly } from './lib/dateOnly';
import { runAutomaticBackupIfDue } from './lib/backupApi';
import { KNOWN_VIEWS, type AppView } from './lib/navigation';
import {
  applyExamDateChange,
  previewExamDateChange,
  type ExamReschedulePreview,
} from './lib/plannerApi';
import { checkRevisionReminder } from './lib/revisionApi';
import { getSettings, saveSettings, type Settings } from './lib/settingsApi';
import {
  applyTheme,
  getStoredSidebarMode,
  getStoredTheme,
  storeSidebarMode,
  storeTheme,
  type SidebarMode,
  type Theme,
} from './lib/uiPreferences';
import { useTodayDateOnly } from './lib/useTodayDateOnly';
import AnalyticsScreen, { type AnalyticsTab } from './screens/AnalyticsScreen';
import BackupScreen from './screens/BackupScreen';
import CalendarScreen from './screens/CalendarScreen';
import ChecklistScreen from './screens/ChecklistScreen';
import MockTestsScreen from './screens/MockTestsScreen';
import PyqScreen from './screens/PyqScreen';
import RevisionScreen from './screens/RevisionScreen';
import SessionsScreen from './screens/SessionsScreen';
import StudyResourcesScreen from './screens/StudyResourcesScreen';
import SyllabusScreen from './screens/SyllabusScreen';
import TimerScreen from './screens/TimerScreen';

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

export default function App({ today }: AppProps) {
  const currentToday = useTodayDateOnly(today);
  const [view, setView] = useState<AppView>('overview');
  const [theme, setTheme] = useState<Theme>(() => getStoredTheme());
  const [sidebarMode, setSidebarMode] = useState<SidebarMode>(() => getStoredSidebarMode());
  const sidebarToggleRef = useRef<HTMLButtonElement>(null);
  const showNavigationRef = useRef<HTMLButtonElement>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [form, setForm] = useState<FormState | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [isSaving, setIsSaving] = useState(false);
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [saveFeedback, setSaveFeedback] = useState<SaveFeedback>(null);
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [focusField, setFocusField] = useState<FormField | null>(null);
  const [examPreview, setExamPreview] = useState<ExamReschedulePreview | null>(null);
  const [analyticsTab, setAnalyticsTab] = useState<AnalyticsTab>('study');

  useEffect(() => {
    applyTheme(theme);
    storeTheme(theme);
  }, [theme]);

  useEffect(() => {
    storeSidebarMode(sidebarMode);
  }, [sidebarMode]);

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

  useEffect(() => {
    checkRevisionReminder().catch(() => {
      // Reminders are optional; a notification failure must not block the app.
    });
  }, [currentToday]);

  useEffect(() => {
    const run = () => {
      runAutomaticBackupIfDue().catch(() => {
        // Backup failures are persisted and shown in Backup and data management.
      });
    };
    run();
    const interval = window.setInterval(run, 60 * 60 * 1000);
    return () => window.clearInterval(interval);
  }, []);

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
    setExamPreview(null);
  }

  function handleSettingsError(error: unknown) {
    const commandError = toCommandError(error);
    const field = commandError.field ? BACKEND_FIELD_MAP[commandError.field] : undefined;

    if (field) {
      setFieldErrors({ [field]: commandError.message });
      setFocusField(field);
    }

    setSaveFeedback({ tone: 'error', message: commandError.message });
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!form || !settings || isSaving) {
      return;
    }

    setSaveFeedback(null);
    setExamPreview(null);
    const validation = validateForm(form);
    setFieldErrors(validation.errors);

    if (!validation.settings) {
      setFocusField(firstErrorField(validation.errors));
      return;
    }

    setIsSaving(true);

    try {
      if (validation.settings.examDate !== settings.examDate) {
        setIsPreviewing(true);
        const preview = await previewExamDateChange(validation.settings);
        setExamPreview(preview);
        setFocusField(null);
        return;
      }

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
      handleSettingsError(error);
    } finally {
      setIsSaving(false);
      setIsPreviewing(false);
    }
  }

  async function handleApplyExamPreview() {
    if (!examPreview || isSaving) {
      return;
    }

    setIsSaving(true);

    try {
      const result = await applyExamDateChange({
        settings: {
          examDate: examPreview.proposedExamDate,
          preparationStartDate: examPreview.proposedPreparationStartDate,
          dailyStudyTargetMinutes: examPreview.proposedDailyStudyTargetMinutes,
        },
        expectedExamDate: examPreview.currentExamDate,
        acceptedChanges: examPreview.changes.map((change) => ({
          taskId: change.taskId,
          expectedScheduledDate: change.currentScheduledDate,
        })),
      });

      setSettings(result.settings);
      setForm(formFromSettings(result.settings));
      setFieldErrors({});
      setFocusField(null);
      setExamPreview(null);
      setSaveFeedback({
        tone: 'success',
        message:
          result.appliedTaskCount === 0
            ? 'Exam date updated. No exam-anchored task dates needed to move.'
            : `Exam date updated. ${result.appliedTaskCount} exam-anchored ${
                result.appliedTaskCount === 1 ? 'task was' : 'tasks were'
              } moved after confirmation.`,
      });
    } catch (error: unknown) {
      setExamPreview(null);
      handleSettingsError(error);
    } finally {
      setIsSaving(false);
    }
  }

  function handleCancelExamPreview() {
    setExamPreview(null);
    setSaveFeedback({
      tone: 'success',
      message: 'Exam-date change cancelled. Nothing was saved.',
    });
  }

  const formMinutes = form ? parseDailyMinutes(form.dailyMinutes) : null;
  const dirty = Boolean(form && settings && isDirty(form, settings));
  const examDeadlineConflicts =
    examPreview?.changes.filter((change) => change.deadlineConflict) ?? [];
  const hasExamDeadlineConflict = examDeadlineConflicts.length > 0;
  const activeExamDate = settings?.examDate ?? form?.examDate ?? currentToday;
  const dashboardDataKey = settings
    ? `${settings.examDate}|${settings.preparationStartDate}|${settings.dailyStudyTargetMinutes}`
    : '';

  function focusSoon(ref: RefObject<HTMLButtonElement | null>) {
    window.setTimeout(() => ref.current?.focus(), 0);
  }

  function handleToggleSidebar() {
    setSidebarMode((current) => (current === 'collapsed' ? 'expanded' : 'collapsed'));
    focusSoon(sidebarToggleRef);
  }

  function handleHideSidebar() {
    setSidebarMode('hidden');
    focusSoon(showNavigationRef);
  }

  function handleShowSidebar() {
    setSidebarMode('expanded');
    focusSoon(sidebarToggleRef);
  }

  function handleDashboardNavigate(view: string) {
    if (view.startsWith('analytics:')) {
      const tab = view.slice('analytics:'.length) as AnalyticsTab;
      if (['study', 'performance', 'topics', 'timeline'].includes(tab)) {
        setAnalyticsTab(tab);
      }
      setView('analytics');
      return;
    }

    const candidate = view as AppView;
    if (KNOWN_VIEWS.includes(candidate)) {
      setView(candidate);
    }
  }

  return (
    <div className="app-frame" data-sidebar-mode={sidebarMode}>
      <a className="skip-link" href="#main-content">
        Skip to main content
      </a>

      {sidebarMode !== 'hidden' && (
        <AppSidebar
          activeView={view}
          collapseButtonRef={sidebarToggleRef}
          key={sidebarMode}
          mode={sidebarMode}
          onHide={handleHideSidebar}
          onNavigate={setView}
          onThemeChange={setTheme}
          onToggleCollapse={handleToggleSidebar}
          theme={theme}
        />
      )}

      <div className="main-column">
        {sidebarMode === 'hidden' && (
          <div className="hidden-sidebar-bar">
            <button
              aria-label="Show navigation"
              className="secondary-button show-navigation-button"
              onClick={handleShowSidebar}
              ref={showNavigationRef}
              title="Show navigation"
              type="button"
            >
              <Icon name="menu" size={18} />
              <span>Show navigation</span>
            </button>
            <ThemeSwitch compact onChange={setTheme} theme={theme} />
          </div>
        )}

        <main className="main-area" id="main-content" tabIndex={-1}>
          {view === 'syllabus' ? (
            <SyllabusScreen />
          ) : view === 'resources' ? (
            <StudyResourcesScreen />
          ) : view === 'timer' ? (
            <TimerScreen />
          ) : view === 'sessions' ? (
            <SessionsScreen today={currentToday} />
          ) : view === 'checklist' ? (
            <ChecklistScreen today={currentToday} examDate={activeExamDate} />
          ) : view === 'calendar' ? (
            <CalendarScreen today={currentToday} examDate={activeExamDate} />
          ) : view === 'revision' ? (
            <RevisionScreen today={currentToday} />
          ) : view === 'pyq' ? (
            <PyqScreen today={currentToday} />
          ) : view === 'mock-tests' ? (
            <MockTestsScreen today={currentToday} />
          ) : view === 'analytics' ? (
            <AnalyticsScreen
              key={`${currentToday}:${activeExamDate}`}
              today={currentToday}
              examDate={activeExamDate}
              initialTab={analyticsTab}
              onTabChange={setAnalyticsTab}
            />
          ) : view === 'backup' ? (
            <BackupScreen />
          ) : (
            <div className="page-container">
              <header className="page-header">
                <p className="eyebrow">Dashboard</p>
                <h1>Preparation dashboard</h1>
                <p className="page-lede">
                  See saved study time, syllabus progress, revision queues, deadlines, recent
                  performance, and evidence-linked insights in one place. The countdown updates as
                  you edit the exam date, and settings remain in the local SQLite database.
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
                <>
                  <PreparationDashboard
                    today={currentToday}
                    previewExamDate={form.examDate}
                    previewPreparationStartDate={form.preparationStartDate}
                    previewTargetMinutes={formMinutes ?? settings.dailyStudyTargetMinutes}
                    refreshKey={dashboardDataKey}
                    onNavigate={handleDashboardNavigate}
                  />

                  <section
                    className="settings-card dashboard-settings-card"
                    aria-labelledby="settings-heading"
                  >
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
                          disabled={!dirty || isSaving || examPreview !== null}
                        >
                          {isSaving && !examPreview
                            ? 'Saving…'
                            : examPreview
                              ? 'Preview ready'
                              : 'Save settings'}
                        </button>
                        <span className="dirty-hint" aria-live="polite">
                          {isSaving
                            ? isPreviewing
                              ? 'Reading the rescheduling preview…'
                              : 'Writing to SQLite…'
                            : examPreview
                              ? 'Confirm or cancel the exam-date preview below'
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

                    {examPreview && (
                      <section className="exam-preview-card" aria-labelledby="exam-preview-heading">
                        <div className="card-heading">
                          <div>
                            <p className="eyebrow">Confirmation required</p>
                            <h2 id="exam-preview-heading">Review the exam-date change</h2>
                          </div>
                          <span className="exam-preview-badge">Nothing saved yet</span>
                        </div>

                        <p className="exam-preview-lede">
                          The exam date moves from{' '}
                          <strong>
                            {formatDateLong(examPreview.currentExamDate) ??
                              examPreview.currentExamDate}
                          </strong>{' '}
                          to{' '}
                          <strong>
                            {formatDateLong(examPreview.proposedExamDate) ??
                              examPreview.proposedExamDate}
                          </strong>
                          . The preparation start becomes{' '}
                          {formatDateLong(examPreview.proposedPreparationStartDate) ??
                            examPreview.proposedPreparationStartDate}
                          . Apply only when this is the date you intend to use.
                        </p>

                        <div className="exam-preview-summary">
                          <div>
                            <span>Exam-anchored tasks to move</span>
                            <strong>{examPreview.changes.length}</strong>
                          </div>
                          <div>
                            <span>Completed tasks preserved</span>
                            <strong>{examPreview.completedTasksUntouched}</strong>
                          </div>
                          <div>
                            <span>Manual tasks preserved</span>
                            <strong>{examPreview.manualTasksUntouched}</strong>
                          </div>
                          <div>
                            <span>Planned blocks preserved</span>
                            <strong>{examPreview.plannedBlocksUntouched}</strong>
                          </div>
                        </div>

                        {examPreview.changes.length > 0 && (
                          <div className="exam-preview-section">
                            <h3>Future exam-anchored tasks</h3>
                            <ul className="exam-preview-list">
                              {examPreview.changes.map((change) => (
                                <li key={`change-${change.taskId}`}>
                                  <span>{change.title}</span>
                                  <span>
                                    {formatDateLong(change.currentScheduledDate) ??
                                      change.currentScheduledDate}{' '}
                                    →{' '}
                                    {formatDateLong(change.proposedScheduledDate) ??
                                      change.proposedScheduledDate}
                                    {change.outsideWindow && (
                                      <span className="exam-preview-warning">
                                        {' '}
                                        Moves outside the new preparation window
                                      </span>
                                    )}
                                    {change.deadlineConflict && (
                                      <span className="exam-preview-warning">
                                        {' '}
                                        Moves past its deadline
                                        {change.deadlineDate
                                          ? ` (${formatDateLong(change.deadlineDate) ?? change.deadlineDate})`
                                          : ''}
                                      </span>
                                    )}
                                  </span>
                                </li>
                              ))}
                            </ul>
                          </div>
                        )}

                        {hasExamDeadlineConflict && (
                          <p className="save-feedback save-feedback-error" role="alert">
                            {examDeadlineConflicts.length === 1
                              ? 'One exam-anchored task would move past its deadline.'
                              : `${examDeadlineConflicts.length} exam-anchored tasks would move past their deadlines.`}{' '}
                            Cancel this preview and correct the deadline before changing the exam
                            date.
                          </p>
                        )}

                        {examPreview.outsideWindow.length > 0 && (
                          <div className="exam-preview-section">
                            <h3>Preserved outside the new window</h3>
                            <p className="exam-preview-note">
                              These dated items will not move. Remove or reschedule them separately
                              if they no longer belong in the plan.
                            </p>
                            <ul className="exam-preview-list exam-preview-warning-list">
                              {examPreview.outsideWindow.map((item) => (
                                <li key={`outside-${item.kind}-${item.id}`}>
                                  <span>{item.title}</span>
                                  <span>
                                    {formatDateLong(item.date) ?? item.date}
                                    {item.kind === 'task_deadline' ? ' · deadline' : ''}
                                    {item.kind === 'planned_block' ? ' · planned block' : ''}
                                  </span>
                                </li>
                              ))}
                            </ul>
                          </div>
                        )}

                        <p className="exam-preview-note">
                          Completed tasks, historical sessions, recorded study time, and planned
                          blocks are preserved. No manual task moves silently.
                        </p>

                        <div className="form-footer">
                          <button
                            type="button"
                            className="primary-button"
                            onClick={() => void handleApplyExamPreview()}
                            disabled={isSaving || hasExamDeadlineConflict}
                          >
                            {isSaving ? 'Applying…' : 'Apply exam-date change'}
                          </button>
                          <button
                            type="button"
                            className="secondary-button"
                            onClick={handleCancelExamPreview}
                            disabled={isSaving}
                          >
                            Cancel
                          </button>
                        </div>
                      </section>
                    )}
                  </section>
                </>
              )}
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
