import { useEffect, useState } from 'react';
import { todayDateOnly } from './dateOnly';

const REFRESH_INTERVAL_MS = 60_000;

/**
 * Keep the local calendar date current while the app stays open. Passing a
 * fixed date disables the timer so tests and previews are deterministic.
 */
export function useTodayDateOnly(fixedToday?: string): string {
  const [liveToday, setLiveToday] = useState(todayDateOnly);

  useEffect(() => {
    if (fixedToday) {
      return;
    }

    const interval = window.setInterval(() => {
      setLiveToday(todayDateOnly());
    }, REFRESH_INTERVAL_MS);

    return () => window.clearInterval(interval);
  }, [fixedToday]);

  return fixedToday ?? liveToday;
}
