import { EXAM_META, FORMULA_SUBJECTS, FORMULA_SUBJECT_ORDER, SUBJECT_LABELS } from './taxonomy';
import type { ExamCode, FormulaEntry, FormulaSubjectId, SyllabusRelevance } from './types';

export type FormulaSearchFilters = {
  examCode?: ExamCode | 'all';
  subject?: FormulaSubjectId | 'all';
  topic?: string | 'all';
  nodeCode?: string | null;
};

export type FormulaSubjectGroup = {
  subject: FormulaSubjectId;
  label: string;
  topics: Array<{
    topic: string;
    entries: FormulaEntry[];
  }>;
};

const LATEX_SYMBOL_ALIASES: Record<string, string> = {
  '\\alpha': 'alpha α',
  '\\beta': 'beta β',
  '\\gamma': 'gamma γ',
  '\\delta': 'delta δ',
  '\\Delta': 'Delta Δ',
  '\\epsilon': 'epsilon ε',
  '\\theta': 'theta θ',
  '\\Theta': 'Theta Θ',
  '\\lambda': 'lambda λ',
  '\\mu': 'mu μ',
  '\\nu': 'nu ν',
  '\\pi': 'pi π',
  '\\rho': 'rho ρ',
  '\\sigma': 'sigma σ',
  '\\tau': 'tau τ',
  '\\phi': 'phi φ',
  '\\Psi': 'Psi Ψ',
  '\\psi': 'psi ψ',
  '\\omega': 'omega ω',
  '\\Omega': 'Omega Ω',
  '\\hbar': 'hbar ħ',
  '\\nabla': 'nabla ∇',
  '\\partial': 'partial ∂',
};

function expandLatexSymbols(text: string): string {
  let expanded = text;
  for (const [command, alias] of Object.entries(LATEX_SYMBOL_ALIASES)) {
    expanded = expanded.split(command).join(` ${alias} `);
  }
  return expanded.replace(/\\/g, ' ');
}

export function normalizeSearchText(text: string): string {
  return expandLatexSymbols(text).normalize('NFKD').replace(/\p{M}/gu, '').toLowerCase();
}

function entrySearchText(entry: FormulaEntry): string {
  return normalizeSearchText(
    [
      entry.title,
      entry.latex,
      entry.plainText,
      entry.summary,
      SUBJECT_LABELS[entry.subject],
      entry.topic,
      entry.subtopic ?? '',
      ...(entry.keywords ?? []),
      ...entry.symbols.flatMap((symbol) => [
        symbol.symbol,
        symbol.meaning,
        symbol.unit ?? '',
        symbol.dimensions ?? '',
      ]),
      ...(entry.conventions ?? []),
      ...(entry.specialCases ?? []),
    ].join(' '),
  );
}

function entrySymbolKeywordText(entry: FormulaEntry): string {
  return normalizeSearchText(
    [
      ...(entry.keywords ?? []),
      ...entry.symbols.flatMap((symbol) => [symbol.symbol, symbol.meaning]),
    ].join(' '),
  );
}

function scoreEntry(entry: FormulaEntry, query: string, text: string): number {
  const title = normalizeSearchText(entry.title);
  if (title === query) {
    return 1000;
  }
  if (title.startsWith(query)) {
    return 800;
  }
  if (title.includes(query)) {
    return 600;
  }
  if (entrySymbolKeywordText(entry).includes(query)) {
    return 400;
  }
  if (text.includes(query)) {
    return 200;
  }
  return 0;
}

function stableEntryOrder(left: FormulaEntry, right: FormulaEntry): number {
  const subjectDifference =
    (FORMULA_SUBJECT_ORDER.get(left.subject) ?? 99) -
    (FORMULA_SUBJECT_ORDER.get(right.subject) ?? 99);
  if (subjectDifference !== 0) {
    return subjectDifference;
  }
  const topicDifference = left.topic.localeCompare(right.topic);
  if (topicDifference !== 0) {
    return topicDifference;
  }
  const titleDifference = left.title.localeCompare(right.title);
  if (titleDifference !== 0) {
    return titleDifference;
  }
  return left.id.localeCompare(right.id);
}

export function entryMatchesFilters(
  entry: FormulaEntry,
  filters: FormulaSearchFilters = {},
): boolean {
  const examCode = filters.examCode ?? 'all';
  const subject = filters.subject ?? 'all';
  const topic = filters.topic ?? 'all';
  const nodeCode = filters.nodeCode ?? null;

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
  if (
    nodeCode !== null &&
    !entry.syllabusLinks.some((syllabusLink) =>
      syllabusLink.nodes.some((node) => node.code === nodeCode),
    )
  ) {
    return false;
  }
  return true;
}

export function searchFormulas(
  entries: FormulaEntry[],
  query: string,
  filters: FormulaSearchFilters = {},
): FormulaEntry[] {
  const normalizedQuery = normalizeSearchText(query.trim());
  const candidates = entries.filter((entry) => entryMatchesFilters(entry, filters));

  if (!normalizedQuery) {
    return [...candidates].sort(stableEntryOrder);
  }

  const scored = candidates
    .map((entry) => ({
      entry,
      score: scoreEntry(entry, normalizedQuery, entrySearchText(entry)),
    }))
    .filter((result) => result.score > 0);

  scored.sort((left, right) => {
    if (right.score !== left.score) {
      return right.score - left.score;
    }
    return stableEntryOrder(left.entry, right.entry);
  });

  return scored.map((result) => result.entry);
}

export function entryExamCodes(entry: FormulaEntry): ExamCode[] {
  const codes = new Set<ExamCode>();
  for (const syllabusLink of entry.syllabusLinks) {
    codes.add(syllabusLink.examCode);
  }
  return [...codes].sort((left, right) => EXAM_META[left].order - EXAM_META[right].order);
}

export function entryExamRelevance(
  entry: FormulaEntry,
  examCode: ExamCode,
): SyllabusRelevance | null {
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

export function topicOptions(
  entries: FormulaEntry[],
  filters: FormulaSearchFilters = {},
): string[] {
  return [
    ...new Set(
      searchFormulas(entries, '', {
        examCode: filters.examCode ?? 'all',
        subject: filters.subject ?? 'all',
        topic: 'all',
        nodeCode: filters.nodeCode ?? null,
      }).map((entry) => entry.topic),
    ),
  ].sort((left, right) => left.localeCompare(right));
}

export function groupFormulas(entries: FormulaEntry[]): FormulaSubjectGroup[] {
  return FORMULA_SUBJECTS.flatMap((subject) => {
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
          entries: subjectEntries.filter((entry) => entry.topic === topic).sort(stableEntryOrder),
        })),
      },
    ];
  });
}
