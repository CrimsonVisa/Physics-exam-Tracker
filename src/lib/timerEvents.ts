export const TIMER_UPDATED_EVENT = 'csir-net-tracker.timer.updated.v1';

/**
 * Announce that the persisted V2 timer state was changed from outside the
 * study screen (for example by a dashboard quick start). The study timer
 * component reacts by re-reading the backend snapshot, so both surfaces keep
 * showing the same persisted timer.
 */
export function emitTimerUpdated(): void {
  try {
    window.dispatchEvent(new Event(TIMER_UPDATED_EVENT));
  } catch {
    // A missing window only exists in non-browser test environments; nothing
    // needs synchronising there.
  }
}

export function onTimerUpdated(listener: () => void): () => void {
  window.addEventListener(TIMER_UPDATED_EVENT, listener);
  return () => {
    window.removeEventListener(TIMER_UPDATED_EVENT, listener);
  };
}
