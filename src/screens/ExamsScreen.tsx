import { useEffect, useState } from 'react';
import { DateField } from '../components/DateField';
import { toCommandError } from '../lib/commandError';
import { countdownFor, formatDateLong, parseDateOnly } from '../lib/dateOnly';
import {
  confirmLegacyExamDate,
  dismissLegacyExamDateNotice,
  getExamProfiles,
  setExamActive,
  updateExamDate,
  type ExamProfile,
} from '../lib/examsApi';

type Feedback = {
  tone: 'success' | 'error';
  message: string;
} | null;

type ExamsScreenProps = {
  today: string;
  selectedExamId: number | null;
  onSelectExam: (examId: number | null) => void;
};

function dateState(profile: ExamProfile, today: string) {
  if (!profile.examDate) {
    return { label: 'Not set', detail: 'Enter a date when it is officially known.', tone: 'unset' };
  }

  const countdown = countdownFor(profile.examDate, today);
  if (!countdown) {
    return { label: 'Not set', detail: 'Enter a real calendar date.', tone: 'unset' };
  }

  if (countdown.state === 'upcoming') {
    return {
      label: `${countdown.daysRemaining} ${countdown.daysRemaining === 1 ? 'day' : 'days'} remaining`,
      detail: formatDateLong(profile.examDate) ?? profile.examDate,
      tone: 'upcoming',
    };
  }

  if (countdown.state === 'exam-day') {
    return {
      label: 'Exam day',
      detail: formatDateLong(profile.examDate) ?? profile.examDate,
      tone: 'today',
    };
  }

  return {
    label: 'Exam date passed',
    detail: `${countdown.daysSinceExam} ${
      countdown.daysSinceExam === 1 ? 'day' : 'days'
    } ago · ${formatDateLong(profile.examDate) ?? profile.examDate}`,
    tone: 'past',
  };
}

export default function ExamsScreen({ today, selectedExamId, onSelectExam }: ExamsScreenProps) {
  const [profiles, setProfiles] = useState<ExamProfile[]>([]);
  const [dateDrafts, setDateDrafts] = useState<Record<number, string>>({});
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [busyExamId, setBusyExamId] = useState<number | null>(null);
  const [feedback, setFeedback] = useState<Feedback>(null);
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    let cancelled = false;

    getExamProfiles()
      .then((loaded) => {
        if (cancelled) {
          return;
        }
        setProfiles(loaded);
        setDateDrafts(
          Object.fromEntries(loaded.map((profile) => [profile.examId, profile.examDate ?? ''])),
        );
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
  }, [reloadKey]);

  function replaceProfile(updated: ExamProfile) {
    setProfiles((current) =>
      current.map((profile) => (profile.examId === updated.examId ? updated : profile)),
    );
    setDateDrafts((current) => ({
      ...current,
      [updated.examId]: updated.examDate ?? '',
    }));
  }

  async function handleSaveDate(profile: ExamProfile) {
    const draft = (dateDrafts[profile.examId] ?? '').trim();
    if (!parseDateOnly(draft)) {
      setFeedback({
        tone: 'error',
        message: 'Enter a real calendar date in YYYY-MM-DD format, or clear the date.',
      });
      return;
    }

    setBusyExamId(profile.examId);
    setFeedback(null);
    try {
      const updated = await updateExamDate(profile.examId, draft);
      replaceProfile(updated);
      setFeedback({ tone: 'success', message: `${profile.shortName} exam date saved.` });
    } catch (error: unknown) {
      setFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setBusyExamId(null);
    }
  }

  async function handleClearDate(profile: ExamProfile) {
    setBusyExamId(profile.examId);
    setFeedback(null);
    try {
      const updated = await updateExamDate(profile.examId, null);
      replaceProfile(updated);
      setFeedback({ tone: 'success', message: `${profile.shortName} exam date cleared.` });
    } catch (error: unknown) {
      setFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setBusyExamId(null);
    }
  }

  async function handleToggleActive(profile: ExamProfile) {
    setBusyExamId(profile.examId);
    setFeedback(null);
    try {
      const updated = await setExamActive(profile.examId, !profile.isActive);
      replaceProfile(updated);
      if (!updated.isActive && selectedExamId === updated.examId) {
        onSelectExam(null);
      }
      setFeedback({
        tone: 'success',
        message: `${profile.shortName} ${
          updated.isActive ? 'activated' : 'deactivated'
        }. Historical records are preserved.`,
      });
    } catch (error: unknown) {
      setFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setBusyExamId(null);
    }
  }

  async function handleConfirmLegacy(profile: ExamProfile) {
    setBusyExamId(profile.examId);
    setFeedback(null);
    try {
      const updated = await confirmLegacyExamDate(profile.examId);
      replaceProfile(updated);
      setFeedback({
        tone: 'success',
        message: `The legacy CSIR-NET date was confirmed as a user-entered date.`,
      });
    } catch (error: unknown) {
      setFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setBusyExamId(null);
    }
  }

  async function handleDismissLegacy(profile: ExamProfile) {
    setBusyExamId(profile.examId);
    setFeedback(null);
    try {
      const updated = await dismissLegacyExamDateNotice(profile.examId);
      replaceProfile(updated);
      setFeedback({
        tone: 'success',
        message: 'The legacy-date notice was dismissed. The original value remains preserved.',
      });
    } catch (error: unknown) {
      setFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setBusyExamId(null);
    }
  }

  return (
    <div className="page-container exams-page">
      <header className="page-header">
        <p className="eyebrow">Examinations</p>
        <h1>Exam profiles</h1>
        <p className="page-lede">
          Manage the four examinations, their one optional date, and whether they appear in
          active-preparation views. Deactivation never deletes progress, sessions, PYQs, mocks,
          revision history, or lifetime study time.
        </p>
      </header>

      {feedback && (
        <p
          className={`save-feedback save-feedback-${feedback.tone}`}
          role={feedback.tone === 'error' ? 'alert' : 'status'}
        >
          {feedback.message}
        </p>
      )}

      {isLoading && (
        <section className="state-card" role="status" aria-live="polite">
          <p className="state-title">Loading exam profiles…</p>
          <p className="state-copy">Reading the four profiles from the local SQLite database.</p>
        </section>
      )}

      {!isLoading && loadError && (
        <section className="state-card state-card-error" role="alert">
          <p className="state-title">Exam profiles could not be loaded</p>
          <p className="state-copy">{loadError}</p>
          <button
            type="button"
            className="primary-button"
            onClick={() => {
              setIsLoading(true);
              setReloadKey((key) => key + 1);
            }}
          >
            Retry
          </button>
        </section>
      )}

      {!isLoading && !loadError && (
        <section className="exam-grid" aria-label="Examination profiles">
          {profiles.map((profile) => {
            const state = dateState(profile, today);
            const busy = busyExamId === profile.examId;
            const selected = selectedExamId === profile.examId;
            const draft = dateDrafts[profile.examId] ?? '';

            return (
              <article
                className={`exam-card${profile.isActive ? '' : ' exam-card-inactive'}${
                  selected ? ' exam-card-selected' : ''
                }`}
                key={profile.examId}
                aria-labelledby={`exam-title-${profile.examId}`}
              >
                <div className="exam-card-header">
                  <div>
                    <p className="eyebrow">{profile.shortName}</p>
                    <h2 id={`exam-title-${profile.examId}`}>{profile.name}</h2>
                    <p className="exam-authority">{profile.authority}</p>
                  </div>
                  <span
                    className={`exam-status ${profile.isActive ? 'exam-status-active' : 'exam-status-inactive'}`}
                  >
                    {profile.isActive ? 'Active' : 'Inactive'}
                  </span>
                </div>

                <div className={`exam-date-state exam-date-state-${state.tone}`} aria-live="polite">
                  <span className="exam-date-state-label">{state.label}</span>
                  <span className="exam-date-state-detail">{state.detail}</span>
                </div>

                <div className="exam-date-editor">
                  <DateField
                    id={`exam-date-${profile.examId}`}
                    label={`${profile.shortName} exam date`}
                    value={draft}
                    onChange={(value) => {
                      setDateDrafts((current) => ({ ...current, [profile.examId]: value }));
                      setFeedback(null);
                    }}
                    hint="Optional. Enter only the date you will use. This app never guesses or scrapes it."
                    disabled={busy}
                  />
                  <div className="exam-date-actions">
                    <button
                      type="button"
                      className="primary-button"
                      onClick={() => void handleSaveDate(profile)}
                      disabled={busy || !draft}
                    >
                      {busy ? 'Saving…' : 'Save date'}
                    </button>
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => void handleClearDate(profile)}
                      disabled={busy || !profile.examDate}
                    >
                      Clear date
                    </button>
                  </div>
                </div>

                {profile.legacyDateNoticeState === 'pending' && profile.legacyExamDate && (
                  <div className="legacy-date-notice" role="note">
                    <p className="legacy-date-title">Previous CSIR-NET date found</p>
                    <p className="legacy-date-copy">
                      The old setting contains{' '}
                      <strong>
                        {formatDateLong(profile.legacyExamDate) ?? profile.legacyExamDate}
                      </strong>
                      . It was not transferred because the database cannot prove whether it was
                      entered by you or supplied as a built-in default.
                    </p>
                    <div className="legacy-date-actions">
                      <button
                        type="button"
                        className="primary-button"
                        onClick={() => void handleConfirmLegacy(profile)}
                        disabled={busy}
                      >
                        Use this legacy date
                      </button>
                      <button
                        type="button"
                        className="secondary-button"
                        onClick={() => void handleDismissLegacy(profile)}
                        disabled={busy}
                      >
                        Dismiss notice
                      </button>
                    </div>
                  </div>
                )}

                <div className="exam-card-footer">
                  <button
                    type="button"
                    className="secondary-button"
                    onClick={() => onSelectExam(selected ? null : profile.examId)}
                    disabled={!profile.isActive}
                    aria-pressed={selected}
                  >
                    {selected ? 'Clear active context' : 'Use as active context'}
                  </button>
                  <button
                    type="button"
                    className={profile.isActive ? 'danger-button' : 'primary-button'}
                    onClick={() => void handleToggleActive(profile)}
                    disabled={busy}
                  >
                    {profile.isActive ? 'Deactivate' : 'Activate'}
                  </button>
                </div>
              </article>
            );
          })}
        </section>
      )}

      <section className="exam-guidance" aria-labelledby="exam-guidance-heading">
        <h2 id="exam-guidance-heading">How dates are handled</h2>
        <p>
          Each profile has exactly one optional date. Empty dates show <strong>Not set</strong>.
          Today shows <strong>Exam day</strong>. Past dates show <strong>Exam date passed</strong>.
          Official multi-day windows are not assumed to be your personal date; enter the date from
          your admit card or notification.
        </p>
      </section>
    </div>
  );
}
