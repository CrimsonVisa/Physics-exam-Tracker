import {
  REFERENCES,
  TEXTBOOKS,
  reference as formulaReference,
  textbook as formulaTextbook,
} from '../../formulas/data/sources';
import type { FormulaSource } from '../../formulas/types';
import type { TermEntry } from '../types';

export { CSIR, GATE, JEST, TIFR, link } from '../../formulas/data/syllabus';
export { REFERENCES, TEXTBOOKS };

export function entrySourceKey(entryId: string, citation: string, locator: string): string {
  return `${entryId}|${citation}|${locator}`;
}

/**
 * Entry-specific chapter checks. Every key includes the term entry ID, so a
 * source can never become chapter-checked merely because another term uses the
 * same textbook chapter. These are chapter/TOC-level checks only, not
 * definition-level verification.
 */
export const TERM_CHAPTER_CHECKED_SOURCES = new Set([
  entrySourceKey('atomic-molecular-fine-structure', TEXTBOOKS.foot, 'Ch. 2'),
  entrySourceKey('atomic-molecular-hyperfine-structure', TEXTBOOKS.foot, 'Ch. 6'),
  entrySourceKey('atomic-molecular-isotope-shift', TEXTBOOKS.foot, 'Ch. 6'),
  entrySourceKey('atomic-molecular-rotational-constant', TEXTBOOKS.bransdenJoachain, 'Ch. 11'),
  entrySourceKey('atomic-molecular-selection-rule', TEXTBOOKS.foot, 'Ch. 7'),
  entrySourceKey('atomic-molecular-term-symbol', TEXTBOOKS.foot, 'Ch. 5'),
  entrySourceKey('atomic-molecular-vibrational-frequency', TEXTBOOKS.bransdenJoachain, 'Ch. 11'),
  entrySourceKey('classical-mechanics-angular-momentum', TEXTBOOKS.taylorClassical, 'Ch. 3'),
  entrySourceKey('classical-mechanics-inertial-frame', TEXTBOOKS.taylorClassical, 'Ch. 1'),
  entrySourceKey('classical-mechanics-kinetic-energy', TEXTBOOKS.taylorClassical, 'Ch. 4'),
  entrySourceKey('classical-mechanics-mass', TEXTBOOKS.taylorClassical, 'Ch. 1'),
  entrySourceKey('classical-mechanics-momentum', TEXTBOOKS.taylorClassical, 'Ch. 1'),
  entrySourceKey('classical-mechanics-reduced-mass', TEXTBOOKS.taylorClassical, 'Ch. 8'),
  entrySourceKey('classical-mechanics-torque', TEXTBOOKS.taylorClassical, 'Ch. 3'),
  entrySourceKey('classical-mechanics-work', TEXTBOOKS.taylorClassical, 'Ch. 4'),
  entrySourceKey('condensed-matter-brillouin-zone', TEXTBOOKS.ashcroftMermin, 'Ch. 5'),
  entrySourceKey('condensed-matter-brillouin-zone', TEXTBOOKS.kittel, 'Ch. 2'),
  entrySourceKey('condensed-matter-density-of-states', TEXTBOOKS.ashcroftMermin, 'Ch. 2'),
  entrySourceKey('condensed-matter-density-of-states', TEXTBOOKS.kittel, 'Ch. 6'),
  entrySourceKey('condensed-matter-effective-mass', TEXTBOOKS.ashcroftMermin, 'Appendix E'),
  entrySourceKey('condensed-matter-effective-mass', TEXTBOOKS.kittel, 'Ch. 7'),
  entrySourceKey('condensed-matter-fermi-energy', TEXTBOOKS.ashcroftMermin, 'Ch. 2'),
  entrySourceKey('condensed-matter-fermi-energy', TEXTBOOKS.kittel, 'Ch. 6'),
  entrySourceKey('condensed-matter-lattice', TEXTBOOKS.ashcroftMermin, 'Ch. 4'),
  entrySourceKey('condensed-matter-lattice', TEXTBOOKS.kittel, 'Ch. 1'),
  entrySourceKey('condensed-matter-phonon', TEXTBOOKS.ashcroftMermin, 'Ch. 22'),
  entrySourceKey('condensed-matter-reciprocal-lattice', TEXTBOOKS.ashcroftMermin, 'Ch. 5'),
  entrySourceKey('condensed-matter-reciprocal-lattice', TEXTBOOKS.kittel, 'Ch. 2'),
  entrySourceKey('foundations-field', TEXTBOOKS.arfken, 'Ch. 3'),
  entrySourceKey('foundations-scalar', TEXTBOOKS.arfken, 'Ch. 3'),
  entrySourceKey('foundations-scalar', TEXTBOOKS.riley, 'Ch. 7'),
  entrySourceKey('foundations-tensor', TEXTBOOKS.arfken, 'Ch. 4'),
  entrySourceKey('foundations-tensor', TEXTBOOKS.riley, 'Ch. 26'),
  entrySourceKey('foundations-vector', TEXTBOOKS.arfken, 'Ch. 3'),
  entrySourceKey('foundations-vector', TEXTBOOKS.riley, 'Ch. 7'),
  entrySourceKey('mathematical-physics-curl', TEXTBOOKS.arfken, 'Ch. 3'),
  entrySourceKey('mathematical-physics-curl', TEXTBOOKS.riley, 'Ch. 10'),
  entrySourceKey('mathematical-physics-divergence', TEXTBOOKS.arfken, 'Ch. 3'),
  entrySourceKey('mathematical-physics-divergence', TEXTBOOKS.riley, 'Ch. 10'),
  entrySourceKey('mathematical-physics-eigenvalue', TEXTBOOKS.arfken, 'Ch. 6'),
  entrySourceKey('mathematical-physics-fourier-transform', TEXTBOOKS.arfken, 'Ch. 20'),
  entrySourceKey('mathematical-physics-gradient', TEXTBOOKS.arfken, 'Ch. 3'),
  entrySourceKey('mathematical-physics-gradient', TEXTBOOKS.riley, 'Ch. 10'),
  entrySourceKey('mathematical-physics-laplacian', TEXTBOOKS.arfken, 'Ch. 3'),
  entrySourceKey('mathematical-physics-laplacian', TEXTBOOKS.riley, 'Ch. 10'),
  entrySourceKey('mathematical-physics-orthogonal-functions', TEXTBOOKS.arfken, 'Ch. 8'),
  entrySourceKey('nuclear-particle-binding-energy', TEXTBOOKS.krane, 'Ch. 3'),
  entrySourceKey('nuclear-particle-cross-section', TEXTBOOKS.krane, 'Ch. 11'),
  entrySourceKey('nuclear-particle-decay-constant', TEXTBOOKS.krane, 'Ch. 6'),
  entrySourceKey('nuclear-particle-half-life', TEXTBOOKS.krane, 'Ch. 6'),
  entrySourceKey('nuclear-particle-isospin', TEXTBOOKS.krane, 'Ch. 11, §11.3'),
  entrySourceKey('nuclear-particle-parity', TEXTBOOKS.krane, 'Ch. 3'),
  entrySourceKey('nuclear-particle-q-value', TEXTBOOKS.krane, 'Ch. 11'),
  entrySourceKey('thermodynamics-chemical-potential', TEXTBOOKS.callen, 'Ch. 2'),
  entrySourceKey('thermodynamics-chemical-potential', TEXTBOOKS.kittelKroemer, 'Ch. 5'),
  entrySourceKey('thermodynamics-ensemble', TEXTBOOKS.callen, 'Ch. 16'),
  entrySourceKey('thermodynamics-enthalpy', TEXTBOOKS.callen, 'Ch. 5'),
  entrySourceKey('thermodynamics-enthalpy', TEXTBOOKS.kittelKroemer, 'Ch. 8'),
  entrySourceKey('thermodynamics-entropy', TEXTBOOKS.kittelKroemer, 'Ch. 2'),
  entrySourceKey('thermodynamics-gibbs-free-energy', TEXTBOOKS.callen, 'Ch. 5'),
  entrySourceKey('thermodynamics-gibbs-free-energy', TEXTBOOKS.kittelKroemer, 'Ch. 9'),
  entrySourceKey('thermodynamics-helmholtz-free-energy', TEXTBOOKS.callen, 'Ch. 16'),
  entrySourceKey('thermodynamics-internal-energy', TEXTBOOKS.kittelKroemer, 'Ch. 2'),
  entrySourceKey('thermodynamics-partition-function', TEXTBOOKS.callen, 'Ch. 16'),
]);

function applyUnverifiedTermCheck(source: FormulaSource): FormulaSource {
  if (!source.locator) {
    return source;
  }
  return { ...source, locatorCheck: 'unverified' };
}

export function applyTermSourceChecks(entries: TermEntry[]): TermEntry[] {
  return entries.map((entry) => ({
    ...entry,
    sources: entry.sources.map((source) => {
      if (!source.locator) {
        return source;
      }
      return {
        ...source,
        locatorCheck: TERM_CHAPTER_CHECKED_SOURCES.has(
          entrySourceKey(entry.id, source.citation, source.locator),
        )
          ? 'chapter-checked'
          : 'unverified',
      };
    }),
  }));
}

export function textbook(citation: string, locator?: string): FormulaSource {
  return applyUnverifiedTermCheck(formulaTextbook(citation, locator));
}

export function reference(citation: string, locator?: string, url?: string): FormulaSource {
  return applyUnverifiedTermCheck(formulaReference(citation, locator, url));
}
