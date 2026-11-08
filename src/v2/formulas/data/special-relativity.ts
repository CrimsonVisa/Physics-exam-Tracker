import type { FormulaEntry } from '../types';
import { CSIR, GATE, JEST, TIFR, link } from './syllabus';
import { TEXTBOOKS, textbook } from './sources';

export const SPECIAL_RELATIVITY_FORMULAS: FormulaEntry[] = [
  {
    id: 'special-relativity-lorentz-factor',
    title: 'Lorentz factor and time dilation',
    latex: String.raw`\gamma=\frac{1}{\sqrt{1-\beta^2}},\qquad \Delta t=\gamma\Delta\tau`,
    plainText: 'gamma = 1/sqrt(1-beta^2), Delta t = gamma Delta tau',
    subject: 'special-relativity',
    topic: 'Lorentz transformations',
    subtopic: 'Time dilation',
    summary:
      'The Lorentz factor and the relation between coordinate time in a frame and proper time measured by a clock moving with the event.',
    symbols: [
      { symbol: String.raw`\gamma`, meaning: 'Lorentz factor' },
      { symbol: String.raw`\beta`, meaning: 'speed ratio v/c' },
      {
        symbol: 'v',
        meaning: 'relative speed between inertial frames',
        unit: 'm s^-1',
        dimensions: 'L T^-1',
      },
      { symbol: 'c', meaning: 'speed of light in vacuum', unit: 'm s^-1', dimensions: 'L T^-1' },
      {
        symbol: String.raw`\Delta t`,
        meaning: 'coordinate time interval',
        unit: 's',
        dimensions: 'T',
      },
      {
        symbol: String.raw`\Delta\tau`,
        meaning: 'proper time interval',
        unit: 's',
        dimensions: 'T',
      },
    ],
    conditions: [
      'Inertial frames in flat spacetime.',
      'Constant relative velocity between the frames.',
      'Proper time is measured by a clock at rest with the event.',
      'Beta is less than 1 for massive particles.',
    ],
    exactness: 'exact',
    framework: 'relativistic',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.relativity]),
      link('gate-physics', 'explicit', [GATE.relativity]),
      link('tifr-gs-physics', 'implicit', [TIFR.classical]),
      link('jest-physics', 'explicit', [JEST.specialRelativity]),
    ],
    sources: [textbook(TEXTBOOKS.taylorWheeler, 'Ch. 2'), textbook(TEXTBOOKS.rindler, 'Ch. 2')],
    relatedIds: ['special-relativity-energy-momentum', 'special-relativity-velocity-addition'],
    specialCases: ['For v much less than c, gamma is approximately 1 + v^2/(2c^2).'],
    conventions: ["Standard inertial-frame convention; proper time is the clock's own time."],
    keywords: ['lorentz factor', 'time dilation', 'gamma', 'proper time', 'relativity'],
  },
  {
    id: 'special-relativity-energy-momentum',
    title: 'Relativistic energy-momentum relation',
    latex: String.raw`E^2=p^2c^2+m^2c^4`,
    plainText: 'E^2 = p^2 c^2 + m^2 c^4',
    subject: 'special-relativity',
    topic: 'Relativistic kinematics',
    subtopic: 'Energy-momentum relation',
    summary:
      'The relativistic relation between total energy, momentum, and rest mass for a free particle.',
    symbols: [
      { symbol: 'E', meaning: 'total relativistic energy', unit: 'J', dimensions: 'M L^2 T^-2' },
      { symbol: 'p', meaning: 'relativistic momentum', unit: 'kg m s^-1', dimensions: 'M L T^-1' },
      { symbol: 'm', meaning: 'rest mass', unit: 'kg', dimensions: 'M' },
      { symbol: 'c', meaning: 'speed of light', unit: 'm s^-1', dimensions: 'L T^-1' },
    ],
    conditions: [
      'Free particle in flat spacetime.',
      'E includes the rest energy mc^2.',
      'p and E are measured in the same inertial frame.',
      'The rest mass is non-negative; massless particles have E = pc.',
    ],
    exactness: 'exact',
    framework: 'relativistic',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.relativity]),
      link('gate-physics', 'explicit', [GATE.relativity]),
      link('tifr-gs-physics', 'implicit', [TIFR.classical]),
      link('jest-physics', 'explicit', [JEST.specialRelativity]),
    ],
    sources: [textbook(TEXTBOOKS.taylorWheeler, 'Ch. 7'), textbook(TEXTBOOKS.rindler, 'Ch. 5')],
    relatedIds: ['special-relativity-lorentz-factor', 'special-relativity-velocity-addition'],
    specialCases: [
      'Massless particles: E = pc.',
      'Non-relativistic limit: E is approximately mc^2 + p^2/(2m).',
    ],
    conventions: ['E is total energy, including rest energy.'],
    keywords: ['energy-momentum', 'relativity', 'rest mass', 'mass-energy', 'E=mc2'],
  },
  {
    id: 'special-relativity-velocity-addition',
    title: 'Relativistic velocity addition',
    latex: String.raw`u=\frac{u'+v}{1+u'v/c^2}`,
    plainText: "u = (u' + v)/(1 + u' v/c^2)",
    subject: 'special-relativity',
    topic: 'Lorentz transformations',
    subtopic: 'Velocity addition',
    summary:
      'The composition of collinear velocities in two inertial frames, ensuring that no signal speed exceeds c.',
    symbols: [
      { symbol: 'u', meaning: 'velocity in frame S', unit: 'm s^-1', dimensions: 'L T^-1' },
      {
        symbol: String.raw`u'`,
        meaning: 'velocity in frame S prime',
        unit: 'm s^-1',
        dimensions: 'L T^-1',
      },
      {
        symbol: 'v',
        meaning: 'relative velocity of S prime with respect to S',
        unit: 'm s^-1',
        dimensions: 'L T^-1',
      },
      { symbol: 'c', meaning: 'speed of light', unit: 'm s^-1', dimensions: 'L T^-1' },
    ],
    conditions: [
      'Collinear velocities along the same axis.',
      'Inertial frames in flat spacetime.',
      'The relative speed satisfies v < c and the input speed satisfies |u prime| < c.',
      'The sign convention for the relative velocity is stated explicitly.',
    ],
    exactness: 'exact',
    framework: 'relativistic',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.relativity]),
      link('gate-physics', 'explicit', [GATE.relativity]),
      link('tifr-gs-physics', 'implicit', [TIFR.classical]),
      link('jest-physics', 'explicit', [JEST.specialRelativity]),
    ],
    sources: [textbook(TEXTBOOKS.taylorWheeler, 'Ch. 3'), textbook(TEXTBOOKS.rindler, 'Ch. 3')],
    relatedIds: ['special-relativity-lorentz-factor', 'special-relativity-energy-momentum'],
    specialCases: [
      'If u prime = c, then u = c for any v < c.',
      'For small v and u prime, the Galilean result u = u prime + v is recovered approximately.',
    ],
    conventions: [
      'Collinear velocities along the x-axis; v is the velocity of S prime relative to S.',
    ],
    keywords: ['velocity addition', 'relativity', 'lorentz', 'speed of light'],
  },
];
