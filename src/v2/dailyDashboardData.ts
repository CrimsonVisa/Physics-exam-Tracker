import { useCallback, useEffect, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import {
  getV2AppSettings,
  getV2PlannerDay,
  getV2RevisionDashboard,
  getV2StudyTotals,
  listV2Sessions,
  type V2AppSettings,
  type V2PlannerDay,
  type V2PracticeAnalytics,
  type V2RevisionDashboard,
  type V2StudySession,
  type V2StudyTotals,
} from './api';

/**
 * Section keys that read the shared daily-dashboard data set. CombinedDashboard
 * fetches that shared set once when any of these sections is visible so that
 * multiple sections never trigger duplicate API calls.
 */
export const DAILY_DATA_SECTION_KEYS = [
  'quickstart',
  'today',
  'plan',
  'recent',
  'revision',
] as const;

export function needsDailyData(sectionKeys: readonly string[]): boolean {
  return sectionKeys.some((key) => (DAILY_DATA_SECTION_KEYS as readonly string[]).includes(key));
}

export type DailyDashboardData = {
  loading: boolean;
  error: string | null;
  retry: () => void;
  sessions: V2StudySession[];
  plannerDay: V2PlannerDay | null;
  revision: V2RevisionDashboard | null;
  settings: V2AppSettings | null;
  totals: V2StudyTotals | null;
};

/**
 * Today's study duration counts every session recorded for the local date
 * exactly once. Exam/topic allocations are breakdowns of a session and are
 * never added a second time.
 */
export function summarizeTodaySessions(
  sessions: V2StudySession[],
  today: string,
): { seconds: number; sessionCount: number } {
  let seconds = 0;
  let sessionCount = 0;
  for (const session of sessions) {
    if (session.sessionDate === today) {
      seconds += session.durationSeconds;
      sessionCount += 1;
    }
  }
  return { seconds, sessionCount };
}

export type PlannerDayCounts = {
  tasksTotal: number;
  tasksDone: number;
  tasksRemaining: number;
  tasksCancelled: number;
  blocksTotal: number;
  blocksDone: number;
  blocksRemaining: number;
  blocksCancelled: number;
};

export function summarizePlannerDay(day: V2PlannerDay | null): PlannerDayCounts {
  const counts: PlannerDayCounts = {
    tasksTotal: 0,
    tasksDone: 0,
    tasksRemaining: 0,
    tasksCancelled: 0,
    blocksTotal: 0,
    blocksDone: 0,
    blocksRemaining: 0,
    blocksCancelled: 0,
  };
  if (!day) {
    return counts;
  }
  for (const task of day.tasks) {
    counts.tasksTotal += 1;
    if (task.status === 'completed') {
      counts.tasksDone += 1;
    } else if (task.status === 'cancelled') {
      counts.tasksCancelled += 1;
    } else {
      counts.tasksRemaining += 1;
    }
  }
  for (const block of day.blocks) {
    counts.blocksTotal += 1;
    if (block.status === 'completed') {
      counts.blocksDone += 1;
    } else if (block.status === 'cancelled') {
      counts.blocksCancelled += 1;
    } else {
      counts.blocksRemaining += 1;
    }
  }
  return counts;
}

export type RecentStudyConfig = {
  key: string;
  sessionDate: string;
  minutes: number;
  topicParts: string[];
};

/**
 * Distinct recent topic-allocation configurations, newest first, derived from
 * already recorded sessions. This is display-only; configurations are never
 * persisted again and never start a timer on their own.
 */
export function recentStudyConfigs(sessions: V2StudySession[], limit = 3): RecentStudyConfig[] {
  const configs: RecentStudyConfig[] = [];
  const seen = new Set<string>();
  const ordered = [...sessions].sort((left, right) => right.id - left.id);
  for (const session of ordered) {
    if (configs.length >= limit) {
      break;
    }
    if (session.topicAllocations.length === 0) {
      continue;
    }
    const topicParts = session.topicAllocations.map((allocation) => {
      const minutes = Math.round(allocation.allocatedSeconds / 60);
      return `${allocation.examCode} · ${allocation.nodeTitle} · ${minutes} min`;
    });
    const key = [...topicParts].sort().join('|');
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    const allocatedSeconds = session.topicAllocations.reduce(
      (total, allocation) => total + allocation.allocatedSeconds,
      0,
    );
    configs.push({
      key,
      sessionDate: session.sessionDate,
      minutes: Math.round(allocatedSeconds / 60),
      topicParts,
    });
  }
  return configs;
}

/**
 * Lowest-accuracy topics from the existing practice analytics command. No new
 * readiness or performance metric is calculated here.
 */
export function weakestTopics(
  analytics: V2PracticeAnalytics | null,
  limit = 3,
): V2PracticeAnalytics['byTopic'] {
  if (!analytics) {
    return [];
  }
  return analytics.byTopic
    .filter((row) => row.accuracyPercentage !== null && row.attempts > 0)
    .slice()
    .sort((left, right) => (left.accuracyPercentage ?? 100) - (right.accuracyPercentage ?? 100))
    .slice(0, limit);
}

export function latestMock(analytics: V2PracticeAnalytics | null) {
  if (!analytics || analytics.mocks.history.length === 0) {
    return null;
  }
  return [...analytics.mocks.history].sort((left, right) =>
    right.testDate.localeCompare(left.testDate),
  )[0];
}

/**
 * Recently studied topics from the existing study totals command, sorted by
 * their most recent study date.
 */
export function recentTopics(totals: V2StudyTotals | null, limit = 6): V2StudyTotals['topics'] {
  if (!totals) {
    return [];
  }
  return totals.topics
    .filter((topic) => topic.lastStudiedDate !== null)
    .slice()
    .sort((left, right) => (right.lastStudiedDate ?? '').localeCompare(left.lastStudiedDate ?? ''))
    .slice(0, limit);
}

const EMPTY_DAILY_DATA = {
  sessions: [] as V2StudySession[],
  plannerDay: null as V2PlannerDay | null,
  revision: null as V2RevisionDashboard | null,
  settings: null as V2AppSettings | null,
  totals: null as V2StudyTotals | null,
};

/**
 * Shared loader for the daily dashboard sections. Loads each backing command
 * exactly once per today change instead of once per section, so sections that
 * share a data source never trigger duplicate API calls. Failures surface
 * through a single retry action; no partial data is merged silently.
 */
export function useDailyDashboardData(today: string, enabled: boolean): DailyDashboardData {
  const [data, setData] = useState(EMPTY_DAILY_DATA);
  const [loading, setLoading] = useState(enabled);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(
    async (signal?: { cancelled: boolean }) => {
      setLoading(true);
      setError(null);
      try {
        const [nextSessions, nextPlannerDay, nextRevision, nextSettings, nextTotals] =
          await Promise.all([
            listV2Sessions(),
            getV2PlannerDay(today),
            getV2RevisionDashboard({ examId: null, today }),
            getV2AppSettings(),
            getV2StudyTotals(),
          ]);
        if (!signal?.cancelled) {
          setData({
            sessions: nextSessions,
            plannerDay: nextPlannerDay,
            revision: nextRevision,
            settings: nextSettings,
            totals: nextTotals,
          });
        }
      } catch (loadError) {
        if (!signal?.cancelled) {
          setError(toCommandError(loadError).message);
        }
      } finally {
        if (!signal?.cancelled) {
          setLoading(false);
        }
      }
    },
    [today],
  );

  useEffect(() => {
    if (!enabled) {
      return undefined;
    }
    const signal = { cancelled: false };
    void (async () => {
      await load(signal);
    })();
    return () => {
      signal.cancelled = true;
    };
  }, [enabled, load]);

  return {
    loading,
    error,
    retry: () => {
      void load({ cancelled: false });
    },
    ...data,
  };
}
