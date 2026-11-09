import type { TermEntry } from '../types';
import { CSIR, GATE, JEST, TEXTBOOKS, TIFR, link, textbook } from './shared';

const opticsLinks = () => [
  link('csir-net-physics', 'implicit', [CSIR.diffraction]),
  link('gate-physics', 'implicit', [GATE.optics]),
  link(
    'tifr-gs-physics',
    'uncertain',
    [TIFR.em],
    'TIFR lists only broad areas; optics placement under Electricity and Magnetism is not explicitly documented.',
  ),
  link('jest-physics', 'implicit', [JEST.optics]),
];

export const OPTICS_TERMS: TermEntry[] = [
  {
    id: 'optics-wavefront',
    term: 'wavefront',
    kind: 'concept',
    subject: 'optics',
    topic: 'Wave optics',
    definition: 'A surface of constant phase in a propagating wave field.',
    conditions: [
      'A wavefront is perpendicular to the local propagation direction in an isotropic medium.',
      'The wavefront shape changes on reflection, refraction and diffraction.',
    ],
    relatedTermIds: ['optics-coherence', 'optics-refractive-index'],
    syllabusLinks: opticsLinks(),
    sources: [textbook(TEXTBOOKS.hecht, 'Ch. 2'), textbook(TEXTBOOKS.bornWolf, 'Ch. 1')],
    keywords: ['wavefront', 'constant phase', 'propagation', 'optics'],
  },
  {
    id: 'optics-coherence',
    term: 'coherence',
    kind: 'concept',
    subject: 'optics',
    topic: 'Wave optics',
    definition:
      'The degree to which two waves or two samples of a wave maintain a predictable phase relationship.',
    conditions: [
      'Temporal coherence is related to spectral bandwidth; spatial coherence describes phase correlation across a wavefront.',
      'Interference visibility is limited by the coherence of the sources.',
    ],
    relatedTermIds: ['optics-wavefront', 'optics-polarization'],
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.diffraction]),
      link('gate-physics', 'implicit', [GATE.optics]),
      link(
        'tifr-gs-physics',
        'uncertain',
        [TIFR.em],
        'TIFR lists only broad areas; optics placement under Electricity and Magnetism is not explicitly documented.',
      ),
      link('jest-physics', 'implicit', [JEST.optics]),
    ],
    sources: [textbook(TEXTBOOKS.hecht, 'Ch. 7'), textbook(TEXTBOOKS.bornWolf, 'Ch. 10')],
    keywords: ['coherence', 'temporal coherence', 'spatial coherence', 'interference'],
  },
  {
    id: 'optics-polarization',
    term: 'polarization',
    kind: 'concept',
    subject: 'optics',
    topic: 'Polarization',
    definition:
      'The polarization state of an electromagnetic wave, determined by the trajectory traced by the electric-field vector at a fixed point; the state may be linear, circular or elliptical.',
    symbol: String.raw`\hat{\boldsymbol{\epsilon}}`,
    symbolPlainText: 'epsilon-hat',
    conditions: [
      'The polarization state can be linear, circular or elliptical.',
      'A polarizer selects a component along its transmission axis.',
    ],
    relatedTermIds: ['optics-wavefront', 'optics-refractive-index'],
    relatedFormulaIds: ['optics-malus-law'],
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.polarization]),
      link('gate-physics', 'explicit', [GATE.polarization]),
      link(
        'tifr-gs-physics',
        'uncertain',
        [TIFR.em],
        'TIFR lists only broad areas; optics placement under Electricity and Magnetism is not explicitly documented.',
      ),
      link('jest-physics', 'explicit', [JEST.optics]),
    ],
    sources: [textbook(TEXTBOOKS.hecht, 'Ch. 8'), textbook(TEXTBOOKS.bornWolf, 'Ch. 1')],
    keywords: ['polarization', 'electric field', 'linear', 'circular', 'elliptical'],
  },
  {
    id: 'optics-refractive-index',
    term: 'refractive index',
    kind: 'quantity',
    subject: 'optics',
    topic: 'Propagation in media',
    definition:
      'The ratio of the speed of light in vacuum to the phase speed in a medium, usually quoted for a specified frequency.',
    symbol: 'n',
    symbolPlainText: 'n',
    siUnit: '1',
    dimensions: '1',
    dimensionFormula: String.raw`[n]=1`,
    conditions: [
      'The refractive index is frequency-dependent in a dispersive medium.',
      'The real part governs phase speed and the imaginary part describes absorption.',
    ],
    relatedTermIds: ['optics-wavefront', 'optics-coherence'],
    syllabusLinks: opticsLinks(),
    sources: [textbook(TEXTBOOKS.hecht, 'Ch. 3'), textbook(TEXTBOOKS.bornWolf, 'Ch. 1')],
    keywords: ['refractive index', 'speed of light', 'dispersion', 'absorption'],
  },
  {
    id: 'optics-diffraction-order',
    term: 'diffraction order',
    kind: 'notation',
    subject: 'optics',
    topic: 'Diffraction',
    definition:
      'The integer label attached to a principal maximum or minimum in a diffraction pattern according to its path difference.',
    symbol: 'm',
    symbolPlainText: 'm',
    conditions: [
      'The order is an integer; its sign labels the direction of the diffracted beam.',
      'The allowed range of orders is limited by the grating spacing and wavelength.',
    ],
    relatedTermIds: ['optics-resolving-power'],
    relatedFormulaIds: ['optics-single-slit-minima', 'optics-diffraction-grating-equation'],
    syllabusLinks: opticsLinks(),
    sources: [textbook(TEXTBOOKS.hecht, 'Ch. 10'), textbook(TEXTBOOKS.bornWolf, 'Ch. 8')],
    keywords: ['diffraction order', 'order m', 'grating', 'path difference'],
  },
  {
    id: 'optics-resolving-power',
    term: 'resolving power',
    kind: 'quantity',
    subject: 'optics',
    topic: 'Diffraction',
    definition:
      'The ability of an optical instrument or grating to distinguish closely spaced wavelengths or objects, often expressed as a dimensionless ratio.',
    symbol: 'R',
    symbolPlainText: 'R',
    siUnit: '1',
    dimensions: '1',
    dimensionFormula: String.raw`[R]=1`,
    conditions: [
      'The definition depends on the instrument: R = lambda/Delta lambda for a grating or R = D/1.22 lambda for a circular aperture.',
      'The Rayleigh criterion is one common resolution convention.',
    ],
    relatedTermIds: ['optics-diffraction-order'],
    syllabusLinks: opticsLinks(),
    sources: [textbook(TEXTBOOKS.hecht, 'Ch. 10'), textbook(TEXTBOOKS.bornWolf, 'Ch. 8')],
    keywords: ['resolving power', 'resolution', 'Rayleigh criterion', 'grating', 'aperture'],
  },
];
