import { type FormEvent, useState } from 'react';
import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification';
import { toCommandError } from '../lib/commandError';
import {
  checkRevisionReminder,
  updateRevisionSettings,
  type RevisionSettings,
  type RevisionSettingsInput,
} from '../lib/revisionApi';

type SettingsForm = {
  initialAgainDays: string;
  initialDifficultDays: string;
  initialGoodDays: string;
  initialEasyDays: string;
  growthMultiplier: string;
  minimumIntervalDays: string;
  maximumIntervalDays: string;
  includeWeekends: boolean;
  dailyRevisionTarget: string;
  remindersEnabled: boolean;
};

type SettingsField =
  | 'initialAgainDays'
  | 'initialDifficultDays'
  | 'initialGoodDays'
  | 'initialEasyDays'
  | 'growthMultiplier'
  | 'minimumIntervalDays'
  | 'maximumIntervalDays'
  | 'dailyRevisionTarget';

type RevisionSettingsPanelProps = {
  settings: RevisionSettings;
  onSaved: (settings: RevisionSettings) => void;
};

function formFromSettings(settings: RevisionSettings): SettingsForm {
  return {
    initialAgainDays: String(settings.initialAgainDays),
    initialDifficultDays: String(settings.initialDifficultDays),
    initialGoodDays: String(settings.initialGoodDays),
    initialEasyDays: String(settings.initialEasyDays),
    growthMultiplier: String(settings.growthMultiplier),
    minimumIntervalDays: String(settings.minimumIntervalDays),
    maximumIntervalDays: String(settings.maximumIntervalDays),
    includeWeekends: settings.includeWeekends,
    dailyRevisionTarget: String(settings.dailyRevisionTarget),
    remindersEnabled: settings.remindersEnabled,
  };
}

function parseInteger(value: string): number | null {
  const trimmed = value.trim();
  if (!/^\d+$/.test(trimmed)) {
    return null;
  }
  return Number(trimmed);
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

export function RevisionSettingsPanel({ settings, onSaved }: RevisionSettingsPanelProps) {
  const [form, setForm] = useState<SettingsForm>(() => formFromSettings(settings));
  const [errors, setErrors] = useState<Partial<Record<SettingsField, string>>>({});
  const [isSaving, setIsSaving] = useState(false);
  const [feedback, setFeedback] = useState<{
    tone: 'success' | 'error';
    message: string;
  } | null>(null);

  function handleField(field: SettingsField, value: string) {
    setForm((current) => ({ ...current, [field]: value }));
    setErrors((current) => {
      if (!current[field]) {
        return current;
      }
      const next = { ...current };
      delete next[field];
      return next;
    });
    setFeedback(null);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (isSaving) {
      return;
    }

    setFeedback(null);
    const nextErrors: Partial<Record<SettingsField, string>> = {};
    const days: Array<[SettingsField, string]> = [
      ['initialAgainDays', form.initialAgainDays],
      ['initialDifficultDays', form.initialDifficultDays],
      ['initialGoodDays', form.initialGoodDays],
      ['initialEasyDays', form.initialEasyDays],
      ['minimumIntervalDays', form.minimumIntervalDays],
      ['maximumIntervalDays', form.maximumIntervalDays],
      ['dailyRevisionTarget', form.dailyRevisionTarget],
    ];
    const parsed: Partial<Record<SettingsField, number>> = {};

    for (const [field, value] of days) {
      const number = parseInteger(value);
      if (number === null || number < 1) {
        nextErrors[field] = 'Enter a whole number of 1 or more.';
      } else {
        parsed[field] = number;
      }
    }

    const growth = Number(form.growthMultiplier.trim());
    if (!Number.isFinite(growth) || growth < 1 || growth > 10) {
      nextErrors.growthMultiplier = 'Growth multiplier must be from 1.0 to 10.0.';
    }

    if (Object.keys(nextErrors).length > 0) {
      setErrors(nextErrors);
      setFeedback({ tone: 'error', message: 'Check the highlighted settings fields.' });
      return;
    }

    const input: RevisionSettingsInput = {
      initialAgainDays: parsed.initialAgainDays!,
      initialDifficultDays: parsed.initialDifficultDays!,
      initialGoodDays: parsed.initialGoodDays!,
      initialEasyDays: parsed.initialEasyDays!,
      growthMultiplier: growth,
      minimumIntervalDays: parsed.minimumIntervalDays!,
      maximumIntervalDays: parsed.maximumIntervalDays!,
      includeWeekends: form.includeWeekends,
      dailyRevisionTarget: parsed.dailyRevisionTarget!,
      remindersEnabled: form.remindersEnabled,
    };

    if (form.remindersEnabled && !settings.remindersEnabled) {
      const granted = await ensureNotificationPermission();
      if (!granted) {
        setFeedback({
          tone: 'error',
          message:
            'Windows did not grant notification permission. Reminders were not enabled; the other settings were not saved either.',
        });
        return;
      }
    }

    setIsSaving(true);

    try {
      const saved = await updateRevisionSettings(input);
      setForm(formFromSettings(saved));
      setErrors({});
      setFeedback({
        tone: 'success',
        message:
          'Revision settings saved. Existing schedules keep their current due dates until their next completed revision.',
      });
      onSaved(saved);
      if (saved.remindersEnabled) {
        checkRevisionReminder().catch(() => {
          // Notification permission or delivery can be unavailable without blocking settings.
        });
      }
    } catch (error: unknown) {
      const commandError = toCommandError(error);
      const field = commandError.field as SettingsField | undefined;
      if (
        field &&
        [
          'initialAgainDays',
          'initialDifficultDays',
          'initialGoodDays',
          'initialEasyDays',
          'growthMultiplier',
          'minimumIntervalDays',
          'maximumIntervalDays',
          'dailyRevisionTarget',
        ].includes(field)
      ) {
        setErrors({ [field]: commandError.message });
      }
      setFeedback({ tone: 'error', message: commandError.message });
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <form className="revision-settings-form" onSubmit={handleSubmit} noValidate>
      <div className="revision-settings-grid">
        {(
          [
            ['initialAgainDays', 'Again initial days'],
            ['initialDifficultDays', 'Difficult initial days'],
            ['initialGoodDays', 'Good initial days'],
            ['initialEasyDays', 'Easy initial days'],
          ] as Array<[SettingsField, string]>
        ).map(([field, label]) => (
          <div className={`form-field${errors[field] ? ' form-field-invalid' : ''}`} key={field}>
            <label htmlFor={`revision-${field}`}>{label}</label>
            <input
              id={`revision-${field}`}
              type="number"
              min={1}
              max={3650}
              step={1}
              value={form[field]}
              onChange={(event) => handleField(field, event.target.value)}
              aria-invalid={errors[field] ? true : undefined}
              disabled={isSaving}
            />
            {errors[field] && (
              <p className="field-error" role="alert">
                {errors[field]}
              </p>
            )}
          </div>
        ))}

        <div className={`form-field${errors.growthMultiplier ? ' form-field-invalid' : ''}`}>
          <label htmlFor="revision-growth">Growth multiplier</label>
          <input
            id="revision-growth"
            type="number"
            min={1}
            max={10}
            step="0.05"
            value={form.growthMultiplier}
            onChange={(event) => handleField('growthMultiplier', event.target.value)}
            aria-invalid={errors.growthMultiplier ? true : undefined}
            disabled={isSaving}
          />
          {errors.growthMultiplier && (
            <p className="field-error" role="alert">
              {errors.growthMultiplier}
            </p>
          )}
        </div>

        <div className={`form-field${errors.minimumIntervalDays ? ' form-field-invalid' : ''}`}>
          <label htmlFor="revision-minimum">Minimum interval days</label>
          <input
            id="revision-minimum"
            type="number"
            min={1}
            max={3650}
            step={1}
            value={form.minimumIntervalDays}
            onChange={(event) => handleField('minimumIntervalDays', event.target.value)}
            aria-invalid={errors.minimumIntervalDays ? true : undefined}
            disabled={isSaving}
          />
          {errors.minimumIntervalDays && (
            <p className="field-error" role="alert">
              {errors.minimumIntervalDays}
            </p>
          )}
        </div>

        <div className={`form-field${errors.maximumIntervalDays ? ' form-field-invalid' : ''}`}>
          <label htmlFor="revision-maximum">Maximum interval days</label>
          <input
            id="revision-maximum"
            type="number"
            min={1}
            max={3650}
            step={1}
            value={form.maximumIntervalDays}
            onChange={(event) => handleField('maximumIntervalDays', event.target.value)}
            aria-invalid={errors.maximumIntervalDays ? true : undefined}
            disabled={isSaving}
          />
          {errors.maximumIntervalDays && (
            <p className="field-error" role="alert">
              {errors.maximumIntervalDays}
            </p>
          )}
        </div>

        <div className={`form-field${errors.dailyRevisionTarget ? ' form-field-invalid' : ''}`}>
          <label htmlFor="revision-daily-target">Daily revision target</label>
          <input
            id="revision-daily-target"
            type="number"
            min={1}
            max={100}
            step={1}
            value={form.dailyRevisionTarget}
            onChange={(event) => handleField('dailyRevisionTarget', event.target.value)}
            aria-invalid={errors.dailyRevisionTarget ? true : undefined}
            disabled={isSaving}
          />
          {errors.dailyRevisionTarget && (
            <p className="field-error" role="alert">
              {errors.dailyRevisionTarget}
            </p>
          )}
        </div>
      </div>

      <div className="revision-settings-checks">
        <label>
          <input
            type="checkbox"
            checked={form.includeWeekends}
            onChange={(event) => {
              setForm((current) => ({ ...current, includeWeekends: event.target.checked }));
              setFeedback(null);
            }}
            disabled={isSaving}
          />
          Include Saturdays and Sundays when calculating the next date
        </label>
        <label>
          <input
            type="checkbox"
            checked={form.remindersEnabled}
            onChange={(event) => {
              setForm((current) => ({ ...current, remindersEnabled: event.target.checked }));
              setFeedback(null);
            }}
            disabled={isSaving}
          />
          Desktop review reminders (off by default)
        </label>
      </div>

      <p className="field-hint">
        Good and Easy outcomes grow from the current interval by the multiplier. Again and Difficult
        reset to their configured initial intervals. All results are clamped between the minimum and
        maximum. Changing the rules does not move existing due dates until the next completed
        revision.
      </p>

      <div className="form-footer">
        <button type="submit" className="primary-button" disabled={isSaving}>
          {isSaving ? 'Saving…' : 'Save revision settings'}
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
    </form>
  );
}
