export type V2View =
  | { kind: 'dashboard' }
  | { kind: 'exam'; examCode: string }
  | { kind: 'concepts'; examCode: string | null; conceptSlug?: string | null }
  | { kind: 'syllabus'; examCode: string | null; focusNodeId?: number | null }
  | { kind: 'practice'; examCode: string | null; questionId?: number | null }
  | { kind: 'planner' }
  | { kind: 'revision' }
  | { kind: 'analytics' }
  | { kind: 'resources'; query?: string | null }
  | { kind: 'backup' }
  | { kind: 'settings' };

export const DASHBOARD_VIEW: V2View = { kind: 'dashboard' };
