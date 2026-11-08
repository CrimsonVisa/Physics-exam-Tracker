import type { FormulaEntry } from '../types';
import { CSIR, GATE, JEST, TIFR, link } from './syllabus';
import { TEXTBOOKS, textbook } from './sources';

export const OPTICS_FORMULAS: FormulaEntry[] = [
  {
    id: 'optics-single-slit-minima',
    title: 'Single-slit diffraction minima',
    latex: String.raw`a\sin\theta=m\lambda,\qquad m=\pm1,\pm2,\ldots`,
    plainText: 'a sin(theta) = m lambda, m = +/-1, +/-2, ...',
    subject: 'optics',
    topic: 'Diffraction',
    subtopic: 'Single-slit minima',
    summary:
      'The directions of destructive interference for Fraunhofer diffraction from a single slit of width a.',
    symbols: [
      { symbol: 'a', meaning: 'slit width', unit: 'm', dimensions: 'L' },
      {
        symbol: String.raw`\theta`,
        meaning: 'diffraction angle measured from the forward direction',
        unit: 'rad',
      },
      { symbol: 'm', meaning: 'nonzero integer diffraction order' },
      { symbol: String.raw`\lambda`, meaning: 'wavelength', unit: 'm', dimensions: 'L' },
    ],
    conditions: [
      'Fraunhofer (far-field) diffraction.',
      'Monochromatic plane-wave illumination of a uniform single slit.',
      'Scalar diffraction theory and a distant observation screen.',
      'The equation gives minima, not the intensity envelope maxima.',
    ],
    exactness: 'special-case',
    framework: 'classical',
    syllabusLinks: [
      link('csir-net-physics', 'implicit', [CSIR.diffraction]),
      link('gate-physics', 'implicit', [GATE.diffraction]),
      link(
        'tifr-gs-physics',
        'uncertain',
        [TIFR.em],
        'TIFR lists only broad areas; optics placement under Electricity and Magnetism is not explicitly documented.',
      ),
      link('jest-physics', 'implicit', [JEST.optics]),
    ],
    sources: [textbook(TEXTBOOKS.hecht, 'Ch. 10'), textbook(TEXTBOOKS.bornWolf, 'Ch. 8')],
    relatedIds: ['optics-diffraction-grating-equation', 'condensed-matter-bragg-law'],
    specialCases: ['The central maximum lies between the m = -1 and m = +1 minima.'],
    conventions: ['Theta is measured from the forward direction; m excludes zero.'],
    keywords: ['single slit', 'diffraction', 'minima', 'fraunhofer', 'wavelength'],
  },
  {
    id: 'optics-diffraction-grating-equation',
    title: 'Diffraction grating equation (normal incidence)',
    latex: String.raw`d\sin\theta_m=m\lambda`,
    plainText: 'd sin(theta_m) = m lambda',
    subject: 'optics',
    topic: 'Diffraction',
    subtopic: 'Diffraction grating',
    summary:
      'The directions of principal maxima for monochromatic light incident normally on a transmission grating.',
    symbols: [
      {
        symbol: 'd',
        meaning: 'grating spacing between adjacent slits',
        unit: 'm',
        dimensions: 'L',
      },
      {
        symbol: String.raw`\theta_m`,
        meaning: 'diffraction angle for order m',
        unit: 'rad',
      },
      { symbol: 'm', meaning: 'integer diffraction order' },
      { symbol: String.raw`\lambda`, meaning: 'wavelength', unit: 'm', dimensions: 'L' },
    ],
    conditions: [
      'Normal incidence on a transmission grating.',
      'Monochromatic light and Fraunhofer observation.',
      'Angles are measured from the normal.',
      'The grating is ideal and the blaze envelope is not included.',
    ],
    exactness: 'exact',
    framework: 'classical',
    syllabusLinks: [
      link('csir-net-physics', 'implicit', [CSIR.diffraction]),
      link('gate-physics', 'explicit', [GATE.grating]),
      link(
        'tifr-gs-physics',
        'uncertain',
        [TIFR.em],
        'TIFR lists only broad areas; optics placement under Electricity and Magnetism is not explicitly documented.',
      ),
      link('jest-physics', 'implicit', [JEST.optics]),
    ],
    sources: [textbook(TEXTBOOKS.hecht, 'Ch. 10'), textbook(TEXTBOOKS.bornWolf, 'Ch. 8')],
    relatedIds: ['optics-single-slit-minima'],
    specialCases: [
      'For oblique incidence, replace d sin(theta_m) with d(sin(theta_i) + sin(theta_m)) with a stated sign convention.',
    ],
    conventions: ['Normal incidence; theta_m is measured from the grating normal.'],
    keywords: ['grating', 'diffraction', 'orders', 'normal incidence', 'wavelength'],
  },
  {
    id: 'optics-malus-law',
    title: 'Malus law',
    latex: String.raw`I=I_0\cos^2\theta`,
    plainText: 'I = I_0 cos^2(theta)',
    subject: 'optics',
    topic: 'Polarization',
    subtopic: 'Malus law',
    summary:
      'The transmitted intensity when linearly polarized light passes through an ideal linear polarizer.',
    symbols: [
      { symbol: 'I', meaning: 'transmitted intensity', unit: 'W m^-2', dimensions: 'M T^-3' },
      {
        symbol: String.raw`I_0`,
        meaning: 'incident intensity of the linearly polarized light before the polarizer',
        unit: 'W m^-2',
        dimensions: 'M T^-3',
      },
      {
        symbol: String.raw`\theta`,
        meaning: 'angle between the incident polarization and the polarizer axis',
        unit: 'rad',
      },
    ],
    conditions: [
      'Ideal linear polarizer with no absorption or reflection loss.',
      'Incident light is linearly polarized.',
      'No depolarization or birefringence in the optical path.',
      'Theta is measured between the polarization direction and the transmission axis.',
    ],
    exactness: 'exact',
    framework: 'classical',
    syllabusLinks: [
      link('csir-net-physics', 'implicit', [CSIR.polarization]),
      link('gate-physics', 'implicit', [GATE.polarization]),
      link(
        'tifr-gs-physics',
        'uncertain',
        [TIFR.em],
        'TIFR lists only broad areas; optics placement under Electricity and Magnetism is not explicitly documented.',
      ),
      link('jest-physics', 'implicit', [JEST.optics]),
    ],
    sources: [textbook(TEXTBOOKS.hecht, 'Ch. 8'), textbook(TEXTBOOKS.bornWolf, 'Ch. 1')],
    relatedIds: [],
    specialCases: ['Parallel axes: theta = 0 and I = I0.', 'Crossed axes: theta = pi/2 and I = 0.'],
    conventions: [
      'Theta is measured between the incident polarization and the polarizer transmission axis.',
    ],
    keywords: ['malus', 'polarizer', 'polarization', 'intensity', 'cosine'],
  },
];
