import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { V2StickyNote } from '../api';
import { NoteCard } from './NoteCard';

function note(overrides: Partial<V2StickyNote> = {}): V2StickyNote {
  return {
    id: 1,
    title: 'Study note',
    body: 'Remember the identity.',
    color: 'blue',
    isPinned: false,
    examId: null,
    examCode: null,
    subjectNodeId: null,
    subjectTitle: null,
    topicNodeId: null,
    topicTitle: null,
    createdAt: '2026-11-10T10:00:00Z',
    updatedAt: '2026-11-10T10:00:00Z',
    ...overrides,
  };
}

describe('NoteCard mathematical content', () => {
  it('renders explicit math in note bodies without changing plain text', async () => {
    render(
      <NoteCard
        note={note({ body: String.raw`Use $E=mc^2$ and the plain identity.` })}
        onOpen={vi.fn()}
      />,
    );

    await waitFor(() => expect(document.querySelector('.v2-note-card-body .katex')).not.toBeNull());
    expect(screen.getByText(/and the plain identity/)).toBeInTheDocument();
  });

  it('preserves ordinary note text when no math is present', () => {
    render(<NoteCard note={note()} onOpen={vi.fn()} />);
    expect(screen.getByText('Remember the identity.')).toBeInTheDocument();
    expect(document.querySelector('.katex')).toBeNull();
  });
});
