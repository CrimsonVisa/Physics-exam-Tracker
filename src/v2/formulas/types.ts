export type ExamCode = 'csir-net-physics' | 'gate-physics' | 'tifr-gs-physics' | 'jest-physics';

export type FormulaSubjectId =
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
  | 'special-relativity'
  | 'foundations';

export type FormulaExactness = 'definition' | 'exact' | 'approximate' | 'special-case';

export type FormulaFramework =
  'general' | 'classical' | 'quantum' | 'relativistic' | 'statistical' | 'thermodynamic';

export type SyllabusRelevance = 'explicit' | 'implicit' | 'uncertain';

export type FormulaSymbol = {
  symbol: string;
  meaning: string;
  unit?: string;
  dimensions?: string;
};

export type FormulaSyllabusNodeRef = {
  code: string;
  title: string;
};

export type FormulaSyllabusLink = {
  examCode: ExamCode;
  nodes: FormulaSyllabusNodeRef[];
  relevance: SyllabusRelevance;
  note?: string;
};

export type FormulaSourceLocatorCheck = 'chapter-checked' | 'unverified';

export type FormulaSource = {
  kind: 'textbook' | 'reference';
  citation: string;
  locator?: string;
  /**
   * Chapter-level check only. `chapter-checked` means the cited chapter or
   * section was inspected, not that the exact formula content was verified.
   */
  locatorCheck?: FormulaSourceLocatorCheck;
  url?: string;
};

export type FormulaEntry = {
  id: string;
  title: string;
  latex: string;
  plainText: string;
  subject: FormulaSubjectId;
  topic: string;
  subtopic?: string;
  summary: string;
  symbols: FormulaSymbol[];
  conditions: string[];
  exactness: FormulaExactness;
  framework: FormulaFramework;
  syllabusLinks: FormulaSyllabusLink[];
  sources: FormulaSource[];
  relatedIds?: string[];
  specialCases?: string[];
  conventions?: string[];
  keywords?: string[];
  conceptSlugs?: string[];
};

export type FormulaLibrary = {
  version: string;
  lastReviewed: string;
  entries: FormulaEntry[];
};
