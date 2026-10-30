import type { RevisionOutcome, RevisionSettings } from './revisionApi';

export const OUTCOME_LABELS: Record<RevisionOutcome, string> = {
  again: 'Again / forgot',
  difficult: 'Difficult',
  good: 'Good',
  easy: 'Easy',
};

export function outcomeLabel(outcome: RevisionOutcome | null): string {
  return outcome ? OUTCOME_LABELS[outcome] : 'No previous outcome';
}

export function initialIntervalFor(settings: RevisionSettings, outcome: RevisionOutcome): number {
  switch (outcome) {
    case 'again':
      return settings.initialAgainDays;
    case 'difficult':
      return settings.initialDifficultDays;
    case 'good':
      return settings.initialGoodDays;
    default:
      return settings.initialEasyDays;
  }
}

/**
 * Mirrors the Rust scheduler: Again and Difficult reset to their configured
 * base interval; Good and Easy grow from the current interval. Every result is
 * clamped to the configured minimum and maximum.
 */
export function nextIntervalFor(
  settings: RevisionSettings,
  currentIntervalDays: number,
  outcome: RevisionOutcome,
): number {
  const base = initialIntervalFor(settings, outcome);
  const candidate =
    outcome === 'good' || outcome === 'easy'
      ? Math.max(base, Math.round(currentIntervalDays * settings.growthMultiplier))
      : base;

  return Math.min(settings.maximumIntervalDays, Math.max(settings.minimumIntervalDays, candidate));
}
