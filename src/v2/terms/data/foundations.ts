import type { TermEntry } from '../types';
import { CSIR, GATE, JEST, TEXTBOOKS, TIFR, link, textbook } from './shared';

const mathLinks = () => [
  link('csir-net-physics', 'implicit', [CSIR.mathCore]),
  link('gate-physics', 'implicit', [GATE.math]),
  link('tifr-gs-physics', 'implicit', [TIFR.math]),
  link('jest-physics', 'implicit', [JEST.math]),
];

const vectorLinks = () => [
  link('csir-net-physics', 'implicit', [CSIR.divergence]),
  link('gate-physics', 'implicit', [GATE.math]),
  link('tifr-gs-physics', 'implicit', [TIFR.math]),
  link('jest-physics', 'implicit', [JEST.vectorCalculus]),
];

export const FOUNDATIONS_TERMS: TermEntry[] = [
  {
    id: 'foundations-scalar',
    term: 'scalar',
    kind: 'concept',
    subject: 'foundations',
    topic: 'Mathematical objects',
    definition:
      'A quantity represented by a single real number and invariant under rotations of the coordinate system; its value at a point does not depend on the choice of axes.',
    conditions: [
      'A scalar may be a function of position and time; coordinate independence refers to its value at a point, not to spatial variation.',
    ],
    relatedTermIds: ['foundations-vector', 'foundations-tensor'],
    syllabusLinks: vectorLinks(),
    sources: [textbook(TEXTBOOKS.arfken, 'Ch. 3'), textbook(TEXTBOOKS.riley, 'Ch. 7')],
    keywords: ['scalar', 'number', 'rotation invariant'],
  },
  {
    id: 'foundations-vector',
    term: 'vector',
    aliases: ['vector quantity'],
    kind: 'concept',
    subject: 'foundations',
    topic: 'Mathematical objects',
    definition:
      'A quantity with magnitude and direction that transforms under rotations like a displacement.',
    symbol: String.raw`\mathbf{A}`,
    symbolPlainText: 'A',
    notationVariants: [String.raw`\vec{A}`, String.raw`\mathbf{A}`, 'A_i'],
    conditions: [
      'Bold or arrow notation denotes a vector; the components depend on the coordinate system.',
    ],
    relatedTermIds: ['foundations-scalar', 'foundations-tensor', 'mathematical-physics-gradient'],
    syllabusLinks: vectorLinks(),
    sources: [textbook(TEXTBOOKS.arfken, 'Ch. 3'), textbook(TEXTBOOKS.riley, 'Ch. 7')],
    keywords: ['vector', 'magnitude', 'direction', 'components', 'rotation'],
  },
  {
    id: 'foundations-tensor',
    term: 'tensor',
    kind: 'concept',
    subject: 'foundations',
    topic: 'Mathematical objects',
    definition:
      'A multilinear object whose components transform with one or more coordinate indices under a change of basis.',
    symbol: String.raw`T_{ij}`,
    symbolPlainText: 'T_ij',
    notationVariants: ['T^{ij}', String.raw`T^{i}_{\ j}`],
    conditions: [
      'The rank and index positions determine the transformation law.',
      'Einstein summation notation is used when an index appears once upstairs and once downstairs.',
    ],
    relatedTermIds: ['foundations-scalar', 'foundations-vector'],
    syllabusLinks: mathLinks(),
    sources: [textbook(TEXTBOOKS.arfken, 'Ch. 4'), textbook(TEXTBOOKS.riley, 'Ch. 26')],
    keywords: ['tensor', 'rank', 'index', 'covariant', 'contravariant'],
  },
  {
    id: 'foundations-field',
    term: 'field',
    aliases: ['physical field'],
    kind: 'concept',
    subject: 'foundations',
    topic: 'Mathematical objects',
    definition:
      'A physical quantity assigned to each point of space and time, such as a scalar field or a vector field.',
    symbol: String.raw`\phi(\mathbf{r},t)`,
    symbolPlainText: 'phi(r,t)',
    conditions: ['A field may be scalar, vector, tensor or spinor valued.'],
    relatedTermIds: ['foundations-scalar', 'foundations-vector', 'electromagnetism-electric-field'],
    syllabusLinks: mathLinks(),
    sources: [textbook(TEXTBOOKS.arfken, 'Ch. 3')],
    keywords: ['field', 'scalar field', 'vector field', 'space', 'time'],
  },
  {
    id: 'foundations-average',
    term: 'average',
    aliases: ['mean', 'arithmetic mean'],
    kind: 'concept',
    subject: 'foundations',
    topic: 'Statistical notation',
    definition:
      'The arithmetic mean of a set of values, obtained by summing the values and dividing by their number.',
    symbol: String.raw`\langle x\rangle`,
    symbolPlainText: '<x>',
    notationVariants: [String.raw`\bar{x}`, 'x̄'],
    conditions: ['The mean has the same dimensions as the averaged quantity.'],
    relatedTermIds: ['foundations-root-mean-square', 'experimental-methods-standard-deviation'],
    syllabusLinks: mathLinks(),
    sources: [textbook(TEXTBOOKS.bevington, 'Ch. 1')],
    keywords: ['average', 'mean', 'arithmetic mean', 'expectation'],
  },
  {
    id: 'foundations-root-mean-square',
    term: 'root-mean-square',
    aliases: ['RMS', 'rms value'],
    kind: 'concept',
    subject: 'foundations',
    topic: 'Statistical notation',
    definition:
      'The square root of the mean of the squares of a set of values or of a time-varying quantity.',
    symbol: String.raw`x_{\mathrm{rms}}`,
    symbolPlainText: 'x_rms',
    conditions: ['The RMS value has the same dimensions as the underlying quantity.'],
    relatedTermIds: ['foundations-average'],
    syllabusLinks: mathLinks(),
    sources: [textbook(TEXTBOOKS.bevington, 'Ch. 1')],
    keywords: ['root mean square', 'RMS', 'quadratic mean', 'effective value'],
  },
  {
    id: 'foundations-order-of-magnitude',
    term: 'order of magnitude',
    kind: 'concept',
    subject: 'foundations',
    topic: 'Approximation',
    definition:
      'A scale estimate expressing a quantity as a power of ten, usually to within a factor of about ten.',
    conditions: [
      'An order-of-magnitude estimate omits numerical factors unless they change the decade.',
    ],
    syllabusLinks: [],
    sources: [textbook(TEXTBOOKS.riley, 'Ch. 1')],
    keywords: ['order of magnitude', 'estimate', 'power of ten', 'approximation'],
  },
];
