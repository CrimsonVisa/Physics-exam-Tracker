import type { FormulaEntry } from '../types';
import { CSIR, GATE, TIFR, link } from './syllabus';
import { REFERENCES, TEXTBOOKS, reference, textbook } from './sources';

export const NUCLEAR_PARTICLE_FORMULAS: FormulaEntry[] = [
  {
    id: 'nuclear-particle-radioactive-decay-law',
    title: 'Radioactive decay law',
    latex: String.raw`N(t)=N_0e^{-\lambda t},\qquad T_{1/2}=\frac{\ln 2}{\lambda}`,
    plainText: 'N(t) = N0 exp(-lambda t), T1/2 = ln2/lambda',
    subject: 'nuclear-particle',
    topic: 'Radioactivity',
    subtopic: 'Decay law',
    summary:
      'Describes the exponential decrease of an unstable population when each nucleus decays independently with a constant decay constant.',
    symbols: [
      { symbol: 'N', meaning: 'number of surviving nuclei at time t' },
      { symbol: String.raw`N_0`, meaning: 'initial number of nuclei' },
      {
        symbol: String.raw`\lambda`,
        meaning: 'decay constant',
        unit: 's^-1',
        dimensions: 'T^-1',
      },
      {
        symbol: String.raw`T_{1/2}`,
        meaning: 'half-life',
        unit: 's',
        dimensions: 'T',
      },
      { symbol: 't', meaning: 'time', unit: 's', dimensions: 'T' },
    ],
    conditions: [
      'A homogeneous sample of independently decaying nuclei.',
      'The decay constant is constant in time.',
      'Production or ingrowth of the same species is neglected.',
      'The statistical limit of a large number of nuclei is assumed.',
    ],
    exactness: 'exact',
    framework: 'quantum',
    syllabusLinks: [
      link('csir-net-physics', 'implicit', [CSIR.nuclear]),
      link('gate-physics', 'implicit', [GATE.nuclear]),
      link(
        'tifr-gs-physics',
        'uncertain',
        [TIFR.modern],
        'TIFR lists Modern Physics broadly; this is not claimed as an explicit TIFR topic.',
      ),
    ],
    sources: [
      textbook(TEXTBOOKS.krane, 'Ch. 6'),
      reference(REFERENCES.pdg, 'Radioactive decay review'),
    ],
    relatedIds: ['nuclear-particle-reaction-q-value'],
    specialCases: ['After one half-life, N = N0/2; after n half-lives, N = N0/2^n.'],
    conventions: ['Lambda is the decay probability per unit time for a single nucleus.'],
    keywords: ['radioactivity', 'decay', 'half-life', 'exponential', 'lambda'],
  },
  {
    id: 'nuclear-particle-semi-empirical-mass-formula',
    title: 'Semi-empirical mass formula',
    latex: String.raw`B(A,Z)=a_V A-a_S A^{2/3}-a_C\frac{Z(Z-1)}{A^{1/3}}-a_A\frac{(A-2Z)^2}{A}+\delta(A,Z)`,
    plainText: 'B(A,Z)=aV A - aS A^(2/3) - aC Z(Z-1)/A^(1/3) - aA (A-2Z)^2/A + delta(A,Z)',
    subject: 'nuclear-particle',
    topic: 'Nuclear binding',
    subtopic: 'Liquid-drop mass formula',
    summary:
      'A symbolic liquid-drop parameterization of nuclear binding energy, including volume, surface, Coulomb, asymmetry, and pairing terms. The coefficients and pairing term are fit-dependent.',
    symbols: [
      {
        symbol: 'B',
        meaning: 'nuclear binding energy',
        unit: 'MeV',
        dimensions: 'M L^2 T^-2',
      },
      { symbol: 'A', meaning: 'mass number' },
      { symbol: 'Z', meaning: 'atomic number' },
      {
        symbol: String.raw`a_V,a_S,a_C,a_A`,
        meaning: 'volume, surface, Coulomb, and asymmetry coefficients',
        unit: 'MeV',
        dimensions: 'M L^2 T^-2',
      },
      {
        symbol: String.raw`\delta(A,Z)`,
        meaning: 'pairing term, with sign depending on even/odd nucleon numbers',
        unit: 'MeV',
        dimensions: 'M L^2 T^-2',
      },
    ],
    conditions: [
      'Liquid-drop model with a specified, self-consistent coefficient set.',
      'The coefficients a_V, a_S, a_C, and a_A are parameter-dependent and must be taken from one stated fit; this entry does not claim a unique numerical set.',
      'The same coefficient and pairing convention must be used for all terms.',
      'Pairing is commonly positive for even-even nuclei, negative for odd-odd nuclei, and zero for odd-A nuclei, but the sign and magnitude depend on the chosen fit.',
      'Shell effects, deformation, and light-nucleus cluster structure are not captured.',
      'The formula is an approximation, not an exact binding-energy law.',
    ],
    exactness: 'approximate',
    framework: 'general',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.semiEmpirical]),
      link('gate-physics', 'explicit', [GATE.semiEmpirical]),
      link(
        'tifr-gs-physics',
        'uncertain',
        [TIFR.modern],
        'TIFR lists Modern Physics broadly; this is not claimed as an explicit TIFR topic.',
      ),
    ],
    sources: [
      textbook(TEXTBOOKS.krane, 'Ch. 3'),
      reference(REFERENCES.pdg, 'Nuclear and particle physics review'),
    ],
    relatedIds: ['nuclear-particle-radioactive-decay-law'],
    specialCases: ['The pairing term favors even-even nuclei and penalizes odd-odd nuclei.'],
    conventions: [
      'This is a symbolic parameterization, not a unique numerical formula. Choose one coefficient set and state it with any numerical application.',
      'Pairing term convention must follow the same fit; do not mix pairing signs or coefficient values from different parameterizations.',
    ],
    keywords: ['semi-empirical', 'mass formula', 'liquid drop', 'binding energy', 'pairing'],
  },
  {
    id: 'nuclear-particle-reaction-q-value',
    title: 'Nuclear reaction Q-value',
    latex: String.raw`Q=(m_i-m_f)c^2`,
    plainText: 'Q = (m_initial - m_final) c^2',
    subject: 'nuclear-particle',
    topic: 'Nuclear reactions',
    subtopic: 'Q-value',
    summary:
      'The energy released or absorbed in a nuclear reaction, determined by the difference between total initial and final rest masses.',
    symbols: [
      {
        symbol: 'Q',
        meaning: 'reaction Q-value',
        unit: 'MeV',
        dimensions: 'M L^2 T^-2',
      },
      {
        symbol: String.raw`m_i`,
        meaning: 'total initial rest mass',
        unit: 'kg',
        dimensions: 'M',
      },
      {
        symbol: String.raw`m_f`,
        meaning: 'total final rest mass',
        unit: 'kg',
        dimensions: 'M',
      },
      { symbol: 'c', meaning: 'speed of light', unit: 'm s^-1', dimensions: 'L T^-1' },
    ],
    conditions: [
      'The reaction is treated as an isolated system.',
      'Total rest masses include every initial and final particle.',
      'Q > 0 is exothermic; Q < 0 is endothermic.',
      'Laboratory-frame threshold energies require a separate kinematic calculation.',
    ],
    exactness: 'exact',
    framework: 'relativistic',
    syllabusLinks: [
      link('csir-net-physics', 'implicit', [CSIR.nuclear]),
      link('gate-physics', 'implicit', [GATE.nuclear]),
      link(
        'tifr-gs-physics',
        'uncertain',
        [TIFR.modern],
        'TIFR lists Modern Physics broadly; this is not claimed as an explicit TIFR topic.',
      ),
    ],
    sources: [
      textbook(TEXTBOOKS.krane, 'Ch. 11'),
      reference(REFERENCES.pdg, 'Kinematics and conservation laws'),
    ],
    relatedIds: ['nuclear-particle-radioactive-decay-law'],
    specialCases: ['For a decay at rest, Q equals the total kinetic energy of the final products.'],
    conventions: [
      'Q uses total rest masses; atomic versus nuclear masses must be used consistently.',
    ],
    keywords: ['q-value', 'nuclear reaction', 'mass defect', 'exothermic', 'endothermic'],
  },
];
