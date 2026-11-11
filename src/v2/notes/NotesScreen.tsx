import { useEffect, useMemo, useRef, useState } from 'react';
import type { V2ExamSummary, V2StickyNote } from '../api';
import { NoteCard } from './NoteCard';
import { NoteEditor } from './NoteEditor';
import { NOTE_COLORS } from './noteColors';
import { useNotes } from './notesContext';

type NotesScreenProps = {
  exams: V2ExamSummary[];
  initialSelectedNoteId?: number | null;
  onSelectNote?: (noteId: number | null) => void;
};

export function NotesScreen({
  exams,
  initialSelectedNoteId = null,
  onSelectNote,
}: NotesScreenProps) {
  const { notes, loading, error, ensureLoaded, deleteNote } = useNotes();
  const [query, setQuery] = useState('');
  const [pinnedOnly, setPinnedOnly] = useState(false);
  const [color, setColor] = useState('all');
  const [examCode, setExamCode] = useState('all');
  const [subjectNodeId, setSubjectNodeId] = useState('all');
  const [topicNodeId, setTopicNodeId] = useState('all');
  const [editingId, setEditingId] = useState<number | 'new' | null>(null);
  const initialSelectionRef = useRef(initialSelectedNoteId);
  const initialSelectionAppliedRef = useRef(false);

  useEffect(() => {
    void ensureLoaded();
  }, [ensureLoaded]);

  useEffect(() => {
    if (initialSelectionAppliedRef.current || notes === null) {
      return;
    }
    initialSelectionAppliedRef.current = true;
    const targetId = initialSelectionRef.current;
    if (targetId === null) {
      return;
    }
    const target = notes.find((note) => note.id === targetId);
    if (target) {
      setEditingId(target.id);
    }
  }, [notes]);

  const examOptions = useMemo(
    () =>
      [...new Set((notes ?? []).map((note) => note.examCode).filter(Boolean))].sort() as string[],
    [notes],
  );
  const subjectOptions = useMemo(() => {
    const values = new Map<number, string>();
    for (const note of notes ?? []) {
      if (
        note.subjectNodeId !== null &&
        note.subjectTitle &&
        (examCode === 'all' || note.examCode === examCode)
      ) {
        values.set(note.subjectNodeId, note.subjectTitle);
      }
    }
    return [...values.entries()].sort((left, right) => left[1].localeCompare(right[1]));
  }, [examCode, notes]);
  const topicOptions = useMemo(() => {
    const values = new Map<number, string>();
    for (const note of notes ?? []) {
      if (
        note.topicNodeId !== null &&
        note.topicTitle &&
        (examCode === 'all' || note.examCode === examCode) &&
        (subjectNodeId === 'all' || note.subjectNodeId === Number(subjectNodeId))
      ) {
        values.set(note.topicNodeId, note.topicTitle);
      }
    }
    return [...values.entries()].sort((left, right) => left[1].localeCompare(right[1]));
  }, [examCode, notes, subjectNodeId]);

  const filtered = useMemo(() => {
    const term = query.trim().toLowerCase();
    return (notes ?? []).filter((note) => {
      if (pinnedOnly && !note.isPinned) {
        return false;
      }
      if (color !== 'all' && note.color !== color) {
        return false;
      }
      if (examCode !== 'all' && note.examCode !== examCode) {
        return false;
      }
      if (subjectNodeId !== 'all' && note.subjectNodeId !== Number(subjectNodeId)) {
        return false;
      }
      if (topicNodeId !== 'all' && note.topicNodeId !== Number(topicNodeId)) {
        return false;
      }
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
  }, [color, examCode, notes, pinnedOnly, query, subjectNodeId, topicNodeId]);

  const editingNote =
    editingId === null || editingId === 'new'
      ? null
      : ((notes ?? []).find((note) => note.id === editingId) ?? null);

  function clearFilters() {
    setQuery('');
    setPinnedOnly(false);
    setColor('all');
    setExamCode('all');
    setSubjectNodeId('all');
    setTopicNodeId('all');
  }

  async function remove(note: V2StickyNote) {
    if (!window.confirm(`Delete the sticky note "${note.title || 'Untitled note'}"?`)) {
      return;
    }
    try {
      await deleteNote(note.id);
      if (editingId === note.id) {
        setEditingId(null);
        onSelectNote?.(null);
      }
    } catch {
      // The provider exposes the delete error through the shared error state.
    }
  }

  function beginNewNote() {
    setEditingId('new');
    onSelectNote?.(null);
  }

  function selectNote(note: V2StickyNote) {
    setEditingId(note.id);
    onSelectNote?.(note.id);
  }

  return (
    <div className="page-container v2-notes-page">
      <header className="page-header">
        <p className="eyebrow">Study workspace</p>
        <h1>Sticky notes</h1>
        <p className="page-lede">
          Persistent notes with optional exam, subject and topic associations. Notes autosave and
          remain available across navigation and restarts.
        </p>
      </header>

      <section aria-label="Sticky note filters" className="v2-notes-toolbar">
        <label className="v2-field v2-notes-search">
          <span>Search notes</span>
          <input
            aria-label="Search notes"
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Title, text, exam, subject or topic"
            type="search"
            value={query}
          />
        </label>
        <label className="v2-field">
          <span>Color</span>
          <select
            aria-label="Note color filter"
            onChange={(event) => setColor(event.target.value)}
            value={color}
          >
            <option value="all">All colors</option>
            {NOTE_COLORS.map((entry) => (
              <option key={entry.id} value={entry.id}>
                {entry.label}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Examination</span>
          <select
            aria-label="Note exam filter"
            onChange={(event) => {
              setExamCode(event.target.value);
              setSubjectNodeId('all');
              setTopicNodeId('all');
            }}
            value={examCode}
          >
            <option value="all">All examinations</option>
            {examOptions.map((code) => (
              <option key={code} value={code}>
                {exams.find((exam) => exam.code === code)?.shortName ?? code}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Subject</span>
          <select
            aria-label="Note subject filter"
            onChange={(event) => {
              setSubjectNodeId(event.target.value);
              setTopicNodeId('all');
            }}
            value={subjectNodeId}
          >
            <option value="all">All subjects</option>
            {subjectOptions.map(([id, title]) => (
              <option key={id} value={id}>
                {title}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Topic</span>
          <select
            aria-label="Note topic filter"
            onChange={(event) => setTopicNodeId(event.target.value)}
            value={topicNodeId}
          >
            <option value="all">All topics</option>
            {topicOptions.map(([id, title]) => (
              <option key={id} value={id}>
                {title}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-checkbox-field v2-note-pinned-filter">
          <input
            checked={pinnedOnly}
            onChange={(event) => setPinnedOnly(event.target.checked)}
            type="checkbox"
          />
          <span>Pinned only</span>
        </label>
        <button className="secondary-button" onClick={clearFilters} type="button">
          Clear filters
        </button>
      </section>

      <div className="v2-notes-page-actions">
        <p aria-live="polite" className="v2-reference-count">
          {notes === null
            ? 'Loading notes…'
            : `${filtered.length} note${filtered.length === 1 ? '' : 's'}`}
        </p>
        <div className="v2-form-actions">
          <button
            className="secondary-button"
            onClick={() => void ensureLoaded(true)}
            type="button"
          >
            Refresh
          </button>
          <button className="primary-button" onClick={beginNewNote} type="button">
            New note
          </button>
        </div>
      </div>

      {error ? (
        <p className="v2-inline-error" role="alert">
          {error}
        </p>
      ) : null}

      <div className="v2-notes-page-layout">
        <section aria-label="Notes" className="v2-notes-list-panel">
          {loading && notes === null ? (
            <p className="v2-empty-copy">Loading notes…</p>
          ) : notes !== null && notes.length === 0 ? (
            <div className="v2-empty-panel v2-notes-empty">
              <h2>No sticky notes yet</h2>
              <p>Create a note for a quick reminder, derivation idea or study task.</p>
              <button className="primary-button" onClick={beginNewNote} type="button">
                Create your first note
              </button>
            </div>
          ) : filtered.length === 0 ? (
            <div className="v2-empty-panel v2-notes-empty">
              <h2>No notes match these filters</h2>
              <p>Try a different search, color or syllabus association.</p>
              <button className="secondary-button" onClick={clearFilters} type="button">
                Clear filters
              </button>
            </div>
          ) : (
            <ul className="v2-note-list">
              {filtered.map((note) => (
                <li key={note.id}>
                  <NoteCard note={note} onOpen={() => selectNote(note)} />
                </li>
              ))}
            </ul>
          )}
        </section>

        <aside aria-label="Note editor" className="v2-notes-editor-panel">
          {editingId === 'new' || editingNote ? (
            <NoteEditor
              exams={exams}
              key={editingId === 'new' ? 'new' : editingId}
              note={editingId === 'new' ? null : editingNote}
              onClose={() => {
                setEditingId(null);
                onSelectNote?.(null);
              }}
              onDelete={editingId === 'new' ? undefined : remove}
              onSaved={(saved) => {
                setEditingId(saved.id);
                onSelectNote?.(saved.id);
              }}
            />
          ) : (
            <div className="v2-empty-panel v2-notes-empty">
              <h2>Select a note</h2>
              <p>Choose a note from the list or create a new one. Edits autosave.</p>
            </div>
          )}
        </aside>
      </div>
    </div>
  );
}
