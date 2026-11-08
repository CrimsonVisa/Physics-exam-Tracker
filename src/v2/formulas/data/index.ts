import type { FormulaEntry, FormulaLibrary } from '../types';
import { ATOMIC_MOLECULAR_FORMULAS } from './atomic-molecular';
import { CLASSICAL_MECHANICS_FORMULAS } from './classical-mechanics';
import { CONDENSED_MATTER_FORMULAS } from './condensed-matter';
import { ELECTROMAGNETISM_FORMULAS } from './electromagnetism';
import { ELECTRONICS_FORMULAS } from './electronics';
import { EXPERIMENTAL_METHODS_FORMULAS } from './experimental-methods';
import { FOUNDATIONS_FORMULAS } from './foundations';
import { MATHEMATICAL_PHYSICS_FORMULAS } from './mathematical-physics';
import { NUCLEAR_PARTICLE_FORMULAS } from './nuclear-particle';
import { OPTICS_FORMULAS } from './optics';
import { QUANTUM_MECHANICS_FORMULAS } from './quantum-mechanics';
import { SPECIAL_RELATIVITY_FORMULAS } from './special-relativity';
import { THERMODYNAMICS_STATISTICAL_FORMULAS } from './thermodynamics-statistical';
import { WAVES_OSCILLATIONS_FORMULAS } from './waves-oscillations';

export const FORMULA_LIBRARY_VERSION = '0.2.0';
export const FORMULA_LIBRARY_LAST_REVIEWED = '2026-11-08';

export const FORMULA_ENTRIES: FormulaEntry[] = [
  ...MATHEMATICAL_PHYSICS_FORMULAS,
  ...CLASSICAL_MECHANICS_FORMULAS,
  ...ELECTROMAGNETISM_FORMULAS,
  ...QUANTUM_MECHANICS_FORMULAS,
  ...THERMODYNAMICS_STATISTICAL_FORMULAS,
  ...ATOMIC_MOLECULAR_FORMULAS,
  ...NUCLEAR_PARTICLE_FORMULAS,
  ...CONDENSED_MATTER_FORMULAS,
  ...ELECTRONICS_FORMULAS,
  ...EXPERIMENTAL_METHODS_FORMULAS,
  ...OPTICS_FORMULAS,
  ...WAVES_OSCILLATIONS_FORMULAS,
  ...SPECIAL_RELATIVITY_FORMULAS,
  ...FOUNDATIONS_FORMULAS,
];

export const FORMULA_LIBRARY: FormulaLibrary = {
  version: FORMULA_LIBRARY_VERSION,
  lastReviewed: FORMULA_LIBRARY_LAST_REVIEWED,
  entries: FORMULA_ENTRIES,
};
