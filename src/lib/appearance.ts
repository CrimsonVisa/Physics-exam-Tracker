export type FontFamily = 'inter' | 'manrope' | 'ibm-plex' | 'jakarta';
export type FontSizePreference = 'compact' | 'default' | 'comfortable';

export const FONT_FAMILY_STORAGE_KEY = 'csir-net-tracker.ui.font-family.v1';
export const FONT_SIZE_STORAGE_KEY = 'csir-net-tracker.ui.font-size.v1';

export type FontFamilyOption = {
  id: FontFamily;
  label: string;
  sampleClassName: string;
};

export const FONT_FAMILIES: FontFamilyOption[] = [
  { id: 'inter', label: 'Inter', sampleClassName: 'v2-font-sample-inter' },
  { id: 'manrope', label: 'Manrope', sampleClassName: 'v2-font-sample-manrope' },
  {
    id: 'ibm-plex',
    label: 'IBM Plex Sans',
    sampleClassName: 'v2-font-sample-ibm',
  },
  {
    id: 'jakarta',
    label: 'Plus Jakarta Sans',
    sampleClassName: 'v2-font-sample-jakarta',
  },
];

export const FONT_SIZE_OPTIONS: Array<{ id: FontSizePreference; label: string }> = [
  { id: 'compact', label: 'Compact' },
  { id: 'default', label: 'Default' },
  { id: 'comfortable', label: 'Comfortable' },
];

function readStorage(key: string): string | null {
  if (typeof window === 'undefined') {
    return null;
  }
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeStorage(key: string, value: string) {
  if (typeof window === 'undefined') {
    return;
  }
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // A blocked preference store must not prevent the UI from working.
  }
}

export function normalizeFontFamily(value: string | null | undefined): FontFamily {
  if (value === 'manrope' || value === 'ibm-plex' || value === 'jakarta') {
    return value;
  }
  return 'inter';
}

export function normalizeFontSize(value: string | null | undefined): FontSizePreference {
  if (value === 'compact' || value === 'comfortable') {
    return value;
  }
  return 'default';
}

export function getStoredFontFamily(): FontFamily {
  return normalizeFontFamily(readStorage(FONT_FAMILY_STORAGE_KEY));
}

export function getStoredFontSize(): FontSizePreference {
  return normalizeFontSize(readStorage(FONT_SIZE_STORAGE_KEY));
}

export function storeFontFamily(fontFamily: FontFamily) {
  writeStorage(FONT_FAMILY_STORAGE_KEY, fontFamily);
}

export function storeFontSize(fontSize: FontSizePreference) {
  writeStorage(FONT_SIZE_STORAGE_KEY, fontSize);
}

/**
 * Applies typography preferences to the document root. The CSS layer reads
 * `data-font-family` and `data-font-size` on the root element, so a change is
 * visible immediately everywhere and survives restarts through local storage.
 * Theme and sidebar preferences remain in their own existing keys.
 */
export function applyAppearance(fontFamily: FontFamily, fontSize: FontSizePreference) {
  if (typeof document === 'undefined') {
    return;
  }
  document.documentElement.dataset.fontFamily = fontFamily;
  document.documentElement.dataset.fontSize = fontSize;
}
