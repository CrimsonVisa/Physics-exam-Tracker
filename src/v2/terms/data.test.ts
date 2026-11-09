import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { FORMULA_LIBRARY } from '../formulas/data';
import { GATE, JEST, TIFR } from '../formulas/data/syllabus';
import { REFERENCES, TEXTBOOKS } from '../formulas/data/sources';
import { buildSyllabusIndex, type SyllabusSeedExam } from '../formulas/validation';
import { TERM_LIBRARY } from './data';
import { TERM_CHAPTER_CHECKED_SOURCES, entrySourceKey } from './data/shared';
import { EXPLICIT_TERM_LINKS, explicitLinkKey } from './explicitAllowlist';
import { FORMULA_REF_TITLES } from './formulaRefs';
import { TERM_SUBJECTS } from './taxonomy';
import type { TermSubjectId } from './types';
import { validateTermLibrary } from './validation';

const syllabusSeed = JSON.parse(
  readFileSync(join(process.cwd(), 'src-tauri', 'src', 'v2', 'syllabus_seed.json'), 'utf8'),
) as SyllabusSeedExam[];

const syllabusIndex = buildSyllabusIndex(syllabusSeed);
const formulaIds = new Set(FORMULA_LIBRARY.entries.map((entry) => entry.id));

function getEntry(id: string) {
  const entry = TERM_LIBRARY.entries.find((candidate) => candidate.id === id);
  expect(entry, id).toBeDefined();
  return entry!;
}

function getSource(entryId: string, citation: string) {
  const source = getEntry(entryId).sources.find((candidate) => candidate.citation === citation);
  expect(source, `${entryId}: ${citation}`).toBeDefined();
  return source!;
}

describe('term library data', () => {
  it('passes the complete structural, source and syllabus validation suite', () => {
    const issues = validateTermLibrary(TERM_LIBRARY.entries, syllabusIndex, formulaIds);
    expect(issues, issues.map((issue) => `${issue.path}: ${issue.message}`).join('\n')).toEqual([]);
  });

  it('contains the corrected 123-entry collection and version', () => {
    expect(TERM_LIBRARY.entries.length).toBe(123);
    expect(TERM_LIBRARY.version).toBe('0.2.0');
    expect(TERM_LIBRARY.lastReviewed).toBe('2026-11-09');
    expect(TERM_LIBRARY.sourceVerificationNote).toMatch(/not a per-source verification date/i);
  });

  it('matches the approved category distribution', () => {
    const expected: Record<TermSubjectId, number> = {
      'units-measurement': 23,
      foundations: 7,
      'mathematical-physics': 7,
      'classical-mechanics': 8,
      electromagnetism: 10,
      'quantum-mechanics': 8,
      'thermodynamics-statistical': 8,
      'atomic-molecular': 7,
      'nuclear-particle': 7,
      'condensed-matter': 7,
      electronics: 6,
      'experimental-methods': 7,
      optics: 6,
      'waves-oscillations': 7,
      'special-relativity': 5,
    };
    const actual = Object.fromEntries(
      TERM_SUBJECTS.map((subject) => [
        subject.id,
        TERM_LIBRARY.entries.filter((entry) => entry.subject === subject.id).length,
      ]),
    );
    expect(actual).toEqual(expected);
    expect(Object.values(actual).reduce((total, count) => total + count, 0)).toBe(123);
  });

  it('keeps formula cross-reference titles exactly in sync', () => {
    const formulaTitles = Object.fromEntries(
      FORMULA_LIBRARY.entries.map((entry) => [entry.id, entry.title]),
    );
    expect(FORMULA_REF_TITLES).toEqual(formulaTitles);
  });

  it('has at least one source for every entry and valid source metadata', () => {
    for (const entry of TERM_LIBRARY.entries) {
      expect(entry.sources.length, entry.id).toBeGreaterThan(0);
      for (const source of entry.sources) {
        expect(source.citation.trim(), entry.id).not.toBe('');
        expect(['textbook', 'reference'], entry.id).toContain(source.kind);
        if (source.locator) {
          expect(['chapter-checked', 'unverified'], entry.id).toContain(source.locatorCheck);
        } else {
          expect(source.locatorCheck, entry.id).toBeUndefined();
        }
      }
    }
  });

  it('keeps every entry ID, term name and alias unique after normalization', () => {
    const ids = new Set<string>();
    const names = new Set<string>();
    for (const entry of TERM_LIBRARY.entries) {
      expect(ids.has(entry.id), entry.id).toBe(false);
      ids.add(entry.id);
      for (const name of [entry.term, ...(entry.aliases ?? [])]) {
        const normalized = name.normalize('NFKD').replace(/\p{M}/gu, '').toLowerCase().trim();
        expect(names.has(normalized), `${entry.id}: ${name}`).toBe(false);
        names.add(normalized);
      }
    }
  });

  it('does not claim unsupported JEST coverage for restricted subjects', () => {
    const restricted = new Set<TermSubjectId>([
      'atomic-molecular',
      'nuclear-particle',
      'condensed-matter',
    ]);
    for (const entry of TERM_LIBRARY.entries) {
      if (!restricted.has(entry.subject)) {
        continue;
      }
      expect(
        entry.syllabusLinks.some((link) => link.examCode === 'jest-physics'),
        entry.id,
      ).toBe(false);
    }
  });

  it('keeps TIFR links broad-area only and never explicit', () => {
    const tifrNodes = syllabusIndex.get('tifr-gs-physics');
    expect(tifrNodes).toBeDefined();
    for (const entry of TERM_LIBRARY.entries) {
      for (const link of entry.syllabusLinks) {
        if (link.examCode !== 'tifr-gs-physics') {
          continue;
        }
        expect(link.relevance, entry.id).not.toBe('explicit');
        for (const node of link.nodes) {
          const indexed = tifrNodes?.get(node.code);
          expect(indexed?.kind, entry.id).toBe('topic');
          expect(indexed?.title.startsWith('Broad area:'), entry.id).toBe(true);
        }
      }
    }
  });

  it('has no duplicate syllabus-node mappings within an entry', () => {
    for (const entry of TERM_LIBRARY.entries) {
      const keys = entry.syllabusLinks.flatMap((link) =>
        link.nodes.map((node) => `${link.examCode}:${node.code}`),
      );
      expect(new Set(keys).size, entry.id).toBe(keys.length);
    }
  });

  it('rejects explicit TIFR relevance at runtime', () => {
    const entry = structuredClone(getEntry('units-metre'));
    const tifrLink = entry.syllabusLinks.find((link) => link.examCode === 'tifr-gs-physics');
    expect(tifrLink).toBeDefined();
    tifrLink!.relevance = 'explicit';
    const issues = validateTermLibrary([entry], syllabusIndex, formulaIds);
    expect(
      issues.some((issue) => /TIFR links must not be explicit at term level/i.test(issue.message)),
      issues.map((issue) => issue.message).join('\n'),
    ).toBe(true);
  });

  it('rejects unsupported JEST links for restricted subjects', () => {
    const entry = structuredClone(getEntry('nuclear-particle-binding-energy'));
    entry.syllabusLinks = [
      {
        examCode: 'jest-physics',
        relevance: 'implicit',
        nodes: [JEST.math],
      },
    ];
    const issues = validateTermLibrary([entry], syllabusIndex, formulaIds);
    expect(
      issues.some((issue) => /JEST relevance is not allowed/i.test(issue.message)),
      issues.map((issue) => issue.message).join('\n'),
    ).toBe(true);
  });

  it('rejects missing or invalid source locator-check metadata', () => {
    const missing = structuredClone(getEntry('units-metre'));
    delete missing.sources[0].locatorCheck;
    const missingIssues = validateTermLibrary([missing], syllabusIndex, formulaIds);
    expect(
      missingIssues.some((issue) => /locatorCheck is required/i.test(issue.message)),
      missingIssues.map((issue) => issue.message).join('\n'),
    ).toBe(true);

    const invalid = structuredClone(getEntry('units-metre'));
    invalid.sources[0].locatorCheck = 'definition-verified' as never;
    const invalidIssues = validateTermLibrary([invalid], syllabusIndex, formulaIds);
    expect(
      invalidIssues.some((issue) => /locatorCheck must be/i.test(issue.message)),
      invalidIssues.map((issue) => issue.message).join('\n'),
    ).toBe(true);
  });

  it('rejects self-referential and duplicate related links', () => {
    const self = structuredClone(getEntry('units-metre'));
    self.relatedTermIds = [self.id];
    const selfIssues = validateTermLibrary([self], syllabusIndex, formulaIds);
    expect(selfIssues.some((issue) => /cannot relate to itself/i.test(issue.message))).toBe(true);

    const duplicate = structuredClone(getEntry('units-metre'));
    duplicate.relatedTermIds = ['units-second', 'units-second'];
    const duplicateIssues = validateTermLibrary([duplicate], syllabusIndex, formulaIds);
    expect(duplicateIssues.some((issue) => /duplicate related term/i.test(issue.message))).toBe(
      true,
    );
  });

  it('separates absolute and relative permittivity and permeability', () => {
    const permittivity = getEntry('electromagnetism-permittivity');
    expect(permittivity.aliases ?? []).not.toContain('relative permittivity');
    expect(permittivity.aliases ?? []).not.toContain('dielectric constant');
    expect(permittivity.notationVariants ?? []).not.toContain(String.raw`\varepsilon_r`);

    const relativePermittivity = getEntry('electromagnetism-relative-permittivity');
    expect(relativePermittivity.siUnit).toBe('1');
    expect(relativePermittivity.dimensions).toBe('1');
    expect(relativePermittivity.aliases).toContain('dielectric constant');

    const permeability = getEntry('electromagnetism-permeability');
    expect(permeability.notationVariants ?? []).not.toContain(String.raw`\mu_r`);

    const relativePermeability = getEntry('electromagnetism-relative-permeability');
    expect(relativePermeability.siUnit).toBe('1');
    expect(relativePermeability.dimensions).toBe('1');
  });

  it('distinguishes angular and ordinary wavenumber', () => {
    const angular = getEntry('waves-oscillations-wavenumber');
    expect(angular.term).toBe('angular wavenumber');
    expect(angular.aliases ?? []).not.toContain('wavenumber');
    expect(angular.siUnit).toBe('rad m^-1');

    const ordinary = getEntry('waves-oscillations-ordinary-wavenumber');
    expect(ordinary.term).toBe('ordinary wavenumber');
    expect(ordinary.siUnit).toBe('m^-1');
    expect(ordinary.dimensions).toBe('L^-1');
    expect(ordinary.aliases).toContain('wavenumber');
  });

  it('keeps wavefunction dimensions representation-specific without an SI-unit field', () => {
    const wavefunction = getEntry('quantum-mechanics-wavefunction');
    expect(wavefunction.kind).toBe('concept');
    expect(wavefunction.siUnit).toBeUndefined();
    expect(wavefunction.dimensions).toBe('L^-3/2');
    expect(wavefunction.conditions?.join(' ')).toMatch(
      /position representation in three dimensions/i,
    );
    expect(wavefunction.conditions?.join(' ')).toMatch(/abstract Hilbert-space notation/i);
  });

  it('records corrected scientific definitions and entry kinds', () => {
    expect(getEntry('foundations-scalar').definition).toMatch(/invariant under rotations/i);
    expect(getEntry('thermodynamics-partition-function').definition).toMatch(
      /normalizing constant/i,
    );
    expect(getEntry('quantum-mechanics-spin').conditions?.join(' ')).toMatch(
      /sqrt\(s\(s\+1\)\) hbar/i,
    );
    expect(getEntry('atomic-molecular-hyperfine-structure').definition).toMatch(
      /electric-quadrupole/i,
    );
    expect(getEntry('electronics-virtual-ground').definition).toMatch(
      /inverting op-amp configuration/i,
    );
    expect(getEntry('thermodynamics-ensemble').definition).toMatch(/macroscopic constraints/i);
    expect(getEntry('classical-mechanics-torque').conditions?.join(' ')).toMatch(
      /fixed origin in an inertial frame/i,
    );
    expect(getEntry('quantum-mechanics-commutator').notationVariants ?? []).not.toContain(
      String.raw`\{A,B\}`,
    );
    expect(getEntry('electronics-thermal-voltage').relatedTermIds ?? []).not.toContain(
      'electronics-ideal-opamp',
    );
    expect(getEntry('optics-polarization').definition).toMatch(/trajectory traced/i);
    expect(getEntry('foundations-average').kind).toBe('concept');
    expect(getEntry('foundations-root-mean-square').kind).toBe('concept');
    expect(getEntry('optics-diffraction-order').kind).toBe('notation');
    expect(getEntry('condensed-matter-density-of-states').kind).toBe('quantity');
    expect(getEntry('foundations-order-of-magnitude').relatedTermIds ?? []).not.toContain(
      'foundations-scalar',
    );
  });

  it('uses term-specific source checks and corrected locators', () => {
    const angular = getSource('classical-mechanics-angular-momentum', TEXTBOOKS.taylorClassical);
    expect(angular.locator).toBe('Ch. 3');
    expect(angular.locatorCheck).toBe('chapter-checked');

    const torque = getSource('classical-mechanics-torque', TEXTBOOKS.taylorClassical);
    expect(torque.locator).toBe('Ch. 3');

    expect(getSource('condensed-matter-lattice', TEXTBOOKS.kittel).locator).toBe('Ch. 1');
    expect(getSource('condensed-matter-lattice', TEXTBOOKS.kittel).locatorCheck).toBe(
      'chapter-checked',
    );
    expect(getSource('condensed-matter-lattice', TEXTBOOKS.ashcroftMermin).locator).toBe('Ch. 4');

    expect(getSource('thermodynamics-enthalpy', TEXTBOOKS.callen).locator).toBe('Ch. 5');
    expect(getSource('thermodynamics-enthalpy', TEXTBOOKS.kittelKroemer).locator).toBe('Ch. 8');
    expect(getSource('thermodynamics-enthalpy', TEXTBOOKS.callen).locatorCheck).toBe(
      'chapter-checked',
    );
    expect(getSource('thermodynamics-enthalpy', TEXTBOOKS.kittelKroemer).locatorCheck).toBe(
      'chapter-checked',
    );

    expect(getSource('thermodynamics-gibbs-free-energy', TEXTBOOKS.callen).locator).toBe('Ch. 5');
    expect(getSource('thermodynamics-gibbs-free-energy', TEXTBOOKS.kittelKroemer).locator).toBe(
      'Ch. 9',
    );
    expect(getSource('thermodynamics-gibbs-free-energy', TEXTBOOKS.callen).locatorCheck).toBe(
      'chapter-checked',
    );
    expect(
      getSource('thermodynamics-gibbs-free-energy', TEXTBOOKS.kittelKroemer).locatorCheck,
    ).toBe('chapter-checked');

    expect(
      getSource('atomic-molecular-rotational-constant', TEXTBOOKS.bransdenJoachain).locator,
    ).toBe('Ch. 11');
    expect(
      getSource('atomic-molecular-vibrational-frequency', TEXTBOOKS.bransdenJoachain).locator,
    ).toBe('Ch. 11');
    expect(getSource('atomic-molecular-vibrational-frequency', TEXTBOOKS.demtroder).locator).toBe(
      'Vibration-Rotation Transitions',
    );

    expect(getSource('foundations-tensor', TEXTBOOKS.arfken).locator).toBe('Ch. 4');
    expect(getSource('foundations-tensor', TEXTBOOKS.riley).locator).toBe('Ch. 26');
    expect(getSource('foundations-field', TEXTBOOKS.arfken).locator).toBe('Ch. 3');
    expect(getSource('mathematical-physics-gradient', TEXTBOOKS.riley).locator).toBe('Ch. 10');
    expect(getSource('mathematical-physics-eigenvalue', TEXTBOOKS.arfken).locator).toBe('Ch. 6');
    expect(getSource('mathematical-physics-orthogonal-functions', TEXTBOOKS.arfken).locator).toBe(
      'Ch. 8',
    );

    expect(getSource('condensed-matter-effective-mass', TEXTBOOKS.kittel).locator).toBe('Ch. 7');
    expect(getSource('condensed-matter-effective-mass', TEXTBOOKS.ashcroftMermin).locator).toBe(
      'Appendix E',
    );
    expect(getSource('nuclear-particle-isospin', TEXTBOOKS.krane).locator).toBe('Ch. 11, §11.3');
    expect(getSource('thermodynamics-chemical-potential', TEXTBOOKS.callen).locator).toBe('Ch. 2');
    expect(getSource('experimental-methods-chi-square', TEXTBOOKS.bevington).locatorCheck).toBe(
      'unverified',
    );
  });

  it('uses only entry-specific chapter-checked source assignments', () => {
    const actual = new Set(
      TERM_LIBRARY.entries.flatMap((entry) =>
        entry.sources
          .filter((source) => source.locatorCheck === 'chapter-checked')
          .map((source) => entrySourceKey(entry.id, source.citation, source.locator ?? '')),
      ),
    );

    expect(TERM_CHAPTER_CHECKED_SOURCES.size).toBe(64);
    expect(actual).toEqual(TERM_CHAPTER_CHECKED_SOURCES);
  });

  it('does not verify Kittel Ch. 2 for crystal lattice or K&K Ch. 9 for enthalpy', () => {
    expect(
      TERM_CHAPTER_CHECKED_SOURCES.has(
        entrySourceKey('condensed-matter-lattice', TEXTBOOKS.kittel, 'Ch. 2'),
      ),
    ).toBe(false);
    expect(
      TERM_CHAPTER_CHECKED_SOURCES.has(
        entrySourceKey('thermodynamics-enthalpy', TEXTBOOKS.kittelKroemer, 'Ch. 9'),
      ),
    ).toBe(false);
    expect(
      TERM_CHAPTER_CHECKED_SOURCES.has(
        entrySourceKey('condensed-matter-reciprocal-lattice', TEXTBOOKS.kittel, 'Ch. 2'),
      ),
    ).toBe(true);
    expect(
      TERM_CHAPTER_CHECKED_SOURCES.has(
        entrySourceKey('thermodynamics-gibbs-free-energy', TEXTBOOKS.kittelKroemer, 'Ch. 9'),
      ),
    ).toBe(true);
  });

  it('uses precise thermodynamic nodes where the titles support them', () => {
    const linkFor = (id: string, examCode: string) =>
      getEntry(id).syllabusLinks.find((link) => link.examCode === examCode);

    expect(linkFor('thermodynamics-enthalpy', 'jest-physics')?.nodes[0]?.code).toBe(
      JEST.firstLaw.code,
    );
    expect(linkFor('thermodynamics-gibbs-free-energy', 'gate-physics')?.nodes[0]?.code).toBe(
      GATE.partition.code,
    );
    expect(linkFor('thermodynamics-gibbs-free-energy', 'jest-physics')?.nodes[0]?.code).toBe(
      JEST.firstLaw.code,
    );
    expect(linkFor('thermodynamics-helmholtz-free-energy', 'gate-physics')?.nodes[0]?.code).toBe(
      GATE.partition.code,
    );
    expect(linkFor('thermodynamics-helmholtz-free-energy', 'jest-physics')?.nodes[0]?.code).toBe(
      JEST.firstLaw.code,
    );
    expect(linkFor('thermodynamics-gibbs-free-energy', 'tifr-gs-physics')?.nodes[0]?.code).toBe(
      TIFR.thermo.code,
    );
  });

  it('keeps BIPM locators explicitly unverified while recording the official URL', () => {
    const bipmSources = TERM_LIBRARY.entries
      .flatMap((entry) => entry.sources)
      .filter((source) => source.citation === REFERENCES.siBrochure);
    expect(bipmSources).toHaveLength(23);
    expect(bipmSources.every((source) => source.locatorCheck === 'unverified')).toBe(true);
    expect(
      bipmSources.every(
        (source) => source.url === 'https://www.bipm.org/en/publications/si-brochure',
      ),
    ).toBe(true);
  });

  it('keeps every explicit link in the reviewed allowlist', () => {
    const actual = new Set(
      TERM_LIBRARY.entries.flatMap((entry) =>
        entry.syllabusLinks
          .filter((link) => link.relevance === 'explicit')
          .flatMap((link) =>
            link.nodes.map((node) => explicitLinkKey(entry.id, link.examCode, node.code)),
          ),
      ),
    );
    expect(actual).toEqual(EXPLICIT_TERM_LINKS);
  });

  it('rejects an explicit link that has not been reviewed', () => {
    const entry = structuredClone(getEntry('electronics-virtual-ground'));
    const csirLink = entry.syllabusLinks.find((link) => link.examCode === 'csir-net-physics');
    expect(csirLink).toBeDefined();
    csirLink!.relevance = 'explicit';
    const issues = validateTermLibrary([entry], syllabusIndex, formulaIds);
    expect(
      issues.some((issue) =>
        /explicit relevance requires a reviewed term\/node match/i.test(issue.message),
      ),
      issues.map((issue) => issue.message).join('\n'),
    ).toBe(true);
  });

  it('rejects malformed dimensions, SI units, citations and notation', () => {
    const badDimensions = structuredClone(getEntry('units-metre'));
    badDimensions.dimensions = 'Q';
    expect(
      validateTermLibrary([badDimensions], syllabusIndex, formulaIds).some((issue) =>
        issue.path.endsWith('.dimensions'),
      ),
    ).toBe(true);

    const badUnit = structuredClone(getEntry('units-metre'));
    badUnit.siUnit = 'furlong';
    badUnit.siUnitLatex = undefined;
    expect(
      validateTermLibrary([badUnit], syllabusIndex, formulaIds).some((issue) =>
        issue.path.endsWith('.siUnit'),
      ),
    ).toBe(true);

    for (const invalidUnit of ['mkg', 'kkg', 'dkg']) {
      const badPrefixedKilogram = structuredClone(getEntry('units-kilogram'));
      badPrefixedKilogram.siUnit = invalidUnit;
      badPrefixedKilogram.siUnitLatex = undefined;
      badPrefixedKilogram.relatedTermIds = [];
      badPrefixedKilogram.relatedFormulaIds = [];
      expect(
        validateTermLibrary([badPrefixedKilogram], syllabusIndex, formulaIds).some((issue) =>
          issue.path.endsWith('.siUnit'),
        ),
        invalidUnit,
      ).toBe(true);
    }

    const badSymbol = structuredClone(getEntry('units-metre'));
    badSymbol.symbol = String.raw`\mathrm{M}`;
    badSymbol.symbolPlainText = 'M';
    const badSymbolIssues = validateTermLibrary([badSymbol], syllabusIndex, formulaIds);
    expect(badSymbolIssues.some((issue) => issue.path.endsWith('.symbol'))).toBe(true);
    expect(badSymbolIssues.some((issue) => issue.path.endsWith('.symbolPlainText'))).toBe(true);

    const badDerivedSymbol = structuredClone(getEntry('units-newton'));
    badDerivedSymbol.symbol = String.raw`\mathrm{J}`;
    badDerivedSymbol.symbolPlainText = 'J';
    expect(
      validateTermLibrary([badDerivedSymbol], syllabusIndex, formulaIds).some((issue) =>
        issue.path.endsWith('.symbol'),
      ),
    ).toBe(true);

    const badCitation = structuredClone(getEntry('units-metre'));
    badCitation.sources[0].citation = 'Unknown Physics Book';
    expect(
      validateTermLibrary([badCitation], syllabusIndex, formulaIds).some((issue) =>
        issue.path.endsWith('.sources[0].citation'),
      ),
    ).toBe(true);

    const badNotation = structuredClone(getEntry('foundations-vector'));
    badNotation.notationVariants = [String.raw`\frac{`];
    expect(
      validateTermLibrary([badNotation], syllabusIndex, formulaIds).some((issue) =>
        issue.path.includes('notationVariants'),
      ),
    ).toBe(true);
  });

  it('keeps the registered SI symbols for base and derived units', () => {
    expect(getEntry('units-metre').symbolPlainText).toBe('m');
    expect(getEntry('units-kilogram').symbolPlainText).toBe('kg');
    expect(getEntry('units-newton').symbolPlainText).toBe('N');
    expect(getEntry('units-ohm').symbolPlainText).toBe('Ω');
  });

  it('accepts prefixed and compound SI units when dimensions agree', () => {
    const prefixed = structuredClone(getEntry('units-newton'));
    prefixed.siUnit = 'kN';
    prefixed.siUnitLatex = String.raw`\mathrm{kN}`;
    prefixed.relatedTermIds = [];
    prefixed.relatedFormulaIds = [];
    const issues = validateTermLibrary([prefixed], syllabusIndex, formulaIds);
    expect(issues.filter((issue) => issue.path.includes('.siUnit'))).toEqual([]);

    const quotient = structuredClone(getEntry('units-newton'));
    quotient.siUnit = 'kg m/s^2';
    quotient.siUnitLatex = String.raw`\mathrm{kg}\,\mathrm{m}/\mathrm{s}^{2}`;
    quotient.relatedTermIds = [];
    quotient.relatedFormulaIds = [];
    const quotientIssues = validateTermLibrary([quotient], syllabusIndex, formulaIds);
    expect(quotientIssues.filter((issue) => issue.path.includes('.siUnit'))).toEqual([]);

    const equivalent = structuredClone(getEntry('units-newton'));
    equivalent.siUnit = 'kg m/s^2';
    equivalent.siUnitLatex = String.raw`\mathrm{kg}\,\mathrm{m}\,\mathrm{s}^{-2}`;
    equivalent.relatedTermIds = [];
    equivalent.relatedFormulaIds = [];
    const equivalentIssues = validateTermLibrary([equivalent], syllabusIndex, formulaIds);
    expect(equivalentIssues.filter((issue) => issue.path.includes('.siUnitLatex'))).toEqual([]);

    const milligram = structuredClone(getEntry('classical-mechanics-mass'));
    milligram.siUnit = 'mg';
    milligram.siUnitLatex = String.raw`\mathrm{mg}`;
    milligram.dimensions = 'M';
    milligram.dimensionFormula = String.raw`[m]=M`;
    milligram.relatedTermIds = [];
    milligram.relatedFormulaIds = [];
    const milligramIssues = validateTermLibrary([milligram], syllabusIndex, formulaIds);
    expect(milligramIssues.filter((issue) => issue.path.includes('.siUnit'))).toEqual([]);

    const dimensionless = structuredClone(getEntry('electromagnetism-relative-permittivity'));
    dimensionless.relatedTermIds = [];
    const dimensionlessIssues = validateTermLibrary([dimensionless], syllabusIndex, formulaIds);
    expect(dimensionlessIssues.filter((issue) => issue.path.includes('.siUnit'))).toEqual([]);
  });
});
