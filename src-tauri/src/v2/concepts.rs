use std::collections::{HashMap, HashSet};

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter, types::Value};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppError,
    v2::mastery::{self, V2ConceptEvidence, V2ConceptMasterySummary, V2MasteryRunSummary},
};

const DEFAULT_SEARCH_LIMIT: i64 = 100;
const MAX_SEARCH_LIMIT: i64 = 200;
const MAPPING_SEED_JSON: &str = include_str!("mapping_seed.json");

type ConceptExamCodes = HashMap<i64, (Vec<String>, Vec<String>)>;
type NodePathMap = HashMap<i64, Vec<String>>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MappingSeed {
    concept_slug: String,
    exam_code: String,
    node_code: String,
    status: String,
    origin: String,
    notes: String,
}

struct CuratedConcept {
    slug: &'static str,
    title: &'static str,
    description: &'static str,
    domain: &'static str,
    parent_slug: Option<&'static str>,
    difficulty: i64,
}

/// A curated, exam-neutral Physics library. These concepts are not claims
/// about any examination's official syllabus. Exam mappings are stored
/// separately and are only shown as verified when their syllabus content
/// version is verified.
const CURATED_CONCEPTS: &[CuratedConcept] = &[
    CuratedConcept {
        slug: "classical-mechanics",
        title: "Classical Mechanics",
        description: "The mechanics of particles, rigid bodies, and systems under classical forces.",
        domain: "Classical Mechanics",
        parent_slug: None,
        difficulty: 2,
    },
    CuratedConcept {
        slug: "kinematics",
        title: "Kinematics",
        description: "Description of motion using position, velocity, acceleration, and reference frames.",
        domain: "Classical Mechanics",
        parent_slug: Some("classical-mechanics"),
        difficulty: 1,
    },
    CuratedConcept {
        slug: "newtons-laws",
        title: "Newton's Laws of Motion",
        description: "The three laws relating force, mass, and changes in motion.",
        domain: "Classical Mechanics",
        parent_slug: Some("classical-mechanics"),
        difficulty: 1,
    },
    CuratedConcept {
        slug: "work-energy-theorem",
        title: "Work and Energy",
        description: "Work, kinetic energy, potential energy, and conservative force fields.",
        domain: "Classical Mechanics",
        parent_slug: Some("classical-mechanics"),
        difficulty: 2,
    },
    CuratedConcept {
        slug: "conservation-of-momentum",
        title: "Conservation of Momentum",
        description: "Linear momentum conservation, impulse, collisions, and center-of-mass motion.",
        domain: "Classical Mechanics",
        parent_slug: Some("classical-mechanics"),
        difficulty: 2,
    },
    CuratedConcept {
        slug: "rotational-motion",
        title: "Rotational Motion",
        description: "Angular kinematics and dynamics, torque, moments of inertia, and rolling motion.",
        domain: "Classical Mechanics",
        parent_slug: Some("classical-mechanics"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "angular-momentum",
        title: "Angular Momentum",
        description: "Angular momentum conservation, central forces, and orbital motion.",
        domain: "Classical Mechanics",
        parent_slug: Some("classical-mechanics"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "gravitation",
        title: "Gravitation",
        description: "Newtonian gravity, gravitational potential, orbits, and Kepler's laws.",
        domain: "Classical Mechanics",
        parent_slug: Some("classical-mechanics"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "simple-harmonic-motion",
        title: "Simple Harmonic Motion",
        description: "Oscillatory motion, restoring forces, damping, and driven oscillations.",
        domain: "Classical Mechanics",
        parent_slug: Some("classical-mechanics"),
        difficulty: 2,
    },
    CuratedConcept {
        slug: "electrodynamics",
        title: "Electrodynamics",
        description: "Electric and magnetic fields, electromagnetic induction, and electromagnetic waves.",
        domain: "Electrodynamics",
        parent_slug: None,
        difficulty: 2,
    },
    CuratedConcept {
        slug: "electrostatics",
        title: "Electrostatics",
        description: "Electric charge, Coulomb's law, electric fields, and electrostatic potential.",
        domain: "Electrodynamics",
        parent_slug: Some("electrodynamics"),
        difficulty: 2,
    },
    CuratedConcept {
        slug: "gauss-law",
        title: "Gauss's Law",
        description: "Electric flux and the use of symmetry to determine electrostatic fields.",
        domain: "Electrodynamics",
        parent_slug: Some("electrodynamics"),
        difficulty: 2,
    },
    CuratedConcept {
        slug: "capacitance-dielectrics",
        title: "Capacitance and Dielectrics",
        description: "Capacitors, stored electrostatic energy, and polarization in dielectric media.",
        domain: "Electrodynamics",
        parent_slug: Some("electrodynamics"),
        difficulty: 2,
    },
    CuratedConcept {
        slug: "electric-current-circuits",
        title: "Electric Current and Circuits",
        description: "Current, resistance, direct-current circuits, and transient circuit response.",
        domain: "Electrodynamics",
        parent_slug: Some("electrodynamics"),
        difficulty: 2,
    },
    CuratedConcept {
        slug: "magnetostatics",
        title: "Magnetostatics",
        description: "Steady currents, magnetic fields, the Biot-Savart law, and Ampere's law.",
        domain: "Electrodynamics",
        parent_slug: Some("electrodynamics"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "electromagnetic-induction",
        title: "Electromagnetic Induction",
        description: "Faraday's law, Lenz's law, inductance, and motional electromotive force.",
        domain: "Electrodynamics",
        parent_slug: Some("electrodynamics"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "maxwell-equations",
        title: "Maxwell's Equations",
        description: "The coupled differential equations governing electric and magnetic fields.",
        domain: "Electrodynamics",
        parent_slug: Some("electrodynamics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "electromagnetic-waves",
        title: "Electromagnetic Waves",
        description: "Propagation, polarization, energy transport, and boundary behavior of electromagnetic waves.",
        domain: "Electrodynamics",
        parent_slug: Some("electrodynamics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "quantum-mechanics",
        title: "Quantum Mechanics",
        description: "Wavefunctions, operators, measurement, and quantum dynamics.",
        domain: "Quantum Mechanics",
        parent_slug: None,
        difficulty: 3,
    },
    CuratedConcept {
        slug: "wave-particle-duality",
        title: "Wave-Particle Duality",
        description: "Photons, matter waves, the de Broglie relation, and complementarity.",
        domain: "Quantum Mechanics",
        parent_slug: Some("quantum-mechanics"),
        difficulty: 2,
    },
    CuratedConcept {
        slug: "schrodinger-equation",
        title: "Schrodinger Equation",
        description: "The time-dependent and time-independent equations governing quantum states.",
        domain: "Quantum Mechanics",
        parent_slug: Some("quantum-mechanics"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "quantum-harmonic-oscillator",
        title: "Quantum Harmonic Oscillator",
        description: "Quantized oscillator levels, ladder operators, and coherent states.",
        domain: "Quantum Mechanics",
        parent_slug: Some("quantum-mechanics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "quantum-angular-momentum",
        title: "Quantum Angular Momentum",
        description: "Quantization of angular momentum, commutation relations, and addition of angular momenta.",
        domain: "Quantum Mechanics",
        parent_slug: Some("quantum-mechanics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "hydrogen-atom",
        title: "Hydrogen Atom",
        description: "The Coulomb-bound quantum two-body system and its orbital structure.",
        domain: "Quantum Mechanics",
        parent_slug: Some("quantum-mechanics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "spin-statistics",
        title: "Spin and Quantum Statistics",
        description: "Intrinsic spin, fermions, bosons, and antisymmetric or symmetric many-body states.",
        domain: "Quantum Mechanics",
        parent_slug: Some("quantum-mechanics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "perturbation-theory",
        title: "Perturbation Theory",
        description: "Approximate quantum solutions for small perturbations, including degeneracy handling.",
        domain: "Quantum Mechanics",
        parent_slug: Some("quantum-mechanics"),
        difficulty: 5,
    },
    CuratedConcept {
        slug: "scattering-theory",
        title: "Scattering Theory",
        description: "Cross sections, phase shifts, the Born approximation, and quantum scattering.",
        domain: "Quantum Mechanics",
        parent_slug: Some("quantum-mechanics"),
        difficulty: 5,
    },
    CuratedConcept {
        slug: "thermodynamics-statistical-physics",
        title: "Thermodynamics and Statistical Physics",
        description: "Macroscopic thermal laws and their microscopic statistical foundations.",
        domain: "Thermodynamics and Statistical Physics",
        parent_slug: None,
        difficulty: 3,
    },
    CuratedConcept {
        slug: "laws-of-thermodynamics",
        title: "Laws of Thermodynamics",
        description: "The zeroth through third laws and their physical consequences.",
        domain: "Thermodynamics and Statistical Physics",
        parent_slug: Some("thermodynamics-statistical-physics"),
        difficulty: 2,
    },
    CuratedConcept {
        slug: "entropy",
        title: "Entropy",
        description: "Reversibility, the second law, and the statistical meaning of entropy.",
        domain: "Thermodynamics and Statistical Physics",
        parent_slug: Some("thermodynamics-statistical-physics"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "thermodynamic-potentials",
        title: "Thermodynamic Potentials",
        description: "Internal energy, enthalpy, Helmholtz free energy, Gibbs free energy, and Maxwell relations.",
        domain: "Thermodynamics and Statistical Physics",
        parent_slug: Some("thermodynamics-statistical-physics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "kinetic-theory-gases",
        title: "Kinetic Theory of Gases",
        description: "Molecular motion, pressure, transport coefficients, and the Maxwell-Boltzmann distribution.",
        domain: "Thermodynamics and Statistical Physics",
        parent_slug: Some("thermodynamics-statistical-physics"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "statistical-ensembles",
        title: "Statistical Ensembles",
        description: "Microcanonical, canonical, and grand canonical descriptions of equilibrium systems.",
        domain: "Thermodynamics and Statistical Physics",
        parent_slug: Some("thermodynamics-statistical-physics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "phase-transitions",
        title: "Phase Transitions",
        description: "Order parameters, critical phenomena, and classification of phase transitions.",
        domain: "Thermodynamics and Statistical Physics",
        parent_slug: Some("thermodynamics-statistical-physics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "optics-waves",
        title: "Optics and Waves",
        description: "Wave propagation, optical phenomena, and light-matter interaction.",
        domain: "Optics and Waves",
        parent_slug: None,
        difficulty: 2,
    },
    CuratedConcept {
        slug: "interference",
        title: "Interference",
        description: "Superposition of coherent waves, path differences, and interference patterns.",
        domain: "Optics and Waves",
        parent_slug: Some("optics-waves"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "diffraction",
        title: "Diffraction",
        description: "Wave bending around apertures, diffraction gratings, and resolving power.",
        domain: "Optics and Waves",
        parent_slug: Some("optics-waves"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "polarization",
        title: "Polarization",
        description: "Polarized light, polarization states, and polarization-changing optical elements.",
        domain: "Optics and Waves",
        parent_slug: Some("optics-waves"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "geometric-optics",
        title: "Geometric Optics",
        description: "Ray propagation, lenses, mirrors, and image formation.",
        domain: "Optics and Waves",
        parent_slug: Some("optics-waves"),
        difficulty: 2,
    },
    CuratedConcept {
        slug: "laser-physics",
        title: "Laser Physics",
        description: "Stimulated emission, population inversion, optical cavities, and laser gain.",
        domain: "Optics and Waves",
        parent_slug: Some("optics-waves"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "mathematical-physics",
        title: "Mathematical Physics",
        description: "Mathematical methods used to formulate and solve physical problems.",
        domain: "Mathematical Physics",
        parent_slug: None,
        difficulty: 3,
    },
    CuratedConcept {
        slug: "vector-calculus",
        title: "Vector Calculus",
        description: "Gradient, divergence, curl, line integrals, and the integral theorems.",
        domain: "Mathematical Physics",
        parent_slug: Some("mathematical-physics"),
        difficulty: 2,
    },
    CuratedConcept {
        slug: "complex-analysis",
        title: "Complex Analysis",
        description: "Analytic functions, contour integration, residues, and conformal mappings.",
        domain: "Mathematical Physics",
        parent_slug: Some("mathematical-physics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "linear-algebra",
        title: "Linear Algebra",
        description: "Vector spaces, matrices, eigenvalues, and diagonalization.",
        domain: "Mathematical Physics",
        parent_slug: Some("mathematical-physics"),
        difficulty: 2,
    },
    CuratedConcept {
        slug: "differential-equations",
        title: "Differential Equations",
        description: "Ordinary and partial differential equations with physical boundary conditions.",
        domain: "Mathematical Physics",
        parent_slug: Some("mathematical-physics"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "fourier-analysis",
        title: "Fourier Analysis",
        description: "Fourier series, Fourier transforms, and spectral decomposition.",
        domain: "Mathematical Physics",
        parent_slug: Some("mathematical-physics"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "special-functions",
        title: "Special Functions",
        description: "Gamma, Bessel, Legendre, and other functions arising in physical boundary-value problems.",
        domain: "Mathematical Physics",
        parent_slug: Some("mathematical-physics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "atomic-molecular-physics",
        title: "Atomic and Molecular Physics",
        description: "Electronic structure, spectra, and interactions of atoms and molecules.",
        domain: "Atomic and Molecular Physics",
        parent_slug: None,
        difficulty: 3,
    },
    CuratedConcept {
        slug: "atomic-structure",
        title: "Atomic Structure",
        description: "Electronic configurations, fine structure, and atomic energy levels.",
        domain: "Atomic and Molecular Physics",
        parent_slug: Some("atomic-molecular-physics"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "molecular-spectra",
        title: "Molecular Spectra",
        description: "Rotational, vibrational, and electronic transitions in molecules.",
        domain: "Atomic and Molecular Physics",
        parent_slug: Some("atomic-molecular-physics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "spectroscopy-selection-rules",
        title: "Spectroscopy and Selection Rules",
        description: "Transition probabilities, dipole selection rules, and spectroscopic notation.",
        domain: "Atomic and Molecular Physics",
        parent_slug: Some("atomic-molecular-physics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "nuclear-particle-physics",
        title: "Nuclear and Particle Physics",
        description: "Nuclear structure, decay, reactions, and elementary particles.",
        domain: "Nuclear and Particle Physics",
        parent_slug: None,
        difficulty: 3,
    },
    CuratedConcept {
        slug: "nuclear-structure",
        title: "Nuclear Structure",
        description: "Nuclear binding, shell structure, and collective nuclear models.",
        domain: "Nuclear and Particle Physics",
        parent_slug: Some("nuclear-particle-physics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "radioactivity",
        title: "Radioactivity",
        description: "Alpha, beta, and gamma decay, decay laws, and nuclear lifetimes.",
        domain: "Nuclear and Particle Physics",
        parent_slug: Some("nuclear-particle-physics"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "nuclear-reactions",
        title: "Nuclear Reactions",
        description: "Reaction kinematics, cross sections, conservation laws, and reaction mechanisms.",
        domain: "Nuclear and Particle Physics",
        parent_slug: Some("nuclear-particle-physics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "particle-physics",
        title: "Particle Physics",
        description: "Fundamental particles, symmetries, interactions, and the Standard Model.",
        domain: "Nuclear and Particle Physics",
        parent_slug: Some("nuclear-particle-physics"),
        difficulty: 5,
    },
    CuratedConcept {
        slug: "condensed-matter-physics",
        title: "Condensed Matter Physics",
        description: "Structural, vibrational, electronic, and collective properties of solids.",
        domain: "Condensed Matter Physics",
        parent_slug: None,
        difficulty: 3,
    },
    CuratedConcept {
        slug: "crystal-structure",
        title: "Crystal Structure",
        description: "Lattices, symmetry, reciprocal space, and diffraction by crystals.",
        domain: "Condensed Matter Physics",
        parent_slug: Some("condensed-matter-physics"),
        difficulty: 3,
    },
    CuratedConcept {
        slug: "lattice-vibrations",
        title: "Lattice Vibrations",
        description: "Phonons, dispersion relations, and thermal properties of lattice vibrations.",
        domain: "Condensed Matter Physics",
        parent_slug: Some("condensed-matter-physics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "band-theory",
        title: "Band Theory",
        description: "Electronic bands, Brillouin zones, metals, and semiconductors.",
        domain: "Condensed Matter Physics",
        parent_slug: Some("condensed-matter-physics"),
        difficulty: 4,
    },
    CuratedConcept {
        slug: "superconductivity",
        title: "Superconductivity",
        description: "Zero resistance, Meissner effect, Cooper pairs, and superconducting gaps.",
        domain: "Condensed Matter Physics",
        parent_slug: Some("condensed-matter-physics"),
        difficulty: 5,
    },
    CuratedConcept {
        slug: "experimental-physics",
        title: "Experimental Physics",
        description: "Measurement, uncertainty, instrumentation, and experimental practice.",
        domain: "Experimental Physics",
        parent_slug: None,
        difficulty: 2,
    },
    CuratedConcept {
        slug: "error-analysis",
        title: "Error Analysis",
        description: "Statistical and systematic uncertainty, propagation of errors, and data fitting.",
        domain: "Experimental Physics",
        parent_slug: Some("experimental-physics"),
        difficulty: 2,
    },
    CuratedConcept {
        slug: "detectors-instrumentation",
        title: "Detectors and Instrumentation",
        description: "Sensors, detectors, signal chains, and measurement instrumentation.",
        domain: "Experimental Physics",
        parent_slug: Some("experimental-physics"),
        difficulty: 3,
    },
];

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct V2ConceptSearchFilter {
    pub query: Option<String>,
    pub domain: Option<String>,
    pub exam_code: Option<String>,
    pub include_archived: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ConceptDetailInput {
    pub slug: String,
    #[serde(default)]
    pub evidence_limit: Option<i64>,
    #[serde(default)]
    pub history_limit: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ConceptSummary {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub domain: String,
    pub parent_concept_id: Option<i64>,
    pub parent_slug: Option<String>,
    pub parent_title: Option<String>,
    pub difficulty: i64,
    pub is_archived: bool,
    pub mastery: Option<V2ConceptMasterySummary>,
    pub exam_codes: Vec<String>,
    pub verified_exam_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ConceptLink {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub domain: String,
    pub difficulty: i64,
    pub is_archived: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ConceptSyllabusMapping {
    pub exam_id: i64,
    pub exam_code: String,
    pub exam_name: String,
    pub exam_short_name: String,
    pub exam_is_active: bool,
    pub node_id: i64,
    pub node_code: String,
    pub node_title: String,
    pub node_kind: String,
    pub node_category: String,
    pub node_display_order: i64,
    pub node_is_archived: bool,
    pub node_path: Vec<String>,
    pub concept_id: i64,
    pub weight: f64,
    pub is_primary: bool,
    pub notes: String,
    pub status: String,
    pub origin: String,
    pub source_reference: String,
    pub provenance: String,
    pub content_version_id: Option<i64>,
    pub reviewed_at: Option<String>,
    pub reviewed_by: String,
    pub updated_at: String,
    pub source_title: String,
    pub source_publisher: String,
    pub source_url: Option<String>,
    pub source_verified_on: String,
    pub content_version_label: String,
    pub content_review_status: String,
    pub content_verified_on: Option<String>,
    pub is_approved: bool,
    pub is_verified: bool,
    pub progress_status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ConceptDetail {
    pub concept: V2ConceptSummary,
    pub children: Vec<V2ConceptLink>,
    pub mappings: Vec<V2ConceptSyllabusMapping>,
    pub evidence: Vec<V2ConceptEvidence>,
    pub mastery_history: Vec<V2MasteryRunSummary>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ConceptMappingUpdateInput {
    pub exam_id: i64,
    pub node_id: i64,
    pub concept_id: i64,
    pub status: String,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ExamConceptCoverage {
    pub exam_id: i64,
    pub exam_code: String,
    pub exam_name: String,
    pub exam_short_name: String,
    pub exam_is_active: bool,
    pub configured: bool,
    pub status: String,
    pub content_version_label: Option<String>,
    pub content_review_status: Option<String>,
    pub content_verified_on: Option<String>,
    pub node_count: i64,
    pub topic_node_count: i64,
    pub revised_node_count: i64,
    pub mapped_concept_count: i64,
    pub verified_mapped_concept_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ConceptSearchResult {
    pub concepts: Vec<V2ConceptSummary>,
    pub total: i64,
    pub domains: Vec<String>,
    pub exam_coverage: Vec<V2ExamConceptCoverage>,
}

pub fn prepare(connection: &mut Connection) -> Result<(), AppError> {
    ensure_curated_concepts(connection)?;
    ensure_mapping_catalogue(connection)
}

fn mapping_column_exists(connection: &Connection, column: &str) -> Result<bool, AppError> {
    let mut statement = connection
        .prepare("PRAGMA table_info(exam_node_concepts)")
        .map_err(|error| {
            AppError::database("The concept mapping schema could not be read", error)
        })?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| {
            AppError::database("The concept mapping columns could not be read", error)
        })?;
    for name in names {
        if name.map_err(|error| AppError::database("A concept mapping column is invalid", error))?
            == column
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn ensure_mapping_catalogue(connection: &mut Connection) -> Result<(), AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The concept mapping transaction could not start", error)
    })?;
    let columns = [
        (
            "status",
            "ALTER TABLE exam_node_concepts ADD COLUMN status TEXT NOT NULL DEFAULT 'proposed' CHECK (status IN ('proposed','reviewed','approved','rejected'))",
        ),
        (
            "origin",
            "ALTER TABLE exam_node_concepts ADD COLUMN origin TEXT NOT NULL DEFAULT 'curated' CHECK (origin IN ('curated','official'))",
        ),
        (
            "source_reference",
            "ALTER TABLE exam_node_concepts ADD COLUMN source_reference TEXT NOT NULL DEFAULT '' CHECK (length(source_reference) <= 2000)",
        ),
        (
            "provenance",
            "ALTER TABLE exam_node_concepts ADD COLUMN provenance TEXT NOT NULL DEFAULT '' CHECK (length(provenance) <= 2000)",
        ),
        (
            "content_version_id",
            "ALTER TABLE exam_node_concepts ADD COLUMN content_version_id INTEGER",
        ),
        (
            "reviewed_at",
            "ALTER TABLE exam_node_concepts ADD COLUMN reviewed_at TEXT",
        ),
        (
            "reviewed_by",
            "ALTER TABLE exam_node_concepts ADD COLUMN reviewed_by TEXT NOT NULL DEFAULT '' CHECK (length(reviewed_by) <= 200)",
        ),
        (
            "updated_at",
            "ALTER TABLE exam_node_concepts ADD COLUMN updated_at TEXT NOT NULL DEFAULT ''",
        ),
    ];
    for (name, statement) in columns {
        if !mapping_column_exists(&transaction, name)? {
            transaction.execute_batch(statement).map_err(|error| {
                AppError::database(
                    "The additive concept mapping schema could not be applied",
                    error,
                )
            })?;
        }
    }
    transaction
        .execute_batch(
            "UPDATE exam_node_concepts
             SET content_version_id = COALESCE(content_version_id, (
                     SELECT s.content_version_id
                     FROM exam_syllabus_nodes n
                     JOIN exam_syllabus_sources s
                       ON s.exam_id = n.exam_id AND s.id = n.source_id
                     WHERE n.exam_id = exam_node_concepts.exam_id
                       AND n.id = exam_node_concepts.node_id
                 )),
                 updated_at = CASE WHEN updated_at = '' THEN created_at ELSE updated_at END,
                 provenance = CASE WHEN provenance = ''
                     THEN 'Legacy mapping migrated; review required.'
                     ELSE provenance END;",
        )
        .map_err(|error| {
            AppError::database("Legacy concept mappings could not be initialized", error)
        })?;

    let seeds: Vec<MappingSeed> = serde_json::from_str(MAPPING_SEED_JSON).map_err(|error| {
        AppError::internal(format!("The concept mapping seed is invalid: {error}"))
    })?;
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    for seed in seeds {
        if seed.status != "proposed" {
            return Err(AppError::v2_integrity(format!(
                "Concept mapping seed {} / {} is not marked proposed.",
                seed.concept_slug, seed.node_code
            )));
        }
        if seed.origin != "curated" && seed.origin != "official" {
            return Err(AppError::v2_integrity(format!(
                "Concept mapping seed {} / {} has an invalid origin.",
                seed.concept_slug, seed.node_code
            )));
        }
        let Some((exam_id, node_id, content_version_id, source_reference, version_label)) =
            transaction
                .query_row(
                    "SELECT e.id, n.id, s.content_version_id,
                            COALESCE(NULLIF(s.source_reference, ''), s.url, ''),
                            cv.version_label
                     FROM exams e
                     JOIN exam_syllabus_nodes n ON n.exam_id = e.id
                     JOIN exam_syllabus_sources s
                       ON s.exam_id = n.exam_id AND s.id = n.source_id
                     JOIN exam_content_versions cv
                       ON cv.exam_id = s.exam_id AND cv.id = s.content_version_id
                     WHERE e.code = ?1 AND n.code = ?2 AND n.is_archived = 0",
                    params![seed.exam_code, seed.node_code],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                        ))
                    },
                )
                .optional()
                .map_err(|error| {
                    AppError::database("A concept mapping seed node could not be resolved", error)
                })?
        else {
            continue;
        };
        let concept_id: Option<i64> = transaction
            .query_row(
                "SELECT id FROM concepts WHERE slug = ?1",
                [&seed.concept_slug],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| {
                AppError::database("A mapping concept could not be resolved", error)
            })?;
        let Some(concept_id) = concept_id else {
            continue;
        };
        let provenance = format!(
            "Curated crosswalk derived from official syllabus wording in {version_label}. \
             This is not an official endorsement and requires local review."
        );
        transaction
            .execute(
                "INSERT INTO exam_node_concepts (
                    exam_id, node_id, concept_id, weight, is_primary, notes, created_at,
                    status, origin, source_reference, provenance, content_version_id,
                    reviewed_at, reviewed_by, updated_at
                 ) VALUES (?1, ?2, ?3, 1, 0, ?4, ?5, 'proposed', ?6, ?7, ?8, ?9,
                           NULL, '', ?5)
                 ON CONFLICT(node_id, concept_id) DO NOTHING",
                params![
                    exam_id,
                    node_id,
                    concept_id,
                    seed.notes,
                    now,
                    seed.origin,
                    source_reference,
                    provenance,
                    content_version_id,
                ],
            )
            .map_err(|error| {
                AppError::database("A proposed concept mapping could not be stored", error)
            })?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("Concept mappings could not be committed", error))
}

pub fn ensure_curated_concepts(connection: &mut Connection) -> Result<(), AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The curated concept transaction could not start", error)
    })?;
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    for concept in CURATED_CONCEPTS {
        let parent_id = match concept.parent_slug {
            Some(parent_slug) => Some(
                transaction
                    .query_row(
                        "SELECT id FROM concepts WHERE slug = ?1",
                        [parent_slug],
                        |row| row.get::<_, i64>(0),
                    )
                    .optional()
                    .map_err(|error| {
                        AppError::database("A curated concept parent could not be checked", error)
                    })?
                    .ok_or_else(|| {
                        AppError::v2_integrity(format!(
                            "Curated concept {} references missing parent {}.",
                            concept.slug, parent_slug
                        ))
                    })?,
            ),
            None => None,
        };
        transaction
            .execute(
                "INSERT INTO concepts (
                    slug, title, description, domain, parent_concept_id, difficulty,
                    is_archived, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?7)
                 ON CONFLICT(slug) DO NOTHING",
                params![
                    concept.slug,
                    concept.title,
                    concept.description,
                    concept.domain,
                    parent_id,
                    concept.difficulty,
                    now,
                ],
            )
            .map_err(|error| AppError::database("A curated concept could not be stored", error))?;
    }

    transaction
        .commit()
        .map_err(|error| AppError::database("Curated concepts could not be committed", error))
}

pub fn search_concepts(
    connection: &Connection,
    filter: V2ConceptSearchFilter,
) -> Result<V2ConceptSearchResult, AppError> {
    let query = normalize_optional(filter.query, 200, "query")?;
    let domain = normalize_optional(filter.domain, 120, "domain")?;
    let exam_code = normalize_optional(filter.exam_code, 40, "examCode")?;
    let include_archived = filter.include_archived.unwrap_or(false);
    let limit = filter.limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
    let offset = filter.offset.unwrap_or(0);
    if !(1..=MAX_SEARCH_LIMIT).contains(&limit) {
        return Err(AppError::validation(
            "limit",
            "Concept search limit must be between 1 and 200.",
        ));
    }
    if offset < 0 {
        return Err(AppError::validation(
            "offset",
            "Concept search offset cannot be negative.",
        ));
    }
    if let Some(code) = &exam_code {
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM exams WHERE code = ?1)",
                [code],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Exam filter could not be checked", error))?;
        if exists == 0 {
            return Err(AppError::validation(
                "examCode",
                "Choose one of the four supported examinations.",
            ));
        }
    }

    let mut where_parts = Vec::new();
    let mut values: Vec<Value> = Vec::new();
    if !include_archived {
        where_parts.push("c.is_archived = 0".to_owned());
    }
    if let Some(query) = &query {
        let pattern = format!("%{}%", escape_like(query));
        where_parts.push(
            "(c.title LIKE ? ESCAPE '\\' COLLATE NOCASE
              OR c.slug LIKE ? ESCAPE '\\' COLLATE NOCASE
              OR c.description LIKE ? ESCAPE '\\' COLLATE NOCASE
              OR c.domain LIKE ? ESCAPE '\\' COLLATE NOCASE)"
                .to_owned(),
        );
        for _ in 0..4 {
            values.push(Value::Text(pattern.clone()));
        }
    }
    if let Some(domain) = &domain {
        where_parts.push("c.domain = ? COLLATE NOCASE".to_owned());
        values.push(Value::Text(domain.clone()));
    }
    if let Some(code) = &exam_code {
        where_parts.push(
            "EXISTS (
                SELECT 1
                FROM exam_node_concepts nc
                JOIN exam_syllabus_nodes n ON n.exam_id = nc.exam_id AND n.id = nc.node_id
                JOIN exams e ON e.id = nc.exam_id
                WHERE nc.concept_id = c.id AND e.code = ?
            )"
            .to_owned(),
        );
        values.push(Value::Text(code.clone()));
    }
    let where_sql = if where_parts.is_empty() {
        "1 = 1".to_owned()
    } else {
        where_parts.join(" AND ")
    };

    let total: i64 = connection
        .query_row(
            &format!("SELECT COUNT(*) FROM concepts c WHERE {where_sql}"),
            params_from_iter(values.iter()),
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Concept search count could not be read", error))?;

    let mut main_values = values;
    main_values.push(Value::Integer(limit));
    main_values.push(Value::Integer(offset));
    let mut statement = connection
        .prepare(&format!(
            "SELECT c.id, c.slug, c.title, c.description, c.domain, c.parent_concept_id,
                    p.slug, p.title, c.difficulty, c.is_archived
             FROM concepts c
             LEFT JOIN concepts p ON p.id = c.parent_concept_id
             WHERE {where_sql}
             ORDER BY c.domain COLLATE NOCASE, c.title COLLATE NOCASE, c.id
             LIMIT ? OFFSET ?"
        ))
        .map_err(|error| AppError::database("Concept search could not be prepared", error))?;
    let rows = statement
        .query_map(params_from_iter(main_values.iter()), |row| {
            Ok(V2ConceptSummary {
                id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                domain: row.get(4)?,
                parent_concept_id: row.get(5)?,
                parent_slug: row.get(6)?,
                parent_title: row.get(7)?,
                difficulty: row.get(8)?,
                is_archived: row.get::<_, i64>(9)? == 1,
                mastery: None,
                exam_codes: Vec::new(),
                verified_exam_codes: Vec::new(),
            })
        })
        .map_err(|error| AppError::database("Concept search could not be read", error))?;
    let mut concepts = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A concept search row is invalid", error))?;
    drop(statement);

    let ids: Vec<i64> = concepts.iter().map(|concept| concept.id).collect();
    let mastery = mastery::load_mastery_summaries(connection, &ids)?;
    let mappings = load_concept_exam_codes(connection, &ids)?;
    for concept in &mut concepts {
        concept.mastery = mastery.get(&concept.id).cloned();
        if let Some((codes, verified)) = mappings.get(&concept.id) {
            concept.exam_codes = codes.clone();
            concept.verified_exam_codes = verified.clone();
        }
    }

    let mut domain_statement = connection
        .prepare(
            "SELECT DISTINCT domain FROM concepts
             WHERE is_archived = 0
             ORDER BY domain COLLATE NOCASE",
        )
        .map_err(|error| AppError::database("Concept domains could not be prepared", error))?;
    let domains = domain_statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| AppError::database("Concept domains could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A concept domain is invalid", error))?;

    Ok(V2ConceptSearchResult {
        concepts,
        total,
        domains,
        exam_coverage: exam_coverage(connection)?,
    })
}

pub fn get_concept_detail(
    connection: &Connection,
    input: V2ConceptDetailInput,
) -> Result<V2ConceptDetail, AppError> {
    let slug = input.slug.trim();
    if slug.is_empty() || slug.len() > 120 {
        return Err(AppError::validation(
            "slug",
            "Choose a concept from the shared library.",
        ));
    }
    let evidence_limit = input
        .evidence_limit
        .unwrap_or(mastery::DEFAULT_EVIDENCE_LIMIT);
    let history_limit = input
        .history_limit
        .unwrap_or(mastery::DEFAULT_HISTORY_LIMIT);

    let mut concept = connection
        .query_row(
            "SELECT c.id, c.slug, c.title, c.description, c.domain, c.parent_concept_id,
                    p.slug, p.title, c.difficulty, c.is_archived
             FROM concepts c
             LEFT JOIN concepts p ON p.id = c.parent_concept_id
             WHERE c.slug = ?1",
            [slug],
            |row| {
                Ok(V2ConceptSummary {
                    id: row.get(0)?,
                    slug: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    domain: row.get(4)?,
                    parent_concept_id: row.get(5)?,
                    parent_slug: row.get(6)?,
                    parent_title: row.get(7)?,
                    difficulty: row.get(8)?,
                    is_archived: row.get::<_, i64>(9)? == 1,
                    mastery: None,
                    exam_codes: Vec::new(),
                    verified_exam_codes: Vec::new(),
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("Concept detail could not be read", error))?
        .ok_or_else(AppError::concept_not_found)?;

    concept.mastery = mastery::load_mastery_summary(connection, concept.id)?;
    if let Some((codes, verified)) =
        load_concept_exam_codes(connection, &[concept.id])?.remove(&concept.id)
    {
        concept.exam_codes = codes;
        concept.verified_exam_codes = verified;
    }

    let mut child_statement = connection
        .prepare(
            "SELECT id, slug, title, domain, difficulty, is_archived
             FROM concepts
             WHERE parent_concept_id = ?1
             ORDER BY title COLLATE NOCASE, id",
        )
        .map_err(|error| AppError::database("Concept children could not be prepared", error))?;
    let children = child_statement
        .query_map([concept.id], |row| {
            Ok(V2ConceptLink {
                id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                domain: row.get(3)?,
                difficulty: row.get(4)?,
                is_archived: row.get::<_, i64>(5)? == 1,
            })
        })
        .map_err(|error| AppError::database("Concept children could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A concept child row is invalid", error))?;

    let mut mapping_statement = connection
        .prepare(
            "SELECT e.id, e.code, e.name, e.short_name, ep.is_active,
                    n.id, n.code, n.title, n.node_kind, n.category, n.display_order,
                    n.is_archived, nc.weight, nc.is_primary, nc.notes,
                    s.title, s.publisher, s.url, s.verified_on,
                    cv.version_label, cv.review_status, cv.verified_on,
                    COALESCE(prog.status, 'not_started'),
                    nc.concept_id, nc.status, nc.origin, nc.source_reference,
                    nc.provenance, nc.content_version_id, nc.reviewed_at,
                    nc.reviewed_by, nc.updated_at
             FROM exam_node_concepts nc
             JOIN exams e ON e.id = nc.exam_id
             JOIN exam_profiles ep ON ep.exam_id = e.id
             JOIN exam_syllabus_nodes n ON n.exam_id = nc.exam_id AND n.id = nc.node_id
             JOIN exam_syllabus_sources s ON s.exam_id = n.exam_id AND s.id = n.source_id
             JOIN exam_content_versions cv ON cv.exam_id = s.exam_id AND cv.id = s.content_version_id
             LEFT JOIN exam_node_progress prog ON prog.exam_id = n.exam_id AND prog.node_id = n.id
             WHERE nc.concept_id = ?1
             ORDER BY e.display_order, n.display_order, n.id",
        )
        .map_err(|error| AppError::database("Concept syllabus mappings could not be prepared", error))?;
    let mut mappings = mapping_statement
        .query_map([concept.id], |row| {
            let node_is_archived = row.get::<_, i64>(11)? == 1;
            let review_status: String = row.get(20)?;
            let mapping_status: String = row.get(24)?;
            Ok(V2ConceptSyllabusMapping {
                exam_id: row.get(0)?,
                exam_code: row.get(1)?,
                exam_name: row.get(2)?,
                exam_short_name: row.get(3)?,
                exam_is_active: row.get::<_, i64>(4)? == 1,
                node_id: row.get(5)?,
                node_code: row.get(6)?,
                node_title: row.get(7)?,
                node_kind: row.get(8)?,
                node_category: row.get(9)?,
                node_display_order: row.get(10)?,
                node_is_archived,
                node_path: Vec::new(),
                concept_id: row.get(23)?,
                weight: row.get(12)?,
                is_primary: row.get::<_, i64>(13)? == 1,
                notes: row.get(14)?,
                status: mapping_status.clone(),
                origin: row.get(25)?,
                source_reference: row.get(26)?,
                provenance: row.get(27)?,
                content_version_id: row.get(28)?,
                reviewed_at: row.get(29)?,
                reviewed_by: row.get(30)?,
                updated_at: row.get(31)?,
                source_title: row.get(15)?,
                source_publisher: row.get(16)?,
                source_url: row.get(17)?,
                source_verified_on: row.get(18)?,
                content_version_label: row.get(19)?,
                is_approved: mapping_status == "approved",
                is_verified: review_status == "verified" && !node_is_archived,
                content_review_status: review_status,
                content_verified_on: row.get(21)?,
                progress_status: row.get(22)?,
            })
        })
        .map_err(|error| AppError::database("Concept syllabus mappings could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A concept syllabus mapping is invalid", error))?;
    drop(mapping_statement);

    let mut path_cache: HashMap<i64, NodePathMap> = HashMap::new();
    for mapping in &mut mappings {
        if let std::collections::hash_map::Entry::Vacant(entry) = path_cache.entry(mapping.exam_id)
        {
            entry.insert(load_node_paths(connection, mapping.exam_id)?);
        }
        mapping.node_path = path_cache
            .get(&mapping.exam_id)
            .and_then(|paths| paths.get(&mapping.node_id))
            .cloned()
            .unwrap_or_else(|| vec![mapping.node_title.clone()]);
    }

    Ok(V2ConceptDetail {
        concept: concept.clone(),
        children,
        mappings,
        evidence: mastery::load_evidence(connection, concept.id, evidence_limit)?,
        mastery_history: mastery::load_history(connection, concept.id, history_limit)?,
    })
}

pub fn update_mapping(
    connection: &mut Connection,
    input: V2ConceptMappingUpdateInput,
    today: chrono::NaiveDate,
    now: chrono::DateTime<Utc>,
) -> Result<V2ConceptDetail, AppError> {
    if !matches!(
        input.status.as_str(),
        "proposed" | "reviewed" | "approved" | "rejected"
    ) {
        return Err(AppError::validation(
            "status",
            "Choose proposed, reviewed, approved, or rejected.",
        ));
    }
    if input.exam_id <= 0 || input.node_id <= 0 || input.concept_id <= 0 {
        return Err(AppError::validation(
            "mapping",
            "Choose an existing concept mapping.",
        ));
    }
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The concept mapping review transaction could not start",
            error,
        )
    })?;
    let current_notes: Option<String> = transaction
        .query_row(
            "SELECT notes FROM exam_node_concepts
             WHERE exam_id = ?1 AND node_id = ?2 AND concept_id = ?3",
            params![input.exam_id, input.node_id, input.concept_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("The concept mapping could not be read", error))?;
    let Some(current_notes) = current_notes else {
        return Err(AppError::validation(
            "mapping",
            "Choose an existing concept mapping.",
        ));
    };
    let notes = input.notes.unwrap_or(current_notes);
    if notes.chars().count() > 1_000 {
        return Err(AppError::validation(
            "notes",
            "Mapping notes must be 1,000 characters or fewer.",
        ));
    }
    let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let reviewed = input.status != "proposed";
    transaction
        .execute(
            "UPDATE exam_node_concepts
             SET status = ?1, notes = ?2, reviewed_at = ?3,
                 reviewed_by = ?4, updated_at = ?5
             WHERE exam_id = ?6 AND node_id = ?7 AND concept_id = ?8",
            params![
                input.status,
                notes.trim(),
                if reviewed {
                    Some(now_text.clone())
                } else {
                    None
                },
                if reviewed { "local reviewer" } else { "" },
                now_text,
                input.exam_id,
                input.node_id,
                input.concept_id,
            ],
        )
        .map_err(|error| {
            AppError::database("The concept mapping review could not be saved", error)
        })?;
    if input.status == "approved" {
        crate::v2::revision::ensure_initial_schedule(&transaction, input.concept_id, today, now)?;
    }
    let slug: String = transaction
        .query_row(
            "SELECT slug FROM concepts WHERE id = ?1",
            [input.concept_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("The mapped concept could not be read", error))?;
    let detail = get_concept_detail(
        &transaction,
        V2ConceptDetailInput {
            slug,
            evidence_limit: None,
            history_limit: None,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The mapping review could not be committed", error))?;
    Ok(detail)
}

pub fn exam_coverage(connection: &Connection) -> Result<Vec<V2ExamConceptCoverage>, AppError> {
    let mut exam_statement = connection
        .prepare(
            "SELECT e.id, e.code, e.name, e.short_name, p.is_active
             FROM exams e
             JOIN exam_profiles p ON p.exam_id = e.id
             ORDER BY e.display_order",
        )
        .map_err(|error| AppError::database("Exam coverage could not be prepared", error))?;
    let exams = exam_statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)? == 1,
            ))
        })
        .map_err(|error| AppError::database("Exam coverage could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("An exam coverage row is invalid", error))?;

    let mut coverage = Vec::with_capacity(exams.len());
    for (exam_id, exam_code, exam_name, exam_short_name, exam_is_active) in exams {
        let content = connection
            .query_row(
                "SELECT version_label, review_status, verified_on
                 FROM exam_content_versions
                 WHERE exam_id = ?1 AND content_type = 'syllabus'
                 ORDER BY id DESC LIMIT 1",
                [exam_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| {
                AppError::database("Syllabus content version could not be read", error)
            })?;
        let (node_count, topic_node_count, revised_node_count): (i64, i64, i64) = connection
            .query_row(
                "SELECT COUNT(*),
                        COALESCE(SUM(CASE WHEN n.node_kind = 'topic' THEN 1 ELSE 0 END), 0),
                        COALESCE(SUM(CASE WHEN prog.status IN ('revised','confident') THEN 1 ELSE 0 END), 0)
                 FROM exam_syllabus_nodes n
                 LEFT JOIN exam_node_progress prog
                    ON prog.exam_id = n.exam_id AND prog.node_id = n.id
                 WHERE n.exam_id = ?1 AND n.is_archived = 0",
                [exam_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|error| AppError::database("Syllabus node counts could not be read", error))?;
        let (mapped_concept_count, verified_mapped_concept_count): (i64, i64) = connection
            .query_row(
                "SELECT COUNT(DISTINCT nc.concept_id),
                        COUNT(DISTINCT CASE WHEN nc.status = 'approved'
                                             AND cv.review_status = 'verified'
                                            THEN nc.concept_id END)
                 FROM exam_node_concepts nc
                 JOIN exam_syllabus_nodes n
                    ON n.exam_id = nc.exam_id AND n.id = nc.node_id
                 JOIN concepts c ON c.id = nc.concept_id AND c.is_archived = 0
                 JOIN exam_syllabus_sources s
                    ON s.exam_id = n.exam_id AND s.id = n.source_id
                 JOIN exam_content_versions cv
                    ON cv.exam_id = s.exam_id AND cv.id = s.content_version_id
                 WHERE nc.exam_id = ?1 AND n.is_archived = 0 AND nc.status <> 'rejected'",
                [exam_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| {
                AppError::database("Concept mapping counts could not be read", error)
            })?;

        let configured = content.is_some() && node_count > 0;
        let status = if !configured {
            "not_configured"
        } else if mapped_concept_count == 0 {
            "unmapped"
        } else if verified_mapped_concept_count == 0 {
            "unverified"
        } else if verified_mapped_concept_count < mapped_concept_count {
            "partially_verified"
        } else {
            "verified"
        };

        coverage.push(V2ExamConceptCoverage {
            exam_id,
            exam_code,
            exam_name,
            exam_short_name,
            exam_is_active,
            configured,
            status: status.to_owned(),
            content_version_label: content.as_ref().map(|value| value.0.clone()),
            content_review_status: content.as_ref().map(|value| value.1.clone()),
            content_verified_on: content.and_then(|value| value.2),
            node_count,
            topic_node_count,
            revised_node_count,
            mapped_concept_count,
            verified_mapped_concept_count,
        });
    }
    Ok(coverage)
}

fn load_concept_exam_codes(
    connection: &Connection,
    concept_ids: &[i64],
) -> Result<ConceptExamCodes, AppError> {
    if concept_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders = vec!["?"; concept_ids.len()].join(",");
    let mut statement = connection
        .prepare(&format!(
            "SELECT nc.concept_id, e.code, cv.review_status, n.is_archived, nc.status
             FROM exam_node_concepts nc
             JOIN exams e ON e.id = nc.exam_id
             JOIN exam_syllabus_nodes n ON n.exam_id = nc.exam_id AND n.id = nc.node_id
             JOIN exam_syllabus_sources s ON s.exam_id = n.exam_id AND s.id = n.source_id
             JOIN exam_content_versions cv ON cv.exam_id = s.exam_id AND cv.id = s.content_version_id
             WHERE nc.concept_id IN ({placeholders}) AND nc.status <> 'rejected'
             ORDER BY e.display_order, n.display_order"
        ))
        .map_err(|error| AppError::database("Concept exam mappings could not be prepared", error))?;
    let rows = statement
        .query_map(params_from_iter(concept_ids.iter().copied()), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)? == 1,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|error| AppError::database("Concept exam mappings could not be read", error))?;

    let mut mappings: ConceptExamCodes = HashMap::new();
    for row in rows {
        let (concept_id, exam_code, review_status, archived, mapping_status) = row
            .map_err(|error| AppError::database("A concept exam mapping row is invalid", error))?;
        let entry = mappings.entry(concept_id).or_default();
        if !entry.0.contains(&exam_code) {
            entry.0.push(exam_code.clone());
        }
        if review_status == "verified"
            && mapping_status == "approved"
            && !archived
            && !entry.1.contains(&exam_code)
        {
            entry.1.push(exam_code);
        }
    }
    Ok(mappings)
}

fn load_node_paths(connection: &Connection, exam_id: i64) -> Result<NodePathMap, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT id, parent_id, title
             FROM exam_syllabus_nodes
             WHERE exam_id = ?1",
        )
        .map_err(|error| AppError::database("Syllabus nodes could not be prepared", error))?;
    let rows = statement
        .query_map([exam_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| AppError::database("Syllabus nodes could not be read", error))?;
    let mut nodes = HashMap::new();
    for row in rows {
        let (id, parent_id, title) =
            row.map_err(|error| AppError::database("A syllabus node row is invalid", error))?;
        nodes.insert(id, (parent_id, title));
    }

    let mut paths = HashMap::new();
    for node_id in nodes.keys().copied().collect::<Vec<_>>() {
        let mut path = Vec::new();
        let mut current = Some(node_id);
        let mut seen = HashSet::new();
        while let Some(id) = current {
            if !seen.insert(id) {
                break;
            }
            if let Some((parent_id, title)) = nodes.get(&id) {
                path.push(title.clone());
                current = *parent_id;
            } else {
                break;
            }
        }
        path.reverse();
        paths.insert(node_id, path);
    }
    Ok(paths)
}

fn normalize_optional(
    value: Option<String>,
    max_len: usize,
    field: &'static str,
) -> Result<Option<String>, AppError> {
    match value {
        None => Ok(None),
        Some(value) => {
            let trimmed = value.trim().to_owned();
            if trimmed.is_empty() {
                Ok(None)
            } else if trimmed.chars().count() > max_len {
                Err(AppError::validation(
                    field,
                    format!("This filter must be {max_len} characters or fewer."),
                ))
            } else {
                Ok(Some(trimmed))
            }
        }
    }
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::mastery::{self, V2UpdateConceptMasteryInput};
    use chrono::{DateTime, Utc};

    fn state() -> (tempfile::TempDir, crate::v2::V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = crate::v2::V2State::initialize(&root.path().join("v2"));
        state
            .with_connection(|connection| {
                connection
                    .execute("DELETE FROM exam_node_concepts", [])
                    .map_err(|error| AppError::database("test mapping clear", error))?;
                Ok(())
            })
            .expect("clear seeded mappings");
        (root, state)
    }

    fn seeded_state() -> (tempfile::TempDir, crate::v2::V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = crate::v2::V2State::initialize(&root.path().join("v2"));
        (root, state)
    }

    fn concept_id(connection: &Connection, slug: &str) -> i64 {
        connection
            .query_row("SELECT id FROM concepts WHERE slug = ?1", [slug], |row| {
                row.get(0)
            })
            .expect("concept id")
    }

    fn add_syllabus(
        connection: &mut Connection,
        exam_id: i64,
        version_label: &str,
        review_status: &str,
    ) -> (i64, i64, i64) {
        let now = "2026-11-04T10:00:00Z";
        connection
            .execute(
                "INSERT INTO exam_content_versions (
                    exam_id, content_type, version_label, source_reference,
                    review_status, created_at
                 ) VALUES (?1, 'syllabus', ?2, 'test fixture', ?3, ?4)",
                params![exam_id, version_label, review_status, now],
            )
            .expect("content version");
        let content_version_id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO exam_syllabus_sources (
                    exam_id, content_version_id, code, title, publisher,
                    source_reference, verified_on, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, 'Test syllabus source', 'Test publisher',
                           'test fixture', '2026-11-04', ?4, ?4)",
                params![
                    exam_id,
                    content_version_id,
                    format!("{version_label}-SOURCE"),
                    now,
                ],
            )
            .expect("syllabus source");
        let source_id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO exam_syllabus_nodes (
                    exam_id, source_id, code, title, node_kind, category,
                    display_order, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, 'Mechanics Unit', 'unit', 'Mechanics', 1, ?4, ?4)",
                params![exam_id, source_id, format!("{version_label}-UNIT"), now,],
            )
            .expect("unit node");
        let unit_id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO exam_syllabus_nodes (
                    exam_id, parent_id, source_id, code, title, node_kind, category,
                    display_order, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, 'Kinematics Topic', 'topic', 'Mechanics',
                           1, ?5, ?5)",
                params![
                    exam_id,
                    unit_id,
                    source_id,
                    format!("{version_label}-TOPIC"),
                    now,
                ],
            )
            .expect("topic node");
        let topic_id = connection.last_insert_rowid();
        (content_version_id, source_id, topic_id)
    }

    #[test]
    fn curated_library_is_complete_idempotent_and_hierarchical() {
        let (_root, state) = state();
        let count = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM concepts", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .map_err(|error| AppError::database("concept count", error))
            })
            .expect("count");
        assert_eq!(count, CURATED_CONCEPTS.len() as i64);
        let mut slugs = HashSet::new();
        for concept in CURATED_CONCEPTS {
            assert!(
                slugs.insert(concept.slug),
                "duplicate slug {}",
                concept.slug
            );
            if let Some(parent) = concept.parent_slug {
                assert_ne!(parent, concept.slug);
                assert!(CURATED_CONCEPTS.iter().any(|item| item.slug == parent));
            }
        }

        let id_before = state
            .with_connection(|connection| Ok(concept_id(connection, "kinematics")))
            .expect("id before");
        state
            .with_connection(ensure_curated_concepts)
            .expect("second library pass");
        let id_after = state
            .with_connection(|connection| Ok(concept_id(connection, "kinematics")))
            .expect("id after");
        assert_eq!(id_before, id_after);
    }

    #[test]
    fn concept_search_filters_domains_archive_state_and_pagination() {
        let (_root, state) = state();
        let result = state
            .with_connection(|connection| {
                search_concepts(
                    connection,
                    V2ConceptSearchFilter {
                        query: Some("schrodinger".to_owned()),
                        ..Default::default()
                    },
                )
            })
            .expect("search");
        assert_eq!(result.total, 1);
        assert_eq!(result.concepts[0].slug, "schrodinger-equation");

        let quantum = state
            .with_connection(|connection| {
                search_concepts(
                    connection,
                    V2ConceptSearchFilter {
                        domain: Some("Quantum Mechanics".to_owned()),
                        limit: Some(3),
                        ..Default::default()
                    },
                )
            })
            .expect("domain search");
        assert!(quantum.total >= 8);
        assert_eq!(quantum.concepts.len(), 3);
        assert!(
            quantum
                .concepts
                .iter()
                .all(|concept| concept.domain == "Quantum Mechanics")
        );

        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE concepts SET is_archived = 1 WHERE slug = 'schrodinger-equation'",
                        [],
                    )
                    .map_err(|error| AppError::database("archive concept", error))?;
                Ok(())
            })
            .expect("archive");
        let hidden = state
            .with_connection(|connection| {
                search_concepts(
                    connection,
                    V2ConceptSearchFilter {
                        query: Some("schrodinger".to_owned()),
                        ..Default::default()
                    },
                )
            })
            .expect("hidden search");
        assert_eq!(hidden.total, 0);
        let visible = state
            .with_connection(|connection| {
                search_concepts(
                    connection,
                    V2ConceptSearchFilter {
                        query: Some("schrodinger".to_owned()),
                        include_archived: Some(true),
                        ..Default::default()
                    },
                )
            })
            .expect("archived search");
        assert_eq!(visible.total, 1);
        assert!(visible.concepts[0].is_archived);
    }

    #[test]
    fn proposed_mapping_catalogue_is_curated_and_requires_review() {
        let (_root, state) = seeded_state();
        let (total, proposed, approved, official, rejected) = state
            .with_connection(|connection| {
                let one = |connection: &Connection, predicate: &str| {
                    connection
                        .query_row(
                            &format!("SELECT COUNT(*) FROM exam_node_concepts WHERE {predicate}"),
                            [],
                            |row| row.get::<_, i64>(0),
                        )
                        .map_err(|error| AppError::database("mapping count", error))
                };
                Ok((
                    one(connection, "1 = 1")?,
                    one(connection, "status = 'proposed'")?,
                    one(connection, "status = 'approved'")?,
                    one(connection, "origin = 'official'")?,
                    one(connection, "status = 'rejected'")?,
                ))
            })
            .expect("seed counts");
        assert_eq!(total, 108);
        assert_eq!(proposed, total);
        assert_eq!(approved, 0);
        assert_eq!(official, 0);
        assert_eq!(rejected, 0);

        let detail = state
            .with_connection(|connection| {
                get_concept_detail(
                    connection,
                    V2ConceptDetailInput {
                        slug: "classical-mechanics".to_owned(),
                        evidence_limit: None,
                        history_limit: None,
                    },
                )
            })
            .expect("seeded concept detail");
        assert!(detail.mappings.len() >= 4);
        assert!(
            detail
                .mappings
                .iter()
                .all(|mapping| mapping.status == "proposed" && mapping.origin == "curated")
        );

        state
            .with_connection(prepare)
            .expect("idempotent mapping prepare");
        let after: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM exam_node_concepts", [], |row| {
                        row.get(0)
                    })
                    .map_err(|error| AppError::database("mapping count", error))
            })
            .expect("mapping count");
        assert_eq!(after, total, "mapping preparation must be idempotent");
    }

    #[test]
    fn copied_legacy_database_gains_mapping_review_columns_without_auto_approval() {
        use std::fs;

        let root = tempfile::tempdir().expect("temporary directory");
        let source_path = root.path().join("legacy.sqlite3");
        {
            let connection = Connection::open(&source_path).expect("legacy database");
            connection
                .execute_batch(
                    "CREATE TABLE concepts (
                        id INTEGER PRIMARY KEY,
                        slug TEXT NOT NULL UNIQUE
                     );
                     CREATE TABLE exams (
                        id INTEGER PRIMARY KEY,
                        code TEXT NOT NULL UNIQUE
                     );
                     CREATE TABLE exam_content_versions (
                        id INTEGER PRIMARY KEY,
                        exam_id INTEGER NOT NULL,
                        version_label TEXT NOT NULL
                     );
                     CREATE TABLE exam_syllabus_sources (
                        id INTEGER PRIMARY KEY,
                        exam_id INTEGER NOT NULL,
                        content_version_id INTEGER NOT NULL,
                        source_reference TEXT NOT NULL DEFAULT '',
                        url TEXT
                     );
                     CREATE TABLE exam_syllabus_nodes (
                        id INTEGER PRIMARY KEY,
                        exam_id INTEGER NOT NULL,
                        source_id INTEGER NOT NULL,
                        code TEXT NOT NULL,
                        title TEXT NOT NULL,
                        is_archived INTEGER NOT NULL DEFAULT 0
                     );
                     CREATE TABLE exam_node_concepts (
                        exam_id INTEGER NOT NULL,
                        node_id INTEGER NOT NULL,
                        concept_id INTEGER NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
                        weight REAL NOT NULL DEFAULT 1 CHECK (weight > 0 AND weight <= 100),
                        is_primary INTEGER NOT NULL DEFAULT 0 CHECK (is_primary IN (0,1)),
                        notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 1000),
                        created_at TEXT NOT NULL,
                        PRIMARY KEY (node_id, concept_id),
                        UNIQUE (exam_id, node_id, concept_id)
                     );
                     INSERT INTO concepts (id, slug) VALUES (1, 'kinematics');
                     INSERT INTO exams (id, code) VALUES (1, 'csir-net-physics');
                     INSERT INTO exam_content_versions (id, exam_id, version_label)
                        VALUES (1, 1, 'legacy-v1');
                     INSERT INTO exam_syllabus_sources (
                        id, exam_id, content_version_id, source_reference
                     ) VALUES (1, 1, 1, 'fixture');
                     INSERT INTO exam_syllabus_nodes (
                        id, exam_id, source_id, code, title, is_archived
                     ) VALUES (1, 1, 1, 'T1', 'Kinematics', 0);
                     INSERT INTO exam_node_concepts (
                        exam_id, node_id, concept_id, weight, is_primary, notes, created_at
                     ) VALUES (1, 1, 1, 1, 0, 'legacy mapping', '2026-11-04T10:00:00Z');",
                )
                .expect("legacy schema");
        }

        let copy_path = root.path().join("copied.sqlite3");
        fs::copy(&source_path, &copy_path).expect("legacy copy");
        let mut connection = Connection::open(&copy_path).expect("copied database");
        ensure_mapping_catalogue(&mut connection).expect("additive mapping migration");

        let mut columns_statement = connection
            .prepare("PRAGMA table_info(exam_node_concepts)")
            .expect("mapping columns");
        let columns: Vec<String> = columns_statement
            .query_map([], |row| row.get(1))
            .expect("mapping columns")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("mapping columns");
        drop(columns_statement);
        for column in [
            "status",
            "origin",
            "source_reference",
            "provenance",
            "content_version_id",
            "reviewed_at",
            "reviewed_by",
            "updated_at",
        ] {
            assert!(
                columns.iter().any(|value| value == column),
                "missing {column}"
            );
        }

        let (count, status, origin, version, notes): (i64, String, String, Option<i64>, String) =
            connection
                .query_row(
                    "SELECT COUNT(*), MIN(status), MIN(origin), MIN(content_version_id), MIN(notes)
                     FROM exam_node_concepts",
                    [],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                        ))
                    },
                )
                .expect("migrated mapping");
        assert_eq!(count, 1);
        assert_eq!(status, "proposed");
        assert_eq!(origin, "curated");
        assert_eq!(version, Some(1));
        assert_eq!(notes, "legacy mapping");
    }

    #[test]
    fn mapping_review_changes_status_and_approved_mappings_become_verified() {
        let (_root, state) = state();
        let concept = state
            .with_connection(|connection| Ok(concept_id(connection, "kinematics")))
            .expect("concept");
        let topic = state
            .with_connection(|connection| {
                let (_, content_version_id, topic) =
                    add_syllabus(connection, 1, "review-v1", "verified");
                connection
                    .execute(
                        "INSERT INTO exam_node_concepts (
                            exam_id, node_id, concept_id, weight, is_primary, notes, created_at,
                            status, origin, source_reference, provenance, content_version_id,
                            reviewed_at, reviewed_by, updated_at
                         ) VALUES (1, ?1, ?2, 1, 0, 'curated', '2026-11-04T10:00:00Z',
                                   'proposed', 'curated', 'test', 'curated proposal', ?3,
                                   NULL, '', '2026-11-04T10:00:00Z')",
                        params![topic, concept, content_version_id],
                    )
                    .map_err(|error| AppError::database("proposed mapping", error))?;
                Ok(topic)
            })
            .expect("proposed mapping");

        let detail = state
            .with_connection(|connection| {
                update_mapping(
                    connection,
                    V2ConceptMappingUpdateInput {
                        exam_id: 1,
                        node_id: topic,
                        concept_id: concept,
                        status: "approved".to_owned(),
                        notes: Some("Reviewed and approved locally.".to_owned()),
                    },
                    crate::date_only::parse_date_only("2026-11-04").unwrap(),
                    Utc::now(),
                )
            })
            .expect("approve mapping");
        let mapping = detail
            .mappings
            .iter()
            .find(|mapping| mapping.node_id == topic)
            .expect("mapping");
        assert_eq!(mapping.status, "approved");
        assert!(mapping.is_approved);
        assert_eq!(mapping.reviewed_by, "local reviewer");
        assert!(mapping.reviewed_at.is_some());

        let result = state
            .with_connection(|connection| {
                search_concepts(
                    connection,
                    V2ConceptSearchFilter {
                        exam_code: Some("csir-net-physics".to_owned()),
                        ..Default::default()
                    },
                )
            })
            .expect("concept search");
        let found = result
            .concepts
            .iter()
            .find(|candidate| candidate.slug == "kinematics")
            .expect("kinematics");
        assert_eq!(found.verified_exam_codes, vec!["csir-net-physics"]);
        assert_eq!(result.exam_coverage[0].verified_mapped_concept_count, 1);
    }

    #[test]
    fn shared_concept_mappings_are_independent_and_expose_verification() {
        let (_root, state) = state();
        let concept = state
            .with_connection(|connection| Ok(concept_id(connection, "kinematics")))
            .expect("concept");
        let (topic_one, topic_two) = state
            .with_connection(|connection| {
                let (_, _, topic_one) = add_syllabus(connection, 1, "v1", "verified");
                let (_, _, topic_two) = add_syllabus(connection, 2, "v1", "draft");
                connection
                    .execute(
                        "INSERT INTO exam_node_concepts (
                            exam_id, node_id, concept_id, weight, is_primary, created_at,
                            status, origin, source_reference, provenance, content_version_id,
                            reviewed_at, reviewed_by, updated_at
                         ) VALUES (1, ?1, ?2, 2.0, 1, '2026-11-04T10:00:00Z',
                                   'approved', 'curated', 'test', 'test mapping',
                                   (SELECT content_version_id FROM exam_syllabus_sources
                                    WHERE exam_id = 1 LIMIT 1),
                                   '2026-11-04T10:00:00Z', 'tester', '2026-11-04T10:00:00Z')",
                        params![topic_one, concept],
                    )
                    .map_err(|error| AppError::database("exam one mapping", error))?;
                connection
                    .execute(
                        "INSERT INTO exam_node_concepts (
                            exam_id, node_id, concept_id, weight, is_primary, created_at,
                            status, origin, source_reference, provenance, content_version_id,
                            reviewed_at, reviewed_by, updated_at
                         ) VALUES (2, ?1, ?2, 1.5, 0, '2026-11-04T10:00:00Z',
                                   'proposed', 'curated', 'test', 'test mapping',
                                   (SELECT content_version_id FROM exam_syllabus_sources
                                    WHERE exam_id = 2 LIMIT 1),
                                   NULL, '', '2026-11-04T10:00:00Z')",
                        params![topic_two, concept],
                    )
                    .map_err(|error| AppError::database("exam two mapping", error))?;
                connection
                    .execute(
                        "INSERT INTO exam_node_progress (
                            exam_id, node_id, status, priority, confidence, created_at, updated_at
                         ) VALUES (1, ?1, 'revised', 'high', 4,
                                   '2026-11-04T10:00:00Z', '2026-11-04T10:00:00Z')",
                        [topic_one],
                    )
                    .map_err(|error| AppError::database("node progress", error))?;
                Ok((topic_one, topic_two))
            })
            .expect("mappings");
        assert!(topic_one != topic_two);

        let duplicate = state.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO exam_node_concepts (
                        exam_id, node_id, concept_id, weight, created_at
                     ) VALUES (1, ?1, ?2, 1, '2026-11-04T10:00:00Z')",
                    params![topic_one, concept],
                )
                .map_err(|error| AppError::database("duplicate mapping", error))
        });
        assert!(
            duplicate.is_err(),
            "duplicate concept mapping must be rejected"
        );

        let exam_one = state
            .with_connection(|connection| {
                search_concepts(
                    connection,
                    V2ConceptSearchFilter {
                        exam_code: Some("csir-net-physics".to_owned()),
                        ..Default::default()
                    },
                )
            })
            .expect("exam one search");
        assert_eq!(exam_one.total, 1);
        assert_eq!(
            exam_one.concepts[0].exam_codes,
            vec!["csir-net-physics", "gate-physics"]
        );
        assert_eq!(
            exam_one.concepts[0].verified_exam_codes,
            vec!["csir-net-physics"]
        );
        assert_eq!(exam_one.exam_coverage[0].status, "verified");
        assert_eq!(exam_one.exam_coverage[0].mapped_concept_count, 1);
        assert_eq!(exam_one.exam_coverage[0].verified_mapped_concept_count, 1);
        assert_eq!(exam_one.exam_coverage[0].revised_node_count, 1);

        let exam_two = state
            .with_connection(|connection| {
                search_concepts(
                    connection,
                    V2ConceptSearchFilter {
                        exam_code: Some("gate-physics".to_owned()),
                        ..Default::default()
                    },
                )
            })
            .expect("exam two search");
        assert_eq!(exam_two.total, 1);
        assert_eq!(
            exam_two.concepts[0].verified_exam_codes,
            vec!["csir-net-physics"]
        );
        assert_eq!(exam_two.exam_coverage[1].status, "unverified");
        assert_eq!(exam_two.exam_coverage[1].mapped_concept_count, 1);
        assert_eq!(exam_two.exam_coverage[1].verified_mapped_concept_count, 0);

        let detail = state
            .with_connection(|connection| {
                get_concept_detail(
                    connection,
                    V2ConceptDetailInput {
                        slug: "kinematics".to_owned(),
                        evidence_limit: None,
                        history_limit: None,
                    },
                )
            })
            .expect("detail");
        assert_eq!(detail.mappings.len(), 2);
        let verified = detail
            .mappings
            .iter()
            .find(|mapping| mapping.exam_id == 1)
            .expect("exam one mapping");
        assert!(verified.is_verified);
        assert_eq!(
            verified.node_path,
            vec!["Mechanics Unit", "Kinematics Topic"]
        );
        assert_eq!(verified.progress_status, "revised");
        assert!((verified.weight - 2.0).abs() < 1e-9);
        let unverified = detail
            .mappings
            .iter()
            .find(|mapping| mapping.exam_id == 2)
            .expect("exam two mapping");
        assert!(!unverified.is_verified);
        assert_eq!(unverified.content_review_status, "draft");
    }

    #[test]
    fn invalid_exam_filter_and_archived_mapping_are_handled() {
        let (_root, state) = state();
        let concept = state
            .with_connection(|connection| Ok(concept_id(connection, "kinematics")))
            .expect("concept");
        let topic = state
            .with_connection(|connection| {
                let (_, _, topic) = add_syllabus(connection, 1, "v1", "verified");
                connection
                    .execute(
                        "INSERT INTO exam_node_concepts (
                            exam_id, node_id, concept_id, weight, created_at
                         ) VALUES (1, ?1, ?2, 1, '2026-11-04T10:00:00Z')",
                        params![topic, concept],
                    )
                    .map_err(|error| AppError::database("mapping", error))?;
                Ok(topic)
            })
            .expect("mapping");

        let invalid = state
            .with_connection(|connection| {
                search_concepts(
                    connection,
                    V2ConceptSearchFilter {
                        exam_code: Some("not-an-exam".to_owned()),
                        ..Default::default()
                    },
                )
            })
            .expect_err("invalid exam filter");
        assert_eq!(invalid.field, Some("examCode"));

        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE exam_syllabus_nodes
                         SET is_archived = 1, archived_at = '2026-11-04T11:00:00Z'
                         WHERE exam_id = 1 AND id = ?1",
                        [topic],
                    )
                    .map_err(|error| AppError::database("archive node", error))?;
                Ok(())
            })
            .expect("archive node");
        let detail = state
            .with_connection(|connection| {
                get_concept_detail(
                    connection,
                    V2ConceptDetailInput {
                        slug: "kinematics".to_owned(),
                        evidence_limit: None,
                        history_limit: None,
                    },
                )
            })
            .expect("detail");
        assert!(detail.mappings[0].node_is_archived);
        assert!(!detail.mappings[0].is_verified);
    }

    #[test]
    fn concept_detail_distinguishes_missing_evidence_and_exposes_history() {
        let (_root, state) = state();
        let before = state
            .with_connection(|connection| {
                get_concept_detail(
                    connection,
                    V2ConceptDetailInput {
                        slug: "kinematics".to_owned(),
                        evidence_limit: None,
                        history_limit: None,
                    },
                )
            })
            .expect("detail before");
        assert!(before.concept.mastery.is_none());
        assert!(before.evidence.is_empty());
        assert!(before.mastery_history.is_empty());

        let now: DateTime<Utc> = DateTime::parse_from_rfc3339("2026-11-04T10:00:00Z")
            .expect("timestamp")
            .with_timezone(&Utc);
        state
            .with_connection(|connection| {
                mastery::update_manual_mastery(
                    connection,
                    V2UpdateConceptMasteryInput {
                        concept_id: before.concept.id,
                        target_score: 65.0,
                        local_date: "2026-11-04".to_owned(),
                        notes: "Manual check".to_owned(),
                    },
                    now,
                )
                .map(|_| ())
            })
            .expect("manual mastery");

        let after = state
            .with_connection(|connection| {
                get_concept_detail(
                    connection,
                    V2ConceptDetailInput {
                        slug: "kinematics".to_owned(),
                        evidence_limit: None,
                        history_limit: Some(5),
                    },
                )
            })
            .expect("detail after");
        let mastery = after.concept.mastery.expect("mastery");
        assert_eq!(mastery.evidence_count, 1);
        assert!((mastery.mastery_score - 65.0).abs() < 1e-9);
        assert_eq!(after.evidence.len(), 1);
        assert_eq!(after.evidence[0].source_kind, "manual");
        assert_eq!(after.mastery_history.len(), 1);
        assert_eq!(after.mastery_history[0].evidence_count, 1);
    }

    #[test]
    fn missing_concept_detail_has_a_structured_error() {
        let (_root, state) = state();
        let error = state
            .with_connection(|connection| {
                get_concept_detail(
                    connection,
                    V2ConceptDetailInput {
                        slug: "not-a-concept".to_owned(),
                        evidence_limit: None,
                        history_limit: None,
                    },
                )
            })
            .expect_err("missing concept");
        assert_eq!(error.code, "concept_not_found");
    }
}
