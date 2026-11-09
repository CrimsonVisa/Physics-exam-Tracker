import type { ExamCode, FormulaSource, SyllabusRelevance } from '../formulas/types';

export type TermSubjectId =
  | 'units-measurement'
  | 'foundations'
  | 'mathematical-physics'
  | 'classical-mechanics'
  | 'electromagnetism'
  | 'quantum-mechanics'
  | 'thermodynamics-statistical'
  | 'atomic-molecular'
  | 'nuclear-particle'
  | 'condensed-matter'
  | 'electronics'
  | 'experimental-methods'
  | 'optics'
  | 'waves-oscillations'
  | 'special-relativity';

export type TermKind =
  'base-unit' | 'derived-unit' | 'quantity' | 'concept' | 'symbol' | 'notation';

export type TermSyllabusNodeRef = {
  code: string;
  title: string;
};

export type TermSyllabusLink = {
  examCode: ExamCode;
  relevance: SyllabusRelevance;
  nodes: TermSyllabusNodeRef[];
  note?: string;
};

export type TermSource = FormulaSource;

export type TermEntry = {
  id: string;
  term: string;
  aliases?: string[];
  kind: TermKind;
  subject: TermSubjectId;
  topic: string;
  subtopic?: string;
  definition: string;
  symbol?: string;
  symbolPlainText?: string;
  quantity?: string;
  siUnit?: string;
  siUnitLatex?: string;
  dimensions?: string;
  dimensionFormula?: string;
  notationVariants?: string[];
  conditions?: string[];
  relatedTermIds?: string[];
  relatedFormulaIds?: string[];
  syllabusLinks: TermSyllabusLink[];
  sources: TermSource[];
  keywords?: string[];
};

export type TermLibrary = {
  version: string;
  /** Library compilation/review date, not a per-source verification date. */
  lastReviewed: string;
  /** Explains how per-source locatorCheck status relates to lastReviewed. */
  sourceVerificationNote: string;
  entries: TermEntry[];
};
