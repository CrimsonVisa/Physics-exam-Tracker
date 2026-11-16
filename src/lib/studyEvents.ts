export const STUDY_SESSIONS_CHANGED_EVENT = 'csir-net-tracker.study.sessions.changed.v1';

/**
 * Announce that study-session records changed outside the component that
 * renders them (timer completion or a manual session edit). Dashboard data
 * subscribers re-read their persisted sources instead of keeping stale totals.
 */
export function emitStudySessionsChanged(): void {
  try {
    window.dispatchEvent(new Event(STUDY_SESSIONS_CHANGED_EVENT));
  } catch {
    // A missing window exists only in non-browser test environments.
  }
}

export function onStudySessionsChanged(listener: () => void): () => void {
  window.addEventListener(STUDY_SESSIONS_CHANGED_EVENT, listener);
  return () => {
    window.removeEventListener(STUDY_SESSIONS_CHANGED_EVENT, listener);
  };
}
