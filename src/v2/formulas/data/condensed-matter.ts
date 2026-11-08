import type { FormulaEntry } from '../types';
import { CSIR, GATE, TIFR, link } from './syllabus';
import { TEXTBOOKS, textbook } from './sources';

export const CONDENSED_MATTER_FORMULAS: FormulaEntry[] = [
  {
    id: 'condensed-matter-fermi-energy-3d',
    title: 'Three-dimensional free-electron Fermi energy',
    latex: String.raw`E_F=\frac{\hbar^2}{2m}\left(3\pi^2n\right)^{2/3}`,
    plainText: 'E_F = (hbar^2/(2m)) (3 pi^2 n)^(2/3)',
    subject: 'condensed-matter',
    topic: 'Free-electron theory',
    subtopic: 'Fermi energy',
    summary:
      'The zero-temperature Fermi energy of a three-dimensional, non-interacting electron gas in terms of the electron density.',
    symbols: [
      {
        symbol: String.raw`E_F`,
        meaning: 'Fermi energy',
        unit: 'J',
        dimensions: 'M L^2 T^-2',
      },
      {
        symbol: String.raw`\hbar`,
        meaning: 'reduced Planck constant',
        unit: 'J s',
        dimensions: 'M L^2 T^-1',
      },
      { symbol: 'm', meaning: 'electron mass or effective mass', unit: 'kg', dimensions: 'M' },
      {
        symbol: 'n',
        meaning: 'electron number density',
        unit: 'm^-3',
        dimensions: 'L^-3',
      },
    ],
    conditions: [
      'Three-dimensional free-electron gas at zero temperature.',
      'Non-relativistic, isotropic, non-interacting electrons.',
      'Spin degeneracy is included in the factor 3 pi^2 n.',
      'An effective mass may replace m in a band-structure approximation.',
    ],
    exactness: 'exact',
    framework: 'quantum',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.freeElectron]),
      link('gate-physics', 'explicit', [GATE.freeElectron]),
      link(
        'tifr-gs-physics',
        'uncertain',
        [TIFR.modern],
        'TIFR lists Modern Physics broadly; this is not claimed as an explicit TIFR topic.',
      ),
    ],
    sources: [textbook(TEXTBOOKS.kittel, 'Ch. 6'), textbook(TEXTBOOKS.ashcroftMermin, 'Ch. 2')],
    relatedIds: ['thermodynamics-fermi-dirac-distribution', 'condensed-matter-drude-conductivity'],
    specialCases: [
      'For a two-dimensional electron gas, the density of states and Fermi energy have a different density dependence.',
    ],
    conventions: ['The factor 3 pi^2 n assumes spin-1/2 electrons in three dimensions.'],
    keywords: ['fermi energy', 'free electron', 'electron gas', 'density of states', 'solid state'],
  },
  {
    id: 'condensed-matter-drude-conductivity',
    title: 'Drude electrical conductivity',
    latex: String.raw`\sigma=\frac{ne^2\tau}{m^*}`,
    plainText: 'sigma = n e^2 tau / m_star',
    subject: 'condensed-matter',
    topic: 'Free-electron and Drude models',
    subtopic: 'DC conductivity',
    summary:
      'The Drude model expresses the DC electrical conductivity of a free-electron gas in terms of carrier density, charge, relaxation time, and effective mass.',
    symbols: [
      {
        symbol: String.raw`\sigma`,
        meaning: 'electrical conductivity',
        unit: 'S m^-1',
        dimensions: 'M^-1 L^-3 T^3 I^2',
      },
      { symbol: 'n', meaning: 'carrier number density', unit: 'm^-3', dimensions: 'L^-3' },
      { symbol: 'e', meaning: 'elementary charge magnitude', unit: 'C', dimensions: 'T I' },
      {
        symbol: String.raw`\tau`,
        meaning: 'mean free time / relaxation time',
        unit: 's',
        dimensions: 'T',
      },
      { symbol: String.raw`m^*`, meaning: 'effective mass', unit: 'kg', dimensions: 'M' },
    ],
    conditions: [
      'Drude free-electron model with independent electrons.',
      'Steady-state, low-frequency (DC) limit.',
      'Isotropic, constant relaxation time.',
      'No magnetic field and no strong electron-electron or electron-phonon corrections.',
    ],
    exactness: 'approximate',
    framework: 'classical',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.drude]),
      link('gate-physics', 'implicit', [GATE.conductivity]),
      link(
        'tifr-gs-physics',
        'uncertain',
        [TIFR.modern],
        'TIFR lists Modern Physics broadly; this is not claimed as an explicit TIFR topic.',
      ),
    ],
    sources: [textbook(TEXTBOOKS.kittel, 'Ch. 6'), textbook(TEXTBOOKS.ashcroftMermin, 'Ch. 1')],
    relatedIds: ['condensed-matter-fermi-energy-3d'],
    specialCases: [
      'Mobility is mu = e tau / m*.',
      'Wiedemann-Franz relates electronic thermal and electrical conductivities under the Drude model.',
    ],
    conventions: [
      'e is the magnitude of the electron charge; the sign is carried by the carrier charge.',
    ],
    keywords: ['drude', 'conductivity', 'relaxation time', 'mobility', 'free electron', 'sigma'],
  },
  {
    id: 'condensed-matter-bragg-law',
    title: 'Bragg law',
    latex: String.raw`2d\sin\theta=n\lambda`,
    plainText: '2 d sin(theta) = n lambda',
    subject: 'condensed-matter',
    topic: 'Crystal diffraction',
    subtopic: 'Bragg law',
    summary:
      'Constructive interference from parallel crystal planes occurs when the path difference between adjacent planes is an integer number of wavelengths.',
    symbols: [
      {
        symbol: 'd',
        meaning: 'interplanar spacing',
        unit: 'm',
        dimensions: 'L',
      },
      {
        symbol: String.raw`\theta`,
        meaning: 'Bragg angle measured from the lattice planes',
        unit: 'rad',
      },
      { symbol: 'n', meaning: 'positive integer diffraction order' },
      {
        symbol: String.raw`\lambda`,
        meaning: 'wavelength',
        unit: 'm',
        dimensions: 'L',
      },
    ],
    conditions: [
      'Kinematic (single-scattering) diffraction from a periodic set of parallel lattice planes.',
      'Specular reflection from each plane; multiple scattering, dynamical diffraction, refraction, and strong-scattering corrections are neglected.',
      'Theta is measured from the lattice planes, not from the surface normal.',
      'The wavelength is comparable to the interplanar spacing.',
      'The equation is exact within the kinematic approximation; it is approximate for real crystals when dynamical effects are significant.',
    ],
    exactness: 'exact',
    framework: 'general',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.diffractionStructure]),
      link('gate-physics', 'explicit', [GATE.crystallography]),
      link(
        'tifr-gs-physics',
        'uncertain',
        [TIFR.modern],
        'TIFR lists Modern Physics broadly; this is not claimed as an explicit TIFR topic.',
      ),
    ],
    sources: [textbook(TEXTBOOKS.kittel, 'Ch. 2'), textbook(TEXTBOOKS.ashcroftMermin, 'Ch. 6')],
    relatedIds: ['optics-single-slit-minima'],
    specialCases: ['For the first diffraction order, n = 1.'],
    conventions: ['Theta is the grazing angle relative to the reflecting planes.'],
    keywords: ['bragg', 'x-ray', 'crystal', 'diffraction', 'lattice', 'interplanar'],
  },
];
