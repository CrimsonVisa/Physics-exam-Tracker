import { describe, expect, it } from 'vitest';
import { FORMULA_ENTRIES } from './data';
import {
  entryExamCodes,
  entryExamRelevance,
  normalizeSearchText,
  searchFormulas,
  topicOptions,
} from './search';

describe('formula search', () => {
  it('ranks an exact title before broader matches', () => {
    const results = searchFormulas(FORMULA_ENTRIES, 'Maxwell-Faraday law (differential form)');
    expect(results[0]?.id).toBe('electromagnetism-maxwell-faraday');
  });

  it('ranks title prefixes and substrings deterministically', () => {
    const prefixResults = searchFormulas(FORMULA_ENTRIES, 'Maxwell');
    expect(prefixResults[0]?.id).toBe('electromagnetism-maxwell-faraday');

    const substringResults = searchFormulas(FORMULA_ENTRIES, 'Faraday');
    expect(substringResults[0]?.id).toBe('electromagnetism-maxwell-faraday');
  });

  it('searches plain text, symbols, meanings and keywords', () => {
    expect(
      searchFormulas(FORMULA_ENTRIES, 'dp/dt').some(
        (entry) => entry.id === 'classical-mechanics-newton-second-law',
      ),
    ).toBe(true);
    expect(
      searchFormulas(FORMULA_ENTRIES, 'relaxation time').some(
        (entry) => entry.id === 'condensed-matter-drude-conductivity',
      ),
    ).toBe(true);
  });

  it.each([
    ['ω', 'waves-oscillations-damped-frequency'],
    ['ħ', 'quantum-mechanics-time-independent-schrodinger'],
    ['∇', 'mathematical-physics-divergence-theorem'],
    ['∂', 'electromagnetism-poynting-theorem'],
  ])('supports one-character Unicode symbol search: %s', (query, expectedId) => {
    expect(searchFormulas(FORMULA_ENTRIES, query).some((entry) => entry.id === expectedId)).toBe(
      true,
    );
  });

  it('expands common LaTeX symbols for search', () => {
    expect(normalizeSearchText(String.raw`\omega \hbar \nabla \partial`)).toContain('ω');
    expect(normalizeSearchText(String.raw`\omega \hbar \nabla \partial`)).toContain('ħ');
    expect(normalizeSearchText(String.raw`\omega \hbar \nabla \partial`)).toContain('∇');
    expect(normalizeSearchText(String.raw`\omega \hbar \nabla \partial`)).toContain('∂');
  });

  it('filters by examination and excludes unsupported JEST subjects', () => {
    const jestEntries = searchFormulas(FORMULA_ENTRIES, '', {
      examCode: 'jest-physics',
    });
    expect(jestEntries.some((entry) => entry.id === 'electromagnetism-maxwell-faraday')).toBe(true);
    expect(jestEntries.some((entry) => entry.id === 'atomic-molecular-rydberg-formula')).toBe(
      false,
    );
    expect(jestEntries.some((entry) => entry.id === 'condensed-matter-drude-conductivity')).toBe(
      false,
    );
  });

  it('filters by subject and topic', () => {
    const quantum = searchFormulas(FORMULA_ENTRIES, '', {
      subject: 'quantum-mechanics',
    });
    expect(quantum).toHaveLength(4);
    expect(quantum.every((entry) => entry.subject === 'quantum-mechanics')).toBe(true);

    const diffraction = searchFormulas(FORMULA_ENTRIES, '', {
      topic: 'Diffraction',
    });
    expect(diffraction.map((entry) => entry.id).sort()).toEqual([
      'optics-diffraction-grating-equation',
      'optics-single-slit-minima',
    ]);
  });

  it('filters by an individual syllabus node code', () => {
    const results = searchFormulas(FORMULA_ENTRIES, '', {
      nodeCode: 'csir-part-a-core-classical-mechanics-t001-newton-s-laws',
    });
    expect(results.map((entry) => entry.id)).toEqual(['classical-mechanics-newton-second-law']);
  });

  it('returns deterministic topic options for a subject', () => {
    const topics = topicOptions(FORMULA_ENTRIES, {
      examCode: 'gate-physics',
      subject: 'quantum-mechanics',
    });
    expect(topics).toEqual([
      'Central potentials and hydrogen',
      'Operators and commutators',
      'Schrodinger equation',
    ]);
  });

  it('derives examination badges and relevance from node-level links', () => {
    const entry = FORMULA_ENTRIES.find(
      (candidate) => candidate.id === 'classical-mechanics-newton-second-law',
    );
    expect(entry).toBeDefined();
    expect(entry ? entryExamCodes(entry) : []).toEqual([
      'csir-net-physics',
      'gate-physics',
      'tifr-gs-physics',
      'jest-physics',
    ]);
    expect(entry ? entryExamRelevance(entry, 'tifr-gs-physics') : null).toBe('implicit');

    const rydberg = FORMULA_ENTRIES.find(
      (candidate) => candidate.id === 'atomic-molecular-rydberg-formula',
    );
    expect(rydberg ? entryExamRelevance(rydberg, 'tifr-gs-physics') : null).toBe('uncertain');
    expect(rydberg ? entryExamRelevance(rydberg, 'jest-physics') : null).toBeNull();
  });
});
