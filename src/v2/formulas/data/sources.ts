import type { FormulaSource } from '../types';

export const TEXTBOOKS = {
  arfken: 'Arfken, Weber & Harris, Mathematical Methods for Physicists, 7th ed.',
  ashcroftMermin: 'Ashcroft & Mermin, Solid State Physics, 1st ed.',
  bevington:
    'Bevington & Robinson, Data Reduction and Error Analysis for the Physical Sciences, 3rd ed.',
  bornWolf: 'Born & Wolf, Principles of Optics, 7th ed.',
  bransdenJoachain: 'Bransden & Joachain, Physics of Atoms and Molecules, 2nd ed.',
  callen: 'Callen, Thermodynamics and an Introduction to Thermostatistics, 2nd ed.',
  crawford: 'Crawford, Waves, Berkeley Physics Course Vol. 3, 1st ed.',
  demtroder: 'Demtröder, Molecular Physics, 1st ed.',
  foot: 'Foot, Atomic Physics, 1st ed.',
  french: 'French, Vibrations and Waves, 1st ed.',
  goldstein: 'Goldstein, Poole & Safko, Classical Mechanics, 3rd ed.',
  griffithsEM: 'Griffiths, Introduction to Electrodynamics, 4th ed.',
  griffithsParticles: 'Griffiths, Introduction to Elementary Particles, 2nd ed.',
  griffithsQM: 'Griffiths, Introduction to Quantum Mechanics, 3rd ed.',
  hecht: 'Hecht, Optics, 5th ed.',
  horowitzHill: 'Horowitz & Hill, The Art of Electronics, 3rd ed.',
  jackson: 'Jackson, Classical Electrodynamics, 3rd ed.',
  kittel: 'Kittel, Introduction to Solid State Physics, 8th ed.',
  kittelKroemer: 'Kittel & Kroemer, Thermal Physics, 2nd ed.',
  knoll: 'Knoll, Radiation Detection and Measurement, 4th ed.',
  krane: 'Krane, Introductory Nuclear Physics, 1st ed.',
  landauMechanics: 'Landau & Lifshitz, Mechanics, 3rd ed.',
  landauStat: 'Landau & Lifshitz, Statistical Physics, Part 1, 3rd ed.',
  pathria: 'Pathria & Beale, Statistical Mechanics, 4th ed.',
  rindler: 'Rindler, Introduction to Special Relativity, 2nd ed.',
  riley: 'Riley, Hobson & Bence, Mathematical Methods for Physics and Engineering, 3rd ed.',
  sakurai: 'Sakurai & Napolitano, Modern Quantum Mechanics, 3rd ed.',
  sedraSmith: 'Sedra & Smith, Microelectronic Circuits, 8th ed.',
  taylorClassical: 'Taylor, Classical Mechanics, 1st ed.',
  taylorError: 'Taylor, An Introduction to Error Analysis, 2nd ed.',
  taylorWheeler: 'Taylor & Wheeler, Spacetime Physics, 2nd ed.',
  zangwill: 'Zangwill, Modern Electrodynamics',
} as const;

export const REFERENCES = {
  codata: 'CODATA 2022 recommended values of the fundamental physical constants',
  dlmf: 'NIST Digital Library of Mathematical Functions',
  pdg: 'Particle Data Group, Review of Particle Physics',
  siBrochure: 'BIPM, The International System of Units (SI), 9th ed.',
} as const;

/**
 * Chapter-level checks: the cited chapter/section or table-of-contents entry
 * was inspected during the Stage 1 scientific audit. This does not establish
 * that the exact formula appears in that chapter. Locators outside this set
 * remain explicitly unverified.
 */
const CHAPTER_CHECKED_LOCATORS = new Set([
  `${TEXTBOOKS.arfken}|Ch. 3`,
  `${TEXTBOOKS.arfken}|Ch. 10`,
  `${TEXTBOOKS.arfken}|Ch. 11`,
  `${TEXTBOOKS.arfken}|Ch. 20`,
  `${TEXTBOOKS.taylorClassical}|Ch. 1`,
  `${TEXTBOOKS.taylorClassical}|Ch. 4`,
  `${TEXTBOOKS.taylorClassical}|Ch. 8`,
  `${TEXTBOOKS.jackson}|Ch. 1`,
  `${TEXTBOOKS.jackson}|Ch. 6`,
  `${TEXTBOOKS.jackson}|Ch. 12`,
  `${TEXTBOOKS.kittel}|Ch. 2`,
  `${TEXTBOOKS.kittel}|Ch. 6`,
  `${TEXTBOOKS.kittelKroemer}|Ch. 2`,
  `${TEXTBOOKS.kittelKroemer}|Ch. 4`,
  `${TEXTBOOKS.pathria}|Ch. 7`,
  `${TEXTBOOKS.pathria}|Ch. 8`,
  `${TEXTBOOKS.sedraSmith}|Ch. 2`,
  `${TEXTBOOKS.sedraSmith}|Ch. 4`,
  `${TEXTBOOKS.horowitzHill}|Ch. 1`,
  `${TEXTBOOKS.horowitzHill}|Ch. 4`,
  `${TEXTBOOKS.riley}|Ch. 11`,
  `${TEXTBOOKS.riley}|Ch. 23`,
  `${TEXTBOOKS.riley}|Ch. 24`,
  `${TEXTBOOKS.bevington}|Ch. 3`,
  `${TEXTBOOKS.bevington}|Ch. 4`,
  `${TEXTBOOKS.bevington}|Ch. 7, §7.4`,
  `${TEXTBOOKS.ashcroftMermin}|Ch. 1`,
  `${TEXTBOOKS.ashcroftMermin}|Ch. 2`,
  `${TEXTBOOKS.bransdenJoachain}|Ch. 1`,
  `${TEXTBOOKS.bransdenJoachain}|Ch. 10, §10.1`,
  `${TEXTBOOKS.callen}|Ch. 16`,
  `${TEXTBOOKS.french}|Ch. 3`,
  `${TEXTBOOKS.french}|Ch. 4`,
]);

function locatorCheck(citation: string, locator: string): 'chapter-checked' | 'unverified' {
  return CHAPTER_CHECKED_LOCATORS.has(`${citation}|${locator}`) ? 'chapter-checked' : 'unverified';
}

export function textbook(citation: string, locator?: string): FormulaSource {
  const source: FormulaSource = { kind: 'textbook', citation };
  if (locator) {
    source.locator = locator;
    source.locatorCheck = locatorCheck(citation, locator);
  }
  return source;
}

export function reference(citation: string, locator?: string, url?: string): FormulaSource {
  const source: FormulaSource = { kind: 'reference', citation };
  if (locator) {
    source.locator = locator;
    source.locatorCheck = locatorCheck(citation, locator);
  }
  if (url) {
    source.url = url;
  }
  return source;
}
