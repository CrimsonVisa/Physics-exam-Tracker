import type {
  ExamCode,
  FormulaSyllabusLink,
  FormulaSyllabusNodeRef,
  SyllabusRelevance,
} from '../types';

export function node(code: string, title: string): FormulaSyllabusNodeRef {
  return { code, title };
}

export function link(
  examCode: ExamCode,
  relevance: SyllabusRelevance,
  nodes: FormulaSyllabusNodeRef[],
  note?: string,
): FormulaSyllabusLink {
  return note ? { examCode, relevance, nodes, note } : { examCode, relevance, nodes };
}

export const CSIR = {
  mathCore: node(
    'csir-part-a-core-mathematical-methods-of-physics',
    'Mathematical Methods of Physics',
  ),
  mathAdvanced: node(
    'csir-part-b-advanced-mathematical-methods-of-physics',
    'Mathematical Methods of Physics',
  ),
  eigenvalue: node(
    'csir-part-a-core-mathematical-methods-of-physics-t004-eigenvalues-and-eigenvectors',
    'Eigenvalues and eigenvectors',
  ),
  classicalCore: node('csir-part-a-core-classical-mechanics', 'Classical Mechanics'),
  classicalAdvanced: node('csir-part-b-advanced-classical-mechanics', 'Classical Mechanics'),
  emCore: node('csir-part-a-core-electromagnetic-theory', 'Electromagnetic Theory'),
  emAdvanced: node('csir-part-b-advanced-electromagnetic-theory', 'Electromagnetic Theory'),
  quantumCore: node('csir-part-a-core-quantum-mechanics', 'Quantum Mechanics'),
  quantumAdvanced: node('csir-part-b-advanced-quantum-mechanics', 'Quantum Mechanics'),
  thermoCore: node(
    'csir-part-a-core-thermodynamic-and-statistical-physics',
    'Thermodynamic and Statistical Physics',
  ),
  thermoAdvanced: node(
    'csir-part-b-advanced-thermodynamic-and-statistical-physics',
    'Thermodynamic and Statistical Physics',
  ),
  electronicsCore: node(
    'csir-part-a-core-electronics-and-experimental-methods',
    'Electronics and Experimental Methods',
  ),
  electronicsAdvanced: node(
    'csir-part-b-advanced-electronics-and-experimental-methods',
    'Electronics and Experimental Methods',
  ),
  atomic: node('csir-part-b-advanced-atomic-and-molecular-physics', 'Atomic and Molecular Physics'),
  condensed: node('csir-part-b-advanced-condensed-matter-physics', 'Condensed Matter Physics'),
  nuclear: node(
    'csir-part-b-advanced-nuclear-and-particle-physics',
    'Nuclear and Particle Physics',
  ),
  divergence: node(
    'csir-part-a-core-mathematical-methods-of-physics-t002-vector-algebra-and-vector-calculus',
    'Vector algebra and vector calculus',
  ),
  cauchy: node(
    'csir-part-a-core-mathematical-methods-of-physics-t008-complex-analysis-analytic-functions-taylor-and-lau',
    'Complex analysis: analytic functions, Taylor and Laurent series, poles, residues and evaluation of integrals',
  ),
  fourier: node(
    'csir-part-a-core-mathematical-methods-of-physics-t007-fourier-series-fourier-and-laplace-transforms',
    'Fourier series, Fourier and Laplace transforms',
  ),
  green: node(
    'csir-part-b-advanced-mathematical-methods-of-physi-t001-green-function',
    'Green function',
  ),
  newton: node('csir-part-a-core-classical-mechanics-t001-newton-s-laws', "Newton's laws"),
  eulerLagrange: node(
    'csir-part-a-core-classical-mechanics-t008-lagrangian-and-hamiltonian-formalism-and-equations',
    'Lagrangian and Hamiltonian formalism and equations of motion',
  ),
  centralForce: node(
    'csir-part-a-core-classical-mechanics-t003-central-force-motions',
    'Central force motions',
  ),
  gauss: node(
    'csir-part-a-core-electromagnetic-theory-t001-electrostatics-gauss-law-and-applications-laplace-',
    'Electrostatics: Gauss law and applications, Laplace and Poisson equations, boundary value problems',
  ),
  maxwell: node(
    'csir-part-a-core-electromagnetic-theory-t004-maxwell-equations-in-free-space-and-linear-isotrop',
    'Maxwell equations in free space and linear isotropic media',
  ),
  lorentzForce: node(
    'csir-part-a-core-electromagnetic-theory-t010-dynamics-of-charged-particles-in-static-and-unifor',
    'Dynamics of charged particles in static and uniform electromagnetic fields',
  ),
  schrodinger: node(
    'csir-part-a-core-quantum-mechanics-t002-schrodinger-equation-time-dependent-and-time-indep',
    'Schrodinger equation: time-dependent and time-independent',
  ),
  commutator: node(
    'csir-part-a-core-quantum-mechanics-t006-commutators-and-heisenberg-uncertainty-principle',
    'Commutators and Heisenberg uncertainty principle',
  ),
  hydrogen: node(
    'csir-part-a-core-quantum-mechanics-t009-hydrogen-atom-and-stern-gerlach-experiment',
    'Hydrogen atom and Stern-Gerlach experiment',
  ),
  firstLaw: node(
    'csir-part-a-core-thermodynamic-and-statistical-phy-t001-laws-of-thermodynamics-and-their-consequences',
    'Laws of thermodynamics and their consequences',
  ),
  partition: node(
    'csir-part-a-core-thermodynamic-and-statistical-phy-t004-micro-canonical-canonical-and-grand-canonical-ense',
    'Micro-canonical, canonical and grand-canonical ensembles and partition functions',
  ),
  fermiDirac: node(
    'csir-part-a-core-thermodynamic-and-statistical-phy-t006-classical-and-quantum-statistics',
    'Classical and quantum statistics',
  ),
  planck: node(
    'csir-part-a-core-thermodynamic-and-statistical-phy-t009-blackbody-radiation-and-planck-distribution-law',
    'Blackbody radiation and Planck distribution law',
  ),
  relativity: node(
    'csir-part-a-core-classical-mechanics-t011-special-theory-of-relativity-lorentz-transformatio',
    'Special theory of relativity: Lorentz transformations, relativistic kinematics and mass-energy equivalence',
  ),
  errorAnalysis: node(
    'csir-part-a-core-electronics-and-experimental-meth-t009-error-analysis-and-propagation-of-errors',
    'Error analysis and propagation of errors',
  ),
  chiSquare: node(
    'csir-part-b-advanced-electronics-and-experimental--t001-linear-and-nonlinear-curve-fitting-and-chi-square-',
    'Linear and nonlinear curve fitting and chi-square test',
  ),
  drude: node(
    'csir-part-b-advanced-condensed-matter-physics-t007-drude-model-of-electrical-and-thermal-conductivity',
    'Drude model of electrical and thermal conductivity',
  ),
  semiEmpirical: node(
    'csir-part-b-advanced-nuclear-and-particle-physics-t002-binding-energy-semi-empirical-mass-formula-and-liq',
    'Binding energy, semi-empirical mass formula and liquid drop model',
  ),
  diffraction: node(
    'csir-part-a-core-electromagnetic-theory-t009-reflection-and-refraction-polarization-fresnel-law',
    'Reflection and refraction, polarization, Fresnel law, interference, coherence and diffraction',
  ),
  polarization: node(
    'csir-part-a-core-electromagnetic-theory-t009-reflection-and-refraction-polarization-fresnel-law',
    'Reflection and refraction, polarization, Fresnel law, interference, coherence and diffraction',
  ),
  dimensionalAnalysis: node(
    'csir-part-a-core-mathematical-methods-of-physics-t001-dimensional-analysis',
    'Dimensional analysis',
  ),
  semiconductorDevices: node(
    'csir-part-a-core-electronics-and-experimental-meth-t001-semiconductor-devices-diodes-junctions-transistors',
    'Semiconductor devices: diodes, junctions, transistors, field-effect devices, device structure, characteristics, frequency dependence and applications',
  ),
  operationalAmplifiers: node(
    'csir-part-a-core-electronics-and-experimental-meth-t004-operational-amplifiers-and-their-applications',
    'Operational amplifiers and their applications',
  ),
  lasersEinstein: node(
    'csir-part-b-advanced-atomic-and-molecular-physics-t010-lasers-spontaneous-and-stimulated-emission-einstei',
    'Lasers: spontaneous and stimulated emission, Einstein A and B coefficients',
  ),
  diffractionStructure: node(
    'csir-part-b-advanced-condensed-matter-physics-t002-diffraction-and-structure-factor',
    'Diffraction and structure factor',
  ),
  freeElectron: node(
    'csir-part-b-advanced-condensed-matter-physics-t005-free-electron-theory-and-electronic-specific-heat',
    'Free electron theory and electronic specific heat',
  ),
  molecularSpectra: node(
    'csir-part-b-advanced-atomic-and-molecular-physics-t009-electronic-rotational-vibrational-and-raman-spectr',
    'Electronic, rotational, vibrational and Raman spectra of diatomic molecules and selection rules',
  ),
  smallOscillations: node(
    'csir-part-a-core-classical-mechanics-t010-periodic-motion-small-oscillations-and-normal-mode',
    'Periodic motion: small oscillations and normal modes',
  ),
};

export const GATE = {
  measurements: node(
    'gate-ph-2027-measurements-and-error-analysis',
    'Measurements and Error Analysis',
  ),
  math: node('gate-ph-2027-mathematical-physics', 'Mathematical Physics'),
  classical: node('gate-ph-2027-classical-mechanics', 'Classical Mechanics'),
  thermo: node(
    'gate-ph-2027-thermodynamics-and-statistical-mechanics',
    'Thermodynamics and Statistical Mechanics',
  ),
  em: node('gate-ph-2027-electromagnetic-theory', 'Electromagnetic Theory'),
  optics: node('gate-ph-2027-optical-physics', 'Optical Physics'),
  quantum: node('gate-ph-2027-quantum-mechanics', 'Quantum Mechanics'),
  atomic: node('gate-ph-2027-atomic-and-molecular-physics', 'Atomic and Molecular Physics'),
  solidState: node('gate-ph-2027-solid-state-physics', 'Solid State Physics'),
  nuclear: node('gate-ph-2027-nuclear-and-particle-physics', 'Nuclear and Particle Physics'),
  electronics: node('gate-ph-2027-electronics', 'Electronics'),
  eulerLagrange: node(
    'gate-ph-2027-classical-mechanics-t001-d-alembert-principle-euler-lagrange-equation-hamil',
    "D'Alembert principle, Euler-Lagrange equation, Hamilton principle and calculus of variations",
  ),
  centralForce: node(
    'gate-ph-2027-classical-mechanics-t003-central-force-motion-and-kepler-problem',
    'Central force motion and Kepler problem',
  ),
  relativity: node(
    'gate-ph-2027-classical-mechanics-t008-special-theory-of-relativity-lorentz-transformatio',
    'Special theory of relativity: Lorentz transformations, relativistic kinematics and mass-energy equivalence',
  ),
  maxwell: node(
    'gate-ph-2027-electromagnetic-theory-t004-maxwell-equations-scalar-and-vector-potentials-cou',
    'Maxwell equations, scalar and vector potentials, Coulomb and Lorentz gauges',
  ),
  poynting: node(
    'gate-ph-2027-electromagnetic-theory-t008-poynting-vector-poynting-theorem-energy-and-moment',
    'Poynting vector, Poynting theorem, energy and momentum of electromagnetic waves',
  ),
  tise: node(
    'gate-ph-2027-quantum-mechanics-t003-time-independent-schrodinger-equation',
    'Time-independent Schrodinger equation',
  ),
  hydrogen: node(
    'gate-ph-2027-quantum-mechanics-t007-central-potentials-and-hydrogen-like-atoms',
    'Central potentials and hydrogen-like atoms',
  ),
  firstLaw: node(
    'gate-ph-2027-thermodynamics-and-statistical-mechan-t001-laws-of-thermodynamics',
    'Laws of thermodynamics',
  ),
  partition: node(
    'gate-ph-2027-thermodynamics-and-statistical-mechan-t003-partition-function-free-energy-and-calculation-of-',
    'Partition function, free energy and calculation of thermodynamic quantities',
  ),
  fermiDirac: node(
    'gate-ph-2027-thermodynamics-and-statistical-mechan-t004-classical-and-quantum-statistics',
    'Classical and quantum statistics',
  ),
  planck: node(
    'gate-ph-2027-thermodynamics-and-statistical-mechan-t006-black-body-radiation-and-planck-distribution-law',
    'Black body radiation and Planck distribution law',
  ),
  lasers: node(
    'gate-ph-2027-optical-physics-t008-lasers-einstein-coefficients-population-inversion-',
    'Lasers: Einstein coefficients, population inversion and two- and three-level systems',
  ),
  diffraction: node(
    'gate-ph-2027-optical-physics-t004-diffraction-fresnel-and-fraunhofer-diffraction-rec',
    'Diffraction: Fresnel and Fraunhofer diffraction, rectangular and circular aperture, Rayleigh criterion',
  ),
  grating: node(
    'gate-ph-2027-optical-physics-t006-dispersion-by-a-grating',
    'Dispersion by a grating',
  ),
  polarization: node(
    'gate-ph-2027-optical-physics-t007-polarization-jones-vectors-and-matrices-birefringe',
    'Polarization: Jones vectors and matrices, birefringence and ray-transfer matrices',
  ),
  waves: node(
    'gate-ph-2027-optical-physics-t001-wave-equation-plane-and-spherical-waves-superposit',
    'Wave equation: plane and spherical waves, superposition, standing waves and phase and group velocities',
  ),
  conductivity: node(
    'gate-ph-2027-solid-state-physics-t007-conductivity-electron-and-hole-statistics-mobility',
    'Conductivity, electron and hole statistics, mobility and effective mass',
  ),
  semiEmpirical: node(
    'gate-ph-2027-nuclear-and-particle-physics-t002-semi-empirical-mass-formula-and-nuclear-models',
    'Semi-empirical mass formula and nuclear models',
  ),
  complexAnalysis: node(
    'gate-ph-2027-mathematical-physics-t004-complex-analysis-cauchy-riemann-conditions-cauchy-',
    'Complex analysis: Cauchy-Riemann conditions, Cauchy theorem, singularities and residue theorem',
  ),
  fourierAnalysis: node(
    'gate-ph-2027-mathematical-physics-t005-fourier-analysis',
    'Fourier analysis',
  ),
  unitsDimensions: node(
    'gate-ph-2027-measurements-and-error-analysis-t001-units-and-dimensions-dimensional-analysis',
    'Units and dimensions, dimensional analysis',
  ),
  pnDiodes: node(
    'gate-ph-2027-electronics-t001-p-n-diodes-bipolar-junction-transistors-and-field-',
    'p-n diodes, bipolar junction transistors and field effect transistors',
  ),
  opAmps: node(
    'gate-ph-2027-electronics-t003-oscillators-operational-amplifiers-and-active-filt',
    'Oscillators, operational amplifiers and active filters',
  ),
  atomicSpectra: node(
    'gate-ph-2027-atomic-and-molecular-physics-t001-spectra-of-one-and-many-electron-atoms',
    'Spectra of one- and many-electron atoms',
  ),
  crystallography: node(
    'gate-ph-2027-solid-state-physics-t001-elements-of-crystallography-and-diffraction-method',
    'Elements of crystallography and diffraction methods',
  ),
  freeElectron: node(
    'gate-ph-2027-solid-state-physics-t004-free-electron-theory',
    'Free electron theory',
  ),
  molecularSpectra: node(
    'gate-ph-2027-atomic-and-molecular-physics-t006-rotational-and-vibrational-spectra-of-diatomic-mol',
    'Rotational and vibrational spectra of diatomic molecules',
  ),
  smallOscillations: node(
    'gate-ph-2027-classical-mechanics-t004-small-oscillations-coupled-oscillations-and-normal',
    'Small oscillations: coupled oscillations and normal modes',
  ),
  errorAnalysis: node(
    'gate-ph-2027-measurements-and-error-analysis-t003-methods-of-measurement-and-error-analysis',
    'Methods of measurement and error analysis',
  ),
};

export const TIFR = {
  math: node(
    'tifr-gs-2024-mathematics-relevant-to-physics-t001-broad-area-mathematics-relevant-to-physics',
    'Coverage note: Mathematics relevant to Physics',
  ),
  classical: node(
    'tifr-gs-2024-classical-mechanics-t001-broad-area-classical-mechanics',
    'Coverage note: Classical Mechanics',
  ),
  em: node(
    'tifr-gs-2024-electricity-and-magnetism-t001-broad-area-electricity-and-magnetism',
    'Coverage note: Electricity and Magnetism',
  ),
  quantum: node(
    'tifr-gs-2024-quantum-mechanics-t001-broad-area-quantum-mechanics',
    'Coverage note: Quantum Mechanics',
  ),
  thermo: node(
    'tifr-gs-2024-heat-thermodynamics-and-statistical-p-t001-broad-area-heat-thermodynamics-and-statistical-phy',
    'Coverage note: Heat, Thermodynamics and Statistical Physics',
  ),
  general: node(
    'tifr-gs-2024-general-physics-t001-broad-area-general-physics',
    'Coverage note: General Physics',
  ),
  modern: node(
    'tifr-gs-2024-modern-physics-t001-broad-area-modern-physics',
    'Coverage note: Modern Physics',
  ),
  electronics: node(
    'tifr-gs-2024-electronics-and-experimental-physics-t001-broad-area-electronics-and-experimental-physics',
    'Coverage note: Electronics and Experimental Physics',
  ),
};

export const JEST = {
  math: node('jest-2026-mathematical-methods', 'Mathematical Methods'),
  classical: node('jest-2026-classical-mechanics', 'Classical Mechanics'),
  emOptics: node('jest-2026-electromagnetism-and-optics', 'Electromagnetism and Optics'),
  quantum: node('jest-2026-quantum-mechanics', 'Quantum Mechanics'),
  thermo: node(
    'jest-2026-thermodynamics-and-statistical-physics',
    'Thermodynamics and Statistical Physics',
  ),
  electronics: node('jest-2026-electronics', 'Electronics'),
  newton: node(
    'jest-2026-classical-mechanics-t001-newton-laws-conservation-of-energy-and-momentum-an',
    'Newton laws, conservation of energy and momentum and collisions',
  ),
  eulerLagrange: node(
    'jest-2026-classical-mechanics-t002-generalized-coordinates-principle-of-least-action-',
    'Generalized coordinates, principle of least action, Lagrangian and Hamiltonian formulations',
  ),
  centralForce: node(
    'jest-2026-classical-mechanics-t004-central-force-problem-and-kepler-problem',
    'Central force problem and Kepler problem',
  ),
  specialRelativity: node(
    'jest-2026-classical-mechanics-t006-special-relativity-in-classical-mechanics',
    'Special relativity in classical mechanics',
  ),
  faraday: node(
    'jest-2026-electromagnetism-and-optics-t003-faraday-law-and-time-varying-fields-displacement-c',
    'Faraday law and time varying fields, displacement current and Maxwell equations',
  ),
  emEnergy: node(
    'jest-2026-electromagnetism-and-optics-t004-energy-and-momentum-of-electromagnetic-fields',
    'Energy and momentum of electromagnetic fields',
  ),
  schrodinger: node(
    'jest-2026-quantum-mechanics-t001-uncertainty-principle-and-schrodinger-equation',
    'Uncertainty principle and Schrodinger equation',
  ),
  hydrogen: node(
    'jest-2026-quantum-mechanics-t002-central-potentials-and-hydrogen-atom',
    'Central potentials and hydrogen atom',
  ),
  firstLaw: node(
    'jest-2026-thermodynamics-and-statistical-physics-t001-laws-of-thermodynamics-work-and-heat-and-thermodyn',
    'Laws of thermodynamics, work and heat and thermodynamic potentials',
  ),
  partition: node(
    'jest-2026-thermodynamics-and-statistical-physics-t003-statistical-ensembles-and-partition-function',
    'Statistical ensembles and partition function',
  ),
  fermiDirac: node(
    'jest-2026-thermodynamics-and-statistical-physics-t005-classical-and-quantum-statistics-fermi-and-bose-ga',
    'Classical and quantum statistics, Fermi and Bose gases',
  ),
  planck: node(
    'jest-2026-thermodynamics-and-statistical-physics-t006-black-body-radiation',
    'Black body radiation',
  ),
  optics: node(
    'jest-2026-electromagnetism-and-optics-t007-diffraction-interference-and-polarization',
    'Diffraction, interference and polarization',
  ),
  vectorCalculus: node(
    'jest-2026-mathematical-methods-t001-vector-algebra-and-vector-calculus',
    'Vector algebra and vector calculus',
  ),
  complexAnalysis: node('jest-2026-mathematical-methods-t007-complex-analysis', 'Complex analysis'),
  fourierTransforms: node(
    'jest-2026-mathematical-methods-t008-fourier-series-fourier-transforms-and-laplace-tran',
    'Fourier series, Fourier transforms and Laplace transforms',
  ),
  pnJunctions: node(
    'jest-2026-electronics-t002-p-n-junctions-diodes-and-transistors',
    'p-n junctions, diodes and transistors',
  ),
  opAmps: node(
    'jest-2026-electronics-t004-basics-of-opamps-and-their-applications',
    'Basics of OPAMPs and their applications',
  ),
  lcrCircuits: node(
    'jest-2026-electronics-t003-lcr-circuits-rectifiers-amplifiers-active-filters-',
    'LCR circuits, rectifiers, amplifiers, active filters and oscillators',
  ),
  probabilityError: node(
    'jest-2026-mathematical-methods-t010-elements-of-probability-theory-and-error-analysis',
    'Elements of probability theory and error analysis',
  ),
};
