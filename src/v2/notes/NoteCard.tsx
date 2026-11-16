import type { V2StickyNote } from '../api';
import { MathText } from '../MathText';
import { noteColorLabel } from './noteColors';

type NoteCardProps = {
  note: V2StickyNote;
  onOpen: () => void;
  compact?: boolean;
  selected?: boolean;
};

function noteLabel(note: V2StickyNote): string {
  return note.title.trim() || note.body.trim().slice(0, 80) || 'Untitled note';
}

export function NoteCard({ note, onOpen, compact = false, selected = false }: NoteCardProps) {
  if (compact) {
    const title = note.title.trim();
    const displayTitle = title || note.body.trim().slice(0, 80) || 'Untitled note';
    return (
      <button
        aria-current={selected ? 'true' : undefined}
        aria-label={`Open sticky note ${noteLabel(note)}${note.isPinned ? ' (pinned)' : ''}`}
        className="v2-note-card v2-note-card-compact"
        data-color={note.color}
        data-selected={selected ? 'true' : undefined}
        onClick={onOpen}
        type="button"
      >
        <span className="v2-note-card-topline">
          <span className="v2-note-card-title">{displayTitle}</span>
          {note.isPinned ? (
            <span className="v2-note-card-pin" title="Pinned">
              <span aria-hidden="true" className="v2-note-card-pin-dot" />
              <span className="sr-only">Pinned</span>
            </span>
          ) : null}
        </span>
        {title && note.body.trim() ? (
          <span className="v2-note-card-body">
            <MathText text={note.body} />
          </span>
        ) : null}
      </button>
    );
  }

  return (
    <button
      aria-label={`Open sticky note ${noteLabel(note)}`}
      className="v2-note-card"
      data-color={note.color}
      onClick={onOpen}
      type="button"
    >
      <span className="v2-note-card-topline">
        <span className="v2-note-card-title">{noteLabel(note)}</span>
        {note.isPinned ? (
          <span className="v2-reference-badge" data-tone="user">
            Pinned
          </span>
        ) : null}
      </span>
      {note.title.trim() && note.body.trim() ? (
        <span className="v2-note-card-body">
          <MathText text={note.body} />
        </span>
      ) : null}
      <span className="v2-note-card-meta">
        <span className="v2-reference-badge" data-tone="category">
          {noteColorLabel(note.color)}
        </span>
        {note.examCode ? (
          <span className="v2-reference-badge" data-tone="muted">
            {note.examCode}
          </span>
        ) : null}
        {note.subjectTitle ? (
          <span className="v2-reference-badge" data-tone="muted">
            {note.subjectTitle}
          </span>
        ) : null}
        {note.topicTitle ? (
          <span className="v2-reference-badge" data-tone="muted">
            {note.topicTitle}
          </span>
        ) : null}
      </span>
      <span className="v2-note-card-date">{note.updatedAt}</span>
    </button>
  );
}
