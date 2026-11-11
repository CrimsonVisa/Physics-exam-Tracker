import katex from 'katex';
import {
  EXAM_CODES,
  EXACTNESS_LABELS,
  FORMULA_SUBJECTS,
  FRAMEWORK_LABELS,
  RELEVANCE_LABELS,
} from './taxonomy';
import type {
  ExamCode,
  FormulaEntry,
  FormulaExactness,
  FormulaFramework,
  FormulaSubjectId,
  SyllabusRelevance,
} from './types';

export type SyllabusSeedNode = {
  code: string;
  title: string;
  kind: string;
  isArchived?: boolean;
};

export type SyllabusSeedExam = {
  examCode: string;
  nodes: SyllabusSeedNode[];
};

export type SyllabusIndex = Map<string, Map<string, SyllabusSeedNode>>;

export type FormulaValidationIssue = {
  path: string;
  message: string;
};

const ID_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const UNSAFE_LATEX_PATTERN = /\\href|\\url|\\html|\\includegraphics|\\class|\\style/i;
const SUBJECT_IDS = new Set<FormulaSubjectId>(FORMULA_SUBJECTS.map((subject) => subject.id));
const EXACTNESS_VALUES = new Set<FormulaExactness>(
  Object.keys(EXACTNESS_LABELS) as FormulaExactness[],
);
const FRAMEWORK_VALUES = new Set<FormulaFramework>(
  Object.keys(FRAMEWORK_LABELS) as FormulaFramework[],
);
const RELEVANCE_VALUES = new Set<SyllabusRelevance>(
  Object.keys(RELEVANCE_LABELS) as SyllabusRelevance[],
);
const EXAM_CODES_SET = new Set<ExamCode>(EXAM_CODES);
const SOURCE_LOCATOR_CHECKS = new Set(['chapter-checked', 'unverified']);
const JEST_RESTRICTED_SUBJECTS = new Set<FormulaSubjectId>([
  'atomic-molecular',
  'nuclear-particle',
  'condensed-matter',
]);

export function buildSyllabusIndex(seed: SyllabusSeedExam[]): SyllabusIndex {
  const index: SyllabusIndex = new Map();
  for (const exam of seed) {
    const nodes = new Map<string, SyllabusSeedNode>();
    for (const node of exam.nodes) {
      nodes.set(node.code, node);
    }
    index.set(exam.examCode, nodes);
  }
  return index;
}

function requiredString(
  entry: FormulaEntry,
  field: string,
  value: string | undefined,
  issues: FormulaValidationIssue[],
): void {
  if (!value || value.trim().length === 0) {
    issues.push({
      path: `${entry.id}.${field}`,
      message: `${field} must be a non-empty string.`,
    });
  }
}

function validateLatex(entry: FormulaEntry, issues: FormulaValidationIssue[]): void {
  if (!entry.latex || entry.latex.trim().length === 0) {
    issues.push({ path: `${entry.id}.latex`, message: 'latex must be non-empty.' });
    return;
  }
  if (UNSAFE_LATEX_PATTERN.test(entry.latex)) {
    issues.push({
      path: `${entry.id}.latex`,
      message: 'latex contains a trust-gated or unsafe command.',
    });
  }
  try {
    katex.renderToString(entry.latex, {
      displayMode: true,
      throwOnError: true,
      strict: 'error',
      trust: false,
      output: 'htmlAndMathml',
    });
  } catch (error) {
    issues.push({
      path: `${entry.id}.latex`,
      message: `latex failed to render: ${error instanceof Error ? error.message : String(error)}`,
    });
  }
}

export function validateFormulaLibrary(
  entries: FormulaEntry[],
  syllabusIndex: SyllabusIndex,
): FormulaValidationIssue[] {
  const issues: FormulaValidationIssue[] = [];
  const ids = new Set<string>();

  for (const entry of entries) {
    if (!ID_PATTERN.test(entry.id)) {
      issues.push({
        path: `${entry.id || '(empty)'}.id`,
        message: 'id must be lowercase kebab-case using letters and digits.',
      });
    }
    if (ids.has(entry.id)) {
      issues.push({ path: `${entry.id}.id`, message: 'id must be unique.' });
    }
    ids.add(entry.id);

    requiredString(entry, 'title', entry.title, issues);
    requiredString(entry, 'plainText', entry.plainText, issues);
    requiredString(entry, 'topic', entry.topic, issues);
    requiredString(entry, 'summary', entry.summary, issues);

    if (!SUBJECT_IDS.has(entry.subject)) {
      issues.push({ path: `${entry.id}.subject`, message: 'subject is not recognized.' });
    }
    if (!EXACTNESS_VALUES.has(entry.exactness)) {
      issues.push({
        path: `${entry.id}.exactness`,
        message: 'exactness is not recognized.',
      });
    }
    if (!FRAMEWORK_VALUES.has(entry.framework)) {
      issues.push({
        path: `${entry.id}.framework`,
        message: 'framework is not recognized.',
      });
    }

    validateLatex(entry, issues);

    if (entry.symbols.length === 0) {
      issues.push({ path: `${entry.id}.symbols`, message: 'at least one symbol is required.' });
    }
    entry.symbols.forEach((symbol, index) => {
      if (!symbol.symbol.trim() || !symbol.meaning.trim()) {
        issues.push({
          path: `${entry.id}.symbols[${index}]`,
          message: 'symbol and meaning must be non-empty.',
        });
      }
    });

    if (entry.conditions.length === 0) {
      issues.push({
        path: `${entry.id}.conditions`,
        message: 'at least one condition or assumption is required.',
      });
    }
    entry.conditions.forEach((condition, index) => {
      if (!condition.trim()) {
        issues.push({
          path: `${entry.id}.conditions[${index}]`,
          message: 'condition must be non-empty.',
        });
      }
    });

    if (entry.sources.length === 0) {
      issues.push({ path: `${entry.id}.sources`, message: 'at least one source is required.' });
    }
    entry.sources.forEach((source, index) => {
      if (!source.citation.trim()) {
        issues.push({
          path: `${entry.id}.sources[${index}]`,
          message: 'source citation must be non-empty.',
        });
      }
      if (source.kind !== 'textbook' && source.kind !== 'reference') {
        issues.push({
          path: `${entry.id}.sources[${index}]`,
          message: 'source kind must be textbook or reference.',
        });
      }
      if (source.locator) {
        if (!source.locatorCheck) {
          issues.push({
            path: `${entry.id}.sources[${index}].locatorCheck`,
            message: 'locatorCheck is required when a locator is present.',
          });
        } else if (!SOURCE_LOCATOR_CHECKS.has(source.locatorCheck)) {
          issues.push({
            path: `${entry.id}.sources[${index}].locatorCheck`,
            message: 'locatorCheck must be chapter-checked or unverified.',
          });
        }
      } else if (source.locatorCheck !== undefined) {
        issues.push({
          path: `${entry.id}.sources[${index}].locatorCheck`,
          message: 'locatorCheck must not be present when no locator is provided.',
        });
      }
    });

    if (entry.syllabusLinks.length === 0) {
      issues.push({
        path: `${entry.id}.syllabusLinks`,
        message: 'at least one syllabus link is required.',
      });
    }

    const linkedNodes = new Set<string>();
    for (const [linkIndex, syllabusLink] of entry.syllabusLinks.entries()) {
      const linkPath = `${entry.id}.syllabusLinks[${linkIndex}]`;
      if (!EXAM_CODES_SET.has(syllabusLink.examCode)) {
        issues.push({
          path: linkPath,
          message: 'examCode is not recognized.',
        });
      }
      if (syllabusLink.nodes.length !== 1) {
        issues.push({
          path: linkPath,
          message: 'each syllabus link must reference exactly one node.',
        });
      }
      if (!RELEVANCE_VALUES.has(syllabusLink.relevance)) {
        issues.push({
          path: linkPath,
          message: 'relevance is not recognized.',
        });
      }
      if (syllabusLink.relevance === 'uncertain' && !syllabusLink.note?.trim()) {
        issues.push({
          path: linkPath,
          message: 'uncertain relevance requires an explanatory note.',
        });
      }
      if (syllabusLink.examCode === 'tifr-gs-physics' && syllabusLink.relevance === 'explicit') {
        issues.push({
          path: linkPath,
          message: 'TIFR links must not be explicit at formula level.',
        });
      }
      if (JEST_RESTRICTED_SUBJECTS.has(entry.subject) && syllabusLink.examCode === 'jest-physics') {
        issues.push({
          path: linkPath,
          message:
            'JEST relevance is not allowed for this subject without explicit syllabus evidence.',
        });
      }

      const examNodes = syllabusIndex.get(syllabusLink.examCode);
      for (const node of syllabusLink.nodes) {
        const nodeKey = `${syllabusLink.examCode}:${node.code}`;
        if (linkedNodes.has(nodeKey)) {
          issues.push({
            path: linkPath,
            message: `duplicate syllabus node link ${node.code} for ${syllabusLink.examCode}.`,
          });
        }
        linkedNodes.add(nodeKey);
        const indexedNode = examNodes?.get(node.code);
        if (!indexedNode) {
          issues.push({
            path: `${linkPath}.nodes`,
            message: `syllabus node ${node.code} does not exist for ${syllabusLink.examCode}.`,
          });
          continue;
        }
        if (indexedNode.title !== node.title) {
          issues.push({
            path: `${linkPath}.nodes`,
            message: `syllabus node ${node.code} title mismatch: expected "${indexedNode.title}", received "${node.title}".`,
          });
        }
        if (indexedNode.isArchived === true) {
          issues.push({
            path: `${linkPath}.nodes`,
            message: `syllabus node ${node.code} is archived.`,
          });
        }
        if (
          syllabusLink.examCode === 'tifr-gs-physics' &&
          (indexedNode.kind !== 'topic' || !indexedNode.title.startsWith('Coverage note:'))
        ) {
          issues.push({
            path: `${linkPath}.nodes`,
            message: 'TIFR links must use one of the eight broad-area topic nodes.',
          });
        }
      }
    }

    const relatedIds = entry.relatedIds ?? [];
    const relatedSet = new Set<string>();
    for (const relatedId of relatedIds) {
      if (relatedId === entry.id) {
        issues.push({
          path: `${entry.id}.relatedIds`,
          message: 'a formula cannot relate to itself.',
        });
      }
      if (relatedSet.has(relatedId)) {
        issues.push({
          path: `${entry.id}.relatedIds`,
          message: `duplicate related formula ${relatedId}.`,
        });
      }
      relatedSet.add(relatedId);
    }

    if (entry.keywords) {
      entry.keywords.forEach((keyword, index) => {
        if (!keyword.trim()) {
          issues.push({
            path: `${entry.id}.keywords[${index}]`,
            message: 'keyword must be non-empty.',
          });
        }
      });
    }
  }

  for (const entry of entries) {
    for (const relatedId of entry.relatedIds ?? []) {
      if (!ids.has(relatedId)) {
        issues.push({
          path: `${entry.id}.relatedIds`,
          message: `related formula ${relatedId} does not exist.`,
        });
      }
    }
  }

  return issues;
}
