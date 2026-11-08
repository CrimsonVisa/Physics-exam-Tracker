import type {
  CalculatorFunctionId,
  CalculatorModeId,
  CalculatorProfile,
  EvidenceStatus,
  FunctionAvailability,
} from './types';

export const DEFAULT_CALCULATOR_MODE: CalculatorModeId = 'practice';

export const CALCULATOR_MODE_ORDER: CalculatorModeId[] = [
  'practice',
  'csir-net-physics',
  'gate-physics',
  'tifr-gs-physics',
  'jest-physics',
];

const ALL_FUNCTIONS: CalculatorFunctionId[] = [
  'add',
  'subtract',
  'multiply',
  'divide',
  'parentheses',
  'percentage',
  'sign',
  'reciprocal',
  'factorial',
  'abs',
  'mod',
  'square',
  'cube',
  'power',
  'sqrt',
  'cbrt',
  'root',
  'scientific-notation',
  'sin',
  'cos',
  'tan',
  'asin',
  'acos',
  'atan',
  'sinh',
  'cosh',
  'tanh',
  'asinh',
  'acosh',
  'atanh',
  'angle-mode',
  'ln',
  'log10',
  'log2',
  'logb',
  'exp',
  'pow10',
  'pi',
  'e',
  'memory',
  'ans',
  'repeat-equals',
];

function functionsWith(
  availability: FunctionAvailability,
): Record<CalculatorFunctionId, FunctionAvailability> {
  return Object.fromEntries(ALL_FUNCTIONS.map((id) => [id, availability])) as Record<
    CalculatorFunctionId,
    FunctionAvailability
  >;
}

const PRACTICE_PROFILE: CalculatorProfile = {
  id: 'practice',
  label: 'Practice',
  shortLabel: 'Practice',
  modeType: 'practice',
  physicalCalculator: 'not-documented',
  onScreenCalculator: 'not-documented',
  functionSet: 'not-documented',
  applicableYear: 'Current session',
  lastVerified: '2026-11-08',
  summary:
    'Practice — not an examination simulation. No examination calculator policy applies to this mode.',
  caveats: [
    'Use this mode for practice and checking work; it does not represent examination conditions.',
  ],
  sources: [],
  functions: functionsWith('available'),
  hasKeypad: true,
};

const CSIR_PROFILE: CalculatorProfile = {
  id: 'csir-net-physics',
  label: 'CSIR-UGC NET Physical Sciences',
  shortLabel: 'CSIR NET',
  modeType: 'exam-policy',
  physicalCalculator: 'confirmed-prohibited',
  onScreenCalculator: 'not-documented',
  functionSet: 'not-documented',
  applicableYear: 'June 2025',
  lastVerified: '2026-11-08',
  summary:
    'Physical calculators are confirmed prohibited. The June 2025 bulletin does not document an on-screen calculator. A virtual keyboard is documented for answer entry, which is not a calculator.',
  caveats: [
    'The virtual keyboard in the NTA CBT interface is for entering answers; it is not an on-screen calculator.',
    'No CSIR-specific calculator keypad is provided. Use Practice for calculations.',
  ],
  sources: [
    {
      title: 'Joint CSIR-UGC NET Information Bulletin (June 2025)',
      url: 'https://csirnet.nta.ac.in/images/infomration-bulletin-of-csir-june-2025.pdf',
      year: '2025',
      reference: 'p. 7 Note 2; p. 29 §6.2.19; p. 55 Annexure XII',
      kind: 'policy',
    },
  ],
  functions: functionsWith('not-documented'),
  hasKeypad: false,
  fallbackMode: 'practice',
};

const GATE_PROFILE: CalculatorProfile = {
  id: 'gate-physics',
  label: 'GATE Physics (PH)',
  shortLabel: 'GATE PH',
  modeType: 'exam-calculator',
  physicalCalculator: 'confirmed-prohibited',
  onScreenCalculator: 'confirmed-provided',
  functionSet: 'not-documented',
  applicableYear: 'GATE 2027',
  lastVerified: '2026-11-08',
  summary:
    'Physical calculators are confirmed prohibited. A virtual scientific calculator is confirmed provided. GATE does not formally enumerate the function set; advanced functions are based on the official 2026 mock reference and may differ in GATE 2027.',
  caveats: [
    'Only the existence of a virtual scientific calculator is formally confirmed by GATE.',
    'The exact function-level capabilities are not formally enumerated in the GATE 2027 brochure or FAQ.',
    'Advanced functions are shown from the official GATE 2026 mock reference and are not guaranteed for the actual examination.',
    'This is not an exact official GATE replica.',
  ],
  sources: [
    {
      title: 'GATE 2027 Information Brochure',
      url: 'https://gate2027ib.iitm.ac.in/GATE2027-IB.pdf',
      year: '2027',
      reference: 'p. 2 Highlights; p. 22 §7; p. 34 Appendix C',
      kind: 'policy',
    },
    {
      title: 'GATE 2027 Frequently Asked Questions',
      url: 'https://gate2027.iitm.ac.in/faqs',
      year: '2027',
      reference: 'GATE 2027 Examination, Q10, Q14, Q15',
      kind: 'policy',
    },
    {
      title: 'GATE 2026 Official Mock Test Links',
      url: 'https://gate2026.iitg.ac.in/mock-test-links.html',
      year: '2026',
      reference: 'Mock test disclaimer: actual exam features may differ',
      kind: 'mock-reference',
    },
    {
      title: 'GATE 2026 Official Mock Calculator',
      url: 'https://cdn4.digialm.com/OnlineAssessment/Calculator.html',
      year: '2026',
      reference: 'Scientific calculator controls and help text; not a policy document',
      kind: 'mock-reference',
    },
  ],
  functions: functionsWith('mock-observed'),
  hasKeypad: true,
  keyboardNote:
    'Keyboard entry is an application convenience. The official GATE mock help indicates keyboard operation is disabled.',
};

const TIFR_PROFILE: CalculatorProfile = {
  id: 'tifr-gs-physics',
  label: 'TIFR Graduate School Physics',
  shortLabel: 'TIFR GS',
  modeType: 'exam-policy',
  physicalCalculator: 'confirmed-prohibited',
  onScreenCalculator: 'confirmed-provided',
  functionSet: 'not-documented',
  applicableYear: 'GS 2024 documented; GS 2027 not published',
  lastVerified: '2026-11-08',
  summary:
    'Physical calculators are confirmed prohibited in the GS 2024 Physics instructions. An on-screen calculator is confirmed for the GS 2024 Physics screening, but its functions are not documented. GS 2027 Physics calculator instructions are not yet published.',
  caveats: [
    'The on-screen calculator evidence is from the GS 2024 Physics screening instructions, not GS 2027.',
    'Function-level capabilities are not documented.',
    'GS 2027 Physics calculator instructions require updated verification.',
    'No TIFR-specific calculator keypad is provided. Use Practice for calculations.',
  ],
  sources: [
    {
      title: 'TIFR GS 2024 Physics Entrance Test — Syllabus and Exam Instructions',
      url: 'https://www.tifr.res.in/academics/docs/Physics_syllabus_exam_instructions.pdf',
      year: '2024',
      reference: 'p. 1, GS 2024 Physics Screening Test Instructions',
      kind: 'policy',
    },
    {
      title: 'TIFR GS-2027 Advertisement',
      url: 'https://www.tifr.res.in/academics/docs/Advertisment_GS-2027.pdf',
      year: '2027',
      reference: 'p. 1 test date; Admission Procedure section (online test)',
      kind: 'policy',
    },
    {
      title: 'TIFR Syllabus and Exam Instructions',
      url: 'https://www.tifr.res.in/academics/syllabus.php',
      year: '2026',
      reference: 'Physics link currently points to the GS 2024 instructions',
      kind: 'policy',
    },
  ],
  functions: functionsWith('not-documented'),
  hasKeypad: false,
  fallbackMode: 'practice',
};

const JEST_PROFILE: CalculatorProfile = {
  id: 'jest-physics',
  label: 'JEST Physics',
  shortLabel: 'JEST',
  modeType: 'exam-policy',
  physicalCalculator: 'confirmed-prohibited',
  onScreenCalculator: 'not-documented',
  functionSet: 'not-documented',
  applicableYear: 'JEST 2026',
  lastVerified: '2026-11-08',
  summary:
    'Physical calculators are confirmed prohibited under JEST 2026 instructions. No on-screen calculator is documented. No JEST-specific calculator keypad is provided.',
  caveats: [
    'JEST 2026 is a computer-based test, but no on-screen calculator provision is documented.',
    'No JEST-specific calculator keypad is provided. Use Practice for calculations.',
  ],
  sources: [
    {
      title: 'JEST 2026 Frequently Asked Questions',
      url: 'https://www.jest.org.in/faq',
      year: '2026',
      reference: 'Exam-Specific Q2, Q3, Q5',
      kind: 'policy',
    },
    {
      title: 'JEST 2026 Sample Physics Question Paper',
      url: 'https://www.jest.org.in/sites/default/files/2026-01/sampleqp-jest2026.pdf',
      year: '2026',
      reference: 'Part C numerical questions; no calculator provision documented',
      kind: 'policy',
    },
  ],
  functions: functionsWith('not-documented'),
  hasKeypad: false,
  fallbackMode: 'practice',
};

export const CALCULATOR_PROFILES: Record<CalculatorModeId, CalculatorProfile> = {
  practice: PRACTICE_PROFILE,
  'csir-net-physics': CSIR_PROFILE,
  'gate-physics': GATE_PROFILE,
  'tifr-gs-physics': TIFR_PROFILE,
  'jest-physics': JEST_PROFILE,
};

export const CALCULATOR_MODE_OPTIONS = CALCULATOR_MODE_ORDER.map((id) => ({
  id,
  label: CALCULATOR_PROFILES[id].label,
  shortLabel: CALCULATOR_PROFILES[id].shortLabel,
}));

export function getCalculatorProfile(mode: CalculatorModeId): CalculatorProfile {
  return CALCULATOR_PROFILES[mode];
}

export function isFunctionEnabled(
  profile: CalculatorProfile,
  functionId: CalculatorFunctionId,
): boolean {
  const availability = profile.functions[functionId];
  return availability === 'available' || availability === 'mock-observed';
}

export function evidenceStatusLabel(status: EvidenceStatus): string {
  switch (status) {
    case 'confirmed-permitted':
      return 'Confirmed permitted';
    case 'confirmed-prohibited':
      return 'Confirmed prohibited';
    case 'confirmed-provided':
      return 'Confirmed provided';
    case 'not-documented':
      return 'Not documented';
    case 'not-provided':
      return 'Not provided';
    case 'year-dependent':
      return 'Year/session dependent';
    default:
      return 'Not documented';
  }
}

export function functionAvailabilityLabel(availability: FunctionAvailability): string {
  switch (availability) {
    case 'available':
      return 'Available';
    case 'unavailable':
      return 'Unavailable';
    case 'not-documented':
      return 'Not documented';
    case 'practice-fallback':
      return 'Practice fallback';
    case 'mock-observed':
      return 'Mock-observed';
    default:
      return 'Not documented';
  }
}
