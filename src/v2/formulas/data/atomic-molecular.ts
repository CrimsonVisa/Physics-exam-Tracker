import type { FormulaEntry } from '../types';
import { CSIR, GATE, TIFR, link } from './syllabus';
import { TEXTBOOKS, textbook } from './sources';

export const ATOMIC_MOLECULAR_FORMULAS: FormulaEntry[] = [
  {
    id: 'atomic-molecular-rydberg-formula',
    title: 'Rydberg formula for hydrogen-like atoms',
    latex: String.raw`\frac{1}{\lambda}=Z^2R_M\left(\frac{1}{n_1^2}-\frac{1}{n_2^2}\right)`,
    plainText: '1/lambda = Z^2 R_M (1/n1^2 - 1/n2^2)',
    subject: 'atomic-molecular',
    topic: 'Atomic spectra',
    subtopic: 'Hydrogen-like Rydberg formula',
    summary:
      'Gives the vacuum wavelengths of electric-dipole transitions in a non-relativistic hydrogen-like atom with nuclear charge Z.',
    symbols: [
      { symbol: String.raw`\lambda`, meaning: 'vacuum wavelength', unit: 'm', dimensions: 'L' },
      {
        symbol: 'Z',
        meaning: 'nuclear charge number, with nuclear charge Ze',
      },
      {
        symbol: String.raw`R_M`,
        meaning: 'reduced-mass Rydberg constant for the electron-nucleus system',
        unit: 'm^-1',
        dimensions: 'L^-1',
      },
      { symbol: String.raw`n_1`, meaning: 'lower principal quantum number' },
      { symbol: String.raw`n_2`, meaning: 'upper principal quantum number, n2 > n1' },
    ],
    conditions: [
      'Hydrogen-like atom with one electron and a point nucleus of charge Ze.',
      'Electric-dipole transitions and the non-relativistic Bohr model.',
      'Fine and hyperfine structure are neglected.',
      'R_M uses the reduced mass of the electron-nucleus system; the infinite-nuclear-mass Rydberg constant is a different convention.',
      'For hydrogen, Z = 1 and R_M is approximately the hydrogen Rydberg constant R_H.',
    ],
    exactness: 'approximate',
    framework: 'quantum',
    syllabusLinks: [
      link('csir-net-physics', 'implicit', [CSIR.atomic]),
      link('gate-physics', 'explicit', [GATE.atomicSpectra]),
      link(
        'tifr-gs-physics',
        'uncertain',
        [TIFR.modern],
        'TIFR lists Modern Physics broadly; this is not claimed as an explicit TIFR topic.',
      ),
    ],
    sources: [textbook(TEXTBOOKS.bransdenJoachain, 'Ch. 1'), textbook(TEXTBOOKS.foot, 'Ch. 2')],
    relatedIds: ['quantum-mechanics-hydrogen-energy-levels'],
    specialCases: [
      'For hydrogen, Z = 1 and R_M is approximately R_H.',
      'In the infinite-nuclear-mass approximation, R_M becomes the infinite-mass Rydberg constant.',
      'Lyman, Balmer, and Paschen series correspond to n1 = 1, 2, and 3.',
    ],
    conventions: [
      'Wavelength is in vacuum; n2 > n1.',
      'R_M includes the reduced-mass correction; R_H is the hydrogen-specific value.',
    ],
    keywords: [
      'rydberg',
      'hydrogen',
      'hydrogen-like',
      'spectrum',
      'wavelength',
      'balmer',
      'lyman',
      'Z',
    ],
  },
  {
    id: 'atomic-molecular-rigid-rotor-energy',
    title: 'Rigid-rotor rotational energy',
    latex: String.raw`E_J=\frac{\hbar^2}{2I}J(J+1),\qquad J=0,1,2,\ldots`,
    plainText: 'E_J = (hbar^2/(2I)) J(J+1)',
    subject: 'atomic-molecular',
    topic: 'Molecular spectra',
    subtopic: 'Rigid rotor',
    summary:
      'The quantized rotational energy levels of a rigid diatomic molecule with moment of inertia I.',
    symbols: [
      {
        symbol: String.raw`E_J`,
        meaning: 'rotational energy level',
        unit: 'J',
        dimensions: 'M L^2 T^-2',
      },
      {
        symbol: 'I',
        meaning: 'moment of inertia about the rotational axis',
        unit: 'kg m^2',
        dimensions: 'M L^2',
      },
      { symbol: 'J', meaning: 'rotational quantum number, J = 0, 1, 2, ...' },
      {
        symbol: String.raw`\hbar`,
        meaning: 'reduced Planck constant',
        unit: 'J s',
        dimensions: 'M L^2 T^-1',
      },
    ],
    conditions: [
      'Rigid diatomic rotor with fixed bond length.',
      'Centrifugal distortion and vibration-rotation coupling are neglected.',
      'The molecule is isolated and in the gas phase.',
      'The rotational axis passes through the centre of mass.',
    ],
    exactness: 'approximate',
    framework: 'quantum',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.molecularSpectra]),
      link('gate-physics', 'explicit', [GATE.molecularSpectra]),
      link(
        'tifr-gs-physics',
        'uncertain',
        [TIFR.modern],
        'TIFR lists Modern Physics broadly; this is not claimed as an explicit TIFR topic.',
      ),
    ],
    sources: [
      textbook(TEXTBOOKS.bransdenJoachain, 'Ch. 10, §10.1'),
      textbook(TEXTBOOKS.demtroder, 'Rotational spectra'),
    ],
    relatedIds: ['atomic-molecular-rydberg-formula'],
    specialCases: [
      'For a diatomic molecule, I = mu r0^2, where mu is the reduced mass and r0 the bond length.',
    ],
    conventions: [
      'J is a non-negative integer; degeneracy is 2J + 1 in the absence of external fields.',
    ],
    keywords: ['rigid rotor', 'rotational', 'molecular spectra', 'moment of inertia', 'J'],
  },
  {
    id: 'atomic-molecular-einstein-coefficients',
    title: 'Einstein A and B coefficients',
    latex: String.raw`A_{21}=\frac{8\pi h\nu^3}{c^3}B_{21},\qquad g_1B_{12}=g_2B_{21}`,
    plainText: 'A21 = (8 pi h nu^3 / c^3) B21; g1 B12 = g2 B21',
    subject: 'atomic-molecular',
    topic: 'Radiation and lasers',
    subtopic: 'Einstein coefficients',
    summary:
      'Relates spontaneous and stimulated transition rates for a two-level atomic system in an isotropic radiation field.',
    symbols: [
      {
        symbol: String.raw`A_{21}`,
        meaning: 'spontaneous emission rate from level 2 to 1',
        unit: 's^-1',
        dimensions: 'T^-1',
      },
      {
        symbol: String.raw`B_{21}`,
        meaning: 'stimulated emission coefficient under the per-frequency convention',
        unit: 'm^3 J^-1 s^-2',
        dimensions: 'M^-1 L',
      },
      {
        symbol: String.raw`B_{12}`,
        meaning: 'absorption coefficient under the per-frequency convention',
        unit: 'm^3 J^-1 s^-2',
        dimensions: 'M^-1 L',
      },
      { symbol: String.raw`g_1, g_2`, meaning: 'statistical weights of levels 1 and 2' },
      { symbol: String.raw`\nu`, meaning: 'transition frequency', unit: 'Hz', dimensions: 'T^-1' },
      { symbol: 'h', meaning: 'Planck constant', unit: 'J s', dimensions: 'M L^2 T^-1' },
      { symbol: 'c', meaning: 'speed of light', unit: 'm s^-1', dimensions: 'L T^-1' },
    ],
    conditions: [
      'Two-level atom in an isotropic radiation field.',
      'Thermal equilibrium or steady-state rate balance.',
      'The radiation field is broadband on the scale of the linewidth unless a line-shape function is included.',
      'Coherence and nonlinear propagation effects are neglected.',
    ],
    exactness: 'exact',
    framework: 'quantum',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.lasersEinstein]),
      link('gate-physics', 'explicit', [GATE.lasers]),
      link(
        'tifr-gs-physics',
        'uncertain',
        [TIFR.modern],
        'TIFR lists Modern Physics broadly; this is not claimed as an explicit TIFR topic.',
      ),
    ],
    sources: [textbook(TEXTBOOKS.foot, 'Ch. 7'), textbook(TEXTBOOKS.bransdenJoachain, 'Ch. 4')],
    relatedIds: ['thermodynamics-planck-blackbody'],
    specialCases: ['For non-degenerate levels, g1 = g2 = 1 and B12 = B21.'],
    conventions: [
      'A21 is the spontaneous rate; B12 and B21 are defined with respect to the spectral energy density per unit frequency, so that the stimulated rate is B times that density.',
      'The numerical factor 8 pi h nu^3 / c^3 applies to this per-frequency convention. A per-angular-frequency convention changes the numerical factor and must not be mixed with this entry.',
      'The stated B units, m^3 J^-1 s^-2, apply specifically to the per-frequency convention.',
    ],
    keywords: [
      'einstein coefficients',
      'spontaneous emission',
      'stimulated emission',
      'laser',
      'transition',
    ],
  },
];
