import { invoke } from '@tauri-apps/api/core';
import type { ImportPreview, ImportResult } from './csvImportApi';

export const PYQ_STATUSES = ['attempted', 'correct', 'incorrect', 'skipped'] as const;
export const PYQ_PARTS = ['A', 'B', 'C'] as const;

export type PyqStatus = (typeof PYQ_STATUSES)[number];
export type PyqPart = (typeof PYQ_PARTS)[number];

export type PyqTopic = {
  id: number;
  code: string;
  title: string;
  archived: boolean;
  sectionCode: string;
  sectionTitle: string;
};

export type PyqSection = {
  code: string;
  title: string;
};

export type PyqOptions = {
  years: number[];
  sections: PyqSection[];
  topics: PyqTopic[];
};

export type PyqAttempt = {
  id: number;
  questionYear: number;
  examSession: string;
  part: PyqPart;
  topic: PyqTopic;
  questionReference: string;
  attemptDate: string;
  status: PyqStatus;
  marksObtained: number | null;
  maxMarks: number | null;
  timeSpentSeconds: number;
  notes: string;
  difficulty: number | null;
  createdAt: string;
  updatedAt: string;
};

export type PyqFilter = {
  year?: number;
  part?: PyqPart;
  sectionCode?: string;
  topicId?: number;
  status?: PyqStatus;
  query?: string;
  fromDate?: string;
  toDate?: string;
};

export type PyqInput = {
  questionYear: number;
  examSession: string;
  part: PyqPart;
  topicId: number;
  questionReference: string;
  attemptDate: string;
  status: PyqStatus;
  marksObtained: number | null;
  maxMarks: number | null;
  timeSpentSeconds: number;
  notes: string;
  difficulty: number | null;
};

export function getPyqOptions(): Promise<PyqOptions> {
  return invoke<PyqOptions>('get_pyq_options');
}

export function getPyqAttempts(filter: PyqFilter = {}): Promise<PyqAttempt[]> {
  return invoke<PyqAttempt[]>('get_pyq_attempts', { filter });
}

export function createPyqAttempt(attempt: PyqInput): Promise<PyqAttempt> {
  return invoke<PyqAttempt>('create_pyq_attempt', { attempt });
}

export function updatePyqAttempt(attemptId: number, attempt: PyqInput): Promise<PyqAttempt> {
  return invoke<PyqAttempt>('update_pyq_attempt', { attemptId, attempt });
}

export function deletePyqAttempt(attemptId: number): Promise<void> {
  return invoke<void>('delete_pyq_attempt', { attemptId });
}

export function savePyqCsvTemplate(): Promise<string> {
  return invoke<string>('save_pyq_csv_template');
}

export function previewPyqImport(csv: string): Promise<ImportPreview> {
  return invoke<ImportPreview>('preview_pyq_import', { csv });
}

export function applyPyqImport(csv: string): Promise<ImportResult> {
  return invoke<ImportResult>('apply_pyq_import', { csv });
}
