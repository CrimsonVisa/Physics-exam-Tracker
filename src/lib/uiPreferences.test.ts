import { beforeEach, describe, expect, it } from 'vitest';
import {
  EXAM_CONTEXT_STORAGE_KEY,
  SIDEBAR_MODE_STORAGE_KEY,
  THEME_STORAGE_KEY,
  applyTheme,
  getStoredExamContext,
  getStoredSidebarMode,
  getStoredTheme,
  normalizeExamContext,
  normalizeSidebarMode,
  normalizeTheme,
  storeExamContext,
  storeSidebarMode,
  storeTheme,
} from './uiPreferences';

describe('uiPreferences', () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
    document.documentElement.style.colorScheme = '';
  });

  it('validates theme and sidebar values with safe defaults', () => {
    expect(normalizeTheme(null)).toBe('dark');
    expect(normalizeTheme('unexpected')).toBe('dark');
    expect(normalizeTheme('light')).toBe('light');
    expect(normalizeSidebarMode(null)).toBe('expanded');
    expect(normalizeSidebarMode('unexpected')).toBe('expanded');
    expect(normalizeSidebarMode('collapsed')).toBe('collapsed');
    expect(normalizeSidebarMode('hidden')).toBe('hidden');
  });

  it('stores and reads validated preferences', () => {
    storeTheme('light');
    storeSidebarMode('hidden');
    storeExamContext(3);

    expect(window.localStorage.getItem(THEME_STORAGE_KEY)).toBe('light');
    expect(window.localStorage.getItem(SIDEBAR_MODE_STORAGE_KEY)).toBe('hidden');
    expect(window.localStorage.getItem(EXAM_CONTEXT_STORAGE_KEY)).toBe('3');
    expect(getStoredTheme()).toBe('light');
    expect(getStoredSidebarMode()).toBe('hidden');
    expect(getStoredExamContext()).toBe(3);

    window.localStorage.setItem(THEME_STORAGE_KEY, 'invalid');
    window.localStorage.setItem(SIDEBAR_MODE_STORAGE_KEY, 'invalid');
    window.localStorage.setItem(EXAM_CONTEXT_STORAGE_KEY, 'invalid');
    expect(getStoredTheme()).toBe('dark');
    expect(getStoredSidebarMode()).toBe('expanded');
    expect(getStoredExamContext()).toBeNull();

    storeExamContext(null);
    expect(window.localStorage.getItem(EXAM_CONTEXT_STORAGE_KEY)).toBeNull();
  });

  it('validates exam-context identifiers', () => {
    expect(normalizeExamContext(null)).toBeNull();
    expect(normalizeExamContext('')).toBeNull();
    expect(normalizeExamContext('0')).toBeNull();
    expect(normalizeExamContext('-1')).toBeNull();
    expect(normalizeExamContext('abc')).toBeNull();
    expect(normalizeExamContext('4')).toBe(4);
  });

  it('applies the theme attribute, colour scheme, and metadata colour', () => {
    const meta = document.createElement('meta');
    meta.name = 'theme-color';
    document.head.append(meta);

    applyTheme('light');
    expect(document.documentElement).toHaveAttribute('data-theme', 'light');
    expect(document.documentElement.style.colorScheme).toBe('light');
    expect(meta).toHaveAttribute('content', '#f3f5f8');

    applyTheme('dark');
    expect(document.documentElement).toHaveAttribute('data-theme', 'dark');
    expect(document.documentElement.style.colorScheme).toBe('dark');
    expect(meta).toHaveAttribute('content', '#0e1319');

    meta.remove();
  });
});
