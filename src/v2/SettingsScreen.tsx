import { useEffect, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import { SHORTCUT_REFERENCE } from '../lib/appShortcuts';
import {
  applyAppearance,
  FONT_FAMILIES,
  FONT_SIZE_OPTIONS,
  getStoredFontFamily,
  getStoredFontSize,
  storeFontFamily,
  storeFontSize,
  type FontFamily,
  type FontSizePreference,
} from '../lib/appearance';
import {
  applyV2ExamDateChange,
  getV2AppSettings,
  listV2ExamProfiles,
  previewV2ExamDateChange,
  setV2ExamActive,
  updateV2AppSettings,
  type V2AppSettings,
  type V2ExamProfile,
} from './api';

type SettingsScreenProps = {
  onChanged: () => void;
};

export function SettingsScreen({ onChanged }: SettingsScreenProps) {
  const [settings, setSettings] = useState<V2AppSettings | null>(null);
  const [profiles, setProfiles] = useState<V2ExamProfile[]>([]);
  const [prepDate, setPrepDate] = useState('');
  const [dailyTarget, setDailyTarget] = useState('120');
  const [dateDrafts, setDateDrafts] = useState<Record<number, string>>({});
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [fontFamily, setFontFamily] = useState<FontFamily>(() => getStoredFontFamily());
  const [fontSize, setFontSize] = useState<FontSizePreference>(() => getStoredFontSize());

  function chooseFontFamily(next: FontFamily) {
    setFontFamily(next);
    storeFontFamily(next);
    applyAppearance(next, fontSize);
  }

  function chooseFontSize(next: FontSizePreference) {
    setFontSize(next);
    storeFontSize(next);
    applyAppearance(fontFamily, next);
  }

  async function load() {
    try {
      const [nextSettings, nextProfiles] = await Promise.all([
        getV2AppSettings(),
        listV2ExamProfiles(),
      ]);
      setSettings(nextSettings);
      setPrepDate(nextSettings.preparationStartDate ?? '');
      setDailyTarget(String(nextSettings.dailyStudyTargetMinutes));
      setProfiles(nextProfiles);
      setDateDrafts(
        Object.fromEntries(nextProfiles.map((profile) => [profile.id, profile.examDate ?? ''])),
      );
      setError(null);
    } catch (loadError) {
      setError(toCommandError(loadError).message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void (async () => {
      await load();
    })();
  }, []);

  async function saveAppSettings() {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const next = await updateV2AppSettings({
        preparationStartDate: prepDate || null,
        dailyStudyTargetMinutes: Number(dailyTarget),
      });
      setSettings(next);
      setPrepDate(next.preparationStartDate ?? '');
      setDailyTarget(String(next.dailyStudyTargetMinutes));
      setNotice('Application settings saved.');
      onChanged();
    } catch (saveError) {
      setError(toCommandError(saveError).message);
    } finally {
      setBusy(false);
    }
  }

  async function saveExamDate(profile: V2ExamProfile) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const draft = dateDrafts[profile.id] ?? '';
      const preview = await previewV2ExamDateChange({
        examId: profile.id,
        examDate: draft || null,
        reason: 'Settings',
      });
      const updated = await applyV2ExamDateChange({
        examId: profile.id,
        examDate: draft || null,
        reason: 'Settings',
      });
      setProfiles((current) => current.map((item) => (item.id === updated.id ? updated : item)));
      setDateDrafts((current) => ({
        ...current,
        [profile.id]: updated.examDate ?? '',
      }));
      setNotice(
        preview.affectedTasks.length > 0
          ? `${profile.shortName} date saved. ${preview.affectedTasks.length} exam-anchored task(s) were rescheduled.`
          : `${profile.shortName} date saved.`,
      );
      onChanged();
    } catch (saveError) {
      setError(toCommandError(saveError).message);
    } finally {
      setBusy(false);
    }
  }

  async function toggleExam(profile: V2ExamProfile) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const updated = await setV2ExamActive({
        examId: profile.id,
        isActive: !profile.isActive,
      });
      setProfiles((current) => current.map((item) => (item.id === updated.id ? updated : item)));
      setNotice(`${profile.shortName} is now ${updated.isActive ? 'active' : 'inactive'}.`);
      onChanged();
    } catch (toggleError) {
      setError(toCommandError(toggleError).message);
    } finally {
      setBusy(false);
    }
  }

  if (loading) {
    return (
      <div className="page-container v2-settings-page">
        <p className="v2-empty-copy">Loading settings and examination profiles…</p>
      </div>
    );
  }

  return (
    <div className="page-container v2-settings-page">
      <header className="page-header">
        <p className="eyebrow">Physics Tracker · configuration</p>
        <h1>Settings</h1>
        <p className="page-lede">
          Shared application settings and the four examination profiles. Exam dates are manual;
          inactive examinations keep their history but cannot receive new work.
        </p>
      </header>

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

      <section className="v2-concept-card" aria-labelledby="appearance-heading">
        <p className="eyebrow">Appearance</p>
        <h2 id="appearance-heading">Typography</h2>
        <p className="v2-empty-copy">
          The interface font and text size apply immediately across Physics Tracker and persist on
          this device.
        </p>
        <div className="v2-appearance-grid">
          <div className="v2-day-plan-list">
            <div aria-label="Interface font family" role="group">
              <p className="eyebrow">Interface font</p>
              <div className="v2-font-options">
                {FONT_FAMILIES.map((option) => (
                  <button
                    aria-pressed={fontFamily === option.id}
                    className="v2-font-option"
                    key={option.id}
                    onClick={() => chooseFontFamily(option.id)}
                    type="button"
                  >
                    <span className={`v2-font-option-sample ${option.sampleClassName}`}>
                      {option.label}
                    </span>
                    <span className="v2-font-option-label">
                      {fontFamily === option.id ? 'Active' : 'Aa · 123 · 45%'}
                    </span>
                  </button>
                ))}
              </div>
            </div>
            <div aria-label="Text size" role="group">
              <p className="eyebrow">Text size</p>
              <div className="v2-density-options">
                {FONT_SIZE_OPTIONS.map((option) => (
                  <button
                    aria-pressed={fontSize === option.id}
                    className={`v2-day-preset${
                      fontSize === option.id ? ' v2-day-preset-active' : ''
                    }`}
                    key={option.id}
                    onClick={() => chooseFontSize(option.id)}
                    type="button"
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            </div>
          </div>
          <aside aria-label="Typography preview" className="v2-appearance-preview">
            <p className="eyebrow">Preview</p>
            <h3>Quantum mechanics — problem set</h3>
            <p>
              Dense study notes stay readable with a consistent body size and clear heading levels.
            </p>
            <p className="v2-appearance-number">25:00 · 67.5% · GATE-PH · ½ ħω</p>
          </aside>
        </div>
      </section>

      <section className="v2-concept-card" aria-labelledby="shortcuts-heading">
        <p className="eyebrow">Keyboard</p>
        <h2 id="shortcuts-heading">Keyboard shortcuts</h2>
        <p className="v2-empty-copy">
          Navigation shortcuts pause while a text field is focused; Ctrl+K always works. Timer and
          destructive actions intentionally have no keyboard shortcuts.
        </p>
        <dl className="v2-shortcut-reference">
          {SHORTCUT_REFERENCE.map((entry) => (
            <div key={`${entry.keys.join('-')}-${entry.label}`}>
              <dt>
                {entry.keys.map((key) => (
                  <kbd key={key}>{key}</kbd>
                ))}
              </dt>
              <dd>{entry.label}</dd>
            </div>
          ))}
        </dl>
      </section>

      <section className="v2-concept-card" aria-labelledby="app-settings-heading">
        <p className="eyebrow">Study preferences</p>
        <h2 id="app-settings-heading">Application settings</h2>
        {settings && (
          <div className="v2-settings-form">
            <label className="v2-field">
              <span>Preparation start date</span>
              <input
                onChange={(event) => setPrepDate(event.target.value)}
                type="date"
                value={prepDate}
              />
            </label>
            <label className="v2-field">
              <span>Daily study target (minutes)</span>
              <input
                max={1440}
                min={1}
                onChange={(event) => setDailyTarget(event.target.value)}
                type="number"
                value={dailyTarget}
              />
            </label>
            <button
              className="primary-button"
              disabled={busy}
              onClick={() => void saveAppSettings()}
              type="button"
            >
              Save application settings
            </button>
          </div>
        )}
      </section>

      <section className="v2-concept-card" aria-labelledby="exam-settings-heading">
        <p className="eyebrow">Examinations</p>
        <h2 id="exam-settings-heading">Exam profiles</h2>
        <div className="v2-exam-settings-list">
          {profiles.map((profile) => (
            <article
              className={`v2-exam-settings-row${profile.isActive ? '' : ' v2-exam-settings-inactive'}`}
              data-exam-code={profile.code}
              key={profile.id}
            >
              <div>
                <div className="v2-session-heading">
                  <strong>{profile.name}</strong>
                  <span
                    className={`v2-status-badge ${
                      profile.isActive ? 'v2-status-active' : 'v2-status-inactive'
                    }`}
                  >
                    {profile.isActive ? 'Active' : 'Inactive'}
                  </span>
                </div>
                <p className="v2-empty-copy">
                  {profile.shortName} · {profile.code} · {profile.authority}
                </p>
              </div>
              <div className="v2-exam-date-editor">
                <label className="v2-field">
                  <span>Exam date</span>
                  <input
                    onChange={(event) =>
                      setDateDrafts((current) => ({
                        ...current,
                        [profile.id]: event.target.value,
                      }))
                    }
                    type="date"
                    value={dateDrafts[profile.id] ?? ''}
                  />
                </label>
                <button
                  className="secondary-button"
                  disabled={busy}
                  onClick={() => void saveExamDate(profile)}
                  type="button"
                >
                  Save date
                </button>
                <button
                  className={`secondary-button${profile.isActive ? ' v2-danger-button' : ''}`}
                  disabled={busy}
                  onClick={() => void toggleExam(profile)}
                  type="button"
                >
                  {profile.isActive ? 'Deactivate' : 'Reactivate'}
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}
