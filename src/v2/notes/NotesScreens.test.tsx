import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2ExamSummary, V2StickyNote, V2StickyNoteInput } from '../api';
import { NotesPanel } from './NotesPanel';
import { NotesScreen } from './NotesScreen';
import { NotesContext, type NotesContextValue } from './notesContext';
import { isCompactNotesWidth } from './notesLayout';

vi.mock('../api', () => ({
  getV2ExamSyllabus: vi.fn().mockResolvedValue({
    examId: 4,
    examCode: 'jest-physics',
    examName: 'JEST Physics',
    examIsActive: true,
    configured: true,
    contentVersionLabel: 'JEST 2026 Physics Syllabus',
    contentReviewStatus: 'verified',
    summary: {
      nodeCount: 0,
      topicCount: 0,
      revisedOrConfidentCount: 0,
      mappedConceptCount: 0,
    },
    nodes: [],
  }),
}));

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

const ORIGINAL_INNER_WIDTH = window.innerWidth;

function note(overrides: Partial<V2StickyNote> = {}): V2StickyNote {
  return {
    id: 1,
    title: 'Pinned formula note',
    body: 'Remember the vector identity.',
    color: 'blue',
    isPinned: true,
    examId: 4,
    examCode: 'jest-physics',
    subjectNodeId: 10,
    subjectTitle: 'Mathematical Methods',
    topicNodeId: 11,
    topicTitle: 'Vector calculus',
    createdAt: '2026-11-10T10:00:00Z',
    updatedAt: '2026-11-10T10:00:00Z',
    ...overrides,
  };
}

function context(
  notes: V2StickyNote[],
  overrides: Partial<NotesContextValue> = {},
): NotesContextValue {
  return {
    notes,
    loading: false,
    error: null,
    panelOpen: true,
    saveState: 'idle',
    saveError: null,
    lastSavedAt: null,
    openPanel: vi.fn(),
    closePanel: vi.fn(),
    ensureLoaded: vi.fn().mockResolvedValue(undefined),
    markDirty: vi.fn(),
    saveNote: vi.fn().mockResolvedValue(notes[0] ?? note()),
    deleteNote: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

function renderWithNotes(
  ui: ReactNode,
  notes: V2StickyNote[] = [note()],
  overrides: Partial<NotesContextValue> = {},
) {
  render(<NotesContext.Provider value={context(notes, overrides)}>{ui}</NotesContext.Provider>);
}

type PanelHostProps = {
  initialNotes: V2StickyNote[];
  initialSelectedNoteId?: number | null;
  saveNote?: NotesContextValue['saveNote'];
  onSaveNote?: (input: V2StickyNoteInput) => void;
  deleteNote?: NotesContextValue['deleteNote'];
  onOpenFullPage?: () => void;
  onSelectionChange?: (noteId: number | null) => void;
  contextOverrides?: Partial<NotesContextValue>;
};

function PanelHost({
  initialNotes,
  initialSelectedNoteId = null,
  saveNote,
  onSaveNote,
  deleteNote,
  onOpenFullPage,
  onSelectionChange,
  contextOverrides,
}: PanelHostProps) {
  const [notes, setNotes] = useState(initialNotes);
  const [selectedNoteId, setSelectedNoteId] = useState<number | null>(initialSelectedNoteId);
  const [open, setOpen] = useState(true);
  const notesRef = useRef(notes);
  const nextIdRef = useRef(100);

  useEffect(() => {
    notesRef.current = notes;
  }, [notes]);

  const handleSelection = useCallback(
    (noteId: number | null) => {
      setSelectedNoteId(noteId);
      onSelectionChange?.(noteId);
    },
    [onSelectionChange],
  );

  const defaultSave = useCallback(
    async (input: V2StickyNoteInput): Promise<V2StickyNote> => {
      onSaveNote?.(input);
      const current =
        input.id === null ? undefined : notesRef.current.find((entry) => entry.id === input.id);
      const saved = note({
        id: input.id ?? nextIdRef.current++,
        title: input.title,
        body: input.body,
        color: input.color,
        isPinned: input.isPinned,
        examId: input.examId,
        examCode: input.examId === null ? null : (current?.examCode ?? 'jest-physics'),
        subjectNodeId: input.subjectNodeId,
        subjectTitle: current?.subjectTitle ?? null,
        topicNodeId: input.topicNodeId,
        topicTitle: current?.topicTitle ?? null,
        createdAt: current?.createdAt ?? '2026-11-10T12:05:00Z',
        updatedAt: '2026-11-10T12:05:00Z',
      });
      setNotes((entries) => [saved, ...entries.filter((entry) => entry.id !== saved.id)]);
      return saved;
    },
    [onSaveNote],
  );

  const defaultDelete = useCallback(async (id: number) => {
    setNotes((entries) => entries.filter((entry) => entry.id !== id));
  }, []);

  const value: NotesContextValue = {
    ...context(notes, { closePanel: () => setOpen(false) }),
    saveNote: saveNote ?? defaultSave,
    deleteNote: deleteNote ?? defaultDelete,
    ...contextOverrides,
  };

  return (
    <NotesContext.Provider value={value}>
      {open ? (
        <NotesPanel
          exams={EXAMS}
          onOpenFullPage={onOpenFullPage}
          onSelectNote={handleSelection}
          selectedNoteId={selectedNoteId}
        />
      ) : (
        <button onClick={() => setOpen(true)} type="button">
          Reopen notes
        </button>
      )}
    </NotesContext.Provider>
  );
}

function setWindowWidth(width: number) {
  Object.defineProperty(window, 'innerWidth', {
    configurable: true,
    value: width,
    writable: true,
  });
}

describe('Notes surfaces', () => {
  beforeEach(() => {
    setWindowWidth(1200);
    vi.spyOn(window, 'confirm').mockReturnValue(true);
  });

  afterEach(() => {
    setWindowWidth(ORIGINAL_INNER_WIDTH);
    vi.restoreAllMocks();
  });

  it('opens quick access with the most recently edited note selected', async () => {
    const onSelectNote = vi.fn();
    const older = note({
      id: 1,
      title: 'Older note',
      updatedAt: '2026-11-09T10:00:00Z',
    });
    const newest = note({
      id: 2,
      title: 'Newest note',
      updatedAt: '2026-11-10T12:00:00Z',
    });

    renderWithNotes(
      <NotesPanel exams={EXAMS} onSelectNote={onSelectNote} selectedNoteId={null} />,
      [older, newest],
    );

    expect(await screen.findByLabelText('Title')).toHaveValue('Newest note');
    expect(
      screen.getByRole('button', { name: 'Open sticky note Newest note (pinned)' }),
    ).toHaveAttribute('aria-current', 'true');
    await waitFor(() => expect(onSelectNote).toHaveBeenCalledWith(2));
  });

  it('creates a note and focuses its title immediately', async () => {
    const user = userEvent.setup();
    renderWithNotes(<NotesPanel exams={EXAMS} />, [note()]);

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Pinned formula note'));
    await user.click(screen.getByRole('button', { name: 'New note' }));

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveFocus());
    expect(screen.getByLabelText('Title')).toHaveValue('');
  });

  it('creates a note from the compact empty state without an intermediate screen', async () => {
    const user = userEvent.setup();
    renderWithNotes(<NotesPanel exams={EXAMS} />, []);

    expect(screen.getByRole('heading', { name: 'Nothing here yet' })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Create note' }));

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveFocus());
    expect(screen.getByLabelText('Title')).toHaveValue('');
    expect(screen.queryByRole('heading', { name: 'Nothing here yet' })).not.toBeInTheDocument();
  });

  it('switches notes in the inline editor without opening another modal', async () => {
    const user = userEvent.setup();
    const older = note({
      id: 1,
      title: 'Older note',
      updatedAt: '2026-11-09T10:00:00Z',
    });
    const newest = note({
      id: 2,
      title: 'Newest note',
      updatedAt: '2026-11-10T12:00:00Z',
    });
    renderWithNotes(<NotesPanel exams={EXAMS} />, [older, newest]);

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Newest note'));
    await user.click(screen.getByRole('button', { name: 'Open sticky note Older note (pinned)' }));

    expect(screen.getByLabelText('Title')).toHaveValue('Older note');
    expect(
      screen.getByRole('button', { name: 'Open sticky note Older note (pinned)' }),
    ).toHaveAttribute('aria-current', 'true');
    expect(screen.getAllByRole('dialog')).toHaveLength(1);
  });

  it('filters notes immediately across title, body, exam, subject and topic', async () => {
    const user = userEvent.setup();
    renderWithNotes(<NotesPanel exams={EXAMS} />, [note()]);

    const search = screen.getByLabelText('Search sticky notes');
    for (const term of [
      'pinned',
      'vector identity',
      'jest-physics',
      'mathematical methods',
      'vector calculus',
    ]) {
      await user.clear(search);
      await user.type(search, term);
      expect(
        screen.getByRole('button', { name: 'Open sticky note Pinned formula note (pinned)' }),
      ).toBeInTheDocument();
    }

    await user.clear(search);
    await user.type(search, 'nothing matches this note');
    expect(screen.getByText('No notes match this search.')).toBeInTheDocument();
  });

  it('autosaves edits through the existing editor queue', async () => {
    const user = userEvent.setup();
    const saveNote = vi.fn().mockResolvedValue(
      note({
        body: 'Remember the vector identity. Updated body.',
      }),
    );
    renderWithNotes(<NotesPanel exams={EXAMS} />, [note()], { saveNote });

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Pinned formula note'));
    await user.type(screen.getByLabelText('Note'), ' Updated body.');

    await waitFor(
      () => {
        const input = saveNote.mock.calls.at(-1)?.[0] as V2StickyNoteInput | undefined;
        expect(input?.body).toContain('Updated body.');
      },
      { timeout: 3000 },
    );
  });

  it('pins and unpins a note through the shared save path', async () => {
    const user = userEvent.setup();
    const onSaveNote = vi.fn();
    render(
      <PanelHost
        initialNotes={[note({ id: 1, title: 'Pin toggle note', isPinned: false })]}
        onSaveNote={onSaveNote}
      />,
    );

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Pin toggle note'));
    expect(
      screen.getByRole('button', { name: 'Open sticky note Pin toggle note' }),
    ).toBeInTheDocument();
    const pinButton = screen.getByRole('button', { name: 'Pin note' });
    expect(pinButton).toHaveAttribute('aria-pressed', 'false');

    await user.click(pinButton);

    expect(pinButton).toHaveAttribute('aria-pressed', 'true');
    expect(pinButton).toHaveTextContent('Pinned');
    await waitFor(
      () => {
        expect(onSaveNote.mock.calls.at(-1)?.[0]).toMatchObject({ id: 1, isPinned: true });
      },
      { timeout: 3000 },
    );
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Open sticky note Pin toggle note (pinned)' }),
      ).toBeInTheDocument(),
    );

    await user.click(pinButton);

    expect(pinButton).toHaveAttribute('aria-pressed', 'false');
    expect(pinButton).toHaveTextContent('Pin note');
    await waitFor(
      () => {
        expect(onSaveNote.mock.calls.at(-1)?.[0]).toMatchObject({ id: 1, isPinned: false });
      },
      { timeout: 3000 },
    );
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Open sticky note Pin toggle note' }),
      ).toBeInTheDocument(),
    );
  });

  it('selects a note color and updates the card after the provider save', async () => {
    const user = userEvent.setup();
    const onSaveNote = vi.fn();
    render(
      <PanelHost
        initialNotes={[note({ id: 2, title: 'Color note', color: 'blue', isPinned: false })]}
        onSaveNote={onSaveNote}
      />,
    );

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Color note'));
    expect(screen.getByRole('button', { name: 'Open sticky note Color note' })).toHaveAttribute(
      'data-color',
      'blue',
    );
    const blue = screen.getByRole('button', { name: 'Use blue note' });
    const green = screen.getByRole('button', { name: 'Use green note' });
    expect(blue).toHaveAttribute('aria-pressed', 'true');
    expect(green).toHaveAttribute('aria-pressed', 'false');

    await user.click(green);

    expect(green).toHaveAttribute('aria-pressed', 'true');
    expect(blue).toHaveAttribute('aria-pressed', 'false');
    await waitFor(
      () => {
        expect(onSaveNote.mock.calls.at(-1)?.[0]).toMatchObject({ id: 2, color: 'green' });
      },
      { timeout: 3000 },
    );
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Open sticky note Color note' })).toHaveAttribute(
        'data-color',
        'green',
      ),
    );
  });

  it('flushes a pending edit when quick access closes', async () => {
    const user = userEvent.setup();
    const saveNote = vi.fn().mockResolvedValue(note());
    render(
      <PanelHost initialNotes={[note({ examId: null, examCode: null })]} saveNote={saveNote} />,
    );

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Pinned formula note'));
    await user.type(screen.getByLabelText('Title'), ' pending');
    await user.click(screen.getByRole('button', { name: 'Close sticky notes' }));

    expect(screen.getByRole('button', { name: 'Reopen notes' })).toBeInTheDocument();
    await waitFor(() => expect(saveNote).toHaveBeenCalledTimes(1));
    expect(saveNote.mock.calls[0][0]).toMatchObject({ title: 'Pinned formula note pending' });
  });

  it('shows the save error and retries the draft', async () => {
    const user = userEvent.setup();
    const saveNote = vi.fn().mockResolvedValue(note());
    renderWithNotes(<NotesPanel exams={EXAMS} />, [note()], {
      saveState: 'error',
      saveError: 'Cannot save right now.',
      saveNote,
    });

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Pinned formula note'));
    expect(screen.getByText('Cannot save right now.')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Retry' }));

    await waitFor(() => expect(saveNote).toHaveBeenCalledTimes(1));
    expect(saveNote.mock.calls[0][0]).toMatchObject({
      title: 'Pinned formula note',
      body: 'Remember the vector identity.',
    });
  });

  it('selects the next available note after deleting the current one', async () => {
    const user = userEvent.setup();
    const onSelectionChange = vi.fn();
    const older = note({
      id: 1,
      title: 'Older note',
      updatedAt: '2026-11-09T10:00:00Z',
    });
    const newest = note({
      id: 2,
      title: 'Newest note',
      updatedAt: '2026-11-10T12:00:00Z',
    });
    render(<PanelHost initialNotes={[older, newest]} onSelectionChange={onSelectionChange} />);

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Newest note'));
    await user.click(screen.getByRole('button', { name: 'Delete note' }));

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Older note'));
    expect(
      screen.queryByRole('button', { name: 'Open sticky note Newest note (pinned)' }),
    ).not.toBeInTheDocument();
    expect(onSelectionChange).toHaveBeenCalledWith(1);
  });

  it('recovers a valid selection when the current note disappears', async () => {
    const onSelectNote = vi.fn();
    const older = note({
      id: 1,
      title: 'Older note',
      updatedAt: '2026-11-09T10:00:00Z',
    });
    const newest = note({
      id: 2,
      title: 'Newest note',
      updatedAt: '2026-11-10T12:00:00Z',
    });
    const panel = (notes: V2StickyNote[]) => (
      <NotesContext.Provider value={context(notes)}>
        <NotesPanel exams={EXAMS} onSelectNote={onSelectNote} selectedNoteId={2} />
      </NotesContext.Provider>
    );

    const { rerender } = render(panel([older, newest]));
    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Newest note'));

    rerender(panel([older]));
    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Older note'));
    expect(onSelectNote).toHaveBeenCalledWith(1);
  });

  it('remembers the last selected note when quick access reopens', async () => {
    const user = userEvent.setup();
    const older = note({
      id: 1,
      title: 'Older note',
      updatedAt: '2026-11-09T10:00:00Z',
    });
    const newest = note({
      id: 2,
      title: 'Newest note',
      updatedAt: '2026-11-10T12:00:00Z',
    });
    render(<PanelHost initialNotes={[older, newest]} />);

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Newest note'));
    await user.click(screen.getByRole('button', { name: 'Open sticky note Older note (pinned)' }));
    await user.click(screen.getByRole('button', { name: 'Close sticky notes' }));
    await user.click(screen.getByRole('button', { name: 'Reopen notes' }));

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Older note'));
  });

  it('supports Ctrl+F, Ctrl+N, Escape and normal text-editing shortcuts', async () => {
    const user = userEvent.setup();
    const closePanel = vi.fn();
    renderWithNotes(<NotesPanel exams={EXAMS} />, [note()], { closePanel });

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Pinned formula note'));

    await user.keyboard('{Control>}f{/Control}');
    const search = screen.getByLabelText('Search sticky notes');
    await waitFor(() => expect(search).toHaveFocus());

    await user.keyboard('{Control>}n{/Control}');
    const title = screen.getByLabelText('Title');
    await waitFor(() => expect(title).toHaveFocus());
    expect(title).toHaveValue('');

    await user.type(title, 'Draft');
    await user.keyboard('{Control>}a{/Control}');
    expect(title).toHaveValue('Draft');
    expect(title).toHaveFocus();

    await user.keyboard('{Escape}');
    await waitFor(() => expect(closePanel).toHaveBeenCalledTimes(1));
  });

  it('exposes compact navigation at narrow widths and closes it after selection', async () => {
    const user = userEvent.setup();
    setWindowWidth(800);
    const older = note({
      id: 1,
      title: 'Older note',
      updatedAt: '2026-11-09T10:00:00Z',
    });
    const newest = note({
      id: 2,
      title: 'Newest note',
      updatedAt: '2026-11-10T12:00:00Z',
    });
    render(<PanelHost initialNotes={[older, newest]} />);

    const panel = document.querySelector('.v2-notes-panel');
    expect(panel).toHaveAttribute('data-layout', 'compact');
    const toggle = screen.getByRole('button', { name: 'Show notes list' });
    expect(toggle).toHaveAttribute('aria-expanded', 'false');

    await user.click(toggle);
    expect(screen.getByRole('button', { name: 'Hide notes list' })).toHaveAttribute(
      'aria-expanded',
      'true',
    );
    expect(document.getElementById('v2-notes-quick-nav')).toHaveClass('is-open');

    await user.click(screen.getByRole('button', { name: 'Open sticky note Older note (pinned)' }));
    expect(screen.getByRole('button', { name: 'Show notes list' })).toHaveAttribute(
      'aria-expanded',
      'false',
    );
    expect(document.getElementById('v2-notes-quick-nav')).not.toHaveClass('is-open');
  });

  it.each([
    [1200, false],
    [1000, false],
    [900, true],
    [800, true],
    [620, true],
  ])('uses the expected layout mode at %ipx', async (width, compact) => {
    setWindowWidth(width);
    renderWithNotes(<NotesPanel exams={EXAMS} />, [note()]);

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Pinned formula note'));
    expect(isCompactNotesWidth(width)).toBe(compact);
    expect(document.querySelector('.v2-notes-panel')).toHaveAttribute(
      'data-layout',
      compact ? 'compact' : 'split',
    );
  });

  it('updates the layout when the window is resized while open', async () => {
    renderWithNotes(<NotesPanel exams={EXAMS} />, [note()]);
    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Pinned formula note'));
    expect(document.querySelector('.v2-notes-panel')).toHaveAttribute('data-layout', 'split');

    act(() => {
      setWindowWidth(620);
      fireEvent(window, new Event('resize'));
    });

    expect(document.querySelector('.v2-notes-panel')).toHaveAttribute('data-layout', 'compact');
  });

  it('expands into the full Notes page through the supplied action', async () => {
    const user = userEvent.setup();
    const onOpenFullPage = vi.fn();
    renderWithNotes(<NotesPanel exams={EXAMS} onOpenFullPage={onOpenFullPage} />, [note()]);

    await waitFor(() => expect(screen.getByLabelText('Title')).toHaveValue('Pinned formula note'));
    await user.click(screen.getByRole('button', { name: 'Open full page' }));

    expect(onOpenFullPage).toHaveBeenCalledTimes(1);
  });

  it('filters the dedicated Notes page by pinned state and association', async () => {
    const user = userEvent.setup();
    renderWithNotes(<NotesScreen exams={EXAMS} />, [
      note(),
      note({
        id: 2,
        title: 'Unpinned general note',
        body: 'General reminder.',
        color: 'green',
        isPinned: false,
        examId: null,
        examCode: null,
        subjectNodeId: null,
        subjectTitle: null,
        topicNodeId: null,
        topicTitle: null,
      }),
    ]);

    expect(screen.getByRole('heading', { name: 'Sticky notes' })).toBeInTheDocument();
    expect(screen.getByText('2 notes')).toBeInTheDocument();

    await user.click(screen.getByLabelText('Pinned only'));
    expect(screen.getByText('Pinned formula note')).toBeInTheDocument();
    expect(screen.queryByText('Unpinned general note')).not.toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText('Note exam filter'), 'jest-physics');
    expect(screen.getByText('Pinned formula note')).toBeInTheDocument();
  });

  it('shows an honest empty state on the Notes page', () => {
    renderWithNotes(<NotesScreen exams={EXAMS} />, []);
    expect(screen.getByRole('heading', { name: 'No sticky notes yet' })).toBeInTheDocument();
  });
});
