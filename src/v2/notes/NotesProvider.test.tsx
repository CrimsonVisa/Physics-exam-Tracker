import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2StickyNote } from '../api';
import { NotesProvider } from './NotesProvider';
import { useNotes } from './notesContext';

vi.mock('../api', () => ({
  listV2StickyNotes: vi.fn(),
  saveV2StickyNote: vi.fn(),
  deleteV2StickyNote: vi.fn(),
}));

import * as api from '../api';

const mockedList = vi.mocked(api.listV2StickyNotes);
const mockedSave = vi.mocked(api.saveV2StickyNote);
const mockedDelete = vi.mocked(api.deleteV2StickyNote);

function note(overrides: Partial<V2StickyNote> = {}): V2StickyNote {
  return {
    id: 1,
    title: 'Note',
    body: 'Body',
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

function Harness() {
  const notes = useNotes();
  return (
    <div>
      <button onClick={notes.openPanel} type="button">
        Open notes
      </button>
      <button
        onClick={() =>
          void notes
            .saveNote({
              id: null,
              title: 'Draft',
              body: 'Body',
              color: 'yellow',
              isPinned: true,
              examId: null,
              subjectNodeId: null,
              topicNodeId: null,
            })
            .catch(() => undefined)
        }
        type="button"
      >
        Save note
      </button>
      <button onClick={() => void notes.deleteNote(1).catch(() => undefined)} type="button">
        Delete note
      </button>
      <span data-testid="loaded">{notes.notes?.length ?? 'null'}</span>
      <span data-testid="save-state">{notes.saveState}</span>
      <span data-testid="save-error">{notes.saveError ?? ''}</span>
    </div>
  );
}

function renderProvider() {
  render(
    <NotesProvider>
      <Harness />
    </NotesProvider>,
  );
}

describe('NotesProvider', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedList.mockResolvedValue([]);
    mockedDelete.mockResolvedValue(undefined);
  });

  it('loads lazily once and does not duplicate API requests', async () => {
    mockedList.mockResolvedValue([note()]);
    renderProvider();

    expect(mockedList).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Open notes' }));
    await waitFor(() => expect(screen.getByTestId('loaded')).toHaveTextContent('1'));
    expect(mockedList).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole('button', { name: 'Open notes' }));
    expect(mockedList).toHaveBeenCalledTimes(1);
  });

  it('saves through one queue and exposes saved state', async () => {
    mockedSave.mockResolvedValue(note({ id: 7, title: 'Draft', isPinned: true }));
    renderProvider();

    fireEvent.click(screen.getByRole('button', { name: 'Save note' }));

    await waitFor(() => expect(mockedSave).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(screen.getByTestId('save-state')).toHaveTextContent('saved'));
    expect(screen.getByTestId('loaded')).toHaveTextContent('1');
  });

  it('keeps a failed save in the error state for retry', async () => {
    mockedSave.mockRejectedValue({ code: 'database_error', message: 'Save failed.' });
    renderProvider();

    fireEvent.click(screen.getByRole('button', { name: 'Save note' }));

    await waitFor(() => expect(screen.getByTestId('save-state')).toHaveTextContent('error'));
    expect(screen.getByTestId('save-error')).toHaveTextContent('Save failed.');
  });

  it('serializes overlapping saves and ignores a stale first result', async () => {
    let resolveFirst!: (value: V2StickyNote) => void;
    const firstSave = new Promise<V2StickyNote>((resolve) => {
      resolveFirst = resolve;
    });
    mockedSave
      .mockImplementationOnce(() => firstSave)
      .mockResolvedValueOnce(note({ id: 2, title: 'Latest save' }));
    renderProvider();

    fireEvent.click(screen.getByRole('button', { name: 'Save note' }));
    fireEvent.click(screen.getByRole('button', { name: 'Save note' }));

    await waitFor(() => expect(mockedSave).toHaveBeenCalledTimes(1));
    await act(async () => {
      resolveFirst(note({ id: 1, title: 'Stale save' }));
      await firstSave;
    });
    await waitFor(() => expect(mockedSave).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(screen.getByTestId('save-state')).toHaveTextContent('saved'));
    expect(screen.getByTestId('loaded')).toHaveTextContent('1');
  });

  it('deletes a note after the save queue and reports the result', async () => {
    mockedList.mockResolvedValue([note()]);
    renderProvider();
    fireEvent.click(screen.getByRole('button', { name: 'Open notes' }));
    await waitFor(() => expect(screen.getByTestId('loaded')).toHaveTextContent('1'));

    fireEvent.click(screen.getByRole('button', { name: 'Delete note' }));
    await waitFor(() => expect(mockedDelete).toHaveBeenCalledWith(1));
    await waitFor(() => expect(screen.getByTestId('loaded')).toHaveTextContent('0'));
  });
});
