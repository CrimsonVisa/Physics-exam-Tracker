import type { FormulaEntry } from '../types';
import { CSIR, GATE, JEST, TIFR, link } from './syllabus';
import { TEXTBOOKS, textbook } from './sources';

export const EXPERIMENTAL_METHODS_FORMULAS: FormulaEntry[] = [
  {
    id: 'experimental-methods-standard-error-mean',
    title: 'Standard error of the mean',
    latex: String.raw`\sigma_{\bar x}=\frac{\sigma}{\sqrt N}`,
    plainText: 'sigma_xbar = sigma / sqrt(N)',
    subject: 'experimental-methods',
    topic: 'Error analysis',
    subtopic: 'Standard error of the mean',
    summary:
      'The standard deviation of the sample mean for N independent measurements with population standard deviation sigma.',
    symbols: [
      {
        symbol: String.raw`\sigma_{\bar x}`,
        meaning: 'standard error of the mean',
        unit: 'same as x',
      },
      { symbol: String.raw`\sigma`, meaning: 'population standard deviation', unit: 'same as x' },
      {
        symbol: 's',
        meaning: 'sample standard deviation, used when sigma is unknown',
        unit: 'same as x',
      },
      { symbol: 'N', meaning: 'number of independent measurements' },
    ],
    conditions: [
      'Independent, identically distributed measurements with finite variance.',
      'The formula as written uses the population standard deviation sigma.',
      'If sigma is unknown and the sample standard deviation s is used, the estimated standard error is s/sqrt(N).',
      'For small N, confidence intervals based on s use the t-distribution with N - 1 degrees of freedom rather than a normal critical value.',
      'Systematic errors and correlations are not included.',
    ],
    exactness: 'exact',
    framework: 'statistical',
    syllabusLinks: [
      link('csir-net-physics', 'implicit', [CSIR.errorAnalysis]),
      link('gate-physics', 'implicit', [GATE.errorAnalysis]),
      link('tifr-gs-physics', 'implicit', [TIFR.electronics]),
      link('jest-physics', 'implicit', [JEST.probabilityError]),
    ],
    sources: [textbook(TEXTBOOKS.bevington, 'Ch. 4'), textbook(TEXTBOOKS.taylorError, 'Ch. 4')],
    relatedIds: ['experimental-methods-error-propagation', 'experimental-methods-chi-square'],
    specialCases: ['When sigma is estimated by the sample standard deviation s, use s/sqrt(N).'],
    keywords: ['standard error', 'mean', 'uncertainty', 'statistics', 'sigma'],
  },
  {
    id: 'experimental-methods-error-propagation',
    title: 'Propagation of independent uncertainties',
    latex: String.raw`\sigma_f^2=\sum_i\left(\frac{\partial f}{\partial x_i}\right)^2\sigma_{x_i}^2`,
    plainText: 'sigma_f^2 = sum_i (df/dx_i)^2 sigma_xi^2',
    subject: 'experimental-methods',
    topic: 'Error analysis',
    subtopic: 'Propagation of uncertainty',
    summary: 'The first-order variance of a derived quantity for independent input uncertainties.',
    symbols: [
      { symbol: 'f', meaning: 'derived quantity' },
      { symbol: String.raw`x_i`, meaning: 'input quantity i' },
      {
        symbol: String.raw`\sigma_f`,
        meaning: 'standard uncertainty in f',
        unit: 'same as f',
      },
      {
        symbol: String.raw`\sigma_{x_i}`,
        meaning: 'standard uncertainty in x_i',
        unit: 'same as x_i',
      },
    ],
    conditions: [
      'Input uncertainties are independent random uncertainties.',
      'The linearized first-order approximation is adequate.',
      'Uncertainties are small enough for the Taylor expansion.',
      'Correlated inputs require covariance terms; systematic errors require separate treatment.',
    ],
    exactness: 'approximate',
    framework: 'statistical',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.errorAnalysis]),
      link('gate-physics', 'implicit', [GATE.errorAnalysis]),
      link('tifr-gs-physics', 'implicit', [TIFR.electronics]),
      link('jest-physics', 'implicit', [JEST.probabilityError]),
    ],
    sources: [textbook(TEXTBOOKS.bevington, 'Ch. 3'), textbook(TEXTBOOKS.taylorError, 'Ch. 3')],
    relatedIds: ['experimental-methods-standard-error-mean', 'experimental-methods-chi-square'],
    specialCases: [
      'For a sum or difference, variances add.',
      'For a product or quotient, relative variances add for independent inputs.',
    ],
    conventions: ['The formula gives a standard uncertainty, not a worst-case bound.'],
    keywords: ['error propagation', 'uncertainty', 'partial derivative', 'variance', 'sigma'],
  },
  {
    id: 'experimental-methods-chi-square',
    title: 'Chi-square goodness-of-fit statistic',
    latex: String.raw`\chi^2=\sum_i\frac{(O_i-E_i)^2}{E_i}`,
    plainText: 'chi^2 = sum_i (O_i - E_i)^2 / E_i',
    subject: 'experimental-methods',
    topic: 'Data analysis',
    subtopic: 'Chi-square goodness of fit',
    summary:
      'Compares observed counts with expected counts under a specified model for independent counting data.',
    symbols: [
      {
        symbol: String.raw`\chi^2`,
        meaning: 'chi-square test statistic',
      },
      { symbol: String.raw`O_i`, meaning: 'observed count in bin i' },
      { symbol: String.raw`E_i`, meaning: 'expected count in bin i under the model' },
    ],
    conditions: [
      'Independent counts or bins.',
      'Expected counts are sufficiently large for the chi-square approximation; a common guideline is E_i at least 5.',
      'The model and any fitted parameters are specified before interpreting degrees of freedom.',
      'For fitted parameters, subtract the number of estimated parameters from the number of bins when determining degrees of freedom.',
    ],
    exactness: 'definition',
    framework: 'statistical',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.chiSquare]),
      link('gate-physics', 'implicit', [GATE.errorAnalysis]),
      link('tifr-gs-physics', 'implicit', [TIFR.electronics]),
      link('jest-physics', 'implicit', [JEST.probabilityError]),
    ],
    sources: [textbook(TEXTBOOKS.bevington, 'Ch. 4'), textbook(TEXTBOOKS.taylorError, 'Ch. 12')],
    relatedIds: [
      'experimental-methods-standard-error-mean',
      'experimental-methods-error-propagation',
    ],
    specialCases: [
      'For a good fit with the correct model, chi-square per degree of freedom is approximately 1.',
    ],
    conventions: [
      'This is the Pearson chi-square form for counts; weighted least-squares uses a different residual expression.',
    ],
    keywords: ['chi-square', 'goodness of fit', 'observed', 'expected', 'degrees of freedom'],
  },
];
