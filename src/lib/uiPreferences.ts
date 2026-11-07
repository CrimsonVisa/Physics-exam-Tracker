export type Theme = 'dark' | 'light';
export type SidebarMode = 'expanded' | 'collapsed' | 'hidden';

export const THEME_STORAGE_KEY = 'csir-net-tracker.ui.theme.v1';
export const SIDEBAR_MODE_STORAGE_KEY = 'csir-net-tracker.ui.sidebar-mode.v1';
export const EXAM_CONTEXT_STORAGE_KEY = 'csir-net-tracker.ui.exam-context.v1';

const THEME_COLORS: Record<Theme, string> = {
  dark: '#0e1319',
  light: '#f3f5f8',
};

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

export function normalizeTheme(value: string | null | undefined): Theme {
  return value === 'light' ? 'light' : 'dark';
}

export function normalizeSidebarMode(value: string | null | undefined): SidebarMode {
  if (value === 'collapsed' || value === 'hidden') {
    return value;
  }

  return 'expanded';
}

export function getStoredTheme(): Theme {
  return normalizeTheme(readStorage(THEME_STORAGE_KEY));
}

export function normalizeExamContext(value: string | null | undefined): number | null {
  if (!value || !/^\d+$/.test(value)) {
    return null;
  }
  const examId = Number(value);
  return Number.isSafeInteger(examId) && examId > 0 ? examId : null;
}

export function getStoredExamContext(): number | null {
  return normalizeExamContext(readStorage(EXAM_CONTEXT_STORAGE_KEY));
}

export function storeExamContext(examId: number | null) {
  if (examId === null) {
    if (typeof window !== 'undefined') {
      try {
        window.localStorage.removeItem(EXAM_CONTEXT_STORAGE_KEY);
      } catch {
        // A blocked preference store must not prevent the UI from working.
      }
    }
    return;
  }
  writeStorage(EXAM_CONTEXT_STORAGE_KEY, String(examId));
}

export function getStoredSidebarMode(): SidebarMode {
  return normalizeSidebarMode(readStorage(SIDEBAR_MODE_STORAGE_KEY));
}

export function storeTheme(theme: Theme) {
  writeStorage(THEME_STORAGE_KEY, theme);
}

export function storeSidebarMode(mode: SidebarMode) {
  writeStorage(SIDEBAR_MODE_STORAGE_KEY, mode);
}

export function applyTheme(theme: Theme) {
  if (typeof document === 'undefined') {
    return;
  }

  document.documentElement.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme;

  const themeColor = document.querySelector('meta[name="theme-color"]');
  if (themeColor) {
    themeColor.setAttribute('content', THEME_COLORS[theme]);
  }
}
