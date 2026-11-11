import { useEffect, useRef, useState } from 'react';
import { toCommandError } from '../../lib/commandError';
import {
  getV2ExamSyllabus,
  type V2ExamSummary,
  type V2StickyNote,
  type V2StickyNoteInput,
  type V2SyllabusNode,
} from '../api';
import { useNotes } from './notesContext';
import { NOTE_COLORS, type NoteColor } from './noteColors';

type NoteEditorProps = {
  exams: V2ExamSummary[];
  note?: V2StickyNote | null;
  compact?: boolean;
  onClose: () => void;
  onSaved?: (note: V2StickyNote) => void;
  onDelete?: (note: V2StickyNote) => void;
};

type Draft = {
  id: number | null;
  title: string;
  body: string;
  color: NoteColor;
  isPinned: boolean;
  examId: number | null;
  subjectNodeId: number | null;
  topicNodeId: number | null;
};

function draftFrom(note: V2StickyNote | null | undefined): Draft {
  return {
    id: note?.id ?? null,
    title: note?.title ?? '',
    body: note?.body ?? '',
    color: (note?.color as NoteColor) ?? 'yellow',
    isPinned: note?.isPinned ?? false,
    examId: note?.examId ?? null,
    subjectNodeId: note?.subjectNodeId ?? null,
    topicNodeId: note?.topicNodeId ?? null,
  };
}

export function NoteEditor({
  exams,
  note,
  compact = false,
  onClose,
  onSaved,
  onDelete,
}: NoteEditorProps) {
  const { markDirty, saveNote, saveState, saveError, lastSavedAt } = useNotes();
  const [draft, setDraft] = useState<Draft>(() => draftFrom(note));
  const [syllabus, setSyllabus] = useState<{
    examId: number | null;
    nodes: V2SyllabusNode[];
  }>({ examId: null, nodes: [] });
  const [loadError, setLoadError] = useState<string | null>(null);
  const [localError, setLocalError] = useState<string | null>(null);
  const timerRef = useRef<number | null>(null);
  const draftRef = useRef(draft);
  const dirtyRef = useRef(false);
  const savingRef = useRef(false);
  const pendingRef = useRef(false);
  const changeSeqRef = useRef(0);
  const mountedRef = useRef(true);

  draftRef.current = draft;

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      if (timerRef.current !== null) {
        window.clearTimeout(timerRef.current);
        timerRef.current = null;
      }
      if (dirtyRef.current && !savingRef.current) {
        const current = draftRef.current;
        if (current.title.trim() || current.body.trim()) {
          void saveNote(toInput(current)).catch(() => undefined);
        }
      }
    };
  }, [saveNote]);

  useEffect(() => {
    const examId = draft.examId;
    if (examId === null) {
      return;
    }
    let cancelled = false;
    getV2ExamSyllabus(examId)
      .then((next) => {
        if (!cancelled) {
          setSyllabus({ examId, nodes: next.nodes });
          setLoadError(null);
        }
      })
      .catch((error) => {
        if (!cancelled) {
          setLoadError(toCommandError(error).message);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [draft.examId]);

  const syllabusNodes = syllabus.examId === draft.examId ? syllabus.nodes : [];

  function toInput(current: Draft): V2StickyNoteInput {
    return {
      id: current.id,
      title: current.title,
      body: current.body,
      color: current.color,
      isPinned: current.isPinned,
      examId: current.examId,
      subjectNodeId: current.subjectNodeId,
      topicNodeId: current.topicNodeId,
    };
  }

  function update<K extends keyof Draft>(key: K, value: Draft[K]) {
    setDraft((current) => ({ ...current, [key]: value }));
    changeSeqRef.current += 1;
    dirtyRef.current = true;
    markDirty();
    setLocalError(null);
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
    }
    timerRef.current = window.setTimeout(() => {
      timerRef.current = null;
      void persist();
    }, 800);
  }

  function changeExam(value: string) {
    setDraft((current) => ({
      ...current,
      examId: value ? Number(value) : null,
      subjectNodeId: null,
      topicNodeId: null,
    }));
    changeSeqRef.current += 1;
    dirtyRef.current = true;
    markDirty();
    setLocalError(null);
  }

  async function persist() {
    if (savingRef.current) {
      pendingRef.current = true;
      return;
    }
    const current = draftRef.current;
    if (!current.title.trim() && !current.body.trim()) {
      setLocalError('Enter a title or some note text before saving.');
      return;
    }
    savingRef.current = true;
    const editSequence = changeSeqRef.current;
    try {
      const saved = await saveNote(toInput(current));
      if (editSequence === changeSeqRef.current) {
        dirtyRef.current = false;
      }
      if (mountedRef.current) {
        setDraft((existing) => ({ ...existing, id: saved.id }));
        setLocalError(null);
        onSaved?.(saved);
      }
    } catch {
      // The provider owns backend save-error presentation. Rendering the
      // command message here as well would duplicate the same alert.
    } finally {
      savingRef.current = false;
      if (pendingRef.current) {
        pendingRef.current = false;
        if (dirtyRef.current) {
          void persist();
        }
      }
    }
  }

  function flush() {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    if (dirtyRef.current) {
      void persist();
    }
  }

  function closeEditor() {
    flush();
    onClose();
  }

  const subjects = syllabusNodes.filter((node) => node.nodeKind === 'unit');
  const topics = syllabusNodes.filter(
    (node) =>
      node.nodeKind === 'topic' &&
      (draft.subjectNodeId === null || node.parentId === draft.subjectNodeId),
  );

  return (
    <section
      aria-label={draft.id === null ? 'New sticky note' : 'Edit sticky note'}
      className={`v2-note-editor${compact ? ' v2-note-editor-compact' : ''}`}
    >
      {compact ? null : (
        <header className="v2-note-editor-header">
          <div>
            <p className="eyebrow">Sticky note</p>
            <h3>{draft.id === null ? 'New note' : 'Edit note'}</h3>
          </div>
          <button
            aria-label="Close note editor"
            className="link-button"
            onClick={closeEditor}
            type="button"
          >
            Close
          </button>
        </header>
      )}

      <label className="v2-field v2-note-title-field">
        <span>Title</span>
        <input
          autoFocus={!compact || note === null}
          maxLength={200}
          onBlur={flush}
          onChange={(event) => update('title', event.target.value)}
          placeholder="Optional note title"
          value={draft.title}
        />
      </label>

      <label className="v2-field v2-note-body-field">
        <span>Note</span>
        <textarea
          maxLength={20000}
          onBlur={flush}
          onChange={(event) => update('body', event.target.value)}
          placeholder="Write your note…"
          rows={compact ? 8 : 9}
          value={draft.body}
        />
      </label>

      <div className="v2-note-editor-row">
        <fieldset className="v2-note-colors">
          <legend>Color</legend>
          {NOTE_COLORS.map((color) => (
            <button
              aria-label={`Use ${color.label.toLowerCase()} note`}
              aria-pressed={draft.color === color.id}
              className="v2-note-color"
              data-color={color.id}
              key={color.id}
              onClick={() => update('color', color.id)}
              title={color.label}
              type="button"
            >
              <span aria-hidden="true" />
            </button>
          ))}
        </fieldset>
        <button
          aria-pressed={draft.isPinned}
          className="secondary-button v2-note-pin"
          onClick={() => update('isPinned', !draft.isPinned)}
          type="button"
        >
          {draft.isPinned ? 'Pinned' : 'Pin note'}
        </button>
      </div>

      <div className="v2-note-associations">
        <label className="v2-field">
          <span>Examination</span>
          <select
            aria-label="Note examination"
            onChange={(event) => changeExam(event.target.value)}
            value={draft.examId ?? ''}
          >
            <option value="">No examination</option>
            {exams.map((exam) => (
              <option key={exam.id} value={exam.id}>
                {exam.shortName}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Subject</span>
          <select
            aria-label="Note subject"
            disabled={draft.examId === null}
            onChange={(event) => {
              const subjectNodeId = event.target.value ? Number(event.target.value) : null;
              setDraft((current) => ({
                ...current,
                subjectNodeId,
                topicNodeId:
                  current.topicNodeId !== null &&
                  syllabusNodes.find((node) => node.id === current.topicNodeId)?.parentId ===
                    subjectNodeId
                    ? current.topicNodeId
                    : null,
              }));
              changeSeqRef.current += 1;
              dirtyRef.current = true;
              markDirty();
            }}
            value={draft.subjectNodeId ?? ''}
          >
            <option value="">No subject</option>
            {subjects.map((subject) => (
              <option key={subject.id} value={subject.id}>
                {subject.title}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Topic</span>
          <select
            aria-label="Note topic"
            disabled={draft.examId === null}
            onChange={(event) => {
              update('topicNodeId', event.target.value ? Number(event.target.value) : null);
            }}
            value={draft.topicNodeId ?? ''}
          >
            <option value="">No topic</option>
            {topics.map((topic) => (
              <option key={topic.id} value={topic.id}>
                {topic.title}
              </option>
            ))}
          </select>
        </label>
      </div>

      {loadError ? (
        <p className="v2-inline-error" role="alert">
          {loadError}
        </p>
      ) : null}
      {localError ? (
        <p className="v2-inline-error" role="alert">
          {localError}
        </p>
      ) : null}
      {saveError ? (
        <p className="v2-inline-error" role="alert">
          {saveError}
        </p>
      ) : null}

      <footer className="v2-note-editor-footer">
        <span aria-live="polite" className="v2-note-save-state" data-save-state={saveState}>
          {saveState === 'saving'
            ? 'Saving…'
            : saveState === 'dirty'
              ? 'Unsaved changes'
              : saveState === 'error'
                ? 'Save failed'
                : saveState === 'saved' && lastSavedAt
                  ? `Saved at ${lastSavedAt}`
                  : 'Autosaves as you type'}
        </span>
        <div className="v2-note-editor-actions">
          {saveState === 'error' ? (
            <button className="secondary-button" onClick={() => void persist()} type="button">
              Retry
            </button>
          ) : null}
          {note && onDelete ? (
            <button
              className="secondary-button v2-danger-button"
              onClick={() => onDelete(note)}
              type="button"
            >
              Delete note
            </button>
          ) : null}
          {compact ? null : (
            <button className="primary-button" onClick={closeEditor} type="button">
              Done
            </button>
          )}
        </div>
      </footer>
    </section>
  );
}
