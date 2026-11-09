import type { TermEntry, TermLibrary } from '../types';
import { ATOMIC_MOLECULAR_TERMS } from './atomic-molecular';
import { CLASSICAL_MECHANICS_TERMS } from './classical-mechanics';
import { CONDENSED_MATTER_TERMS } from './condensed-matter';
import { ELECTROMAGNETISM_TERMS } from './electromagnetism';
import { ELECTRONICS_TERMS } from './electronics';
import { EXPERIMENTAL_METHODS_TERMS } from './experimental-methods';
import { FOUNDATIONS_TERMS } from './foundations';
import { MATHEMATICAL_PHYSICS_TERMS } from './mathematical-physics';
import { NUCLEAR_PARTICLE_TERMS } from './nuclear-particle';
import { OPTICS_TERMS } from './optics';
import { QUANTUM_MECHANICS_TERMS } from './quantum-mechanics';
import { SPECIAL_RELATIVITY_TERMS } from './special-relativity';
import { THERMODYNAMICS_STATISTICAL_TERMS } from './thermodynamics-statistical';
import { UNITS_MEASUREMENT_TERMS } from './units-measurement';
import { WAVES_OSCILLATIONS_TERMS } from './waves-oscillations';
import { applyTermSourceChecks } from './shared';

export const TERM_LIBRARY_VERSION = '0.2.0';
// Compilation/review date for the library, not a per-source verification date.
export const TERM_LIBRARY_LAST_REVIEWED = '2026-11-09';

export const TERM_ENTRIES: TermEntry[] = applyTermSourceChecks([
  ...UNITS_MEASUREMENT_TERMS,
  ...FOUNDATIONS_TERMS,
  ...MATHEMATICAL_PHYSICS_TERMS,
  ...CLASSICAL_MECHANICS_TERMS,
  ...ELECTROMAGNETISM_TERMS,
  ...QUANTUM_MECHANICS_TERMS,
  ...THERMODYNAMICS_STATISTICAL_TERMS,
  ...ATOMIC_MOLECULAR_TERMS,
  ...NUCLEAR_PARTICLE_TERMS,
  ...CONDENSED_MATTER_TERMS,
  ...ELECTRONICS_TERMS,
  ...EXPERIMENTAL_METHODS_TERMS,
  ...OPTICS_TERMS,
  ...WAVES_OSCILLATIONS_TERMS,
  ...SPECIAL_RELATIVITY_TERMS,
]);

export const TERM_LIBRARY: TermLibrary = {
  version: TERM_LIBRARY_VERSION,
  lastReviewed: TERM_LIBRARY_LAST_REVIEWED,
  sourceVerificationNote:
    'The library date is a compilation/review date and is not a per-source verification date. Source verification status is recorded per source; an unverified locator is not a claim that every source was checked on this date.',
  entries: TERM_ENTRIES,
};
