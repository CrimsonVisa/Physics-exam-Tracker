import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useCallback, useState } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2ExamSummary, V2StickyNote, V2StickyNoteInput } from '../api';
import { NoteEditor } from './NoteEditor';
import { NotesContext, type NotesContextValue, type NotesSaveState } from './notesContext';

vi.mock('../api', () => ({
  getV2ExamSyllabus: vi.fn(),
}));

import * as api from '../api';

const mockedSyllabus = vi.mocked(api.getV2ExamSyllabus);

const EXAMS: V2ExamSummary[] = [
  {
    id: 4,
    code: 'jest-physics',
    name: 'JEST Physics',
    shortName: 'JEST',
    authority: 'JEST Consortium',
    officialUrl: 'https://example.com/jest',
    displayOrder: 4,
    isActive: true,
    examDate: null,
  },
];

function note(overrides: Partial<V2StickyNote> = {}): V2StickyNote {
  return {
    id: 9,
    title: 'Saved note',
    body: 'Saved body',
    color: 'yellow',
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

function context(saveNote: NotesContextValue['saveNote']): NotesContextValue {
  return {
    notes: [],
    loading: false,
    error: null,
    panelOpen: false,
    saveState: 'idle',
    saveError: null,
    lastSavedAt: null,
    openPanel: vi.fn(),
    closePanel: vi.fn(),
    ensureLoaded: vi.fn().mockResolvedValue(undefined),
    markDirty: vi.fn(),
    saveNote,
    deleteNote: vi.fn().mockResolvedValue(undefined),
  };
}

function renderEditor(saveNote: NotesContextValue['saveNote'], onClose = vi.fn()) {
  render(
    <NotesContext.Provider value={context(saveNote)}>
      <NoteEditor exams={EXAMS} onClose={onClose} onSaved={vi.fn()} />
    </NotesContext.Provider>,
  );
  return { onClose };
}

function StatefulEditor({
  saveNote,
  onSaved,
}: {
  saveNote: NotesContextValue['saveNote'];
  onSaved: (note: V2StickyNote) => void;
}) {
  const [saveState, setSaveState] = useState<NotesSaveState>('idle');
  const [saveError, setSaveError] = useState<string | null>(null);
  const [lastSavedAt, setLastSavedAt] = useState<string | null>(null);

  const wrappedSave = useCallback(
    async (input: V2StickyNoteInput) => {
      setSaveState('saving');
      setSaveError(null);
      try {
        const saved = await saveNote(input);
        setSaveState('saved');
        setLastSavedAt('21:00');
        return saved;
      } catch (error) {
        const message =
          error && typeof error === 'object' && 'message' in error
            ? String((error as { message?: unknown }).message)
            : 'Save failed.';
        setSaveState('error');
        setSaveError(message);
        throw error;
      }
    },
    [saveNote],
  );

  return (
    <NotesContext.Provider
      value={{
        ...context(wrappedSave),
        saveState,
        saveError,
        lastSavedAt,
      }}
    >
      <NoteEditor exams={EXAMS} onClose={vi.fn()} onSaved={onSaved} />
    </NotesContext.Provider>
  );
}

describe('NoteEditor', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedSyllabus.mockResolvedValue({
      examId: 4,
      examCode: 'jest-physics',
      examName: 'JEST Physics',
      examIsActive: true,
      configured: true,
      contentVersionLabel: 'JEST 2026 Physics Syllabus',
      contentReviewStatus: 'verified',
      summary: {
        nodeCount: 49,
        topicCount: 42,
        revisedOrConfidentCount: 0,
        mappedConceptCount: 0,
      },
      nodes: [
        {
          id: 10,
          examId: 4,
          parentId: null,
          code: 'jest-2026-mathematical-methods',
          title: 'Mathematical Methods',
          nodeKind: 'unit',
          category: 'JEST 2026 Physics Syllabus',
          displayOrder: 1,
          readinessWeight: 1,
          isArchived: false,
          sourceTitle: 'JEST source',
          sourceVerifiedOn: '2026-11-10',
          sourceUrl: null,
          sourceReference: 'source',
          contentVersionLabel: 'JEST 2026 Physics Syllabus',
          contentReviewStatus: 'verified',
          progressStatus: 'not_started',
          priority: 'normal',
          confidence: 3,
          notes: '',
          nodePath: ['JEST 2026 Physics Syllabus', 'Mathematical Methods'],
        },
        {
          id: 11,
          examId: 4,
          parentId: 10,
          code: 'jest-2026-vector-calculus',
          title: 'Vector calculus',
          nodeKind: 'topic',
          category: 'Mathematical Methods',
          displayOrder: 1,
          readinessWeight: 1,
          isArchived: false,
          sourceTitle: 'JEST source',
          sourceVerifiedOn: '2026-11-10',
          sourceUrl: null,
          sourceReference: 'source',
          contentVersionLabel: 'JEST 2026 Physics Syllabus',
          contentReviewStatus: 'verified',
          progressStatus: 'not_started',
          priority: 'normal',
          confidence: 3,
          notes: '',
          nodePath: ['JEST 2026 Physics Syllabus', 'Mathematical Methods', 'Vector calculus'],
        },
      ],
    });
  });

  it('autosaves a new note after the debounce', async () => {
    const user = userEvent.setup();
    const saveNote = vi.fn().mockResolvedValue(note());
    renderEditor(saveNote);

    await user.type(screen.getByLabelText('Title'), 'New note');
    await user.type(screen.getByLabelText('Note'), 'Autosaved body');

    await waitFor(
      () => {
        const input = saveNote.mock.calls.at(-1)?.[0] as V2StickyNoteInput | undefined;
        expect(input).toMatchObject({
          title: 'New note',
          body: 'Autosaved body',
          color: 'yellow',
          isPinned: false,
        });
      },
      { timeout: 3000 },
    );
  });

  it('flushes the draft immediately when a field loses focus', async () => {
    const user = userEvent.setup();
    const saveNote = vi.fn().mockResolvedValue(note());
    renderEditor(saveNote);

    await user.type(screen.getByLabelText('Title'), 'Blurred note');
    await user.tab();

    await waitFor(() => expect(saveNote).toHaveBeenCalledTimes(1), { timeout: 2000 });
    expect(saveNote.mock.calls[0][0]).toMatchObject({ title: 'Blurred note' });
  });

  it('flushes the draft when the editor is closed', async () => {
    const user = userEvent.setup();
    const saveNote = vi.fn().mockResolvedValue(note());
    const { onClose } = renderEditor(saveNote);

    await user.type(screen.getByLabelText('Title'), 'Closing note');
    await user.click(screen.getByRole('button', { name: 'Close note editor' }));

    expect(onClose).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(saveNote).toHaveBeenCalledTimes(1), { timeout: 2000 });
    expect(saveNote.mock.calls[0][0]).toMatchObject({ title: 'Closing note' });
  });

  it('loads exam subjects and topics for associations', async () => {
    const user = userEvent.setup();
    renderEditor(vi.fn().mockResolvedValue(note()));

    await user.selectOptions(screen.getByLabelText('Note examination'), '4');
    await waitFor(() => expect(mockedSyllabus).toHaveBeenCalledWith(4));
    await screen.findByRole('option', { name: 'Mathematical Methods' });
    await user.selectOptions(screen.getByLabelText('Note subject'), '10');
    expect(screen.getByRole('option', { name: 'Vector calculus' })).toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText('Note topic'), '11');
    expect(screen.getByLabelText('Note topic')).toHaveValue('11');
  });

  it('shows exactly one save alert and retries successfully', async () => {
    const user = userEvent.setup();
    const onSaved = vi.fn();
    const saveNote = vi
      .fn()
      .mockRejectedValueOnce({
        code: 'validation_error',
        field: 'title',
        message: 'Note titles must be 200 characters or fewer.',
      })
      .mockResolvedValueOnce(note({ id: 12, title: 'Retry note' }));

    render(<StatefulEditor saveNote={saveNote} onSaved={onSaved} />);
    await user.type(screen.getByLabelText('Title'), 'Retry note');

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Note titles must be 200 characters or fewer.');
    expect(screen.getAllByRole('alert')).toHaveLength(1);

    await user.click(screen.getByRole('button', { name: 'Retry' }));
    await waitFor(() => expect(onSaved).toHaveBeenCalledTimes(1));
    expect(saveNote).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByText('Saved at 21:00')).toBeInTheDocument();
  });

  it('keeps the note visible when a save fails', async () => {
    const user = userEvent.setup();
    const saveNote = vi
      .fn()
      .mockRejectedValue({ code: 'database_error', message: 'Cannot save right now.' });
    renderEditor(saveNote);

    await user.type(screen.getByLabelText('Title'), 'Failing note');
    await waitFor(() => expect(saveNote).toHaveBeenCalledTimes(1), { timeout: 3000 });
    expect(screen.getByLabelText('Title')).toHaveValue('Failing note');
  });
});
