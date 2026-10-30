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
  accuracy: number | null;
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

export type PyqAnalytics = {
  totalRecords: number;
  correct: number;
  incorrect: number;
  skipped: number;
  outcomeUnknown: number;
  attempts: number;
  accuracy: number | null;
  minimumWeakAttempts: number;
  weakAccuracyThreshold: number;
  byTopic: PyqTopicPerformance[];
  byUnit: PyqUnitPerformance[];
  byYear: PyqYearPerformance[];
  byPart: PyqPartPerformance[];
  weakTopics: PyqTopicPerformance[];
};

export type MockTrendPoint = {
  id: number;
  testName: string;
  testDate: string;
  maxMarks: number;
  computedMarks: number;
  computedPercentage: number;
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

export type MockAnalytics = {
  testCount: number;
  averagePercentage: number | null;
  bestPercentage: number | null;
  latestPercentage: number | null;
  trend: MockTrendPoint[];
  byPart: MockPartPerformance[];
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
