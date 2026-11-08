import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { FORMULA_LIBRARY } from './data';
import { CSIR, GATE, JEST } from './data/syllabus';
import { TEXTBOOKS } from './data/sources';
import { FORMULA_SUBJECTS } from './taxonomy';
import type { ExamCode, FormulaEntry, FormulaSource } from './types';
import { buildSyllabusIndex, validateFormulaLibrary, type SyllabusSeedExam } from './validation';

const syllabusSeed = JSON.parse(
  readFileSync(join(process.cwd(), 'src-tauri', 'src', 'v2', 'syllabus_seed.json'), 'utf8'),
) as SyllabusSeedExam[];

const syllabusIndex = buildSyllabusIndex(syllabusSeed);

function getEntry(id: string): FormulaEntry {
  const entry = FORMULA_LIBRARY.entries.find((candidate) => candidate.id === id);
  expect(entry, id).toBeDefined();
  return entry as FormulaEntry;
}

function linksFor(id: string, examCode: ExamCode) {
  return getEntry(id).syllabusLinks.filter((link) => link.examCode === examCode);
}

function sourceFor(id: string, fragment: string): FormulaSource {
  const source = getEntry(id).sources.find((candidate) => candidate.citation.includes(fragment));
  expect(source, `${id} source ${fragment}`).toBeDefined();
  return source as FormulaSource;
}

const CHAPTER_CHECKED_EXPECTED: Array<[string, string, string]> = [
  ['mathematical-physics-divergence-theorem', TEXTBOOKS.arfken, 'Ch. 3'],
  ['mathematical-physics-divergence-theorem', TEXTBOOKS.riley, 'Ch. 11'],
  ['mathematical-physics-cauchy-integral-formula', TEXTBOOKS.arfken, 'Ch. 11'],
  ['mathematical-physics-cauchy-integral-formula', TEXTBOOKS.riley, 'Ch. 24'],
  ['mathematical-physics-fourier-transform', TEXTBOOKS.arfken, 'Ch. 20'],
  ['mathematical-physics-green-function-poisson', TEXTBOOKS.arfken, 'Ch. 10'],
  ['mathematical-physics-green-function-poisson', TEXTBOOKS.riley, 'Ch. 23'],
  ['classical-mechanics-newton-second-law', TEXTBOOKS.taylorClassical, 'Ch. 1'],
  ['classical-mechanics-work-energy-theorem', TEXTBOOKS.taylorClassical, 'Ch. 4'],
  ['classical-mechanics-kepler-third-law', TEXTBOOKS.taylorClassical, 'Ch. 8'],
  ['electromagnetism-gauss-law-differential', TEXTBOOKS.jackson, 'Ch. 1'],
  ['electromagnetism-maxwell-faraday', TEXTBOOKS.jackson, 'Ch. 6'],
  ['electromagnetism-poynting-theorem', TEXTBOOKS.jackson, 'Ch. 6'],
  ['electromagnetism-lorentz-force', TEXTBOOKS.jackson, 'Ch. 12'],
  ['thermodynamics-first-law', TEXTBOOKS.kittelKroemer, 'Ch. 2'],
  ['thermodynamics-canonical-partition-function', TEXTBOOKS.callen, 'Ch. 16'],
  ['thermodynamics-fermi-dirac-distribution', TEXTBOOKS.pathria, 'Ch. 8'],
  ['thermodynamics-planck-blackbody', TEXTBOOKS.pathria, 'Ch. 7'],
  ['thermodynamics-planck-blackbody', TEXTBOOKS.kittelKroemer, 'Ch. 4'],
  ['condensed-matter-fermi-energy-3d', TEXTBOOKS.kittel, 'Ch. 6'],
  ['condensed-matter-fermi-energy-3d', TEXTBOOKS.ashcroftMermin, 'Ch. 2'],
  ['condensed-matter-drude-conductivity', TEXTBOOKS.kittel, 'Ch. 6'],
  ['condensed-matter-drude-conductivity', TEXTBOOKS.ashcroftMermin, 'Ch. 1'],
  ['condensed-matter-bragg-law', TEXTBOOKS.kittel, 'Ch. 2'],
  ['electronics-diode-equation', TEXTBOOKS.sedraSmith, 'Ch. 4'],
  ['electronics-diode-equation', TEXTBOOKS.horowitzHill, 'Ch. 1'],
  ['electronics-inverting-opamp-gain', TEXTBOOKS.sedraSmith, 'Ch. 2'],
  ['electronics-inverting-opamp-gain', TEXTBOOKS.horowitzHill, 'Ch. 4'],
  ['electronics-lc-resonance-frequency', TEXTBOOKS.horowitzHill, 'Ch. 1'],
  ['experimental-methods-standard-error-mean', TEXTBOOKS.bevington, 'Ch. 4'],
  ['experimental-methods-error-propagation', TEXTBOOKS.bevington, 'Ch. 3'],
  ['experimental-methods-chi-square', TEXTBOOKS.bevington, 'Ch. 4'],
  ['atomic-molecular-rydberg-formula', TEXTBOOKS.bransdenJoachain, 'Ch. 1'],
  ['atomic-molecular-rigid-rotor-energy', TEXTBOOKS.bransdenJoachain, 'Ch. 10, §10.1'],
  ['waves-oscillations-damped-frequency', TEXTBOOKS.french, 'Ch. 3'],
  ['waves-oscillations-quality-factor', TEXTBOOKS.french, 'Ch. 4'],
  ['foundations-power-law-linearization', TEXTBOOKS.bevington, 'Ch. 7, §7.4'],
];

describe('formula library data', () => {
  it('passes the complete structural and syllabus validation suite', () => {
    const issues = validateFormulaLibrary(FORMULA_LIBRARY.entries, syllabusIndex);
    expect(issues, issues.map((issue) => `${issue.path}: ${issue.message}`).join('\n')).toEqual([]);
  });

  it('contains the approved Stage 1 collection size and corrected version', () => {
    expect(FORMULA_LIBRARY.entries.length).toBe(47);
    expect(FORMULA_LIBRARY.version).toBe('0.2.0');
    expect(FORMULA_LIBRARY.lastReviewed).toBe('2026-11-08');
  });

  it('covers every formula subject with a balanced selection', () => {
    for (const subject of FORMULA_SUBJECTS) {
      const count = FORMULA_LIBRARY.entries.filter((entry) => entry.subject === subject.id).length;
      expect(count, subject.label).toBeGreaterThanOrEqual(3);
    }
  });

  it('links formulas to every supported examination', () => {
    const examCodes: ExamCode[] = [
      'csir-net-physics',
      'gate-physics',
      'tifr-gs-physics',
      'jest-physics',
    ];
    for (const examCode of examCodes) {
      const linked = FORMULA_LIBRARY.entries.filter((entry) =>
        entry.syllabusLinks.some((link) => link.examCode === examCode),
      );
      expect(linked.length, examCode).toBeGreaterThanOrEqual(8);
    }
  });

  it('keeps every syllabus link at the individual node level', () => {
    for (const entry of FORMULA_LIBRARY.entries) {
      for (const syllabusLink of entry.syllabusLinks) {
        expect(syllabusLink.nodes, entry.id).toHaveLength(1);
        expect(['explicit', 'implicit', 'uncertain']).toContain(syllabusLink.relevance);
      }
    }
  });

  it('contains no duplicate examination-node links', () => {
    for (const entry of FORMULA_LIBRARY.entries) {
      const keys = entry.syllabusLinks.flatMap((link) =>
        link.nodes.map((node) => `${link.examCode}:${node.code}`),
      );
      expect(new Set(keys).size, entry.id).toBe(keys.length);
    }
  });

  it('records the corrected final link counts', () => {
    const countFor = (examCode: ExamCode) =>
      FORMULA_LIBRARY.entries.reduce(
        (total, entry) =>
          total + entry.syllabusLinks.filter((link) => link.examCode === examCode).length,
        0,
      );
    expect(countFor('csir-net-physics')).toBe(47);
    expect(countFor('gate-physics')).toBe(47);
    expect(countFor('tifr-gs-physics')).toBe(47);
    expect(countFor('jest-physics')).toBe(38);
    expect(
      FORMULA_LIBRARY.entries.reduce((total, entry) => total + entry.syllabusLinks.length, 0),
    ).toBe(179);

    const relevanceCounts = { explicit: 0, implicit: 0, uncertain: 0 };
    for (const entry of FORMULA_LIBRARY.entries) {
      for (const link of entry.syllabusLinks) {
        relevanceCounts[link.relevance] += 1;
      }
    }
    expect(relevanceCounts).toEqual({ explicit: 83, implicit: 84, uncertain: 12 });
  });

  it('does not infer JEST coverage for atomic, nuclear or condensed matter', () => {
    const restrictedSubjects = new Set([
      'atomic-molecular',
      'nuclear-particle',
      'condensed-matter',
    ]);
    for (const entry of FORMULA_LIBRARY.entries) {
      if (!restrictedSubjects.has(entry.subject)) {
        continue;
      }
      expect(
        entry.syllabusLinks.some((link) => link.examCode === 'jest-physics'),
        `${entry.id} must not claim JEST relevance`,
      ).toBe(false);
    }
  });

  it('keeps TIFR links on the eight broad-area topics', () => {
    const tifrNodes = syllabusIndex.get('tifr-gs-physics');
    expect(tifrNodes).toBeDefined();
    for (const entry of FORMULA_LIBRARY.entries) {
      for (const syllabusLink of entry.syllabusLinks) {
        if (syllabusLink.examCode !== 'tifr-gs-physics') {
          continue;
        }
        expect(syllabusLink.relevance, `${entry.id} TIFR relevance`).not.toBe('explicit');
        for (const node of syllabusLink.nodes) {
          const indexed = tifrNodes?.get(node.code);
          expect(indexed?.kind, `${entry.id} TIFR node`).toBe('topic');
          expect(indexed?.title.startsWith('Broad area:'), entry.id).toBe(true);
        }
      }
    }
  });

  it('generalizes the Rydberg formula to hydrogen-like atoms', () => {
    const entry = getEntry('atomic-molecular-rydberg-formula');
    expect(entry.latex).toContain(String.raw`Z^2R_M`);
    expect(entry.plainText).toContain('Z^2 R_M');
    expect(entry.symbols.some((symbol) => symbol.symbol === 'Z')).toBe(true);
    expect(entry.symbols.some((symbol) => symbol.symbol === String.raw`R_M`)).toBe(true);
    expect(entry.conditions.join(' ')).toMatch(/hydrogen-like/i);
    expect(entry.conditions.join(' ')).toMatch(/reduced mass/i);
    expect(entry.specialCases?.join(' ')).toMatch(/Z = 1/);
  });

  it('states the Green-function sign convention and Malus incident intensity', () => {
    const green = getEntry('mathematical-physics-green-function-poisson');
    expect([...green.conditions, ...(green.conventions ?? [])].join(' ')).toMatch(
      /laplacian G = \+delta\^3/i,
    );

    const malus = getEntry('optics-malus-law');
    const incident = malus.symbols.find((symbol) => symbol.symbol === String.raw`I_0`);
    expect(incident?.meaning).toMatch(/before the polarizer/i);
  });

  it('includes the Z-squared dependence in the hydrogen-like energy levels', () => {
    const entry = getEntry('quantum-mechanics-hydrogen-energy-levels');
    expect(entry.title).toMatch(/hydrogen-like/i);
    expect(entry.latex).toContain(String.raw`Z^2`);
    expect(entry.plainText).toContain('Z^2');
    expect(entry.symbols.some((symbol) => symbol.symbol === 'Z')).toBe(true);
    expect(entry.conditions.join(' ')).toMatch(/Z = 1 recovers the hydrogen spectrum/i);
    expect(entry.specialCases?.join(' ')).toMatch(/Z = 1/);
    expect(entry.specialCases?.join(' ')).toMatch(/Z > 1/);
    expect(entry.keywords).toContain('Z');
  });

  it('states both Fourier transform directions and the unitary convention', () => {
    const entry = getEntry('mathematical-physics-fourier-transform');
    expect(entry.title).toMatch(/pair/i);
    expect(entry.latex).toContain(String.raw`e^{-ikx}`);
    expect(entry.latex).toContain(String.raw`e^{ikx}`);
    expect(entry.plainText).toContain('exp(-i k x)');
    expect(entry.plainText).toContain('exp(i k x)');
    expect(entry.conditions.join(' ')).toMatch(/unitary convention/i);
    expect(entry.conditions.join(' ')).toMatch(/angular wavenumber/i);
    expect(entry.conditions.join(' ')).toMatch(/angular frequency/i);
  });

  it('restricts the Poynting medium definitions correctly', () => {
    const entry = getEntry('electromagnetism-poynting-theorem');
    const conditions = entry.conditions.join(' ');
    expect(conditions).toMatch(/nondispersive/i);
    expect(conditions).toMatch(/lossless/i);
    expect(conditions).toMatch(/dispersive or absorptive/i);
    expect(conditions).toMatch(/not generally valid/i);
  });

  it('defines the Einstein coefficient spectral-density convention', () => {
    const entry = getEntry('atomic-molecular-einstein-coefficients');
    const conventions = entry.conventions?.join(' ') ?? '';
    expect(conventions).toMatch(/per unit frequency/i);
    expect(conventions).toMatch(/angular-frequency/i);
    expect(conventions).toMatch(/numerical factor/i);
    const bSymbols = entry.symbols.filter((candidate) => /B_\{?(12|21)\}?/.test(candidate.symbol));
    expect(bSymbols).toHaveLength(2);
    for (const symbol of bSymbols) {
      expect(symbol.unit).toBe('m^3 J^-1 s^-2');
      expect(symbol.dimensions).toBe('M^-1 L');
    }
  });

  it('makes the semi-empirical coefficients explicitly parameter-dependent', () => {
    const entry = getEntry('nuclear-particle-semi-empirical-mass-formula');
    const text = [...entry.conditions, ...(entry.conventions ?? [])].join(' ');
    expect(text).toMatch(/parameter-dependent/i);
    expect(text).toMatch(/coefficient set/i);
    expect(text).toMatch(/pairing/i);
  });

  it('states the kinematic Bragg approximation and limitations', () => {
    const entry = getEntry('condensed-matter-bragg-law');
    const conditions = entry.conditions.join(' ');
    expect(conditions).toMatch(/kinematic/i);
    expect(conditions).toMatch(/single-scattering/i);
    expect(conditions).toMatch(/dynamical diffraction/i);
    expect(conditions).toMatch(/strong-scattering/i);
    expect(conditions).toMatch(/exact within the kinematic approximation/i);
  });

  it('distinguishes population and sample standard errors and the t qualification', () => {
    const entry = getEntry('experimental-methods-standard-error-mean');
    const conditions = entry.conditions.join(' ');
    expect(conditions).toMatch(/population standard deviation/i);
    expect(conditions).toMatch(/sample standard deviation/i);
    expect(conditions).toMatch(/t-distribution/i);
    expect(conditions).toMatch(/N - 1 degrees of freedom/i);
    expect(entry.symbols.some((symbol) => symbol.symbol === 's')).toBe(true);
  });

  it('records the inverse-temperature units of the canonical beta symbol', () => {
    const entry = getEntry('thermodynamics-canonical-partition-function');
    const beta = entry.symbols.find((symbol) => symbol.symbol === String.raw`\beta`);
    expect(beta?.unit).toBe('J^-1');
    expect(beta?.dimensions).toBe('M^-1 L^-2 T^2');
  });

  it('defines the quality factor using the power-resonance FWHM', () => {
    const entry = getEntry('waves-oscillations-quality-factor');
    expect(entry.title).toMatch(/power-resonance/i);
    expect(entry.title).toMatch(/high-Q/i);
    expect(entry.latex).toContain(String.raw`\Delta\omega_{\mathrm{FWHM}}`);
    expect(entry.conditions.join(' ')).toMatch(/power resonance/i);
    expect(entry.conditions.join(' ')).toMatch(/half maximum/i);
    expect(entry.conditions.join(' ')).toMatch(/high-Q\/narrowband/i);
  });

  it('clarifies ideal op-amp impedances and the external input resistor', () => {
    const entry = getEntry('electronics-inverting-opamp-gain');
    const conditions = entry.conditions.join(' ');
    expect(conditions).toMatch(/zero output impedance/i);
    expect(conditions).toMatch(/external resistors/i);
    expect(
      entry.symbols.some(
        (symbol) =>
          symbol.symbol === String.raw`R_{\mathrm{in}}` &&
          /external input resistor/i.test(symbol.meaning),
      ),
    ).toBe(true);
  });

  it('records editions for previously incomplete citations', () => {
    for (const citation of [
      TEXTBOOKS.foot,
      TEXTBOOKS.krane,
      TEXTBOOKS.ashcroftMermin,
      TEXTBOOKS.taylorClassical,
      TEXTBOOKS.french,
      TEXTBOOKS.crawford,
      TEXTBOOKS.demtroder,
    ]) {
      expect(citation).toMatch(/1st ed\./);
    }
  });

  it('does not retain the weak Sedra source for LC resonance', () => {
    const entry = getEntry('electronics-lc-resonance-frequency');
    expect(entry.sources).toHaveLength(1);
    expect(entry.sources[0].citation).toContain('Horowitz');
  });

  it('records the corrected source locators and chapter-check status', () => {
    const arfken = sourceFor('mathematical-physics-divergence-theorem', 'Arfken');
    expect(arfken.locator).toBe('Ch. 3');
    expect(arfken.locatorCheck).toBe('chapter-checked');

    const taylor = sourceFor('classical-mechanics-newton-second-law', 'Taylor, Classical');
    expect(taylor.locator).toBe('Ch. 1');
    expect(taylor.locatorCheck).toBe('chapter-checked');

    const jackson = sourceFor('electromagnetism-lorentz-force', 'Jackson');
    expect(jackson.locator).toBe('Ch. 12');
    expect(jackson.locatorCheck).toBe('chapter-checked');

    const sedra = sourceFor('electronics-diode-equation', 'Sedra');
    expect(sedra.locator).toBe('Ch. 4');
    expect(sedra.locatorCheck).toBe('chapter-checked');
  });

  it('covers every chapter-checked source assignment exactly once', () => {
    const actual = FORMULA_LIBRARY.entries.flatMap((entry) =>
      entry.sources
        .filter((source) => source.locatorCheck === 'chapter-checked')
        .map((source) => `${entry.id}|${source.citation}|${source.locator ?? ''}` as const),
    );
    const expected = CHAPTER_CHECKED_EXPECTED.map(
      ([id, citation, locator]) => `${id}|${citation}|${locator}` as const,
    );
    expect(actual.sort()).toEqual(expected.sort());
  });

  it('records chapter-level checks without claiming formula-level verification', () => {
    const powerLaw = sourceFor('foundations-power-law-linearization', 'Bevington');
    expect(powerLaw.locator).toBe('Ch. 7, §7.4');
    expect(powerLaw.locatorCheck).toBe('chapter-checked');
    expect(sourceFor('foundations-power-law-linearization', 'Riley').locatorCheck).toBe(
      'unverified',
    );

    const rydberg = sourceFor('atomic-molecular-rydberg-formula', 'Bransden');
    expect(rydberg.locator).toBe('Ch. 1');
    expect(rydberg.locatorCheck).toBe('chapter-checked');

    const rigidRotor = sourceFor('atomic-molecular-rigid-rotor-energy', 'Bransden');
    expect(rigidRotor.locator).toBe('Ch. 10, §10.1');
    expect(rigidRotor.locatorCheck).toBe('chapter-checked');

    const firstLaw = sourceFor('thermodynamics-first-law', 'Kittel');
    expect(firstLaw.locator).toBe('Ch. 2');
    expect(firstLaw.locatorCheck).toBe('chapter-checked');

    const canonical = sourceFor('thermodynamics-canonical-partition-function', 'Callen');
    expect(canonical.locator).toBe('Ch. 16');
    expect(canonical.locatorCheck).toBe('chapter-checked');

    const qualityFactor = sourceFor('waves-oscillations-quality-factor', 'French');
    expect(qualityFactor.locator).toBe('Ch. 4');
    expect(qualityFactor.locatorCheck).toBe('chapter-checked');

    const green = sourceFor('mathematical-physics-green-function-poisson', 'Riley');
    expect(green.locator).toBe('Ch. 23');
    expect(green.locatorCheck).toBe('chapter-checked');

    const damped = sourceFor('waves-oscillations-damped-frequency', 'French');
    expect(damped.locator).toBe('Ch. 3');
    expect(damped.locatorCheck).toBe('chapter-checked');
  });

  it('marks unresolved locators explicitly and removes the weak dimensional-form source', () => {
    const demtroder = sourceFor('atomic-molecular-rigid-rotor-energy', 'Demtröder');
    expect(demtroder.locatorCheck).toBe('unverified');

    const pdg = sourceFor('nuclear-particle-radioactive-decay-law', 'Particle Data Group');
    expect(pdg.locatorCheck).toBe('unverified');

    const dimensional = getEntry('foundations-dimensional-form');
    expect(dimensional.sources.map((source) => source.citation)).toEqual([
      'BIPM, The International System of Units (SI), 9th ed.',
    ]);
    expect(dimensional.sources[0].locatorCheck).toBe('unverified');
  });

  it('records the final source assignment and chapter-check counts', () => {
    const sources = FORMULA_LIBRARY.entries.flatMap((entry) => entry.sources);
    expect(sources).toHaveLength(93);
    expect(sources.filter((source) => source.locatorCheck === 'chapter-checked')).toHaveLength(37);
    expect(sources.filter((source) => source.locatorCheck === 'unverified')).toHaveLength(56);
  });

  it('removes the four weak CSIR advanced-unit duplicate links', () => {
    for (const id of [
      'classical-mechanics-newton-second-law',
      'electromagnetism-maxwell-faraday',
      'quantum-mechanics-time-independent-schrodinger',
      'thermodynamics-first-law',
    ]) {
      expect(linksFor(id, 'csir-net-physics'), id).toHaveLength(1);
    }
  });

  it('reclassifies the broad optics and error links and TIFR optics links', () => {
    for (const id of [
      'optics-single-slit-minima',
      'optics-diffraction-grating-equation',
      'optics-malus-law',
    ]) {
      for (const examCode of ['csir-net-physics', 'gate-physics', 'jest-physics'] as ExamCode[]) {
        const links = linksFor(id, examCode);
        expect(links, `${id} ${examCode}`).toHaveLength(1);
        const expectedRelevance =
          id === 'optics-diffraction-grating-equation' && examCode === 'gate-physics'
            ? 'explicit'
            : 'implicit';
        expect(links[0].relevance, `${id} ${examCode}`).toBe(expectedRelevance);
      }
      const tifr = linksFor(id, 'tifr-gs-physics');
      expect(tifr).toHaveLength(1);
      expect(tifr[0].relevance).toBe('uncertain');
      expect(tifr[0].note?.trim().length).toBeGreaterThan(0);
    }

    expect(
      linksFor('experimental-methods-standard-error-mean', 'csir-net-physics')[0].relevance,
    ).toBe('implicit');
    expect(linksFor('experimental-methods-standard-error-mean', 'gate-physics')[0].relevance).toBe(
      'implicit',
    );
    expect(linksFor('experimental-methods-error-propagation', 'gate-physics')[0].relevance).toBe(
      'implicit',
    );
    expect(linksFor('condensed-matter-drude-conductivity', 'gate-physics')[0].relevance).toBe(
      'implicit',
    );
  });

  it('replaces the coarse unit links with precise explicit topic links', () => {
    const expected: Array<[string, ExamCode, string]> = [
      [
        'mathematical-physics-divergence-theorem',
        'jest-physics',
        'jest-2026-mathematical-methods-t001-vector-algebra-and-vector-calculus',
      ],
      [
        'mathematical-physics-cauchy-integral-formula',
        'gate-physics',
        'gate-ph-2027-mathematical-physics-t004-complex-analysis-cauchy-riemann-conditions-cauchy-',
      ],
      [
        'mathematical-physics-cauchy-integral-formula',
        'jest-physics',
        'jest-2026-mathematical-methods-t007-complex-analysis',
      ],
      [
        'mathematical-physics-fourier-transform',
        'gate-physics',
        'gate-ph-2027-mathematical-physics-t005-fourier-analysis',
      ],
      [
        'mathematical-physics-fourier-transform',
        'jest-physics',
        'jest-2026-mathematical-methods-t008-fourier-series-fourier-transforms-and-laplace-tran',
      ],
      [
        'foundations-dimensional-form',
        'csir-net-physics',
        'csir-part-a-core-mathematical-methods-of-physics-t001-dimensional-analysis',
      ],
      [
        'foundations-dimensional-form',
        'gate-physics',
        'gate-ph-2027-measurements-and-error-analysis-t001-units-and-dimensions-dimensional-analysis',
      ],
      [
        'electronics-diode-equation',
        'csir-net-physics',
        'csir-part-a-core-electronics-and-experimental-meth-t001-semiconductor-devices-diodes-junctions-transistors',
      ],
      [
        'electronics-diode-equation',
        'gate-physics',
        'gate-ph-2027-electronics-t001-p-n-diodes-bipolar-junction-transistors-and-field-',
      ],
      [
        'electronics-diode-equation',
        'jest-physics',
        'jest-2026-electronics-t002-p-n-junctions-diodes-and-transistors',
      ],
      [
        'electronics-inverting-opamp-gain',
        'csir-net-physics',
        'csir-part-a-core-electronics-and-experimental-meth-t004-operational-amplifiers-and-their-applications',
      ],
      [
        'electronics-inverting-opamp-gain',
        'gate-physics',
        'gate-ph-2027-electronics-t003-oscillators-operational-amplifiers-and-active-filt',
      ],
      [
        'electronics-inverting-opamp-gain',
        'jest-physics',
        'jest-2026-electronics-t004-basics-of-opamps-and-their-applications',
      ],
      [
        'electronics-lc-resonance-frequency',
        'jest-physics',
        'jest-2026-electronics-t003-lcr-circuits-rectifiers-amplifiers-active-filters-',
      ],
      [
        'atomic-molecular-einstein-coefficients',
        'csir-net-physics',
        'csir-part-b-advanced-atomic-and-molecular-physics-t010-lasers-spontaneous-and-stimulated-emission-einstei',
      ],
    ];

    for (const [id, examCode, nodeCode] of expected) {
      const link = linksFor(id, examCode)[0];
      expect(link, `${id} ${examCode}`).toBeDefined();
      expect(link.relevance, `${id} ${examCode}`).toBe('explicit');
      expect(link.nodes[0].code, `${id} ${examCode}`).toBe(nodeCode);
    }
  });

  it('uses the precise atomic, condensed, oscillator and error-analysis nodes', () => {
    const expected: Array<[string, ExamCode, string, 'explicit' | 'implicit']> = [
      [
        'condensed-matter-bragg-law',
        'csir-net-physics',
        CSIR.diffractionStructure.code,
        'explicit',
      ],
      ['condensed-matter-bragg-law', 'gate-physics', GATE.crystallography.code, 'explicit'],
      ['condensed-matter-fermi-energy-3d', 'csir-net-physics', CSIR.freeElectron.code, 'explicit'],
      ['condensed-matter-fermi-energy-3d', 'gate-physics', GATE.freeElectron.code, 'explicit'],
      ['atomic-molecular-rydberg-formula', 'gate-physics', GATE.atomicSpectra.code, 'explicit'],
      [
        'atomic-molecular-rigid-rotor-energy',
        'csir-net-physics',
        CSIR.molecularSpectra.code,
        'explicit',
      ],
      [
        'atomic-molecular-rigid-rotor-energy',
        'gate-physics',
        GATE.molecularSpectra.code,
        'explicit',
      ],
      [
        'waves-oscillations-damped-frequency',
        'csir-net-physics',
        CSIR.smallOscillations.code,
        'explicit',
      ],
      [
        'waves-oscillations-damped-frequency',
        'gate-physics',
        GATE.smallOscillations.code,
        'explicit',
      ],
      [
        'waves-oscillations-quality-factor',
        'csir-net-physics',
        CSIR.smallOscillations.code,
        'explicit',
      ],
      [
        'waves-oscillations-quality-factor',
        'gate-physics',
        GATE.smallOscillations.code,
        'explicit',
      ],
      [
        'experimental-methods-standard-error-mean',
        'gate-physics',
        GATE.errorAnalysis.code,
        'implicit',
      ],
      [
        'experimental-methods-standard-error-mean',
        'jest-physics',
        JEST.probabilityError.code,
        'implicit',
      ],
      [
        'experimental-methods-error-propagation',
        'gate-physics',
        GATE.errorAnalysis.code,
        'implicit',
      ],
      [
        'experimental-methods-error-propagation',
        'jest-physics',
        JEST.probabilityError.code,
        'implicit',
      ],
      ['experimental-methods-chi-square', 'gate-physics', GATE.errorAnalysis.code, 'implicit'],
      ['experimental-methods-chi-square', 'jest-physics', JEST.probabilityError.code, 'implicit'],
    ];

    for (const [id, examCode, nodeCode, relevance] of expected) {
      const links = linksFor(id, examCode);
      expect(links, `${id} ${examCode}`).toHaveLength(1);
      expect(links[0].nodes[0].code, `${id} ${examCode}`).toBe(nodeCode);
      expect(links[0].relevance, `${id} ${examCode}`).toBe(relevance);
    }
  });

  it('retains only defensible related-formula links', () => {
    expect(getEntry('condensed-matter-drude-conductivity').relatedIds).not.toContain(
      'electronics-lc-resonance-frequency',
    );
    expect(getEntry('electronics-inverting-opamp-gain').relatedIds).not.toContain(
      'electronics-lc-resonance-frequency',
    );
    expect(getEntry('electronics-lc-resonance-frequency').relatedIds).toContain(
      'waves-oscillations-quality-factor',
    );
  });

  it('rejects explicit TIFR relevance in runtime validation', () => {
    const entry = structuredClone(getEntry('condensed-matter-bragg-law'));
    const tifrLink = entry.syllabusLinks.find((link) => link.examCode === 'tifr-gs-physics');
    expect(tifrLink).toBeDefined();
    tifrLink!.relevance = 'explicit';
    const issues = validateFormulaLibrary([entry], syllabusIndex);
    expect(
      issues.some((issue) => /TIFR links must not be explicit/i.test(issue.message)),
      issues.map((issue) => issue.message).join('\n'),
    ).toBe(true);
  });

  it('requires explicit locator-check metadata in runtime validation', () => {
    const entry = structuredClone(getEntry('condensed-matter-bragg-law'));
    delete entry.sources[0].locatorCheck;
    const missing = validateFormulaLibrary([entry], syllabusIndex);
    expect(
      missing.some((issue) => /locatorCheck is required/i.test(issue.message)),
      missing.map((issue) => issue.message).join('\n'),
    ).toBe(true);

    const invalid = structuredClone(getEntry('condensed-matter-bragg-law'));
    invalid.sources[0].locatorCheck = 'formula-verified' as never;
    const invalidIssues = validateFormulaLibrary([invalid], syllabusIndex);
    expect(
      invalidIssues.some((issue) => /locatorCheck must be/i.test(issue.message)),
      invalidIssues.map((issue) => issue.message).join('\n'),
    ).toBe(true);
  });
});
