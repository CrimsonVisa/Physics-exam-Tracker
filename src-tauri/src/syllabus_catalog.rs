//! Static, official-source syllabus catalog.
//!
//! Topic titles are transcribed from the linked official documents with
//! typographic normalization. The CSIR PDF calls its own organization Core and
//! Advanced; those labels are intentionally kept distinct from the exam paper's
//! Parts A, B and C.

pub(crate) const GENERAL_APTITUDE: &str = "General Aptitude";
pub(crate) const CORE: &str = "Core";
pub(crate) const ADVANCED: &str = "Advanced";

pub(crate) const SOURCE_CSIR: &str = "csir-physical-sciences";
pub(crate) const SOURCE_NTA: &str = "nta-csir-net-dec-2026";

pub(crate) const CSIR_CORE_REFERENCE: &str = "CSIR syllabus PDF — Core";
pub(crate) const CSIR_ADVANCED_REFERENCE: &str = "CSIR syllabus PDF — Advanced";
pub(crate) const NTA_GENERAL_APTITUDE_REFERENCE: &str =
    "NTA December 2026 bulletin — General Aptitude";

#[derive(Debug, Clone, Copy)]
pub(crate) struct SourceDef {
    pub code: &'static str,
    pub title: &'static str,
    pub publisher: &'static str,
    pub url: &'static str,
    pub landing_url: Option<&'static str>,
    pub verified_on: &'static str,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SectionDef {
    pub code: &'static str,
    pub title: &'static str,
    pub source_code: &'static str,
    pub source_reference: &'static str,
    pub display_order: i64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TopicDef {
    pub code: &'static str,
    pub section_code: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub source_code: &'static str,
    pub source_reference: &'static str,
    pub display_order: i64,
}

pub(crate) struct CatalogDef<'a> {
    pub sources: &'a [SourceDef],
    pub sections: &'a [SectionDef],
    pub topics: &'a [TopicDef],
}

macro_rules! ga_topic {
    ($code:literal, $order:literal, $title:literal $(,)?) => {
        TopicDef {
            code: $code,
            section_code: "GA",
            title: $title,
            category: GENERAL_APTITUDE,
            source_code: SOURCE_NTA,
            source_reference: NTA_GENERAL_APTITUDE_REFERENCE,
            display_order: $order,
        }
    };
}

macro_rules! core_topic {
    ($code:literal, $section:literal, $order:literal, $title:literal $(,)?) => {
        TopicDef {
            code: $code,
            section_code: $section,
            title: $title,
            category: CORE,
            source_code: SOURCE_CSIR,
            source_reference: CSIR_CORE_REFERENCE,
            display_order: $order,
        }
    };
}

macro_rules! advanced_topic {
    ($code:literal, $section:literal, $order:literal, $title:literal $(,)?) => {
        TopicDef {
            code: $code,
            section_code: $section,
            title: $title,
            category: ADVANCED,
            source_code: SOURCE_CSIR,
            source_reference: CSIR_ADVANCED_REFERENCE,
            display_order: $order,
        }
    };
}

pub(crate) static SOURCES: &[SourceDef] = &[
    SourceDef {
        code: SOURCE_CSIR,
        title: "Proposed Syllabus for Physical Sciences",
        publisher: "CSIR Human Resource Development Group",
        url: "https://csirhrdg.res.in/SiteContent/ManagedContent/ContentFiles/20201221140202824physicalscience_syllbus.pdf",
        landing_url: Some("https://csirhrdg.res.in/Home/Index/1/Default/3485/78"),
        verified_on: "2026-10-29",
        notes: "Official CSIR-HRDG PDF. The document uses Core and Advanced categories; that organization is distinct from the exam question-paper Parts A, B and C.",
    },
    SourceDef {
        code: SOURCE_NTA,
        title: "Joint CSIR-UGC NET December 2026 Information Bulletin",
        publisher: "National Testing Agency",
        url: "https://cdnbbsr.s3waas.gov.in/s3efdf562ce2fb0ad460fd8e9d33e57f57/uploads/2026/10/202610071407707951.pdf",
        landing_url: Some("https://csirnet.nta.nic.in/"),
        verified_on: "2026-10-29",
        notes: "Official NTA bulletin. General Aptitude topics are limited to the bulletin's stated emphases; no topic weightings are implied.",
    },
];

pub(crate) static SECTIONS: &[SectionDef] = &[
    SectionDef {
        code: "GA",
        title: "General Aptitude",
        source_code: SOURCE_NTA,
        source_reference: "NTA December 2026 bulletin — General Aptitude",
        display_order: 1,
    },
    SectionDef {
        code: "MMP",
        title: "Mathematical Methods of Physics",
        source_code: SOURCE_CSIR,
        source_reference: "CSIR syllabus PDF — Core and Advanced",
        display_order: 2,
    },
    SectionDef {
        code: "CM",
        title: "Classical Mechanics",
        source_code: SOURCE_CSIR,
        source_reference: "CSIR syllabus PDF — Core and Advanced",
        display_order: 3,
    },
    SectionDef {
        code: "EMT",
        title: "Electromagnetic Theory",
        source_code: SOURCE_CSIR,
        source_reference: "CSIR syllabus PDF — Core and Advanced",
        display_order: 4,
    },
    SectionDef {
        code: "QM",
        title: "Quantum Mechanics",
        source_code: SOURCE_CSIR,
        source_reference: "CSIR syllabus PDF — Core and Advanced",
        display_order: 5,
    },
    SectionDef {
        code: "TSP",
        title: "Thermodynamic and Statistical Physics",
        source_code: SOURCE_CSIR,
        source_reference: "CSIR syllabus PDF — Core and Advanced",
        display_order: 6,
    },
    SectionDef {
        code: "EEM",
        title: "Electronics and Experimental Methods",
        source_code: SOURCE_CSIR,
        source_reference: "CSIR syllabus PDF — Core and Advanced",
        display_order: 7,
    },
    SectionDef {
        code: "AMP",
        title: "Atomic & Molecular Physics",
        source_code: SOURCE_CSIR,
        source_reference: "CSIR syllabus PDF — Advanced",
        display_order: 8,
    },
    SectionDef {
        code: "CMP",
        title: "Condensed Matter Physics",
        source_code: SOURCE_CSIR,
        source_reference: "CSIR syllabus PDF — Advanced",
        display_order: 9,
    },
    SectionDef {
        code: "NPP",
        title: "Nuclear and Particle Physics",
        source_code: SOURCE_CSIR,
        source_reference: "CSIR syllabus PDF — Advanced",
        display_order: 10,
    },
];

pub(crate) static TOPICS: &[TopicDef] = &[
    // General Aptitude: the six emphases stated in the NTA December 2026 bulletin.
    ga_topic!("GA-01", 1, "Logical reasoning"),
    ga_topic!("GA-02", 2, "Graphical analysis"),
    ga_topic!("GA-03", 3, "Analytical and numerical ability"),
    ga_topic!("GA-04", 4, "Quantitative comparison"),
    ga_topic!("GA-05", 5, "Series formation"),
    ga_topic!("GA-06", 6, "Puzzles"),
    // I. Mathematical Methods of Physics
    core_topic!("MMP-C01", "MMP", 1, "Dimensional analysis"),
    core_topic!("MMP-C02", "MMP", 2, "Vector algebra and vector calculus"),
    core_topic!(
        "MMP-C03",
        "MMP",
        3,
        "Linear algebra, matrices and the Cayley–Hamilton theorem"
    ),
    core_topic!("MMP-C04", "MMP", 4, "Eigenvalues and eigenvectors"),
    core_topic!(
        "MMP-C05",
        "MMP",
        5,
        "Linear ordinary differential equations of first and second order"
    ),
    core_topic!(
        "MMP-C06",
        "MMP",
        6,
        "Special functions: Hermite, Bessel, Laguerre and Legendre functions"
    ),
    core_topic!(
        "MMP-C07",
        "MMP",
        7,
        "Fourier series, Fourier and Laplace transforms"
    ),
    core_topic!(
        "MMP-C08",
        "MMP",
        8,
        "Elements of complex analysis and analytic functions"
    ),
    core_topic!(
        "MMP-C09",
        "MMP",
        9,
        "Taylor and Laurent series; poles, residues and evaluation of integrals"
    ),
    core_topic!("MMP-C10", "MMP", 10, "Elementary probability theory"),
    core_topic!(
        "MMP-C11",
        "MMP",
        11,
        "Random variables; binomial, Poisson and normal distributions"
    ),
    core_topic!("MMP-C12", "MMP", 12, "Central limit theorem"),
    advanced_topic!("MMP-A01", "MMP", 1, "Green's functions"),
    advanced_topic!(
        "MMP-A02",
        "MMP",
        2,
        "Partial differential equations: Laplace, wave and heat equations in two and three dimensions"
    ),
    advanced_topic!(
        "MMP-A03",
        "MMP",
        3,
        "Elements of computational techniques: root finding, interpolation and extrapolation"
    ),
    advanced_topic!(
        "MMP-A04",
        "MMP",
        4,
        "Integration by trapezoid and Simpson's rule"
    ),
    advanced_topic!(
        "MMP-A05",
        "MMP",
        5,
        "Solution of first-order differential equations using the Runge–Kutta method"
    ),
    advanced_topic!("MMP-A06", "MMP", 6, "Finite difference methods"),
    advanced_topic!("MMP-A07", "MMP", 7, "Tensors"),
    advanced_topic!(
        "MMP-A08",
        "MMP",
        8,
        "Introductory group theory: SU(2), O(3)"
    ),
    // II. Classical Mechanics
    core_topic!("CM-C01", "CM", 1, "Newton's laws"),
    core_topic!(
        "CM-C02",
        "CM",
        2,
        "Dynamical systems, phase space dynamics and stability analysis"
    ),
    core_topic!("CM-C03", "CM", 3, "Central force motions"),
    core_topic!(
        "CM-C04",
        "CM",
        4,
        "Two-body collisions: scattering in laboratory and centre-of-mass frames"
    ),
    core_topic!(
        "CM-C05",
        "CM",
        5,
        "Rigid body dynamics: moment of inertia tensor"
    ),
    core_topic!("CM-C06", "CM", 6, "Non-inertial frames and pseudoforces"),
    core_topic!("CM-C07", "CM", 7, "Variational principle"),
    core_topic!("CM-C08", "CM", 8, "Generalized coordinates"),
    core_topic!(
        "CM-C09",
        "CM",
        9,
        "Lagrangian and Hamiltonian formalism and equations of motion"
    ),
    core_topic!(
        "CM-C10",
        "CM",
        10,
        "Conservation laws and cyclic coordinates"
    ),
    core_topic!(
        "CM-C11",
        "CM",
        11,
        "Periodic motion: small oscillations and normal modes"
    ),
    core_topic!(
        "CM-C12",
        "CM",
        12,
        "Special theory of relativity: Lorentz transformations, relativistic kinematics and mass–energy equivalence"
    ),
    advanced_topic!(
        "CM-A01",
        "CM",
        1,
        "Dynamical systems, phase space dynamics and stability analysis"
    ),
    advanced_topic!(
        "CM-A02",
        "CM",
        2,
        "Poisson brackets and canonical transformations"
    ),
    advanced_topic!(
        "CM-A03",
        "CM",
        3,
        "Symmetry, invariance and Noether's theorem"
    ),
    advanced_topic!("CM-A04", "CM", 4, "Hamilton–Jacobi theory"),
    // III. Electromagnetic Theory
    core_topic!(
        "EMT-C01",
        "EMT",
        1,
        "Electrostatics: Gauss's law and its applications"
    ),
    core_topic!(
        "EMT-C02",
        "EMT",
        2,
        "Laplace and Poisson equations; boundary value problems"
    ),
    core_topic!(
        "EMT-C03",
        "EMT",
        3,
        "Magnetostatics: Biot–Savart law and Ampère's theorem"
    ),
    core_topic!("EMT-C04", "EMT", 4, "Electromagnetic induction"),
    core_topic!(
        "EMT-C05",
        "EMT",
        5,
        "Maxwell's equations in free space and linear isotropic media"
    ),
    core_topic!(
        "EMT-C06",
        "EMT",
        6,
        "Boundary conditions on the fields at interfaces"
    ),
    core_topic!(
        "EMT-C07",
        "EMT",
        7,
        "Scalar and vector potentials; gauge invariance"
    ),
    core_topic!("EMT-C08", "EMT", 8, "Electromagnetic waves in free space"),
    core_topic!("EMT-C09", "EMT", 9, "Dielectrics and conductors"),
    core_topic!("EMT-C10", "EMT", 10, "Reflection and refraction"),
    core_topic!("EMT-C11", "EMT", 11, "Polarization and Fresnel's law"),
    core_topic!(
        "EMT-C12",
        "EMT",
        12,
        "Interference, coherence and diffraction"
    ),
    core_topic!(
        "EMT-C13",
        "EMT",
        13,
        "Dynamics of charged particles in static and uniform electromagnetic fields"
    ),
    advanced_topic!("EMT-A01", "EMT", 1, "Dispersion relations in plasma"),
    advanced_topic!(
        "EMT-A02",
        "EMT",
        2,
        "Lorentz invariance of Maxwell's equations"
    ),
    advanced_topic!("EMT-A03", "EMT", 3, "Transmission lines and wave guides"),
    advanced_topic!(
        "EMT-A04",
        "EMT",
        4,
        "Radiation from moving charges and dipoles; retarded potentials"
    ),
    // IV. Quantum Mechanics
    core_topic!("QM-C01", "QM", 1, "Wave–particle duality"),
    core_topic!(
        "QM-C02",
        "QM",
        2,
        "Schrödinger equation: time-dependent and time-independent"
    ),
    core_topic!(
        "QM-C03",
        "QM",
        3,
        "Eigenvalue problems: particle in a box, harmonic oscillator, etc."
    ),
    core_topic!("QM-C04", "QM", 4, "Tunneling through a barrier"),
    core_topic!(
        "QM-C05",
        "QM",
        5,
        "Wavefunction in coordinate and momentum representations"
    ),
    core_topic!(
        "QM-C06",
        "QM",
        6,
        "Commutators and the Heisenberg uncertainty principle"
    ),
    core_topic!("QM-C07", "QM", 7, "Dirac notation for state vectors"),
    core_topic!(
        "QM-C08",
        "QM",
        8,
        "Motion in a central potential: orbital angular momentum"
    ),
    core_topic!("QM-C09", "QM", 9, "Angular momentum algebra"),
    core_topic!("QM-C10", "QM", 10, "Spin and addition of angular momenta"),
    core_topic!("QM-C11", "QM", 11, "Hydrogen atom"),
    core_topic!("QM-C12", "QM", 12, "Stern–Gerlach experiment"),
    core_topic!(
        "QM-C13",
        "QM",
        13,
        "Time-independent perturbation theory and applications"
    ),
    core_topic!("QM-C14", "QM", 14, "Variational method"),
    core_topic!(
        "QM-C15",
        "QM",
        15,
        "Time-dependent perturbation theory and Fermi's golden rule"
    ),
    core_topic!("QM-C16", "QM", 16, "Selection rules"),
    core_topic!(
        "QM-C17",
        "QM",
        17,
        "Identical particles and the Pauli exclusion principle"
    ),
    core_topic!("QM-C18", "QM", 18, "Spin–statistics connection"),
    advanced_topic!("QM-A01", "QM", 1, "Spin–orbit coupling and fine structure"),
    advanced_topic!("QM-A02", "QM", 2, "WKB approximation"),
    advanced_topic!(
        "QM-A03",
        "QM",
        3,
        "Elementary theory of scattering: phase shifts, partial waves and Born approximation"
    ),
    advanced_topic!(
        "QM-A04",
        "QM",
        4,
        "Relativistic quantum mechanics: Klein–Gordon and Dirac equations"
    ),
    advanced_topic!("QM-A05", "QM", 5, "Semi-classical theory of radiation"),
    // V. Thermodynamic and Statistical Physics
    core_topic!(
        "TSP-C01",
        "TSP",
        1,
        "Laws of thermodynamics and their consequences"
    ),
    core_topic!("TSP-C02", "TSP", 2, "Thermodynamic potentials"),
    core_topic!("TSP-C03", "TSP", 3, "Maxwell relations"),
    core_topic!(
        "TSP-C04",
        "TSP",
        4,
        "Chemical potential and phase equilibria"
    ),
    core_topic!(
        "TSP-C05",
        "TSP",
        5,
        "Phase space, microstates and macrostates"
    ),
    core_topic!(
        "TSP-C06",
        "TSP",
        6,
        "Micro-canonical, canonical and grand-canonical ensembles"
    ),
    core_topic!("TSP-C07", "TSP", 7, "Partition functions"),
    core_topic!(
        "TSP-C08",
        "TSP",
        8,
        "Free energy and its connection with thermodynamic quantities"
    ),
    core_topic!("TSP-C09", "TSP", 9, "Classical and quantum statistics"),
    core_topic!("TSP-C10", "TSP", 10, "Ideal Bose and Fermi gases"),
    core_topic!("TSP-C11", "TSP", 11, "Principle of detailed balance"),
    core_topic!(
        "TSP-C12",
        "TSP",
        12,
        "Blackbody radiation and Planck's distribution law"
    ),
    advanced_topic!(
        "TSP-A01",
        "TSP",
        1,
        "First- and second-order phase transitions"
    ),
    advanced_topic!(
        "TSP-A02",
        "TSP",
        2,
        "Diamagnetism, paramagnetism and ferromagnetism"
    ),
    advanced_topic!("TSP-A03", "TSP", 3, "Ising model"),
    advanced_topic!("TSP-A04", "TSP", 4, "Bose–Einstein condensation"),
    advanced_topic!("TSP-A05", "TSP", 5, "Diffusion equation"),
    advanced_topic!("TSP-A06", "TSP", 6, "Random walk and Brownian motion"),
    advanced_topic!(
        "TSP-A07",
        "TSP",
        7,
        "Introduction to nonequilibrium processes"
    ),
    // VI. Electronics and Experimental Methods
    core_topic!(
        "EEM-C01",
        "EEM",
        1,
        "Semiconductor devices: diodes, junctions, transistors, field-effect devices, homo- and hetero-junction devices"
    ),
    core_topic!(
        "EEM-C02",
        "EEM",
        2,
        "Device structure, device characteristics, frequency dependence and applications"
    ),
    core_topic!(
        "EEM-C03",
        "EEM",
        3,
        "Optoelectronic devices: solar cells, photo-detectors and LEDs"
    ),
    core_topic!(
        "EEM-C04",
        "EEM",
        4,
        "Operational amplifiers and their applications"
    ),
    core_topic!(
        "EEM-C05",
        "EEM",
        5,
        "Digital techniques: registers, counters, comparators and similar circuits"
    ),
    core_topic!("EEM-C06", "EEM", 6, "A/D and D/A converters"),
    core_topic!(
        "EEM-C07",
        "EEM",
        7,
        "Microprocessor and microcontroller basics"
    ),
    core_topic!("EEM-C08", "EEM", 8, "Data interpretation and analysis"),
    core_topic!("EEM-C09", "EEM", 9, "Precision and accuracy"),
    core_topic!(
        "EEM-C10",
        "EEM",
        10,
        "Error analysis and propagation of errors"
    ),
    core_topic!("EEM-C11", "EEM", 11, "Least squares fitting"),
    advanced_topic!("EEM-A01", "EEM", 1, "Linear and nonlinear curve fitting"),
    advanced_topic!("EEM-A02", "EEM", 2, "Chi-square test"),
    advanced_topic!(
        "EEM-A03",
        "EEM",
        3,
        "Transducers: temperature, pressure/vacuum, magnetic fields, vibration, optical and particle detectors"
    ),
    advanced_topic!("EEM-A04", "EEM", 4, "Measurement and control"),
    advanced_topic!("EEM-A05", "EEM", 5, "Signal conditioning and recovery"),
    advanced_topic!("EEM-A06", "EEM", 6, "Impedance matching"),
    advanced_topic!(
        "EEM-A07",
        "EEM",
        7,
        "Amplification: op-amp based, instrumentation amplifier and feedback"
    ),
    advanced_topic!("EEM-A08", "EEM", 8, "Filtering and noise reduction"),
    advanced_topic!("EEM-A09", "EEM", 9, "Shielding and grounding"),
    advanced_topic!(
        "EEM-A10",
        "EEM",
        10,
        "Fourier transforms, lock-in detector and box-car integrator"
    ),
    advanced_topic!("EEM-A11", "EEM", 11, "Modulation techniques"),
    advanced_topic!(
        "EEM-A12",
        "EEM",
        12,
        "High-frequency devices, including generators and detectors"
    ),
    // VII. Atomic & Molecular Physics
    advanced_topic!(
        "AMP-A01",
        "AMP",
        1,
        "Quantum states of an electron in an atom"
    ),
    advanced_topic!("AMP-A02", "AMP", 2, "Electron spin"),
    advanced_topic!("AMP-A03", "AMP", 3, "Spectrum of helium and alkali atoms"),
    advanced_topic!(
        "AMP-A04",
        "AMP",
        4,
        "Relativistic corrections for energy levels of the hydrogen atom"
    ),
    advanced_topic!(
        "AMP-A05",
        "AMP",
        5,
        "Hyperfine structure and isotopic shift"
    ),
    advanced_topic!("AMP-A06", "AMP", 6, "Width of spectral lines"),
    advanced_topic!("AMP-A07", "AMP", 7, "LS and JJ couplings"),
    advanced_topic!(
        "AMP-A08",
        "AMP",
        8,
        "Zeeman, Paschen–Bach and Stark effects"
    ),
    advanced_topic!("AMP-A09", "AMP", 9, "Electron spin resonance"),
    advanced_topic!(
        "AMP-A10",
        "AMP",
        10,
        "Nuclear magnetic resonance and chemical shift"
    ),
    advanced_topic!("AMP-A11", "AMP", 11, "Franck–Condon principle"),
    advanced_topic!("AMP-A12", "AMP", 12, "Born–Oppenheimer approximation"),
    advanced_topic!(
        "AMP-A13",
        "AMP",
        13,
        "Electronic, rotational, vibrational and Raman spectra of diatomic molecules"
    ),
    advanced_topic!("AMP-A14", "AMP", 14, "Selection rules"),
    advanced_topic!(
        "AMP-A15",
        "AMP",
        15,
        "Lasers: spontaneous and stimulated emission; Einstein A and B coefficients"
    ),
    advanced_topic!(
        "AMP-A16",
        "AMP",
        16,
        "Optical pumping, population inversion and rate equations"
    ),
    advanced_topic!(
        "AMP-A17",
        "AMP",
        17,
        "Modes of resonators and coherence length"
    ),
    // VIII. Condensed Matter Physics
    advanced_topic!("CMP-A01", "CMP", 1, "Bravais lattices"),
    advanced_topic!("CMP-A02", "CMP", 2, "Reciprocal lattice"),
    advanced_topic!("CMP-A03", "CMP", 3, "Diffraction and the structure factor"),
    advanced_topic!("CMP-A04", "CMP", 4, "Bonding of solids"),
    advanced_topic!("CMP-A05", "CMP", 5, "Elastic properties"),
    advanced_topic!("CMP-A06", "CMP", 6, "Phonons and lattice specific heat"),
    advanced_topic!(
        "CMP-A07",
        "CMP",
        7,
        "Free electron theory and electronic specific heat"
    ),
    advanced_topic!("CMP-A08", "CMP", 8, "Response and relaxation phenomena"),
    advanced_topic!(
        "CMP-A09",
        "CMP",
        9,
        "Drude model of electrical and thermal conductivity"
    ),
    advanced_topic!("CMP-A10", "CMP", 10, "Hall effect and thermoelectric power"),
    advanced_topic!(
        "CMP-A11",
        "CMP",
        11,
        "Electron motion in a periodic potential"
    ),
    advanced_topic!(
        "CMP-A12",
        "CMP",
        12,
        "Band theory of solids: metals, insulators and semiconductors"
    ),
    advanced_topic!(
        "CMP-A13",
        "CMP",
        13,
        "Superconductivity: type-I and type-II superconductors"
    ),
    advanced_topic!("CMP-A14", "CMP", 14, "Josephson junctions"),
    advanced_topic!("CMP-A15", "CMP", 15, "Superfluidity"),
    advanced_topic!("CMP-A16", "CMP", 16, "Defects and dislocations"),
    advanced_topic!(
        "CMP-A17",
        "CMP",
        17,
        "Ordered phases of matter: translational and orientational order"
    ),
    advanced_topic!("CMP-A18", "CMP", 18, "Kinds of liquid crystalline order"),
    advanced_topic!("CMP-A19", "CMP", 19, "Quasicrystals"),
    // IX. Nuclear and Particle Physics
    advanced_topic!(
        "NPP-A01",
        "NPP",
        1,
        "Basic nuclear properties: size, shape and charge distribution, spin and parity"
    ),
    advanced_topic!(
        "NPP-A02",
        "NPP",
        2,
        "Binding energy and semi-empirical mass formula"
    ),
    advanced_topic!("NPP-A03", "NPP", 3, "Liquid drop model"),
    advanced_topic!("NPP-A04", "NPP", 4, "Nature of the nuclear force"),
    advanced_topic!("NPP-A05", "NPP", 5, "Form of the nucleon–nucleon potential"),
    advanced_topic!(
        "NPP-A06",
        "NPP",
        6,
        "Charge independence and charge symmetry of nuclear forces"
    ),
    advanced_topic!("NPP-A07", "NPP", 7, "Deuteron problem"),
    advanced_topic!("NPP-A08", "NPP", 8, "Evidence of shell structure"),
    advanced_topic!(
        "NPP-A09",
        "NPP",
        9,
        "Single-particle shell model, its validity and limitations"
    ),
    advanced_topic!("NPP-A10", "NPP", 10, "Rotational spectra"),
    advanced_topic!(
        "NPP-A11",
        "NPP",
        11,
        "Alpha, beta and gamma decays and their selection rules"
    ),
    advanced_topic!("NPP-A12", "NPP", 12, "Fission and fusion"),
    advanced_topic!(
        "NPP-A13",
        "NPP",
        13,
        "Nuclear reactions and reaction mechanisms"
    ),
    advanced_topic!("NPP-A14", "NPP", 14, "Compound nuclei and direct reactions"),
    advanced_topic!("NPP-A15", "NPP", 15, "Classification of fundamental forces"),
    advanced_topic!(
        "NPP-A16",
        "NPP",
        16,
        "Elementary particles and their quantum numbers: charge, spin, parity, isospin, strangeness, etc."
    ),
    advanced_topic!("NPP-A17", "NPP", 17, "Gell-Mann–Nishijima formula"),
    advanced_topic!("NPP-A18", "NPP", 18, "Quark model, baryons and mesons"),
    advanced_topic!("NPP-A19", "NPP", 19, "C, P and T invariance"),
    advanced_topic!(
        "NPP-A20",
        "NPP",
        20,
        "Application of symmetry arguments to particle reactions"
    ),
    advanced_topic!(
        "NPP-A21",
        "NPP",
        21,
        "Parity non-conservation in weak interaction"
    ),
    advanced_topic!("NPP-A22", "NPP", 22, "Relativistic kinematics"),
];

pub(crate) static CATALOG: CatalogDef<'static> = CatalogDef {
    sources: SOURCES,
    sections: SECTIONS,
    topics: TOPICS,
};
