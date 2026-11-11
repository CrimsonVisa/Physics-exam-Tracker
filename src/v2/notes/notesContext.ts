import { createContext, useContext } from 'react';
import type { V2StickyNote, V2StickyNoteInput } from '../api';

export type NotesSaveState = 'idle' | 'dirty' | 'saving' | 'saved' | 'error';

export type NotesContextValue = {
  notes: V2StickyNote[] | null;
  loading: boolean;
  error: string | null;
  panelOpen: boolean;
  saveState: NotesSaveState;
  saveError: string | null;
  lastSavedAt: string | null;
  openPanel: () => void;
  closePanel: () => void;
  ensureLoaded: (force?: boolean) => Promise<void>;
  markDirty: () => void;
  saveNote: (input: V2StickyNoteInput) => Promise<V2StickyNote>;
  deleteNote: (id: number) => Promise<void>;
};

export const NotesContext = createContext<NotesContextValue | null>(null);

export function useNotes(): NotesContextValue {
  const value = useContext(NotesContext);
  if (!value) {
    throw new Error('useNotes must be used inside NotesProvider.');
  }
  return value;
}
