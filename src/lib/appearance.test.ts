import { beforeEach, describe, expect, it } from 'vitest';
import {
  applyAppearance,
  FONT_FAMILIES,
  FONT_FAMILY_STORAGE_KEY,
  FONT_SIZE_OPTIONS,
  FONT_SIZE_STORAGE_KEY,
  getStoredFontFamily,
  getStoredFontSize,
  normalizeFontFamily,
  normalizeFontSize,
  storeFontFamily,
  storeFontSize,
} from './appearance';

describe('appearance preferences', () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.removeAttribute('data-font-family');
    document.documentElement.removeAttribute('data-font-size');
  });

  it('offers exactly the four approved UI families and three text sizes', () => {
    expect(FONT_FAMILIES.map((family) => family.label)).toEqual([
      'Inter',
      'Manrope',
      'IBM Plex Sans',
      'Plus Jakarta Sans',
    ]);
    expect(FONT_SIZE_OPTIONS.map((size) => size.label)).toEqual([
      'Compact',
      'Default',
      'Comfortable',
    ]);
  });

  it('validates stored values with Inter and Default as safe defaults', () => {
    expect(normalizeFontFamily(null)).toBe('inter');
    expect(normalizeFontFamily('unexpected')).toBe('inter');
    expect(normalizeFontFamily('manrope')).toBe('manrope');
    expect(normalizeFontFamily('ibm-plex')).toBe('ibm-plex');
    expect(normalizeFontFamily('jakarta')).toBe('jakarta');
    expect(normalizeFontSize(null)).toBe('default');
    expect(normalizeFontSize('unexpected')).toBe('default');
    expect(normalizeFontSize('compact')).toBe('compact');
    expect(normalizeFontSize('comfortable')).toBe('comfortable');
  });

  it('persists and restores the selected family and size', () => {
    storeFontFamily('jakarta');
    storeFontSize('comfortable');

    expect(window.localStorage.getItem(FONT_FAMILY_STORAGE_KEY)).toBe('jakarta');
    expect(window.localStorage.getItem(FONT_SIZE_STORAGE_KEY)).toBe('comfortable');
    expect(getStoredFontFamily()).toBe('jakarta');
    expect(getStoredFontSize()).toBe('comfortable');
  });

  it('applies preferences to the document root for the CSS layer', () => {
    applyAppearance('ibm-plex', 'compact');

    expect(document.documentElement).toHaveAttribute('data-font-family', 'ibm-plex');
    expect(document.documentElement).toHaveAttribute('data-font-size', 'compact');
  });
});
