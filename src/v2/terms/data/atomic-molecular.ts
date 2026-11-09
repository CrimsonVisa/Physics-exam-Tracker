import type { TermEntry } from '../types';
import { CSIR, GATE, TEXTBOOKS, TIFR, link, textbook } from './shared';

const atomicLinks = () => [
  link('csir-net-physics', 'implicit', [CSIR.atomic]),
  link('gate-physics', 'implicit', [GATE.atomic]),
  link(
    'tifr-gs-physics',
    'uncertain',
    [TIFR.modern],
    'TIFR lists Modern Physics broadly; this is not claimed as an explicit TIFR topic.',
  ),
];

const molecularLinks = () => [
  link('csir-net-physics', 'implicit', [CSIR.molecularSpectra]),
  link('gate-physics', 'implicit', [GATE.molecularSpectra]),
  link(
    'tifr-gs-physics',
    'uncertain',
    [TIFR.modern],
    'TIFR lists Modern Physics broadly; this is not claimed as an explicit TIFR topic.',
  ),
];

export const ATOMIC_MOLECULAR_TERMS: TermEntry[] = [
  {
    id: 'atomic-molecular-term-symbol',
    term: 'term symbol',
    kind: 'symbol',
    subject: 'atomic-molecular',
    topic: 'Atomic structure',
    definition:
      'A spectroscopic notation for an atomic level that encodes its total spin, orbital angular momentum and total angular momentum.',
    symbol: String.raw`^{2S+1}L_J`,
    symbolPlainText: '^(2S+1)L_J',
    notationVariants: [String.raw`^{2S+1}L_J`, String.raw`n^{2S+1}L_J`],
    conditions: [
      'L is written as S, P, D, F, ... for L = 0, 1, 2, 3, ...',
      'The superscript gives the spin multiplicity 2S + 1.',
    ],
    relatedTermIds: ['atomic-molecular-fine-structure', 'atomic-molecular-selection-rule'],
    syllabusLinks: atomicLinks(),
    sources: [textbook(TEXTBOOKS.foot, 'Ch. 5'), textbook(TEXTBOOKS.bransdenJoachain, 'Ch. 4')],
    keywords: ['term symbol', 'spectroscopic notation', 'LS coupling', 'multiplicity'],
  },
  {
    id: 'atomic-molecular-selection-rule',
    term: 'selection rule',
    kind: 'concept',
    subject: 'atomic-molecular',
    topic: 'Atomic transitions',
    definition:
      'A condition on quantum numbers that determines whether a transition is allowed in a given radiative or collision process.',
    symbol: String.raw`\Delta l=\pm1`,
    symbolPlainText: 'Delta l = +/-1',
    conditions: [
      'The displayed rule is the electric-dipole rule for a one-electron atom.',
      'Selection rules depend on the transition operator and on the coupling scheme.',
    ],
    relatedTermIds: ['atomic-molecular-term-symbol', 'atomic-molecular-fine-structure'],
    relatedFormulaIds: ['atomic-molecular-einstein-coefficients'],
    syllabusLinks: atomicLinks(),
    sources: [textbook(TEXTBOOKS.bransdenJoachain, 'Ch. 4'), textbook(TEXTBOOKS.foot, 'Ch. 7')],
    keywords: ['selection rule', 'electric dipole', 'allowed transition', 'quantum numbers'],
  },
  {
    id: 'atomic-molecular-fine-structure',
    term: 'fine structure',
    kind: 'concept',
    subject: 'atomic-molecular',
    topic: 'Atomic structure',
    definition:
      'The splitting of atomic levels due to relativistic corrections, spin-orbit coupling and the Darwin term.',
    conditions: [
      'Fine structure is smaller than the gross non-relativistic level spacing.',
      'The splitting depends on the nuclear charge and the quantum numbers of the level.',
    ],
    relatedTermIds: ['atomic-molecular-term-symbol', 'atomic-molecular-hyperfine-structure'],
    syllabusLinks: atomicLinks(),
    sources: [textbook(TEXTBOOKS.foot, 'Ch. 2'), textbook(TEXTBOOKS.bransdenJoachain, 'Ch. 5')],
    keywords: ['fine structure', 'spin-orbit', 'relativistic correction', 'Darwin term'],
  },
  {
    id: 'atomic-molecular-hyperfine-structure',
    term: 'hyperfine structure',
    kind: 'concept',
    subject: 'atomic-molecular',
    topic: 'Atomic structure',
    definition:
      'The splitting of atomic levels due to the interaction of the electron angular momentum with the nuclear spin and magnetic moment, together with the electric-quadrupole interaction when the nucleus has a nonspherical charge distribution.',
    conditions: [
      'Hyperfine splitting is typically much smaller than fine structure.',
      'The magnetic-dipole interaction requires a nonzero nuclear spin quantum number.',
      'The electric-quadrupole interaction requires nuclear spin I >= 1 and a nonspherical nuclear charge distribution.',
    ],
    relatedTermIds: ['atomic-molecular-fine-structure', 'atomic-molecular-isotope-shift'],
    syllabusLinks: atomicLinks(),
    sources: [textbook(TEXTBOOKS.foot, 'Ch. 6'), textbook(TEXTBOOKS.bransdenJoachain, 'Ch. 5')],
    keywords: ['hyperfine structure', 'nuclear spin', 'magnetic moment', 'isotope'],
  },
  {
    id: 'atomic-molecular-isotope-shift',
    term: 'isotope shift',
    kind: 'quantity',
    subject: 'atomic-molecular',
    topic: 'Atomic structure',
    definition:
      'The shift of atomic spectral lines between isotopes of the same element, arising from nuclear mass and nuclear charge-radius differences.',
    symbol: String.raw`\delta\nu`,
    symbolPlainText: 'delta nu',
    siUnit: 'Hz',
    siUnitLatex: String.raw`\mathrm{Hz}`,
    dimensions: 'T^-1',
    dimensionFormula: String.raw`[\delta\nu]=T^{-1}`,
    conditions: [
      'The shift combines a mass shift and a field shift.',
      'Isotope shifts are measured relative to a chosen reference isotope.',
    ],
    relatedTermIds: ['atomic-molecular-hyperfine-structure'],
    syllabusLinks: atomicLinks(),
    sources: [textbook(TEXTBOOKS.foot, 'Ch. 6'), textbook(TEXTBOOKS.bransdenJoachain, 'Ch. 5')],
    keywords: ['isotope shift', 'mass shift', 'field shift', 'spectral line'],
  },
  {
    id: 'atomic-molecular-rotational-constant',
    term: 'rotational constant',
    kind: 'quantity',
    subject: 'atomic-molecular',
    topic: 'Molecular spectra',
    definition:
      'The energy scale of a rigid rotor, equal to hbar squared divided by twice the moment of inertia.',
    symbol: 'B',
    symbolPlainText: 'B',
    siUnit: 'J',
    siUnitLatex: String.raw`\mathrm{J}`,
    dimensions: 'M L^2 T^-2',
    dimensionFormula: String.raw`[B]=M L^{2} T^{-2}`,
    conditions: [
      'The rigid-rotor expression assumes a fixed bond length.',
      'Spectroscopic rotational constants are often quoted in wavenumbers or frequency units.',
    ],
    relatedTermIds: ['classical-mechanics-reduced-mass', 'atomic-molecular-vibrational-frequency'],
    relatedFormulaIds: ['atomic-molecular-rigid-rotor-energy'],
    syllabusLinks: molecularLinks(),
    sources: [
      textbook(TEXTBOOKS.bransdenJoachain, 'Ch. 11'),
      textbook(TEXTBOOKS.demtroder, 'Rotational spectra'),
    ],
    keywords: ['rotational constant', 'rigid rotor', 'moment of inertia', 'molecular spectra'],
  },
  {
    id: 'atomic-molecular-vibrational-frequency',
    term: 'vibrational frequency',
    kind: 'quantity',
    subject: 'atomic-molecular',
    topic: 'Molecular spectra',
    definition:
      'The frequency of the normal vibrational mode of a molecule in the harmonic approximation.',
    symbol: String.raw`\nu_{\mathrm{vib}}`,
    symbolPlainText: 'nu_vib',
    siUnit: 'Hz',
    siUnitLatex: String.raw`\mathrm{Hz}`,
    dimensions: 'T^-1',
    dimensionFormula: String.raw`[\nu_{\mathrm{vib}}]=T^{-1}`,
    conditions: [
      'The harmonic approximation assumes a quadratic potential about the equilibrium geometry.',
      'Anharmonicity and vibration-rotation coupling shift the observed frequencies.',
    ],
    relatedTermIds: ['atomic-molecular-rotational-constant'],
    syllabusLinks: molecularLinks(),
    sources: [
      textbook(TEXTBOOKS.bransdenJoachain, 'Ch. 11'),
      textbook(TEXTBOOKS.demtroder, 'Vibration-Rotation Transitions'),
    ],
    keywords: ['vibrational frequency', 'normal mode', 'harmonic oscillator', 'molecular spectra'],
  },
];
