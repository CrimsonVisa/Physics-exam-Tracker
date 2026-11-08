export type V2View =
  | { kind: 'dashboard' }
  | { kind: 'exam'; examCode: string }
  | { kind: 'concepts'; examCode: string | null; conceptSlug?: string | null }
  | {
      kind: 'flashcards';
      examCode?: string | null;
      nodeId?: number | null;
      nodeTitle?: string | null;
      conceptSlug?: string | null;
      conceptTitle?: string | null;
      flashcardId?: number | null;
      query?: string | null;
    }
  | { kind: 'calculator' }
  | { kind: 'syllabus'; examCode: string | null; focusNodeId?: number | null }
  | { kind: 'practice'; examCode: string | null; questionId?: number | null }
  | { kind: 'planner' }
  | { kind: 'revision' }
  | { kind: 'analytics' }
  | { kind: 'resources'; query?: string | null }
  | { kind: 'backup' }
  | { kind: 'settings' };

export const DASHBOARD_VIEW: V2View = { kind: 'dashboard' };
