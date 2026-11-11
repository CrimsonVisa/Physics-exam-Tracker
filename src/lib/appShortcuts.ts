import { useEffect, useRef } from 'react';

export type ShortcutReferenceEntry = {
  keys: string[];
  label: string;
};

/**
 * Single source of truth for the shortcut reference shown in the search
 * overlay footer, the shortcuts dialog and Settings.
 */
export const SHORTCUT_REFERENCE: ShortcutReferenceEntry[] = [
  { keys: ['Ctrl', 'K'], label: 'Open global search (works while typing)' },
  { keys: ['Ctrl', 'Shift', 'N'], label: 'Open quick sticky notes' },
  {
    keys: ['Ctrl', '1'],
    label: 'Open the CSIR-UGC NET Physical Sciences workspace',
  },
  { keys: ['Ctrl', '2'], label: 'Open the GATE Physics workspace' },
  { keys: ['Ctrl', '3'], label: 'Open the TIFR GS Physics workspace' },
  { keys: ['Ctrl', '4'], label: 'Open the JEST Physics workspace' },
  { keys: ['Ctrl', '/'], label: 'Show this keyboard shortcut reference' },
  { keys: ['Esc'], label: 'Close the active dialog' },
  { keys: ['↑', '↓'], label: 'Move through search results' },
  { keys: ['Enter'], label: 'Open the selected search result' },
];

export const EXAM_SHORTCUTS: Array<{ key: string; examCode: string }> = [
  { key: '1', examCode: 'csir-net-physics' },
  { key: '2', examCode: 'gate-physics' },
  { key: '3', examCode: 'tifr-gs-physics' },
  { key: '4', examCode: 'jest-physics' },
];

export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const tag = target.tagName.toLowerCase();
  if (tag === 'input' || tag === 'textarea' || tag === 'select') {
    return true;
  }
  const editableAttribute = target.getAttribute('contenteditable');
  if (
    editableAttribute === '' ||
    editableAttribute === 'true' ||
    editableAttribute === 'plaintext-only'
  ) {
    return true;
  }
  return target.isContentEditable === true;
}

type AppShortcutHandlers = {
  onOpenSearch: () => void;
  onOpenShortcuts: () => void;
  onOpenNotes: () => void;
  onOpenExam: (examCode: string) => void;
};

/**
 * Installs exactly one window-level keyboard listener for the application
 * shell. Navigation shortcuts are suppressed while an editable element is
 * focused; Ctrl+K is the only combination explicitly allowed while typing.
 * Key repeat never re-triggers an action. Dialogs manage Escape themselves.
 */
export function useAppShortcuts(handlers: AppShortcutHandlers, enabled = true) {
  const handlersRef = useRef(handlers);
  const enabledRef = useRef(enabled);

  useEffect(() => {
    handlersRef.current = handlers;
  }, [handlers]);

  useEffect(() => {
    enabledRef.current = enabled;
  }, [enabled]);

  useEffect(() => {
    const listener = (event: KeyboardEvent) => {
      if (!enabledRef.current || event.defaultPrevented || event.repeat) {
        return;
      }
      const editable = isEditableTarget(event.target);
      const withCtrl = event.ctrlKey && !event.altKey && !event.metaKey;
      const withoutShift = !event.shiftKey;

      if (withCtrl && withoutShift && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        handlersRef.current.onOpenSearch();
        return;
      }

      if (withCtrl && event.shiftKey && event.key.toLowerCase() === 'n') {
        event.preventDefault();
        handlersRef.current.onOpenNotes();
        return;
      }

      if (editable) {
        return;
      }

      if (withCtrl && withoutShift && event.key === '/') {
        event.preventDefault();
        handlersRef.current.onOpenShortcuts();
        return;
      }

      if (withCtrl && withoutShift) {
        const shortcut = EXAM_SHORTCUTS.find((entry) => entry.key === event.key);
        if (shortcut) {
          event.preventDefault();
          handlersRef.current.onOpenExam(shortcut.examCode);
        }
      }
    };

    window.addEventListener('keydown', listener);
    return () => {
      window.removeEventListener('keydown', listener);
    };
  }, []);
}
