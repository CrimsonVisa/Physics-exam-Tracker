export const REVISIONS_CHANGED_EVENT = 'csir-net-tracker.revisions.changed.v1';

/**
 * Announce that revision records, schedules, or mapping eligibility changed
 * outside the component that renders them. Dashboard data subscribers re-read
 * the persisted revision dashboard instead of keeping stale due/upcoming data.
 */
export function emitRevisionsChanged(): void {
  try {
    window.dispatchEvent(new Event(REVISIONS_CHANGED_EVENT));
  } catch {
    // A missing window exists only in non-browser test environments.
  }
}

export function onRevisionsChanged(listener: () => void): () => void {
  window.addEventListener(REVISIONS_CHANGED_EVENT, listener);
  return () => {
    window.removeEventListener(REVISIONS_CHANGED_EVENT, listener);
  };
}
