import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2ExamSummary, V2Resource } from './api';
import { ResourcesScreen } from './ResourcesScreen';

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

vi.mock('./api', () => ({
  listV2Resources: vi.fn(),
  saveV2Resource: vi.fn(),
  archiveV2Resource: vi.fn(),
  updateV2ResourceProgress: vi.fn(),
  linkV2ResourceExam: vi.fn(),
  importV2ResourceFile: vi.fn(),
  removeV2ResourceFile: vi.fn(),
}));

import * as api from './api';

const mockedList = vi.mocked(api.listV2Resources);
const mockedSave = vi.mocked(api.saveV2Resource);
const mockedArchive = vi.mocked(api.archiveV2Resource);
const mockedProgress = vi.mocked(api.updateV2ResourceProgress);
const mockedLink = vi.mocked(api.linkV2ResourceExam);

const EXAMS: V2ExamSummary[] = [
  {
    id: 1,
    code: 'csir-net-physics',
    name: 'CSIR-UGC NET Physical Sciences',
    shortName: 'CSIR-NET',
    authority: 'CSIR-HRDG / NTA',
    officialUrl: 'https://csirnet.nta.nic.in/',
    displayOrder: 1,
    isActive: true,
    examDate: null,
  },
];

const RESOURCE: V2Resource = {
  id: 4,
  title: 'Mechanics notes',
  description: 'Core notes',
  subjectArea: 'Mechanics',
  url: 'https://example.com/mechanics',
  resourceType: 'notes',
  source: 'Curated',
  isArchived: false,
  createdAt: '2026-11-04T10:00:00Z',
  updatedAt: '2026-11-04T10:00:00Z',
  progress: {
    studyStatus: 'not_started',
    personalNotes: '',
    lastOpenedAt: null,
    updatedAt: '2026-11-04T10:00:00Z',
  },
  conceptIds: [],
  examLinks: [],
  file: null,
};

describe('V2 resources', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedList.mockResolvedValue([RESOURCE]);
  });

  it('loads resources and saves progress', async () => {
    const user = userEvent.setup();
    mockedProgress.mockResolvedValue({
      ...RESOURCE,
      progress: { ...RESOURCE.progress, studyStatus: 'completed' },
    });
    render(<ResourcesScreen exams={EXAMS} />);

    expect(await screen.findByText('Mechanics notes')).toBeInTheDocument();
    await user.selectOptions(screen.getByLabelText('Study status'), 'completed');
    await waitFor(() =>
      expect(mockedProgress).toHaveBeenCalledWith({
        resourceId: 4,
        studyStatus: 'completed',
        personalNotes: '',
      }),
    );
  });

  it('adds a resource and archives it', async () => {
    const user = userEvent.setup();
    mockedSave.mockResolvedValue(RESOURCE);
    mockedArchive.mockResolvedValue({ ...RESOURCE, isArchived: true });
    render(<ResourcesScreen exams={EXAMS} />);
    await screen.findByText('Mechanics notes');

    await user.type(screen.getByLabelText('Title'), 'New resource');
    await user.type(screen.getByLabelText('URL'), 'https://example.com/new');
    await user.click(screen.getByRole('button', { name: 'Save resource' }));
    await waitFor(() =>
      expect(mockedSave).toHaveBeenCalledWith(
        expect.objectContaining({
          title: 'New resource',
          url: 'https://example.com/new',
          resourceType: 'website',
        }),
      ),
    );

    await user.click(screen.getByRole('button', { name: 'Archive' }));
    await waitFor(() =>
      expect(mockedArchive).toHaveBeenCalledWith({ resourceId: 4, archived: true }),
    );
  });

  it('labels a missing managed file as relinkable', async () => {
    mockedList.mockResolvedValue([
      {
        ...RESOURCE,
        file: {
          fileName: 'notes.pdf',
          filePath: 'C:\\managed\\notes.pdf',
          sha256: 'a'.repeat(64),
          sizeBytes: 120,
          storageMode: 'managed',
          status: 'missing',
          importedAt: '2026-11-04T10:00:00Z',
          lastVerifiedAt: '2026-11-04T10:00:00Z',
          missingSince: '2026-11-05T10:00:00Z',
        },
      },
    ]);
    render(<ResourcesScreen exams={EXAMS} />);

    expect(await screen.findByText('Mechanics notes')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Relink PDF' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Remove PDF' })).toBeInTheDocument();
  });

  it('links a resource to an examination', async () => {
    const user = userEvent.setup();
    mockedLink.mockResolvedValue({
      ...RESOURCE,
      examLinks: [
        {
          examId: 1,
          examCode: 'csir-net-physics',
          examShortName: 'CSIR-NET',
          category: 'general',
          syllabusNodeId: null,
          priority: 'normal',
          notes: '',
        },
      ],
    });
    render(<ResourcesScreen exams={EXAMS} />);
    await screen.findByText('Mechanics notes');

    await user.selectOptions(screen.getByLabelText('Link exam'), '1');
    await user.click(screen.getByRole('button', { name: 'Link exam' }));
    await waitFor(() =>
      expect(mockedLink).toHaveBeenCalledWith({
        resourceId: 4,
        link: {
          examId: 1,
          category: 'general',
          priority: 'normal',
          notes: '',
        },
      }),
    );
  });
});

describe('Resource deep links', () => {
  it('applies the deep-link search query to the resource list request', async () => {
    render(<ResourcesScreen exams={EXAMS} initialQuery="Mechanics" />);
    await waitFor(() =>
      expect(mockedList).toHaveBeenCalledWith(expect.objectContaining({ query: 'Mechanics' })),
    );
    expect(await screen.findByText('Mechanics notes')).toBeInTheDocument();
  });
});
