import { invoke } from '@tauri-apps/api/core';

export type LegacyDateNoticeState = 'none' | 'pending' | 'dismissed' | 'transferred';

export type ExamProfile = {
  examId: number;
  code: string;
  name: string;
  shortName: string;
  authority: string;
  officialUrl: string;
  displayOrder: number;
  isBuiltin: boolean;
  isActive: boolean;
  examDate: string | null;
  legacyDateNoticeState: LegacyDateNoticeState;
  legacyExamDate: string | null;
  createdAt: string;
  updatedAt: string;
};

export function getExamProfiles(): Promise<ExamProfile[]> {
  return invoke<ExamProfile[]>('get_exam_profiles');
}

export function updateExamDate(examId: number, examDate: string | null): Promise<ExamProfile> {
  return invoke<ExamProfile>('update_exam_date', { examId, examDate });
}

export function setExamActive(examId: number, isActive: boolean): Promise<ExamProfile> {
  return invoke<ExamProfile>('set_exam_active', { examId, isActive });
}

export function confirmLegacyExamDate(examId: number): Promise<ExamProfile> {
  return invoke<ExamProfile>('confirm_legacy_exam_date', { examId });
}

export function dismissLegacyExamDateNotice(examId: number): Promise<ExamProfile> {
  return invoke<ExamProfile>('dismiss_legacy_exam_date_notice', { examId });
}
