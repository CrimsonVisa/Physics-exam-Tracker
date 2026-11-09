import { describe, expect, it } from 'vitest';
import { TERM_ENTRIES } from './data';
import {
  searchTerms,
  subjectCounts,
  termExamCodes,
  termExamRelevance,
  topicOptions,
} from './search';

describe('term search', () => {
  it('ranks an exact term before broader matches', () => {
    const results = searchTerms(TERM_ENTRIES, 'newton');
    expect(results[0]?.id).toBe('units-newton');
  });

  it('searches aliases', () => {
    const results = searchTerms(TERM_ENTRIES, 'meter');
    expect(results[0]?.id).toBe('units-metre');
    expect(searchTerms(TERM_ENTRIES, 'moment of force')[0]?.id).toBe('classical-mechanics-torque');
  });

  it('searches Unicode symbols and LaTeX symbol commands', () => {
    expect(
      searchTerms(TERM_ENTRIES, '∇').some((entry) => entry.id === 'mathematical-physics-gradient'),
    ).toBe(true);
    expect(
      searchTerms(TERM_ENTRIES, String.raw`\hbar`).some(
        (entry) => entry.id === 'quantum-mechanics-uncertainty-principle',
      ),
    ).toBe(true);
    expect(
      searchTerms(TERM_ENTRIES, 'ω').some(
        (entry) => entry.id === 'waves-oscillations-angular-frequency',
      ),
    ).toBe(true);
  });

  it('keeps angular and ordinary wavenumber searches distinct', () => {
    expect(searchTerms(TERM_ENTRIES, 'angular wavenumber')[0]?.id).toBe(
      'waves-oscillations-wavenumber',
    );
    expect(searchTerms(TERM_ENTRIES, 'ordinary wavenumber')[0]?.id).toBe(
      'waves-oscillations-ordinary-wavenumber',
    );
    expect(searchTerms(TERM_ENTRIES, 'wavenumber')[0]?.id).toBe(
      'waves-oscillations-ordinary-wavenumber',
    );
  });

  it('searches SI units, dimensions and notation variants', () => {
    expect(
      searchTerms(TERM_ENTRIES, 'kg m s^-1').some(
        (entry) => entry.id === 'classical-mechanics-momentum',
      ),
    ).toBe(true);
    expect(searchTerms(TERM_ENTRIES, 'M L T^-2').some((entry) => entry.id === 'units-newton')).toBe(
      true,
    );
    expect(
      searchTerms(TERM_ENTRIES, 'grad f').some(
        (entry) => entry.id === 'mathematical-physics-gradient',
      ),
    ).toBe(true);
  });

  it('filters by examination, subject, kind, syllabus node and related formula', () => {
    const gateUnits = searchTerms(TERM_ENTRIES, '', {
      examCode: 'gate-physics',
      subject: 'units-measurement',
      kind: 'base-unit',
    });
    expect(gateUnits).toHaveLength(7);
    expect(gateUnits.every((entry) => entry.subject === 'units-measurement')).toBe(true);

    const dimensionalTerms = searchTerms(TERM_ENTRIES, '', {
      nodeCode: 'csir-part-a-core-mathematical-methods-of-physics-t001-dimensional-analysis',
    });
    expect(dimensionalTerms.some((entry) => entry.id === 'units-dimension')).toBe(true);

    const related = searchTerms(TERM_ENTRIES, '', {
      formulaId: 'classical-mechanics-newton-second-law',
    });
    expect(related.map((entry) => entry.id)).toContain('classical-mechanics-momentum');
  });

  it('keeps general-reference terms out of a specific exam filter', () => {
    const general = searchTerms(TERM_ENTRIES, '', {
      examCode: 'csir-net-physics',
    });
    expect(general.some((entry) => entry.id === 'foundations-order-of-magnitude')).toBe(false);
    expect(searchTerms(TERM_ENTRIES, '', { examCode: 'all' })).toContainEqual(
      expect.objectContaining({ id: 'foundations-order-of-magnitude' }),
    );
  });

  it('returns deterministic topic options and subject counts', () => {
    const topics = topicOptions(TERM_ENTRIES, {
      subject: 'units-measurement',
    });
    expect(topics).toEqual([
      'Coherent SI derived units',
      'Dimensions',
      'SI base units',
      'SI conventions',
    ]);

    const counts = subjectCounts(TERM_ENTRIES);
    expect(counts['units-measurement']).toBe(23);
    expect(counts['special-relativity']).toBe(5);

    const searchedCounts = subjectCounts(TERM_ENTRIES, {}, 'candela');
    expect(searchedCounts['units-measurement']).toBe(1);
    expect(Object.values(searchedCounts).reduce((total, count) => total + count, 0)).toBe(1);
  });

  it('derives exam relevance and supports TIFR uncertainty', () => {
    const momentum = TERM_ENTRIES.find((entry) => entry.id === 'classical-mechanics-momentum');
    expect(momentum ? termExamCodes(momentum) : []).toEqual([
      'csir-net-physics',
      'gate-physics',
      'tifr-gs-physics',
      'jest-physics',
    ]);
    expect(momentum ? termExamRelevance(momentum, 'tifr-gs-physics') : null).toBe('implicit');

    const binding = TERM_ENTRIES.find((entry) => entry.id === 'nuclear-particle-binding-energy');
    expect(binding ? termExamRelevance(binding, 'tifr-gs-physics') : null).toBe('uncertain');
    expect(binding ? termExamRelevance(binding, 'jest-physics') : null).toBeNull();
  });

  it('returns an empty result for an unmatched query', () => {
    expect(searchTerms(TERM_ENTRIES, 'zzzz-no-match')).toEqual([]);
  });
});
