import { useCallback, useMemo, useRef, useState, type ReactNode } from 'react';
import { toCommandError } from '../../lib/commandError';
import {
  deleteV2StickyNote,
  listV2StickyNotes,
  saveV2StickyNote,
  type V2StickyNote,
  type V2StickyNoteInput,
} from '../api';
import { NotesContext, type NotesContextValue, type NotesSaveState } from './notesContext';

function sortNotes(notes: V2StickyNote[]): V2StickyNote[] {
  return [...notes].sort((left, right) => {
    if (left.isPinned !== right.isPinned) {
      return left.isPinned ? -1 : 1;
    }
    return right.updatedAt.localeCompare(left.updatedAt) || right.id - left.id;
  });
}

function savedTime(): string {
  return new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

export function NotesProvider({ children }: { children: ReactNode }) {
  const [notes, setNotes] = useState<V2StickyNote[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [panelOpen, setPanelOpen] = useState(false);
  const [saveState, setSaveState] = useState<NotesSaveState>('idle');
  const [saveError, setSaveError] = useState<string | null>(null);
  const [lastSavedAt, setLastSavedAt] = useState<string | null>(null);
  const loadingRef = useRef(false);
  const loadedRef = useRef(false);
  const queueRef = useRef<Promise<void>>(Promise.resolve());
  const sequenceRef = useRef(0);
  const dirtyRef = useRef(false);

  const ensureLoaded = useCallback(async (force = false) => {
    if (loadingRef.current || (!force && loadedRef.current)) {
      return;
    }
    loadingRef.current = true;
    setLoading(true);
    try {
      const next = await listV2StickyNotes();
      setNotes(sortNotes(next));
      setError(null);
      loadedRef.current = true;
    } catch (loadError) {
      setError(toCommandError(loadError).message);
    } finally {
      loadingRef.current = false;
      setLoading(false);
    }
  }, []);

  const openPanel = useCallback(() => {
    setPanelOpen(true);
    void ensureLoaded();
  }, [ensureLoaded]);

  const closePanel = useCallback(() => {
    setPanelOpen(false);
  }, []);

  const markDirty = useCallback(() => {
    dirtyRef.current = true;
    setSaveState('dirty');
    setSaveError(null);
  }, []);

  const mergeNote = useCallback((note: V2StickyNote) => {
    setNotes((current) => {
      if (!current) {
        return [note];
      }
      const exists = current.some((entry) => entry.id === note.id);
      return sortNotes(
        exists ? current.map((entry) => (entry.id === note.id ? note : entry)) : [...current, note],
      );
    });
  }, []);

  const saveNote = useCallback(
    async (input: V2StickyNoteInput): Promise<V2StickyNote> => {
      const sequence = ++sequenceRef.current;
      dirtyRef.current = false;
      setSaveState('saving');
      setSaveError(null);
      let result: V2StickyNote | null = null;
      let failure: unknown = null;
      const operation = queueRef.current.then(async () => {
        try {
          result = await saveV2StickyNote(input);
          if (sequence === sequenceRef.current) {
            mergeNote(result);
          }
        } catch (saveFailure) {
          failure = saveFailure;
        }
      });
      queueRef.current = operation;
      await operation;
      if (failure) {
        if (sequence === sequenceRef.current) {
          dirtyRef.current = true;
          setSaveState('error');
          setSaveError(toCommandError(failure).message);
        }
        throw failure;
      }
      if (sequence === sequenceRef.current) {
        if (dirtyRef.current) {
          setSaveState('dirty');
        } else {
          setSaveState('saved');
          setLastSavedAt(savedTime());
        }
      }
      if (!result) {
        throw new Error('The sticky note save returned no result.');
      }
      return result;
    },
    [mergeNote],
  );

  const deleteNote = useCallback(async (id: number) => {
    const sequence = ++sequenceRef.current;
    let failure: unknown = null;
    const operation = queueRef.current.then(async () => {
      try {
        await deleteV2StickyNote(id);
        setNotes((current) => (current ? current.filter((note) => note.id !== id) : current));
      } catch (deleteFailure) {
        failure = deleteFailure;
      }
    });
    queueRef.current = operation;
    await operation;
    if (failure) {
      if (sequence === sequenceRef.current) {
        setSaveState('error');
        setSaveError(toCommandError(failure).message);
      }
      throw failure;
    }
    if (sequence === sequenceRef.current) {
      setSaveState('saved');
      setLastSavedAt(savedTime());
    }
  }, []);

  const value = useMemo<NotesContextValue>(
    () => ({
      notes,
      loading,
      error,
      panelOpen,
      saveState,
      saveError,
      lastSavedAt,
      openPanel,
      closePanel,
      ensureLoaded,
      markDirty,
      saveNote,
      deleteNote,
    }),
    [
      notes,
      loading,
      error,
      panelOpen,
      saveState,
      saveError,
      lastSavedAt,
      openPanel,
      closePanel,
      ensureLoaded,
      markDirty,
      saveNote,
      deleteNote,
    ],
  );

  return <NotesContext.Provider value={value}>{children}</NotesContext.Provider>;
}
