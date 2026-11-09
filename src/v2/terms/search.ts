import { EXAM_META } from '../formulas/taxonomy';
import { normalizeSearchText } from '../formulas/search';
import type { ExamCode, SyllabusRelevance } from '../formulas/types';
import { TERM_SUBJECTS, TERM_SUBJECT_LABELS, TERM_SUBJECT_ORDER } from './taxonomy';
import type { TermEntry, TermKind, TermSubjectId } from './types';

export type TermSearchFilters = {
  examCode?: ExamCode | 'all';
  subject?: TermSubjectId | 'all';
  topic?: string | 'all';
  kind?: TermKind | 'all';
  nodeCode?: string | null;
  formulaId?: string | null;
};

export type TermSubjectGroup = {
  subject: TermSubjectId;
  label: string;
  topics: Array<{
    topic: string;
    entries: TermEntry[];
  }>;
};

function termSearchText(entry: TermEntry): string {
  return normalizeSearchText(
    [
      entry.term,
      ...(entry.aliases ?? []),
      entry.symbol ?? '',
      entry.symbolPlainText ?? '',
      entry.siUnit ?? '',
      entry.siUnitLatex ?? '',
      entry.dimensions ?? '',
      entry.dimensionFormula ?? '',
      entry.definition,
      TERM_SUBJECT_LABELS[entry.subject],
      entry.topic,
      entry.subtopic ?? '',
      ...(entry.notationVariants ?? []),
      ...(entry.conditions ?? []),
      ...(entry.keywords ?? []),
    ].join(' '),
  );
}

function termSymbolKeywordText(entry: TermEntry): string {
  return normalizeSearchText(
    [
      ...(entry.keywords ?? []),
      entry.symbol ?? '',
      entry.symbolPlainText ?? '',
      entry.siUnit ?? '',
      entry.siUnitLatex ?? '',
      entry.dimensions ?? '',
      entry.dimensionFormula ?? '',
      ...(entry.notationVariants ?? []),
      ...(entry.aliases ?? []),
    ].join(' '),
  );
}

function scoreEntry(entry: TermEntry, query: string, text: string): number {
  const names = [entry.term, ...(entry.aliases ?? [])].map((name) =>
    normalizeSearchText(name).trim(),
  );
  if (names.includes(query)) {
    return 1000;
  }
  if (names.some((name) => name.startsWith(query))) {
    return 800;
  }
  if (names.some((name) => name.includes(query))) {
    return 600;
  }
  if (termSymbolKeywordText(entry).includes(query)) {
    return 400;
  }
  if (text.includes(query)) {
    return 200;
  }
  return 0;
}

function stableTermOrder(left: TermEntry, right: TermEntry): number {
  const subjectDifference =
    (TERM_SUBJECT_ORDER.get(left.subject) ?? 99) - (TERM_SUBJECT_ORDER.get(right.subject) ?? 99);
  if (subjectDifference !== 0) {
    return subjectDifference;
  }
  const topicDifference = left.topic.localeCompare(right.topic);
  if (topicDifference !== 0) {
    return topicDifference;
  }
  const termDifference = left.term.localeCompare(right.term);
  if (termDifference !== 0) {
    return termDifference;
  }
  return left.id.localeCompare(right.id);
}

export function termMatchesFilters(entry: TermEntry, filters: TermSearchFilters = {}): boolean {
  const examCode = filters.examCode ?? 'all';
  const subject = filters.subject ?? 'all';
  const topic = filters.topic ?? 'all';
  const kind = filters.kind ?? 'all';
  const nodeCode = filters.nodeCode ?? null;
  const formulaId = filters.formulaId ?? null;

  if (
    examCode !== 'all' &&
    !entry.syllabusLinks.some((syllabusLink) => syllabusLink.examCode === examCode)
  ) {
    return false;
  }
  if (subject !== 'all' && entry.subject !== subject) {
    return false;
  }
  if (topic !== 'all' && entry.topic !== topic) {
    return false;
  }
  if (kind !== 'all' && entry.kind !== kind) {
    return false;
  }
  if (
    nodeCode !== null &&
    !entry.syllabusLinks.some((syllabusLink) =>
      syllabusLink.nodes.some((node) => node.code === nodeCode),
    )
  ) {
    return false;
  }
  if (formulaId !== null && !(entry.relatedFormulaIds ?? []).includes(formulaId)) {
    return false;
  }
  return true;
}

export function searchTerms(
  entries: TermEntry[],
  query: string,
  filters: TermSearchFilters = {},
): TermEntry[] {
  const normalizedQuery = normalizeSearchText(query.trim());
  const candidates = entries.filter((entry) => termMatchesFilters(entry, filters));

  if (!normalizedQuery) {
    return [...candidates].sort(stableTermOrder);
  }

  const scored = candidates
    .map((entry) => ({
      entry,
      score: scoreEntry(entry, normalizedQuery, termSearchText(entry)),
    }))
    .filter((result) => result.score > 0);

  scored.sort((left, right) => {
    if (right.score !== left.score) {
      return right.score - left.score;
    }
    return stableTermOrder(left.entry, right.entry);
  });

  return scored.map((result) => result.entry);
}

export function termExamCodes(entry: TermEntry): ExamCode[] {
  const codes = new Set<ExamCode>();
  for (const syllabusLink of entry.syllabusLinks) {
    codes.add(syllabusLink.examCode);
  }
  return [...codes].sort((left, right) => EXAM_META[left].order - EXAM_META[right].order);
}

export function termExamRelevance(entry: TermEntry, examCode: ExamCode): SyllabusRelevance | null {
  const links = entry.syllabusLinks.filter((syllabusLink) => syllabusLink.examCode === examCode);
  if (links.some((link) => link.relevance === 'explicit')) {
    return 'explicit';
  }
  if (links.some((link) => link.relevance === 'implicit')) {
    return 'implicit';
  }
  if (links.length > 0) {
    return 'uncertain';
  }
  return null;
}

export function topicOptions(entries: TermEntry[], filters: TermSearchFilters = {}): string[] {
  return [
    ...new Set(
      searchTerms(entries, '', {
        examCode: filters.examCode ?? 'all',
        subject: filters.subject ?? 'all',
        topic: 'all',
        kind: filters.kind ?? 'all',
        nodeCode: filters.nodeCode ?? null,
        formulaId: filters.formulaId ?? null,
      }).map((entry) => entry.topic),
    ),
  ].sort((left, right) => left.localeCompare(right));
}

export function subjectCounts(
  entries: TermEntry[],
  filters: TermSearchFilters = {},
  query = '',
): Record<TermSubjectId, number> {
  const counts = Object.fromEntries(TERM_SUBJECTS.map((subject) => [subject.id, 0])) as Record<
    TermSubjectId,
    number
  >;

  const filtered = searchTerms(entries, query, {
    examCode: filters.examCode ?? 'all',
    subject: 'all',
    topic: filters.topic ?? 'all',
    kind: filters.kind ?? 'all',
    nodeCode: filters.nodeCode ?? null,
    formulaId: filters.formulaId ?? null,
  });

  for (const entry of filtered) {
    counts[entry.subject] += 1;
  }
  return counts;
}

export function groupTerms(entries: TermEntry[]): TermSubjectGroup[] {
  return TERM_SUBJECTS.flatMap((subject) => {
    const subjectEntries = entries.filter((entry) => entry.subject === subject.id);
    if (subjectEntries.length === 0) {
      return [];
    }
    const topics = [...new Set(subjectEntries.map((entry) => entry.topic))].sort((left, right) =>
      left.localeCompare(right),
    );
    return [
      {
        subject: subject.id,
        label: subject.label,
        topics: topics.map((topic) => ({
          topic,
          entries: subjectEntries.filter((entry) => entry.topic === topic).sort(stableTermOrder),
        })),
      },
    ];
  });
}
