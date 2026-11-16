import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { open } from '@tauri-apps/plugin-dialog';
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
  openV2ResourceFile: vi.fn(),
  validateV2ResourcePdf: vi.fn(),
}));

import * as api from './api';

const mockedList = vi.mocked(api.listV2Resources);
const mockedSave = vi.mocked(api.saveV2Resource);
const mockedArchive = vi.mocked(api.archiveV2Resource);
const mockedProgress = vi.mocked(api.updateV2ResourceProgress);
const mockedLink = vi.mocked(api.linkV2ResourceExam);
const mockedImport = vi.mocked(api.importV2ResourceFile);
const mockedOpenFile = vi.mocked(api.openV2ResourceFile);
const mockedValidate = vi.mocked(api.validateV2ResourcePdf);
const mockedOpenDialog = vi.mocked(open);

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
    mockedValidate.mockResolvedValue(undefined);
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

  it('imports a selected PDF with spaces and Unicode into managed storage', async () => {
    const user = userEvent.setup();
    const filePath = 'C:\\Users\\Test User\\Physics Notes – Ω.pdf';
    const imported = {
      ...RESOURCE,
      id: 9,
      title: 'Physics Notes – Ω',
      resourceType: 'book',
      source: 'Local PDF',
      file: {
        fileName: 'Physics Notes – Ω.pdf',
        sha256: 'b'.repeat(64),
        sizeBytes: 320,
        storageMode: 'managed',
        status: 'ok',
        importedAt: '2026-11-04T10:00:00Z',
        lastVerifiedAt: '2026-11-04T10:00:00Z',
        missingSince: null,
      },
    } satisfies V2Resource;
    mockedOpenDialog.mockResolvedValue(filePath);
    mockedSave.mockResolvedValue({ ...RESOURCE, id: 9, title: 'Physics Notes – Ω' });
    mockedImport.mockResolvedValue(imported);
    render(<ResourcesScreen exams={EXAMS} />);
    await screen.findByText('Mechanics notes');

    await user.click(screen.getByRole('button', { name: 'Import PDF' }));

    expect(mockedOpenDialog).toHaveBeenCalledWith(
      expect.objectContaining({
        multiple: false,
        directory: false,
        filters: [{ name: 'PDF documents', extensions: ['pdf'] }],
      }),
    );
    await waitFor(() =>
      expect(mockedSave).toHaveBeenCalledWith(
        expect.objectContaining({
          title: 'Physics Notes – Ω',
          url: null,
          resourceType: 'book',
          source: 'Local PDF',
        }),
      ),
    );
    expect(mockedImport).toHaveBeenCalledWith({
      resourceId: 9,
      filePath,
    });
    expect(mockedValidate).toHaveBeenCalledWith(filePath);
    expect(
      await screen.findByText(
        'Imported Physics Notes – Ω.pdf into managed PDF storage. The original file was not changed.',
      ),
    ).toBeInTheDocument();
  });

  it('does nothing when the PDF picker is cancelled', async () => {
    const user = userEvent.setup();
    mockedOpenDialog.mockResolvedValue(null);
    render(<ResourcesScreen exams={EXAMS} />);
    await screen.findByText('Mechanics notes');

    await user.click(screen.getByRole('button', { name: 'Import PDF' }));
    await waitFor(() => expect(mockedOpenDialog).toHaveBeenCalledTimes(1));

    expect(mockedSave).not.toHaveBeenCalled();
    expect(mockedImport).not.toHaveBeenCalled();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('rejects an invalid PDF before creating resource metadata', async () => {
    const user = userEvent.setup();
    mockedOpenDialog.mockResolvedValue('C:\\Users\\Test User\\fake.pdf');
    mockedValidate.mockRejectedValue(
      Object.assign(new Error('The selected file is not a valid PDF.'), {
        code: 'validation_error',
      }),
    );
    render(<ResourcesScreen exams={EXAMS} />);
    await screen.findByText('Mechanics notes');

    await user.click(screen.getByRole('button', { name: 'Import PDF' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'The PDF could not be imported: The selected file is not a valid PDF.',
    );
    expect(mockedValidate).toHaveBeenCalledWith('C:\\Users\\Test User\\fake.pdf');
    expect(mockedSave).not.toHaveBeenCalled();
    expect(mockedImport).not.toHaveBeenCalled();
  });

  it('reports a saved resource when PDF import fails', async () => {
    const user = userEvent.setup();
    mockedOpenDialog.mockResolvedValue('C:\\Users\\Test User\\broken.pdf');
    mockedSave.mockResolvedValue({ ...RESOURCE, id: 10, title: 'broken' });
    mockedImport.mockRejectedValue(
      Object.assign(new Error('The selected file is not a valid PDF.'), {
        code: 'validation_error',
      }),
    );
    render(<ResourcesScreen exams={EXAMS} />);
    await screen.findByText('Mechanics notes');

    await user.click(screen.getByRole('button', { name: 'Import PDF' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      /The PDF could not be imported: The selected file is not a valid PDF\. The resource entry was saved without a PDF; use Attach PDF to retry\./u,
    );
    expect(screen.queryByText(/into managed PDF storage/u)).not.toBeInTheDocument();
  });

  it('opens an imported managed PDF without exposing a filesystem path', async () => {
    const user = userEvent.setup();
    mockedList.mockResolvedValue([
      {
        ...RESOURCE,
        file: {
          fileName: 'notes.pdf',
          sha256: 'c'.repeat(64),
          sizeBytes: 120,
          storageMode: 'managed',
          status: 'ok',
          importedAt: '2026-11-04T10:00:00Z',
          lastVerifiedAt: '2026-11-04T10:00:00Z',
          missingSince: null,
        },
      },
    ]);
    mockedOpenFile.mockResolvedValue(undefined);
    render(<ResourcesScreen exams={EXAMS} />);
    await screen.findByText('Mechanics notes');

    await user.click(screen.getByRole('button', { name: 'Open PDF' }));

    await waitFor(() => expect(mockedOpenFile).toHaveBeenCalledWith(4));
    expect(
      await screen.findByText('Opened notes.pdf in the default PDF viewer.'),
    ).toBeInTheDocument();
  });

  it('keeps URL resources as links without a local PDF open action', async () => {
    render(<ResourcesScreen exams={EXAMS} />);
    const link = await screen.findByRole('link', { name: 'https://example.com/mechanics' });
    expect(link).toHaveAttribute('href', 'https://example.com/mechanics');
    expect(screen.queryByRole('button', { name: 'Open PDF' })).not.toBeInTheDocument();
  });

  it('labels a missing managed file as relinkable', async () => {
    mockedList.mockResolvedValue([
      {
        ...RESOURCE,
        file: {
          fileName: 'notes.pdf',
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
    expect(screen.queryByRole('button', { name: 'Open PDF' })).not.toBeInTheDocument();
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
