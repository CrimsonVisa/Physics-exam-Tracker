import type {
  ExamCode,
  FormulaExactness,
  FormulaFramework,
  FormulaSubjectId,
  SyllabusRelevance,
} from './types';

export type FormulaSubjectDefinition = {
  id: FormulaSubjectId;
  label: string;
};

export const FORMULA_SUBJECTS: FormulaSubjectDefinition[] = [
  { id: 'mathematical-physics', label: 'Mathematical Physics' },
  { id: 'classical-mechanics', label: 'Classical Mechanics' },
  { id: 'electromagnetism', label: 'Electromagnetic Theory' },
  { id: 'quantum-mechanics', label: 'Quantum Mechanics' },
  { id: 'thermodynamics-statistical', label: 'Thermodynamics and Statistical Physics' },
  { id: 'atomic-molecular', label: 'Atomic and Molecular Physics' },
  { id: 'nuclear-particle', label: 'Nuclear and Particle Physics' },
  { id: 'condensed-matter', label: 'Condensed Matter Physics' },
  { id: 'electronics', label: 'Electronics' },
  { id: 'experimental-methods', label: 'Experimental Methods' },
  { id: 'optics', label: 'Optics' },
  { id: 'waves-oscillations', label: 'Waves and Oscillations' },
  { id: 'special-relativity', label: 'Special Relativity' },
  { id: 'foundations', label: 'Foundations' },
];

export const SUBJECT_LABELS = Object.fromEntries(
  FORMULA_SUBJECTS.map((subject) => [subject.id, subject.label]),
) as Record<FormulaSubjectId, string>;

export const FORMULA_SUBJECT_ORDER = new Map(
  FORMULA_SUBJECTS.map((subject, index) => [subject.id, index]),
);

export const EXAM_META: Record<ExamCode, { name: string; shortName: string; order: number }> = {
  'csir-net-physics': {
    name: 'CSIR-UGC NET Physical Sciences',
    shortName: 'CSIR-NET',
    order: 1,
  },
  'gate-physics': { name: 'GATE Physics (PH)', shortName: 'GATE PH', order: 2 },
  'tifr-gs-physics': {
    name: 'TIFR Graduate School Physics',
    shortName: 'TIFR GS',
    order: 3,
  },
  'jest-physics': { name: 'JEST Physics', shortName: 'JEST', order: 4 },
};

export const EXAM_CODES = Object.keys(EXAM_META) as ExamCode[];

export const EXACTNESS_LABELS: Record<FormulaExactness, string> = {
  definition: 'Definition',
  exact: 'Exact',
  approximate: 'Approximate',
  'special-case': 'Special case',
};

export const FRAMEWORK_LABELS: Record<FormulaFramework, string> = {
  general: 'General',
  classical: 'Classical',
  quantum: 'Quantum',
  relativistic: 'Relativistic',
  statistical: 'Statistical',
  thermodynamic: 'Thermodynamic',
};

export const RELEVANCE_LABELS: Record<SyllabusRelevance, string> = {
  explicit: 'Syllabus topic',
  implicit: 'Related syllabus area',
  uncertain: 'Uncertain relevance',
};

export function subjectLabel(subject: FormulaSubjectId): string {
  return SUBJECT_LABELS[subject];
}

export function examShortName(examCode: ExamCode): string {
  return EXAM_META[examCode].shortName;
}
