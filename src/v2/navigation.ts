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
  | {
      kind: 'terms';
      examCode?: string | null;
      termId?: string | null;
      subject?: string | null;
      topic?: string | null;
      kindFilter?: string | null;
      nodeCode?: string | null;
      nodeTitle?: string | null;
      formulaId?: string | null;
      formulaTitle?: string | null;
      query?: string | null;
    }
  | {
      kind: 'formulas';
      examCode?: string | null;
      formulaId?: string | null;
      subject?: string | null;
      topic?: string | null;
      nodeCode?: string | null;
      nodeTitle?: string | null;
      query?: string | null;
    }
  | {
      kind: 'syllabus';
      examCode: string | null;
      focusNodeId?: number | null;
      focusNodeCode?: string | null;
    }
  | { kind: 'practice'; examCode: string | null; questionId?: number | null }
  | { kind: 'planner' }
  | { kind: 'revision' }
  | { kind: 'analytics' }
  | { kind: 'resources'; query?: string | null }
  | { kind: 'backup' }
  | { kind: 'settings' };

export const DASHBOARD_VIEW: V2View = { kind: 'dashboard' };
