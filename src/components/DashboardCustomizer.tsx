import { useState } from 'react';
import { toCommandError } from '../lib/commandError';
import {
  resetDashboardLayout,
  updateDashboardLayout,
  type DashboardLayout,
  type DashboardSection,
  type DashboardSectionKey,
} from '../lib/preparationApi';

const SECTION_LABELS: Record<DashboardSectionKey, string> = {
  'exam-countdown': 'Exam countdown and preparation dates',
  'syllabus-progress': 'Syllabus progress',
  'study-totals': 'Study-time totals',
  'today-study': "Today's study time",
  'revision-status': 'Revision status',
  'deadlines-blocks': 'Deadlines and planned blocks',
  'recent-pyq': 'Recent PYQ attempts',
  'latest-mock': 'Latest mock-test result',
  insights: 'Actionable insights',
};

type Feedback = {
  tone: 'success' | 'error';
  message: string;
} | null;

type DashboardCustomizerProps = {
  sections: DashboardSection[];
  onChanged: (layout: DashboardLayout) => void;
  onClose: () => void;
};

export function DashboardCustomizer({ sections, onChanged, onClose }: DashboardCustomizerProps) {
  const [draft, setDraft] = useState(() =>
    [...sections].sort((left, right) => left.displayOrder - right.displayOrder),
  );
  const [isSaving, setIsSaving] = useState(false);
  const [feedback, setFeedback] = useState<Feedback>(null);

  function move(index: number, offset: number) {
    const target = index + offset;
    if (target < 0 || target >= draft.length) {
      return;
    }
    const next = [...draft];
    [next[index], next[target]] = [next[target], next[index]];
    setDraft(
      next.map((section, position) => ({
        ...section,
        displayOrder: position,
      })),
    );
    setFeedback(null);
  }

  function toggle(key: DashboardSectionKey, visible: boolean) {
    setDraft((current) =>
      current.map((section) => (section.key === key ? { ...section, visible } : section)),
    );
    setFeedback(null);
  }

  async function handleSave() {
    if (isSaving) {
      return;
    }
    setIsSaving(true);
    setFeedback(null);
    try {
      const saved = await updateDashboardLayout(
        draft.map((section, index) => ({
          key: section.key,
          visible: section.visible,
          displayOrder: index,
        })),
      );
      onChanged(saved);
      setDraft([...saved.sections].sort((left, right) => left.displayOrder - right.displayOrder));
      setFeedback({ tone: 'success', message: 'Dashboard layout saved to SQLite.' });
    } catch (error: unknown) {
      setFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setIsSaving(false);
    }
  }

  async function handleReset() {
    if (isSaving) {
      return;
    }
    setIsSaving(true);
    setFeedback(null);
    try {
      const reset = await resetDashboardLayout();
      onChanged(reset);
      setDraft([...reset.sections].sort((left, right) => left.displayOrder - right.displayOrder));
      setFeedback({
        tone: 'success',
        message: 'Default dashboard layout restored. All sections are visible.',
      });
    } catch (error: unknown) {
      setFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <section className="dashboard-customizer" aria-labelledby="dashboard-customizer-heading">
      <div className="card-heading">
        <div>
          <p className="eyebrow">SQLite-backed layout</p>
          <h2 id="dashboard-customizer-heading">Customize dashboard</h2>
        </div>
        <button type="button" className="secondary-button" onClick={onClose}>
          Close customizer
        </button>
      </div>

      <p className="settings-lede">
        Move sections with the arrow buttons and clear a checkbox to hide it. Save writes the
        complete order to the local SQLite database, so it survives restarts. Defaults suit existing
        users and can be restored at any time.
      </p>

      <ol className="dashboard-customizer-list">
        {draft.map((section, index) => (
          <li key={section.key}>
            <label className="dashboard-customizer-visible">
              <input
                type="checkbox"
                checked={section.visible}
                onChange={(event) => toggle(section.key, event.target.checked)}
                disabled={isSaving}
              />
              <span>{SECTION_LABELS[section.key]}</span>
            </label>
            <div className="dashboard-customizer-actions">
              <button
                type="button"
                className="secondary-button"
                aria-label={`Move ${SECTION_LABELS[section.key]} up`}
                onClick={() => move(index, -1)}
                disabled={isSaving || index === 0}
              >
                ↑ Up
              </button>
              <button
                type="button"
                className="secondary-button"
                aria-label={`Move ${SECTION_LABELS[section.key]} down`}
                onClick={() => move(index, 1)}
                disabled={isSaving || index === draft.length - 1}
              >
                ↓ Down
              </button>
            </div>
          </li>
        ))}
      </ol>

      <div className="form-footer">
        <button
          type="button"
          className="primary-button"
          onClick={() => void handleSave()}
          disabled={isSaving}
        >
          {isSaving ? 'Saving…' : 'Save layout'}
        </button>
        <button
          type="button"
          className="secondary-button"
          onClick={() => void handleReset()}
          disabled={isSaving}
        >
          Restore default layout
        </button>
      </div>

      {feedback && (
        <p
          className={`save-feedback save-feedback-${feedback.tone}`}
          role={feedback.tone === 'error' ? 'alert' : 'status'}
        >
          {feedback.message}
        </p>
      )}
    </section>
  );
}
