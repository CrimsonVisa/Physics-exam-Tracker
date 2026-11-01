import { invoke } from '@tauri-apps/api/core';
import type { ImportPreview, ImportResult } from './csvImportApi';

export type MockPartResult = {
  questions: number;
  attempted: number;
  correct: number;
  incorrect: number;
  unanswered: number;
  correctMarks: number;
  incorrectMarks: number;
  unansweredMarks: number;
  marks: number;
};

export type MockTest = {
  id: number;
  testName: string;
  testDate: string;
  durationMinutes: number;
  maxMarks: number;
  marksObtained: number | null;
  partA: MockPartResult;
  partB: MockPartResult;
  partC: MockPartResult;
  totalQuestions: number;
  totalAttempted: number;
  totalCorrect: number;
  totalIncorrect: number;
  totalUnanswered: number;
  computedMarks: number;
  computedPercentage: number;
  configurationValid: boolean;
  notes: string;
  reviewObservations: string;
  createdAt: string;
  updatedAt: string;
};

export type MockPartInput = {
  questions: number;
  correct: number;
  incorrect: number;
  unanswered: number;
  correctMarks: number | null;
  incorrectMarks: number | null;
  unansweredMarks: number | null;
};

export type MockTestInput = {
  testName: string;
  testDate: string;
  durationMinutes: number;
  maxMarks: number;
  marksObtained: number | null;
  partA: MockPartInput;
  partB: MockPartInput;
  partC: MockPartInput;
  notes: string;
  reviewObservations: string;
};

export function getMockTests(): Promise<MockTest[]> {
  return invoke<MockTest[]>('get_mock_tests');
}

export function createMockTest(test: MockTestInput): Promise<MockTest> {
  return invoke<MockTest>('create_mock_test', { test });
}

export function updateMockTest(testId: number, test: MockTestInput): Promise<MockTest> {
  return invoke<MockTest>('update_mock_test', { testId, test });
}

export function deleteMockTest(testId: number): Promise<void> {
  return invoke<void>('delete_mock_test', { testId });
}

export function saveMockCsvTemplate(): Promise<string> {
  return invoke<string>('save_mock_csv_template');
}

export function previewMockImport(csv: string): Promise<ImportPreview> {
  return invoke<ImportPreview>('preview_mock_import', { csv });
}

export function applyMockImport(csv: string): Promise<ImportResult> {
  return invoke<ImportResult>('apply_mock_import', { csv });
}
