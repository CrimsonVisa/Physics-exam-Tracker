import type { V2ExamSummary } from './api';
import type { V2View } from './navigation';

/**
 * Reconcile the active view with a fresh database status without discarding
 * valid workspaces. Notes is intentionally preserved alongside the other
 * non-exam-scoped views so a late status response cannot close the page or
 * its editor.
 */
export function retainViewAfterStatus(current: V2View, exams: V2ExamSummary[]): V2View {
  if (current.kind === 'exam') {
    return exams.some((exam) => exam.code === current.examCode) ? current : { kind: 'dashboard' };
  }
  if (current.kind === 'concepts' || current.kind === 'syllabus' || current.kind === 'practice') {
    return current.examCode === null || exams.some((exam) => exam.code === current.examCode)
      ? current
      : { kind: current.kind, examCode: null };
  }
  if (current.kind === 'formulas') {
    if (
      current.examCode === null ||
      current.examCode === undefined ||
      exams.some((exam) => exam.code === current.examCode)
    ) {
      return current;
    }
    return {
      ...current,
      examCode: null,
      nodeCode: null,
      nodeTitle: null,
    };
  }
  if (current.kind === 'terms') {
    if (
      current.examCode === null ||
      current.examCode === undefined ||
      exams.some((exam) => exam.code === current.examCode)
    ) {
      return current;
    }
    return {
      ...current,
      examCode: null,
      nodeCode: null,
      nodeTitle: null,
    };
  }
  if (
    current.kind === 'planner' ||
    current.kind === 'revision' ||
    current.kind === 'analytics' ||
    current.kind === 'notes' ||
    current.kind === 'resources' ||
    current.kind === 'backup' ||
    current.kind === 'settings' ||
    current.kind === 'calculator'
  ) {
    return current;
  }
  return { kind: 'dashboard' };
}
