import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import StudyResourcesScreen from './StudyResourcesScreen';
import type {
  DiscoverySummary,
  LocalResourceFile,
  Resource,
  ResourceOverview,
} from '../lib/resourcesApi';

const { invokeMock, openDialogMock, openPathMock, openUrlMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  openDialogMock: vi.fn(),
  openPathMock: vi.fn(),
  openUrlMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: openDialogMock,
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  openPath: openPathMock,
  openUrl: openUrlMock,
}));

const LOCAL_FILE: LocalResourceFile = {
  fileName: 'advanced-engineering-mathematics.pdf',
  filePath: 'C:\\Users\\Test\\PDF Library\\advanced-engineering-mathematics.pdf',
  sizeBytes: 1_048_576,
  sha256: 'a'.repeat(64),
  importedAt: '2026-10-30T11:00:00.000Z',
};

const RESOURCES: Resource[] = [
  {
    id: 1,
    resourceKey: 'a'.repeat(64),
    title: 'Quantum Mechanics Short Notes',
    description: 'Hand Written Short Notes',
    category: 'handwritten_notes',
    unit: 'Quantum Mechanics',
    topicCode: 'QM-C01',
    sourcePageUrl: 'https://www.csirnetphysics.in/p/hand-written-notes_28.html',
    originalUrl: 'https://drive.google.com/file/d/quantumNotes/view?usp=sharing',
    externalId: 'quantumNotes',
    externalKind: 'drive_file',
    mimeType: null,
    availability: 'online',
    discoveredAt: '2026-10-30T10:00:00.000Z',
    lastSeenAt: '2026-10-30T10:00:00.000Z',
    lastVerifiedAt: '2026-10-30T10:00:00.000Z',
    removedAt: null,
    studyStatus: 'not_started',
    personalNotes: '',
    lastOpenedAt: null,
    localFile: null,
  },
  {
    id: 2,
    resourceKey: 'b'.repeat(64),
    title: 'Advanced Engineering Mathematics.pdf',
    description: 'Mathematical Method of Physics',
    category: 'recommended_books',
    unit: 'Mathematical Methods of Physics',
    topicCode: null,
    sourcePageUrl: 'https://www.csirnetphysics.in/p/book.html',
    originalUrl: 'https://drive.google.com/file/d/engineeringMaths/view?usp=sharing',
    externalId: 'engineeringMaths',
    externalKind: 'drive_file',
    mimeType: null,
    availability: 'online',
    discoveredAt: '2026-10-30T10:00:00.000Z',
    lastSeenAt: '2026-10-30T10:00:00.000Z',
    lastVerifiedAt: '2026-10-30T10:00:00.000Z',
    removedAt: null,
    studyStatus: 'studying',
    personalNotes: 'Read chapters 1–2.',
    lastOpenedAt: '2026-10-30T11:15:00.000Z',
    localFile: LOCAL_FILE,
  },
  {
    id: 3,
    resourceKey: 'c'.repeat(64),
    title: 'Classical Mechanics playlist',
    description: 'YouTube playlist linked from the video page.',
    category: 'youtube_playlists',
    unit: 'Classical Mechanics',
    topicCode: null,
    sourcePageUrl: 'https://www.csirnetphysics.in/p/nptel-video-link.html',
    originalUrl: 'https://www.youtube.com/playlist?list=classicalPlaylist',
    externalId: 'classicalPlaylist',
    externalKind: 'youtube_playlist',
    mimeType: null,
    availability: 'online',
    discoveredAt: '2026-10-30T10:00:00.000Z',
    lastSeenAt: '2026-10-30T10:00:00.000Z',
    lastVerifiedAt: '2026-10-30T10:00:00.000Z',
    removedAt: null,
    studyStatus: 'completed',
    personalNotes: '',
    lastOpenedAt: null,
    localFile: null,
  },
];

const OVERVIEW: ResourceOverview = {
  total: 3,
  localCount: 1,
  onlineCount: 3,
  removedCount: 0,
  studyingCount: 1,
  completedCount: 1,
  lastDiscovery: {
    status: 'success',
    finishedAt: '2026-10-30T10:00:00.000Z',
    pagesOk: 6,
    pagesFailed: 0,
    resourcesSeen: 3,
    resourcesAdded: 3,
    resourcesUpdated: 0,
    resourcesRemoved: 0,
    message: 'Catalogue refreshed.',
  },
};

const BLOCKED_SUMMARY: DiscoverySummary = {
  status: 'blocked',
  pagesOk: 0,
  pagesFailed: 1,
  resourcesSeen: 0,
  resourcesAdded: 0,
  resourcesUpdated: 0,
  resourcesRemoved: 0,
  message: 'The site returned HTTP 429. Existing resources were preserved.',
};

function mockBackend() {
  invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
    switch (command) {
      case 'get_resource_catalogue':
        return Promise.resolve(RESOURCES);
      case 'get_resource_overview':
        return Promise.resolve(OVERVIEW);
      case 'refresh_resource_catalogue':
        return Promise.resolve(BLOCKED_SUMMARY);
      case 'update_resource_progress': {
        const resourceId = Number(args?.resourceId);
        const progress = args?.progress as { status: Resource['studyStatus']; notes: string };
        const resource = RESOURCES.find((item) => item.id === resourceId);
        return Promise.resolve({
          ...resource,
          studyStatus: progress.status,
          personalNotes: progress.notes,
        });
      }
      case 'create_manual_resource': {
        const input = args?.resource as {
          title: string;
          category: Resource['category'];
          unit: Resource['unit'];
          originalUrl: string;
        };
        return Promise.resolve({
          ...RESOURCES[0],
          id: 99,
          title: input.title,
          category: input.category,
          unit: input.unit,
          originalUrl: input.originalUrl,
          sourcePageUrl: null,
          studyStatus: 'not_started',
          localFile: null,
        });
      }
      case 'import_resource_pdf':
        return Promise.resolve(LOCAL_FILE);
      case 'remove_resource_pdf':
        return Promise.resolve(undefined);
      case 'mark_resource_opened':
        return Promise.resolve(undefined);
      default:
        return Promise.reject(new Error(`Unexpected command: ${command}`));
    }
  });
}

describe('study resources screen', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    openDialogMock.mockReset();
    openPathMock.mockReset();
    openUrlMock.mockReset();
    openPathMock.mockResolvedValue(undefined);
    openUrlMock.mockResolvedValue(undefined);
    vi.spyOn(window, 'confirm').mockReturnValue(true);
  });

  it('loads the catalogue, summary, availability, and study status', async () => {
    mockBackend();
    render(<StudyResourcesScreen />);

    expect(await screen.findByText('Quantum Mechanics Short Notes')).toBeInTheDocument();
    expect(screen.getByText('Advanced Engineering Mathematics.pdf')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
    expect(screen.getByText('Offline copy')).toBeInTheDocument();
    expect(screen.getAllByText('Online only', { selector: '.resource-availability' })).toHaveLength(
      2,
    );
    expect(screen.getByText('Read chapters 1–2.')).toBeInTheDocument();
  });

  it('filters by search, unit, and source', async () => {
    mockBackend();
    render(<StudyResourcesScreen />);
    await screen.findByText('Quantum Mechanics Short Notes');

    await userEvent.type(screen.getByLabelText('Search resources'), 'playlist');
    expect(screen.getByText('Classical Mechanics playlist')).toBeInTheDocument();
    expect(screen.queryByText('Quantum Mechanics Short Notes')).not.toBeInTheDocument();

    await userEvent.clear(screen.getByLabelText('Search resources'));
    await userEvent.selectOptions(
      screen.getByLabelText('Filter by syllabus unit'),
      'Quantum Mechanics',
    );
    expect(screen.getByText('Quantum Mechanics Short Notes')).toBeInTheDocument();
    expect(screen.queryByText('Classical Mechanics playlist')).not.toBeInTheDocument();

    await userEvent.selectOptions(screen.getByLabelText('Filter by syllabus unit'), 'all');
    await userEvent.selectOptions(
      screen.getByLabelText('Filter by source website'),
      'Google Drive',
    );
    expect(screen.getByText('Advanced Engineering Mathematics.pdf')).toBeInTheDocument();
    expect(screen.queryByText('Classical Mechanics playlist')).not.toBeInTheDocument();
  });

  it('adds a manual resource through the form', async () => {
    mockBackend();
    render(<StudyResourcesScreen />);
    await screen.findByText('Quantum Mechanics Short Notes');

    await userEvent.click(screen.getByText('Add a resource manually'));
    await userEvent.type(screen.getByLabelText('Resource title'), 'Manual formula sheet');
    await userEvent.type(
      screen.getByLabelText('Original resource URL'),
      'https://example.com/formula.pdf',
    );
    await userEvent.selectOptions(screen.getByLabelText('Resource type'), 'formula_sheets');
    await userEvent.selectOptions(
      screen.getByLabelText('Syllabus unit'),
      'Mathematical Methods of Physics',
    );
    await userEvent.click(screen.getByRole('button', { name: 'Add resource' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('create_manual_resource', {
        resource: {
          title: 'Manual formula sheet',
          description: '',
          category: 'formula_sheets',
          unit: 'Mathematical Methods of Physics',
          topicCode: null,
          originalUrl: 'https://example.com/formula.pdf',
          sourcePageUrl: null,
        },
      }),
    );
    expect(
      await screen.findByText('Manual resource added to the local catalogue.'),
    ).toBeInTheDocument();
  });

  it('saves study status and personal notes', async () => {
    mockBackend();
    render(<StudyResourcesScreen />);
    await screen.findByText('Quantum Mechanics Short Notes');

    await userEvent.selectOptions(
      screen.getByLabelText('Study status for Quantum Mechanics Short Notes'),
      'studying',
    );
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('update_resource_progress', {
        resourceId: 1,
        progress: { status: 'studying', notes: '' },
      }),
    );

    await userEvent.click(screen.getAllByText('Personal notes')[0]);
    const notes = screen.getAllByLabelText('Notes for this resource')[0];
    await userEvent.type(notes, 'Revisit the derivation.');
    await userEvent.click(screen.getAllByRole('button', { name: 'Save notes' })[0]);

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('update_resource_progress', {
        resourceId: 1,
        progress: { status: 'studying', notes: 'Revisit the derivation.' },
      }),
    );
  });

  it('keeps resources visible when refresh is rate-limited', async () => {
    mockBackend();
    render(<StudyResourcesScreen />);
    await screen.findByText('Quantum Mechanics Short Notes');

    await userEvent.click(screen.getByRole('button', { name: 'Refresh catalogue' }));

    expect(await screen.findByText(/HTTP 429/)).toBeInTheDocument();
    expect(screen.getByText('Quantum Mechanics Short Notes')).toBeInTheDocument();
    expect(screen.getByText('Advanced Engineering Mathematics.pdf')).toBeInTheDocument();
  });

  it('opens original links and the local PDF, and imports/removes a local copy', async () => {
    mockBackend();
    openDialogMock
      .mockResolvedValueOnce('C:\\Users\\Test\\Downloads\\notes.pdf')
      .mockResolvedValueOnce('C:\\Users\\Test\\PDF Library');
    render(<StudyResourcesScreen />);
    await screen.findByText('Quantum Mechanics Short Notes');

    await userEvent.click(screen.getAllByRole('button', { name: 'Open original' })[0]);
    expect(openUrlMock).toHaveBeenCalledWith(
      'https://drive.google.com/file/d/quantumNotes/view?usp=sharing',
    );
    expect(invokeMock).toHaveBeenCalledWith('mark_resource_opened', { resourceId: 1 });

    await userEvent.click(screen.getAllByRole('button', { name: 'Open local PDF' })[0]);
    expect(openPathMock).toHaveBeenCalledWith(LOCAL_FILE.filePath);

    await userEvent.click(screen.getAllByRole('button', { name: 'Open local' })[0]);
    expect(openPathMock).toHaveBeenCalledTimes(2);

    await userEvent.click(screen.getAllByRole('button', { name: 'Remove local copy' })[0]);
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('remove_resource_pdf', { resourceId: 2 }),
    );

    await userEvent.click(screen.getAllByRole('button', { name: 'Import authorized PDF' })[0]);
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('import_resource_pdf', {
        resourceId: 1,
        sourcePath: 'C:\\Users\\Test\\Downloads\\notes.pdf',
        destinationDir: 'C:\\Users\\Test\\PDF Library',
      }),
    );
  });
});
