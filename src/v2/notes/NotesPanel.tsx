import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { V2ExamSummary, V2StickyNote } from '../api';
import { DialogShell } from '../DialogShell';
import { NoteCard } from './NoteCard';
import { NoteEditor } from './NoteEditor';
import { useNotes } from './notesContext';
import { isCompactNotesWidth } from './notesLayout';

type NotesPanelProps = {
  exams: V2ExamSummary[];
  selectedNoteId?: number | null;
  onSelectNote?: (noteId: number | null) => void;
  onOpenFullPage?: () => void;
};

type DraftSession = {
  key: number;
  savedId: number | null;
};

function useCompactNotesLayout(): boolean {
  const [compact, setCompact] = useState(() =>
    typeof window === 'undefined' ? false : isCompactNotesWidth(window.innerWidth),
  );

  useEffect(() => {
    const onResize = () => setCompact(isCompactNotesWidth(window.innerWidth));
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  return compact;
}

function mostRecentlyUpdated(notes: V2StickyNote[]): V2StickyNote | null {
  return notes.reduce<V2StickyNote | null>((latest, note) => {
    if (!latest) {
      return note;
    }
    const order = note.updatedAt.localeCompare(latest.updatedAt);
    return order > 0 || (order === 0 && note.id > latest.id) ? note : latest;
  }, null);
}

export function NotesPanel({
  exams,
  selectedNoteId,
  onSelectNote,
  onOpenFullPage,
}: NotesPanelProps) {
  const { notes, loading, error, deleteNote, closePanel } = useNotes();
  const [query, setQuery] = useState('');
  const [draft, setDraft] = useState<DraftSession | null>(null);
  const [localSelectedNoteId, setLocalSelectedNoteId] = useState<number | null>(null);
  const [listOpen, setListOpen] = useState(false);
  const compactLayout = useCompactNotesLayout();
  const searchRef = useRef<HTMLInputElement>(null);
  const drawerToggleRef = useRef<HTMLButtonElement>(null);
  const focusTimerRef = useRef<number | null>(null);
  const newNoteSequenceRef = useRef(0);

  const filtered = useMemo(() => {
    const term = query.trim().toLowerCase();
    return (notes ?? []).filter((note) => {
      if (!term) {
        return true;
      }
      return [
        note.title,
        note.body,
        note.color,
        note.examCode ?? '',
        note.subjectTitle ?? '',
        note.topicTitle ?? '',
      ].some((value) => value.toLowerCase().includes(term));
    });
  }, [notes, query]);

  const activeSelectedNoteId = selectedNoteId === undefined ? localSelectedNoteId : selectedNoteId;

  const selectedNote = useMemo(() => {
    if (draft) {
      return draft.savedId === null
        ? null
        : ((notes ?? []).find((note) => note.id === draft.savedId) ?? null);
    }
    if (notes === null) {
      return null;
    }
    const preferred =
      activeSelectedNoteId === null
        ? null
        : (notes.find((note) => note.id === activeSelectedNoteId) ?? null);
    return preferred ?? mostRecentlyUpdated(notes);
  }, [activeSelectedNoteId, draft, notes]);

  useEffect(() => {
    if (notes === null || draft) {
      return;
    }
    const preferred =
      activeSelectedNoteId === null
        ? null
        : (notes.find((note) => note.id === activeSelectedNoteId) ?? null);
    const target = preferred ?? mostRecentlyUpdated(notes);
    if (target) {
      if (activeSelectedNoteId !== target.id) {
        onSelectNote?.(target.id);
      }
    } else if (activeSelectedNoteId !== null) {
      onSelectNote?.(null);
    }
  }, [activeSelectedNoteId, draft, notes, onSelectNote]);

  useEffect(() => {
    return () => {
      if (focusTimerRef.current !== null) {
        window.clearTimeout(focusTimerRef.current);
      }
    };
  }, []);

  const selectNote = useCallback(
    (next: V2StickyNote) => {
      setDraft(null);
      if (selectedNoteId === undefined) {
        setLocalSelectedNoteId(next.id);
      }
      onSelectNote?.(next.id);
      setListOpen(false);
      if (compactLayout) {
        window.setTimeout(() => drawerToggleRef.current?.focus(), 0);
      }
    },
    [compactLayout, onSelectNote, selectedNoteId],
  );

  const beginNewNote = useCallback(() => {
    newNoteSequenceRef.current += 1;
    setDraft({ key: newNoteSequenceRef.current, savedId: null });
    if (selectedNoteId === undefined) {
      setLocalSelectedNoteId(null);
    }
    onSelectNote?.(null);
    setListOpen(false);
  }, [onSelectNote, selectedNoteId]);

  const focusSearch = useCallback(() => {
    setListOpen(true);
    if (focusTimerRef.current !== null) {
      window.clearTimeout(focusTimerRef.current);
    }
    focusTimerRef.current = window.setTimeout(() => {
      focusTimerRef.current = null;
      searchRef.current?.focus();
      searchRef.current?.select();
    }, 0);
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((!event.ctrlKey && !event.metaKey) || event.altKey) {
        return;
      }
      const key = event.key.toLowerCase();
      if (key === 'n') {
        event.preventDefault();
        beginNewNote();
      } else if (key === 'f') {
        event.preventDefault();
        focusSearch();
      }
    };

    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [beginNewNote, focusSearch]);

  const handleSaved = useCallback(
    (saved: V2StickyNote) => {
      setDraft((current) => (current ? { ...current, savedId: saved.id } : current));
      if (selectedNoteId === undefined) {
        setLocalSelectedNoteId(saved.id);
      }
      onSelectNote?.(saved.id);
    },
    [onSelectNote, selectedNoteId],
  );

  async function remove(note: V2StickyNote) {
    if (!window.confirm(`Delete the sticky note "${note.title || 'Untitled note'}"?`)) {
      return;
    }
    try {
      await deleteNote(note.id);
      setDraft((current) => (current?.savedId === note.id ? null : current));
      if (selectedNoteId === undefined && localSelectedNoteId === note.id) {
        setLocalSelectedNoteId(null);
      }
    } catch {
      // The provider exposes the delete error through the shared error state.
    }
  }

  const editorKey = draft ? `new-${draft.key}` : selectedNote ? `note-${selectedNote.id}` : null;

  const hasNotes = notes !== null && notes.length > 0;
  const countLabel =
    notes === null
      ? 'Loading'
      : `${filtered.length}${query.trim() ? ` of ${notes.length}` : ''} ${
          notes.length === 1 ? 'note' : 'notes'
        }`;

  return (
    <DialogShell ariaLabel="Sticky notes" className="v2-notes-dialog" onClose={closePanel}>
      <div className="v2-notes-panel" data-layout={compactLayout ? 'compact' : 'split'}>
        <header className="v2-notes-quick-header">
          <div className="v2-notes-quick-heading">
            <h2>Sticky notes</h2>
            <span className="v2-notes-quick-count">{countLabel}</span>
          </div>
          <div className="v2-notes-quick-actions">
            {onOpenFullPage ? (
              <button
                className="secondary-button v2-notes-expand"
                onClick={onOpenFullPage}
                type="button"
              >
                Open full page
              </button>
            ) : null}
            <button
              aria-label="Close sticky notes"
              className="v2-notes-icon-button"
              onClick={closePanel}
              type="button"
            >
              <span aria-hidden="true">×</span>
            </button>
          </div>
        </header>

        <div className="v2-notes-quick-body">
          <aside
            aria-label="Notes navigation"
            className={`v2-notes-quick-nav${listOpen ? ' is-open' : ''}`}
            id="v2-notes-quick-nav"
          >
            <label className="v2-field v2-notes-quick-search">
              <span className="sr-only">Search notes</span>
              <input
                aria-label="Search sticky notes"
                data-autofocus={compactLayout ? undefined : true}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="Search notes"
                ref={searchRef}
                type="search"
                value={query}
              />
            </label>
            <button className="primary-button v2-notes-new" onClick={beginNewNote} type="button">
              New note
            </button>

            {error ? (
              <p className="v2-inline-error" role="alert">
                {error}
              </p>
            ) : null}

            <div className="v2-notes-quick-list">
              {loading && notes === null ? (
                <p className="v2-empty-copy">Loading notes…</p>
              ) : notes !== null && notes.length === 0 ? (
                <p className="v2-notes-quick-list-empty">No notes yet.</p>
              ) : filtered.length === 0 ? (
                <p className="v2-notes-quick-list-empty">No notes match this search.</p>
              ) : (
                <ul className="v2-note-list v2-note-list-compact">
                  {filtered.map((note) => (
                    <li key={note.id}>
                      <NoteCard
                        compact
                        note={note}
                        onOpen={() => selectNote(note)}
                        selected={selectedNote?.id === note.id}
                      />
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </aside>

          <section aria-label="Note editor" className="v2-notes-quick-editor">
            <div className="v2-notes-quick-mobilebar">
              <button
                aria-controls="v2-notes-quick-nav"
                aria-expanded={listOpen}
                aria-label={listOpen ? 'Hide notes list' : 'Show notes list'}
                className="secondary-button v2-notes-drawer-toggle"
                data-autofocus={compactLayout ? true : undefined}
                onClick={() => setListOpen((current) => !current)}
                ref={drawerToggleRef}
                type="button"
              >
                Notes
                {hasNotes ? <span className="v2-notes-drawer-count">{notes.length}</span> : null}
              </button>
              {selectedNote ? (
                <span className="v2-notes-quick-selected-caption">
                  {selectedNote.title.trim() || 'Untitled note'}
                </span>
              ) : (
                <span className="v2-notes-quick-selected-caption">
                  {draft ? 'New note' : 'No note selected'}
                </span>
              )}
            </div>

            {editorKey ? (
              <NoteEditor
                compact
                exams={exams}
                key={editorKey}
                note={selectedNote}
                onClose={() => setDraft(null)}
                onDelete={selectedNote ? remove : undefined}
                onSaved={handleSaved}
              />
            ) : notes !== null && notes.length === 0 ? (
              <div className="v2-notes-quick-empty">
                <p className="eyebrow">Quick capture</p>
                <h3>Nothing here yet</h3>
                <p>Create a note for a thought, formula reminder or study task. It autosaves.</p>
                <button className="primary-button" onClick={beginNewNote} type="button">
                  Create note
                </button>
              </div>
            ) : (
              <div className="v2-notes-quick-empty v2-notes-quick-empty-muted">
                <p className="eyebrow">Quick capture</p>
                <h3>{loading && notes === null ? 'Loading notes…' : 'Select a note'}</h3>
                <p>Choose a note from the list or create a new one.</p>
              </div>
            )}
          </section>
        </div>
      </div>
    </DialogShell>
  );
}
