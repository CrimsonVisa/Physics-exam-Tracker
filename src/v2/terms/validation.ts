import katex from 'katex';
import { REFERENCES, TEXTBOOKS } from '../formulas/data/sources';
import { EXAM_CODES, RELEVANCE_LABELS } from '../formulas/taxonomy';
import { normalizeSearchText } from '../formulas/search';
import type { ExamCode, SyllabusRelevance } from '../formulas/types';
import type { SyllabusIndex } from '../formulas/validation';
import { EXPLICIT_TERM_LINKS, explicitLinkKey } from './explicitAllowlist';
import { TERM_KINDS, TERM_SUBJECTS } from './taxonomy';
import type { TermEntry, TermKind, TermSubjectId } from './types';

export type TermValidationIssue = {
  path: string;
  message: string;
};

const ID_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const UNSAFE_LATEX_PATTERN = /\\href|\\url|\\html|\\includegraphics|\\class|\\style/i;
const SUBJECT_IDS = new Set<TermSubjectId>(TERM_SUBJECTS.map((subject) => subject.id));
const KIND_IDS = new Set<TermKind>(TERM_KINDS.map((kind) => kind.id));
const EXAM_CODES_SET = new Set<ExamCode>(EXAM_CODES);
const RELEVANCE_VALUES = new Set<SyllabusRelevance>(
  Object.keys(RELEVANCE_LABELS) as SyllabusRelevance[],
);
const SOURCE_LOCATOR_CHECKS = new Set(['chapter-checked', 'unverified']);
const KNOWN_SOURCE_CITATIONS = new Set<string>([
  ...Object.values(TEXTBOOKS),
  ...Object.values(REFERENCES),
]);
const JEST_RESTRICTED_SUBJECTS = new Set<TermSubjectId>([
  'atomic-molecular',
  'nuclear-particle',
  'condensed-matter',
]);

const DIMENSION_SYMBOLS = ['M', 'L', 'T', 'I', 'Θ', 'N', 'J'] as const;
type DimensionSymbol = (typeof DIMENSION_SYMBOLS)[number];
type DimensionMap = Partial<Record<DimensionSymbol, number>>;

const DIMENSION_EXPONENT = /^([+-]?\d+)(?:\/(\d+))?$/;

function parseExponent(value: string): number | null {
  const match = DIMENSION_EXPONENT.exec(value);
  if (!match) {
    return null;
  }
  const numerator = Number(match[1]);
  const denominator = match[2] ? Number(match[2]) : 1;
  if (!Number.isFinite(numerator) || !Number.isFinite(denominator) || denominator === 0) {
    return null;
  }
  return numerator / denominator;
}

function addDimensions(target: DimensionMap, source: DimensionMap, power = 1): void {
  for (const symbol of DIMENSION_SYMBOLS) {
    const value = source[symbol];
    if (!value) {
      continue;
    }
    target[symbol] = (target[symbol] ?? 0) + value * power;
  }
}

function normalizeDimensionMath(value: string): string {
  return value
    .replace(/\\Theta/g, 'Θ')
    .replace(/\\mathrm/g, '')
    .replace(/\\left|\\right/g, '')
    .replace(/\\[,;! ]/g, '')
    .replace(/[{}]/g, '')
    .replace(/\\/g, '')
    .replace(/[·*]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

function parseDimensions(value: string): DimensionMap | null {
  const normalized = normalizeDimensionMath(value);
  if (normalized === '1' || normalized === '') {
    return {};
  }
  const dimensions: DimensionMap = {};
  const tokens = normalized.split(/\s+/);
  for (const token of tokens) {
    const match = /^([MLTIΘNJ])(?:\^([+-]?\d+(?:\/\d+)?))?$/.exec(token);
    if (!match) {
      return null;
    }
    const exponent = match[2] ? parseExponent(match[2]) : 1;
    if (exponent === null) {
      return null;
    }
    addDimensions(dimensions, { [match[1] as DimensionSymbol]: 1 }, exponent);
  }
  return dimensions;
}

function parseDimensionFormula(value: string): {
  symbolic: boolean;
  dimensions: DimensionMap | null;
} {
  const equalsIndex = value.indexOf('=');
  const rightHandSide = equalsIndex >= 0 ? value.slice(equalsIndex + 1) : value;
  const normalized = normalizeDimensionMath(rightHandSide);
  if (normalized === '1') {
    return { symbolic: false, dimensions: {} };
  }
  if (/\^[a-g]/.test(normalized)) {
    return { symbolic: true, dimensions: null };
  }
  const dimensions = parseDimensions(normalized);
  return { symbolic: false, dimensions };
}

function dimensionMapsEqual(left: DimensionMap, right: DimensionMap): boolean {
  return DIMENSION_SYMBOLS.every((symbol) => (left[symbol] ?? 0) === (right[symbol] ?? 0));
}

const BASE_UNIT_DIMENSIONS: Record<string, DimensionMap> = {
  m: { L: 1 },
  kg: { M: 1 },
  g: { M: 1 },
  s: { T: 1 },
  A: { I: 1 },
  K: { Θ: 1 },
  mol: { N: 1 },
  cd: { J: 1 },
};

const DERIVED_UNIT_DIMENSIONS: Record<string, DimensionMap> = {
  N: { M: 1, L: 1, T: -2 },
  J: { M: 1, L: 2, T: -2 },
  W: { M: 1, L: 2, T: -3 },
  Pa: { M: 1, L: -1, T: -2 },
  Hz: { T: -1 },
  C: { T: 1, I: 1 },
  V: { M: 1, L: 2, T: -3, I: -1 },
  Ω: { M: 1, L: 2, T: -3, I: -2 },
  F: { M: -1, L: -2, T: 4, I: 2 },
  H: { M: 1, L: 2, T: -2, I: -2 },
  T: { M: 1, T: -2, I: -1 },
  Wb: { M: 1, L: 2, T: -2, I: -1 },
  rad: {},
  sr: {},
  '1': {},
};

const UNIT_DIMENSIONS: Record<string, DimensionMap> = {
  ...BASE_UNIT_DIMENSIONS,
  ...DERIVED_UNIT_DIMENSIONS,
};

// kg is the only SI base symbol that already contains a prefix. Prefixes are
// applied to gram (g), never to kg.
const NON_PREFIXABLE_UNIT_SYMBOLS = new Set(['kg']);

// Independent expected-symbol catalogue for the approved base and derived-unit
// entries. The entry data carries the display symbol; this map prevents a
// KaTeX-valid but scientifically wrong symbol (for example M for metre).
const SI_UNIT_SYMBOL_BY_ENTRY: Record<string, string> = {
  'units-metre': 'm',
  'units-kilogram': 'kg',
  'units-second': 's',
  'units-ampere': 'A',
  'units-kelvin': 'K',
  'units-mole': 'mol',
  'units-candela': 'cd',
  'units-newton': 'N',
  'units-joule': 'J',
  'units-watt': 'W',
  'units-pascal': 'Pa',
  'units-hertz': 'Hz',
  'units-coulomb': 'C',
  'units-volt': 'V',
  'units-ohm': 'Ω',
  'units-farad': 'F',
  'units-henry': 'H',
  'units-tesla': 'T',
  'units-weber': 'Wb',
};

const SI_PREFIXES = [
  'da',
  'Y',
  'Z',
  'E',
  'P',
  'T',
  'G',
  'M',
  'k',
  'h',
  'd',
  'c',
  'm',
  'µ',
  'μ',
  'n',
  'p',
  'f',
  'a',
  'z',
  'y',
];

function lookupUnitDimensions(symbol: string): DimensionMap | null {
  const exact = UNIT_DIMENSIONS[symbol];
  if (exact) {
    return exact;
  }
  for (const prefix of SI_PREFIXES) {
    if (symbol.startsWith(prefix) && symbol.length > prefix.length) {
      const remainder = symbol.slice(prefix.length);
      if (NON_PREFIXABLE_UNIT_SYMBOLS.has(remainder)) {
        continue;
      }
      const dimensions = UNIT_DIMENSIONS[remainder];
      if (dimensions) {
        return dimensions;
      }
    }
  }
  return null;
}

function parseUnitDimensions(value: string): DimensionMap | null {
  const trimmed = value.trim();
  if (trimmed === '1') {
    return {};
  }
  if (!trimmed) {
    return null;
  }
  const normalized = trimmed.replace(/[·*]/g, ' ').replace(/\s+/g, ' ').trim();
  const parts = normalized.split(/\/(?![0-9])/);
  const dimensions: DimensionMap = {};
  for (let index = 0; index < parts.length; index += 1) {
    const direction = index === 0 ? 1 : -1;
    const tokens = parts[index].trim().split(/\s+/).filter(Boolean);
    if (tokens.length === 0) {
      return null;
    }
    for (const token of tokens) {
      if (token === '1') {
        continue;
      }
      const match = /^([A-Za-zΩµμ]+)(?:\^([+-]?\d+(?:\/\d+)?))?$/.exec(token);
      if (!match) {
        return null;
      }
      const exponent = match[2] ? parseExponent(match[2]) : 1;
      if (exponent === null) {
        return null;
      }
      const unitDimensions = lookupUnitDimensions(match[1]);
      if (!unitDimensions) {
        return null;
      }
      addDimensions(dimensions, unitDimensions, direction * exponent);
    }
  }
  return dimensions;
}

function parseUnitExpression(value: string): Map<string, number> | null {
  const trimmed = value.trim();
  if (trimmed === '1') {
    return new Map();
  }
  if (!trimmed) {
    return null;
  }
  const normalized = trimmed.replace(/[·*]/g, ' ').replace(/\s+/g, ' ').trim();
  const parts = normalized.split(/\/(?![0-9])/);
  const expression = new Map<string, number>();
  for (let index = 0; index < parts.length; index += 1) {
    const direction = index === 0 ? 1 : -1;
    const tokens = parts[index].trim().split(/\s+/).filter(Boolean);
    if (tokens.length === 0) {
      return null;
    }
    for (const token of tokens) {
      if (token === '1') {
        continue;
      }
      const match = /^([A-Za-zΩµμ]+)(?:\^([+-]?\d+(?:\/\d+)?))?$/.exec(token);
      if (!match) {
        return null;
      }
      const exponent = match[2] ? parseExponent(match[2]) : 1;
      if (exponent === null || !lookupUnitDimensions(match[1])) {
        return null;
      }
      const symbol = match[1];
      expression.set(symbol, (expression.get(symbol) ?? 0) + direction * exponent);
    }
  }
  return expression;
}

function latexUnitToPlain(value: string): string {
  return value
    .replace(/\\mathrm\{([^{}]*)\}/g, ' $1 ')
    .replace(/\\Omega/g, ' Ω ')
    .replace(/\\mu/g, ' μ ')
    .replace(/\\[,;! ]/g, ' ')
    .replace(/\s*\^\s*/g, '^')
    .replace(/[{}]/g, '')
    .replace(/\\/g, '')
    .replace(/\s+/g, ' ')
    .trim();
}

function unitExpressionsEqual(left: Map<string, number>, right: Map<string, number>): boolean {
  const symbols = new Set([...left.keys(), ...right.keys()]);
  return [...symbols].every((symbol) => (left.get(symbol) ?? 0) === (right.get(symbol) ?? 0));
}

function normalizeUnitText(value: string): string {
  return value
    .replace(/\\Omega/g, 'Ω')
    .replace(/\\mathrm/g, '')
    .replace(/\\[,;! ]/g, '')
    .replace(/[{}]/g, '')
    .replace(/\\/g, '')
    .replace(/[·*]/g, '')
    .replace(/\s+/g, '')
    .replace(/µ/g, 'μ');
}

function requiredString(
  entry: TermEntry,
  field: string,
  value: string | undefined,
  issues: TermValidationIssue[],
): void {
  if (!value || value.trim().length === 0) {
    issues.push({
      path: `${entry.id}.${field}`,
      message: `${field} must be a non-empty string.`,
    });
  }
}

function validateLatex(
  entry: TermEntry,
  field: string,
  value: string | undefined,
  issues: TermValidationIssue[],
): void {
  if (!value || value.trim().length === 0) {
    return;
  }
  if (UNSAFE_LATEX_PATTERN.test(value)) {
    issues.push({
      path: `${entry.id}.${field}`,
      message: `${field} contains a trust-gated or unsafe command.`,
    });
    return;
  }
  try {
    katex.renderToString(value, {
      displayMode: true,
      throwOnError: true,
      strict: 'error',
      trust: false,
      output: 'htmlAndMathml',
    });
  } catch (error) {
    issues.push({
      path: `${entry.id}.${field}`,
      message: `${field} failed to render: ${error instanceof Error ? error.message : String(error)}`,
    });
  }
}

function validateKindFields(entry: TermEntry, issues: TermValidationIssue[]): void {
  const unitKind = entry.kind === 'base-unit' || entry.kind === 'derived-unit';
  if (unitKind) {
    requiredString(entry, 'symbol', entry.symbol, issues);
    requiredString(entry, 'symbolPlainText', entry.symbolPlainText, issues);
    requiredString(entry, 'siUnit', entry.siUnit, issues);
    requiredString(entry, 'quantity', entry.quantity, issues);
    requiredString(entry, 'dimensions', entry.dimensions, issues);
    requiredString(entry, 'dimensionFormula', entry.dimensionFormula, issues);

    const expectedSymbol = SI_UNIT_SYMBOL_BY_ENTRY[entry.id];
    if (!expectedSymbol) {
      issues.push({
        path: `${entry.id}.symbol`,
        message: 'base and derived units require a registered expected SI symbol.',
      });
    } else {
      if (normalizeUnitText(entry.symbol ?? '') !== normalizeUnitText(expectedSymbol)) {
        issues.push({
          path: `${entry.id}.symbol`,
          message: `symbol must be the SI symbol ${expectedSymbol}.`,
        });
      }
      if (normalizeUnitText(entry.symbolPlainText ?? '') !== normalizeUnitText(expectedSymbol)) {
        issues.push({
          path: `${entry.id}.symbolPlainText`,
          message: `symbolPlainText must be the SI symbol ${expectedSymbol}.`,
        });
      }
    }
  }
  if (entry.kind === 'quantity') {
    requiredString(entry, 'dimensions', entry.dimensions, issues);
    if (entry.dimensions !== '1') {
      requiredString(entry, 'siUnit', entry.siUnit, issues);
    }
  }
  if (entry.kind === 'symbol') {
    requiredString(entry, 'symbol', entry.symbol, issues);
  }
  if (entry.kind === 'notation' && !entry.symbol && !(entry.notationVariants?.length ?? 0)) {
    issues.push({
      path: `${entry.id}.notationVariants`,
      message: 'notation entries require a symbol or at least one notation variant.',
    });
  }
  if (entry.symbol && !entry.symbolPlainText) {
    issues.push({
      path: `${entry.id}.symbolPlainText`,
      message: 'symbolPlainText is required when symbol is present.',
    });
  }
  if (entry.siUnitLatex && !entry.siUnit) {
    issues.push({
      path: `${entry.id}.siUnit`,
      message: 'siUnit is required when siUnitLatex is present.',
    });
  }

  const parsedDimensions = entry.dimensions ? parseDimensions(entry.dimensions) : null;
  if (entry.dimensions && parsedDimensions === null) {
    issues.push({
      path: `${entry.id}.dimensions`,
      message:
        'dimensions must use M, L, T, I, Θ, N, J with integer or fractional exponents, or be 1.',
    });
  }

  if (entry.dimensionFormula) {
    const formula = parseDimensionFormula(entry.dimensionFormula);
    if (formula.symbolic) {
      if (entry.id !== 'units-dimensional-formula') {
        issues.push({
          path: `${entry.id}.dimensionFormula`,
          message:
            'symbolic dimension exponents are only supported for the general dimensional-formula entry.',
        });
      }
    } else if (formula.dimensions === null) {
      issues.push({
        path: `${entry.id}.dimensionFormula`,
        message: 'dimensionFormula has an unsupported dimensional expression.',
      });
    } else if (parsedDimensions && !dimensionMapsEqual(parsedDimensions, formula.dimensions)) {
      issues.push({
        path: `${entry.id}.dimensionFormula`,
        message: 'dimensionFormula does not agree with the stored dimensions.',
      });
    }
  }

  if (entry.siUnit) {
    const unitDimensions = parseUnitDimensions(entry.siUnit);
    if (unitDimensions === null) {
      issues.push({
        path: `${entry.id}.siUnit`,
        message: 'siUnit is not a supported SI unit or compound expression.',
      });
    } else if (parsedDimensions && !dimensionMapsEqual(parsedDimensions, unitDimensions)) {
      issues.push({
        path: `${entry.id}.siUnit`,
        message: 'siUnit dimensions do not agree with the stored dimensions.',
      });
    }
  }

  if (entry.siUnit && entry.siUnitLatex) {
    const plainExpression = parseUnitExpression(entry.siUnit);
    const latexExpression = parseUnitExpression(latexUnitToPlain(entry.siUnitLatex));
    if (
      !plainExpression ||
      !latexExpression ||
      !unitExpressionsEqual(plainExpression, latexExpression)
    ) {
      issues.push({
        path: `${entry.id}.siUnitLatex`,
        message: 'siUnitLatex does not agree with the plain siUnit expression.',
      });
    }
  }
}

export function validateTermLibrary(
  entries: TermEntry[],
  syllabusIndex: SyllabusIndex,
  formulaIds: ReadonlySet<string>,
): TermValidationIssue[] {
  const issues: TermValidationIssue[] = [];
  const ids = new Set<string>();
  const names = new Map<string, string>();

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

    requiredString(entry, 'term', entry.term, issues);
    requiredString(entry, 'definition', entry.definition, issues);
    requiredString(entry, 'topic', entry.topic, issues);

    if (!SUBJECT_IDS.has(entry.subject)) {
      issues.push({ path: `${entry.id}.subject`, message: 'subject is not recognized.' });
    }
    if (!KIND_IDS.has(entry.kind)) {
      issues.push({ path: `${entry.id}.kind`, message: 'kind is not recognized.' });
    }

    validateKindFields(entry, issues);

    validateLatex(entry, 'symbol', entry.symbol, issues);
    validateLatex(entry, 'siUnitLatex', entry.siUnitLatex, issues);
    validateLatex(entry, 'dimensionFormula', entry.dimensionFormula, issues);
    for (const [index, variant] of (entry.notationVariants ?? []).entries()) {
      validateLatex(entry, `notationVariants[${index}]`, variant, issues);
    }

    for (const field of ['aliases', 'notationVariants', 'conditions', 'keywords'] as const) {
      const values = entry[field];
      if (!values) {
        continue;
      }
      values.forEach((value, index) => {
        if (!value.trim()) {
          issues.push({
            path: `${entry.id}.${field}[${index}]`,
            message: `${field} entries must be non-empty.`,
          });
        }
      });
    }

    const localNames = new Set<string>();
    for (const name of [entry.term, ...(entry.aliases ?? [])]) {
      const key = normalizeSearchText(name).trim();
      if (!key) {
        issues.push({
          path: `${entry.id}.aliases`,
          message: 'term names and aliases must normalize to a non-empty string.',
        });
        continue;
      }
      if (localNames.has(key)) {
        issues.push({
          path: `${entry.id}.aliases`,
          message: `duplicate term name or alias "${name}".`,
        });
      }
      localNames.add(key);
      const owner = names.get(key);
      if (owner && owner !== entry.id) {
        issues.push({
          path: `${entry.id}.aliases`,
          message: `term name or alias "${name}" is already used by ${owner}.`,
        });
      } else {
        names.set(key, entry.id);
      }
    }

    if (entry.sources.length === 0) {
      issues.push({ path: `${entry.id}.sources`, message: 'at least one source is required.' });
    }
    entry.sources.forEach((source, index) => {
      if (!source.citation.trim()) {
        issues.push({
          path: `${entry.id}.sources[${index}]`,
          message: 'source citation must be non-empty.',
        });
      } else if (!KNOWN_SOURCE_CITATIONS.has(source.citation.trim())) {
        issues.push({
          path: `${entry.id}.sources[${index}].citation`,
          message: 'source citation is not in the known source catalogue.',
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

    const linkedNodes = new Set<string>();
    for (const [linkIndex, syllabusLink] of entry.syllabusLinks.entries()) {
      const linkPath = `${entry.id}.syllabusLinks[${linkIndex}]`;
      if (!EXAM_CODES_SET.has(syllabusLink.examCode)) {
        issues.push({ path: linkPath, message: 'examCode is not recognized.' });
      }
      if (syllabusLink.nodes.length !== 1) {
        issues.push({
          path: linkPath,
          message: 'each syllabus link must reference exactly one node.',
        });
      }
      if (!RELEVANCE_VALUES.has(syllabusLink.relevance)) {
        issues.push({ path: linkPath, message: 'relevance is not recognized.' });
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
          message: 'TIFR links must not be explicit at term level.',
        });
      }
      if (JEST_RESTRICTED_SUBJECTS.has(entry.subject) && syllabusLink.examCode === 'jest-physics') {
        issues.push({
          path: linkPath,
          message:
            'JEST relevance is not allowed for this subject without explicit syllabus evidence.',
        });
      }
      if (syllabusLink.relevance === 'explicit') {
        const firstNode = syllabusLink.nodes[0];
        if (
          firstNode &&
          !EXPLICIT_TERM_LINKS.has(explicitLinkKey(entry.id, syllabusLink.examCode, firstNode.code))
        ) {
          issues.push({
            path: linkPath,
            message:
              'explicit relevance requires a reviewed term/node match in the explicit-link allowlist.',
          });
        }
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
          (indexedNode.kind !== 'topic' || !indexedNode.title.startsWith('Broad area:'))
        ) {
          issues.push({
            path: `${linkPath}.nodes`,
            message: 'TIFR links must use one of the eight broad-area topic nodes.',
          });
        }
      }
    }

    const relatedTerms = entry.relatedTermIds ?? [];
    const relatedTermSet = new Set<string>();
    for (const relatedId of relatedTerms) {
      if (relatedId === entry.id) {
        issues.push({
          path: `${entry.id}.relatedTermIds`,
          message: 'a term cannot relate to itself.',
        });
      }
      if (relatedTermSet.has(relatedId)) {
        issues.push({
          path: `${entry.id}.relatedTermIds`,
          message: `duplicate related term ${relatedId}.`,
        });
      }
      relatedTermSet.add(relatedId);
    }

    const relatedFormulas = entry.relatedFormulaIds ?? [];
    const relatedFormulaSet = new Set<string>();
    for (const relatedId of relatedFormulas) {
      if (relatedFormulaSet.has(relatedId)) {
        issues.push({
          path: `${entry.id}.relatedFormulaIds`,
          message: `duplicate related formula ${relatedId}.`,
        });
      }
      relatedFormulaSet.add(relatedId);
    }
  }

  for (const entry of entries) {
    for (const relatedId of entry.relatedTermIds ?? []) {
      if (!ids.has(relatedId)) {
        issues.push({
          path: `${entry.id}.relatedTermIds`,
          message: `related term ${relatedId} does not exist.`,
        });
      }
    }
    for (const relatedId of entry.relatedFormulaIds ?? []) {
      if (!formulaIds.has(relatedId)) {
        issues.push({
          path: `${entry.id}.relatedFormulaIds`,
          message: `related formula ${relatedId} does not exist.`,
        });
      }
    }
  }

  return issues;
}
