import { beforeEach, describe, expect, it } from 'vitest';
import {
  SIDEBAR_MODE_STORAGE_KEY,
  THEME_STORAGE_KEY,
  applyTheme,
  getStoredSidebarMode,
  getStoredTheme,
  normalizeSidebarMode,
  normalizeTheme,
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

    expect(window.localStorage.getItem(THEME_STORAGE_KEY)).toBe('light');
    expect(window.localStorage.getItem(SIDEBAR_MODE_STORAGE_KEY)).toBe('hidden');
    expect(getStoredTheme()).toBe('light');
    expect(getStoredSidebarMode()).toBe('hidden');

    window.localStorage.setItem(THEME_STORAGE_KEY, 'invalid');
    window.localStorage.setItem(SIDEBAR_MODE_STORAGE_KEY, 'invalid');
    expect(getStoredTheme()).toBe('dark');
    expect(getStoredSidebarMode()).toBe('expanded');
  });

  it('applies the theme attribute, colour scheme, and metadata colour', () => {
    const meta = document.createElement('meta');
    meta.name = 'theme-color';
    document.head.append(meta);

    applyTheme('light');
    expect(document.documentElement).toHaveAttribute('data-theme', 'light');
    expect(document.documentElement.style.colorScheme).toBe('light');
    expect(meta).toHaveAttribute('content', '#F3F7F4');

    applyTheme('dark');
    expect(document.documentElement).toHaveAttribute('data-theme', 'dark');
    expect(document.documentElement.style.colorScheme).toBe('dark');
    expect(meta).toHaveAttribute('content', '#18252B');

    meta.remove();
  });
});
