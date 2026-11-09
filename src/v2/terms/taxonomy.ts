import type { TermKind, TermSubjectId } from './types';

export type TermSubjectDefinition = {
  id: TermSubjectId;
  label: string;
};

export const TERM_SUBJECTS: TermSubjectDefinition[] = [
  { id: 'units-measurement', label: 'Units, Dimensions & Measurement' },
  { id: 'foundations', label: 'Foundations & Notation' },
  { id: 'mathematical-physics', label: 'Mathematical Physics' },
  { id: 'classical-mechanics', label: 'Classical Mechanics' },
  { id: 'electromagnetism', label: 'Electromagnetism' },
  { id: 'quantum-mechanics', label: 'Quantum Mechanics' },
  { id: 'thermodynamics-statistical', label: 'Thermodynamics & Statistical Physics' },
  { id: 'atomic-molecular', label: 'Atomic & Molecular Physics' },
  { id: 'nuclear-particle', label: 'Nuclear & Particle Physics' },
  { id: 'condensed-matter', label: 'Condensed Matter Physics' },
  { id: 'electronics', label: 'Electronics' },
  { id: 'experimental-methods', label: 'Experimental Methods & Data Analysis' },
  { id: 'optics', label: 'Optics' },
  { id: 'waves-oscillations', label: 'Waves & Oscillations' },
  { id: 'special-relativity', label: 'Special Relativity' },
];

export const TERM_SUBJECT_LABELS = Object.fromEntries(
  TERM_SUBJECTS.map((subject) => [subject.id, subject.label]),
) as Record<TermSubjectId, string>;

export const TERM_SUBJECT_ORDER = new Map(
  TERM_SUBJECTS.map((subject, index) => [subject.id, index]),
);

export const TERM_KINDS: Array<{ id: TermKind; label: string }> = [
  { id: 'base-unit', label: 'Base unit' },
  { id: 'derived-unit', label: 'Derived unit' },
  { id: 'quantity', label: 'Physical quantity' },
  { id: 'concept', label: 'Concept' },
  { id: 'symbol', label: 'Symbol' },
  { id: 'notation', label: 'Notation' },
];

export const TERM_KIND_LABELS = Object.fromEntries(
  TERM_KINDS.map((kind) => [kind.id, kind.label]),
) as Record<TermKind, string>;

export const TERM_KIND_ORDER = new Map(TERM_KINDS.map((kind, index) => [kind.id, index]));

export function termSubjectLabel(subject: TermSubjectId): string {
  return TERM_SUBJECT_LABELS[subject];
}

export function termKindLabel(kind: TermKind): string {
  return TERM_KIND_LABELS[kind];
}
