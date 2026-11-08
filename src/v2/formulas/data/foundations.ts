import type { FormulaEntry } from '../types';
import { CSIR, GATE, JEST, TIFR, link } from './syllabus';
import { REFERENCES, TEXTBOOKS, reference, textbook } from './sources';

export const FOUNDATIONS_FORMULAS: FormulaEntry[] = [
  {
    id: 'foundations-dimensional-form',
    title: 'Dimensional form of a physical quantity',
    latex: String.raw`[Q]=M^aL^bT^cI^d\Theta^eN^fJ^g`,
    plainText: '[Q] = M^a L^b T^c I^d Theta^e N^f J^g',
    subject: 'foundations',
    topic: 'Dimensional analysis',
    subtopic: 'Dimensional form',
    summary: 'Expresses the dimensions of a physical quantity as powers of the SI base dimensions.',
    symbols: [
      { symbol: 'Q', meaning: 'physical quantity whose dimensions are being expressed' },
      {
        symbol: 'M,L,T,I,Theta,N,J',
        meaning:
          'SI base dimensions: mass, length, time, electric current, thermodynamic temperature, amount of substance, luminous intensity',
      },
      { symbol: 'a,b,c,d,e,f,g', meaning: 'real exponents for the base dimensions' },
    ],
    conditions: [
      'SI base quantities are used.',
      'Dimensionless quantities have all exponents equal to zero.',
      'Angles, counts, and some ratios are dimensionless unless a convention states otherwise.',
      'Dimensional analysis constrains equations but does not replace a physical model.',
    ],
    exactness: 'definition',
    framework: 'general',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.dimensionalAnalysis]),
      link('gate-physics', 'explicit', [GATE.unitsDimensions]),
      link('tifr-gs-physics', 'implicit', [TIFR.math]),
      link('jest-physics', 'implicit', [JEST.math]),
    ],
    sources: [reference(REFERENCES.siBrochure, 'Base units and dimensions')],
    relatedIds: ['foundations-power-law-linearization'],
    conventions: [
      'Theta denotes thermodynamic temperature; N denotes amount of substance; J denotes luminous intensity.',
    ],
    keywords: ['dimensions', 'dimensional analysis', 'SI units', 'base quantities'],
  },
  {
    id: 'foundations-power-law-linearization',
    title: 'Power-law linearization',
    latex: String.raw`\log y=\log a+n\log x`,
    plainText: 'log y = log a + n log x',
    subject: 'foundations',
    topic: 'Scaling and data analysis',
    subtopic: 'Power-law linearization',
    summary:
      'A power-law relation becomes linear when both variables are plotted on logarithmic axes.',
    symbols: [
      { symbol: 'y', meaning: 'dependent variable' },
      { symbol: 'x', meaning: 'independent variable' },
      { symbol: 'a', meaning: 'positive proportionality constant' },
      { symbol: 'n', meaning: 'constant power-law exponent' },
    ],
    conditions: [
      'The relation is y = a x^n.',
      'x and y are positive so that the logarithms are real.',
      'The same logarithm base is used on both sides.',
      'An additive offset or a changing exponent over the range invalidates the linearization.',
    ],
    exactness: 'definition',
    framework: 'general',
    syllabusLinks: [
      link('csir-net-physics', 'implicit', [CSIR.mathCore]),
      link('gate-physics', 'implicit', [GATE.measurements]),
      link('tifr-gs-physics', 'implicit', [TIFR.math]),
      link('jest-physics', 'implicit', [JEST.math]),
    ],
    sources: [textbook(TEXTBOOKS.riley, 'Ch. 1'), textbook(TEXTBOOKS.bevington, 'Ch. 7, §7.4')],
    relatedIds: ['foundations-dimensional-form', 'foundations-binomial-approximation'],
    specialCases: ['A log-log plot has slope n and intercept log a.'],
    conventions: ['Natural or base-10 logarithms may be used consistently.'],
    keywords: ['power law', 'log-log', 'scaling', 'linearization', 'exponent'],
  },
  {
    id: 'foundations-binomial-approximation',
    title: 'First-order binomial approximation',
    latex: String.raw`(1+x)^n\approx1+nx,\qquad |x|\ll1`,
    plainText: '(1+x)^n ~ 1 + n x for |x| << 1',
    subject: 'foundations',
    topic: 'Mathematical approximations',
    subtopic: 'Binomial approximation',
    summary: 'The first-order expansion of a binomial power for a small dimensionless parameter.',
    symbols: [
      { symbol: 'x', meaning: 'small dimensionless parameter' },
      { symbol: 'n', meaning: 'finite real exponent' },
    ],
    conditions: [
      'The absolute value of x is much less than 1.',
      'Only the first-order term in x is retained.',
      'The error is of order x^2 and must be estimated if it matters.',
      'The approximation is not valid near x = -1 or when the exponent or parameter is outside its stated range.',
    ],
    exactness: 'approximate',
    framework: 'general',
    syllabusLinks: [
      link('csir-net-physics', 'implicit', [CSIR.mathCore]),
      link('gate-physics', 'implicit', [GATE.measurements]),
      link('tifr-gs-physics', 'implicit', [TIFR.math]),
      link('jest-physics', 'implicit', [JEST.math]),
    ],
    sources: [textbook(TEXTBOOKS.riley, 'Ch. 1'), textbook(TEXTBOOKS.arfken, 'Ch. 1')],
    relatedIds: ['foundations-power-law-linearization'],
    specialCases: ['For n = 1 and n = 0 the approximation is exact.'],
    conventions: ['x is dimensionless; the error is O(x^2).'],
    keywords: ['binomial', 'approximation', 'small parameter', 'expansion'],
  },
];
