import { describe, expect, it } from 'vitest';
import { CalcError } from './errors';
import {
  DEFAULT_CALCULATOR_MODE,
  CALCULATOR_MODE_ORDER,
  CALCULATOR_PROFILES,
  evidenceStatusLabel,
  functionAvailabilityLabel,
  getCalculatorProfile,
  isFunctionEnabled,
} from './profiles';
import type { CalculatorFunctionId } from './types';

const POLICY_MODES = [
  'csir-net-physics',
  'gate-physics',
  'tifr-gs-physics',
  'jest-physics',
] as const;

describe('calculator policy profiles', () => {
  it('defines exactly the five approved modes and defaults to Practice', () => {
    expect(CALCULATOR_MODE_ORDER).toEqual([
      'practice',
      'csir-net-physics',
      'gate-physics',
      'tifr-gs-physics',
      'jest-physics',
    ]);
    expect(DEFAULT_CALCULATOR_MODE).toBe('practice');
    expect(Object.keys(CALCULATOR_PROFILES)).toHaveLength(5);
  });

  it('makes Practice a full calculator with every approved function enabled', () => {
    const practice = getCalculatorProfile('practice');
    expect(practice.modeType).toBe('practice');
    expect(practice.hasKeypad).toBe(true);
    expect(practice.summary).toContain('not an examination simulation');
    for (const availability of Object.values(practice.functions)) {
      expect(availability).toBe('available');
    }
  });

  it('defines GATE as a scientific calculator with mock-observed evidence', () => {
    const gate = getCalculatorProfile('gate-physics');
    expect(gate.modeType).toBe('exam-calculator');
    expect(gate.hasKeypad).toBe(true);
    expect(gate.physicalCalculator).toBe('confirmed-prohibited');
    expect(gate.onScreenCalculator).toBe('confirmed-provided');
    expect(gate.functionSet).toBe('not-documented');
    expect(gate.caveats.join(' ')).toContain('not guaranteed');
    expect(gate.caveats.join(' ')).toContain('not an exact official GATE replica');
    for (const availability of Object.values(gate.functions)) {
      expect(availability).toBe('mock-observed');
    }
    expect(isFunctionEnabled(gate, 'sin')).toBe(true);
    expect(isFunctionEnabled(gate, 'memory')).toBe(true);
    expect(gate.sources.some((source) => source.kind === 'policy')).toBe(true);
    expect(gate.sources.some((source) => source.kind === 'mock-reference')).toBe(true);
    expect(gate.keyboardNote).toContain('keyboard operation is disabled');
  });

  it('does not give CSIR, TIFR or JEST an examination keypad', () => {
    for (const mode of ['csir-net-physics', 'tifr-gs-physics', 'jest-physics'] as const) {
      const profile = getCalculatorProfile(mode);
      expect(profile.modeType).toBe('exam-policy');
      expect(profile.hasKeypad).toBe(false);
      expect(profile.fallbackMode).toBe('practice');
      for (const availability of Object.values(profile.functions)) {
        expect(availability).toBe('not-documented');
      }
    }
  });

  it('records the approved policy distinctions without translating them', () => {
    const csir = getCalculatorProfile('csir-net-physics');
    expect(csir.physicalCalculator).toBe('confirmed-prohibited');
    expect(csir.onScreenCalculator).toBe('not-documented');
    expect(evidenceStatusLabel(csir.onScreenCalculator)).toBe('Not documented');
    expect(evidenceStatusLabel(csir.physicalCalculator)).toBe('Confirmed prohibited');

    const tifr = getCalculatorProfile('tifr-gs-physics');
    expect(tifr.physicalCalculator).toBe('confirmed-prohibited');
    expect(tifr.onScreenCalculator).toBe('confirmed-provided');
    expect(tifr.functionSet).toBe('not-documented');
    expect(tifr.applicableYear).toContain('GS 2024');
    expect(tifr.applicableYear).toContain('GS 2027');

    const jest = getCalculatorProfile('jest-physics');
    expect(jest.physicalCalculator).toBe('confirmed-prohibited');
    expect(jest.onScreenCalculator).toBe('not-documented');
    expect(jest.applicableYear).toBe('JEST 2026');

    const gate = getCalculatorProfile('gate-physics');
    expect(gate.onScreenCalculator).toBe('confirmed-provided');
    expect(evidenceStatusLabel(gate.onScreenCalculator)).toBe('Confirmed provided');
  });

  it('prevents GATE capabilities from leaking into other profiles', () => {
    const gate = getCalculatorProfile('gate-physics');
    for (const mode of POLICY_MODES) {
      const profile = getCalculatorProfile(mode);
      if (mode === 'gate-physics') {
        continue;
      }
      for (const functionId of Object.keys(gate.functions) as CalculatorFunctionId[]) {
        expect(profile.functions[functionId]).not.toBe('mock-observed');
        expect(profile.functions[functionId]).not.toBe('available');
        expect(isFunctionEnabled(profile, functionId)).toBe(false);
      }
    }
  });

  it('stores sources, years, references and last-verified dates', () => {
    for (const mode of CALCULATOR_MODE_ORDER) {
      const profile = getCalculatorProfile(mode);
      expect(profile.lastVerified).toBe('2026-11-08');
      expect(profile.applicableYear.length).toBeGreaterThan(0);
      for (const source of profile.sources) {
        expect(source.title.length).toBeGreaterThan(0);
        expect(source.url.startsWith('https://')).toBe(true);
        expect(source.year.length).toBeGreaterThan(0);
        expect(source.reference.length).toBeGreaterThan(0);
      }
    }
  });

  it('exposes availability labels for the UI', () => {
    expect(functionAvailabilityLabel('mock-observed')).toBe('Mock-observed');
    expect(functionAvailabilityLabel('not-documented')).toBe('Not documented');
    expect(functionAvailabilityLabel('available')).toBe('Available');
  });

  it('keeps typed calculation errors distinguishable', () => {
    const error = new CalcError('domain_error');
    expect(error.code).toBe('domain_error');
    expect(error.message).toContain('domain');
  });
});
