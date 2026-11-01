import { invoke } from '@tauri-apps/api/core';

export type PyqTopicPerformance = {
  topicId: number;
  code: string;
  title: string;
  archived: boolean;
  sectionCode: string;
  sectionTitle: string;
  correct: number;
  incorrect: number;
  skipped: number;
  attempts: number;
  totalAttempts: number;
  distinctQuestions: number;
  evidenceQuestions: number;
  accuracy: number | null;
  evidenceAccuracy: number | null;
  lastScoredAttemptDate: string | null;
  skippedOnly: boolean;
  sufficientData: boolean;
};

export type PyqUnitPerformance = {
  code: string;
  title: string;
  correct: number;
  incorrect: number;
  skipped: number;
  attempts: number;
  accuracy: number | null;
};

export type PyqYearPerformance = {
  year: number;
  correct: number;
  incorrect: number;
  skipped: number;
  attempts: number;
  accuracy: number | null;
};

export type PyqPartPerformance = {
  part: string;
  correct: number;
  incorrect: number;
  skipped: number;
  attempts: number;
  accuracy: number | null;
};

export type PyqResponseTrendPoint = {
  date: string;
  correct: number;
  incorrect: number;
  skipped: number;
  outcomeUnknown: number;
};

export type PyqAnalytics = {
  totalRecords: number;
  correct: number;
  incorrect: number;
  skipped: number;
  outcomeUnknown: number;
  attempts: number;
  accuracy: number | null;
  minimumEvidenceQuestions: number;
  evidenceWindowDays: number;
  weakAccuracyThreshold: number;
  byTopic: PyqTopicPerformance[];
  byUnit: PyqUnitPerformance[];
  byYear: PyqYearPerformance[];
  byPart: PyqPartPerformance[];
  weakTopics: PyqTopicPerformance[];
  responseTrend: PyqResponseTrendPoint[];
};

export type MockTrendPoint = {
  id: number;
  testName: string;
  testDate: string;
  maxMarks: number;
  computedMarks: number;
  computedPercentage: number;
  configurationValid: boolean;
  reportedMarks: number | null;
};

export type MockPartPerformance = {
  part: string;
  questions: number;
  attempted: number;
  correct: number;
  incorrect: number;
  unanswered: number;
  accuracy: number | null;
};

export type MockResponseTrendPoint = {
  date: string;
  testCount: number;
  correct: number;
  incorrect: number;
  unanswered: number;
  averagePercentage: number | null;
};

export type MockAnalytics = {
  testCount: number;
  averagePercentage: number | null;
  bestPercentage: number | null;
  latestPercentage: number | null;
  totalQuestions: number;
  totalAttempted: number;
  totalCorrect: number;
  totalIncorrect: number;
  totalUnanswered: number;
  trend: MockTrendPoint[];
  byPart: MockPartPerformance[];
  responseTrend: MockResponseTrendPoint[];
};

export type PerformanceAnalytics = {
  fromDate: string;
  toDate: string;
  pyq: PyqAnalytics;
  mock: MockAnalytics;
  separationNote: string;
};

export type StudyPerformanceDay = {
  date: string;
  studySeconds: number;
  pyqCorrect: number;
  pyqIncorrect: number;
  pyqAccuracy: number | null;
  mockCount: number;
  mockAveragePercentage: number | null;
};

export type StudyPerformance = {
  days: StudyPerformanceDay[];
};

export function getPyqAnalytics(): Promise<PyqAnalytics> {
  return invoke<PyqAnalytics>('get_pyq_analytics');
}

export function getMockAnalytics(): Promise<MockAnalytics> {
  return invoke<MockAnalytics>('get_mock_analytics');
}

export function getStudyPerformance(): Promise<StudyPerformance> {
  return invoke<StudyPerformance>('get_study_performance');
}

export function getPerformanceAnalytics(
  fromDate: string,
  toDate: string,
): Promise<PerformanceAnalytics> {
  return invoke<PerformanceAnalytics>('get_performance_analytics', { fromDate, toDate });
}
