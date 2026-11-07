import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  V2ExtractionCandidate,
  V2PracticeOptions,
  V2PyqAnswerKey,
  V2PyqSource,
  V2SourceCounts,
} from './api';
import { SourcesPanel } from './SourcesPanel';

vi.mock('./api', () => ({
  acquireV2PyqSource: vi.fn(),
  extractV2PyqSource: vi.fn(),
  getV2PyqSourceCounts: vi.fn(),
  listV2PyqAnswerKeyConflicts: vi.fn(),
  listV2PyqAnswerKeys: vi.fn(),
  listV2PyqExtractionCandidates: vi.fn(),
  listV2PyqSources: vi.fn(),
  previewV2PyqAnswerKey: vi.fn(),
  reconcileV2PyqSource: vi.fn(),
  rejectV2PyqExtractionCandidate: vi.fn(),
  saveV2PyqAnswerKey: vi.fn(),
  saveV2PyqSource: vi.fn(),
  setV2PyqSourceStatus: vi.fn(),
  updateV2PyqExtractionCandidate: vi.fn(),
  verifyV2PyqExtractionCandidate: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn().mockResolvedValue(null),
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  openPath: vi.fn().mockResolvedValue(undefined),
  openUrl: vi.fn().mockResolvedValue(undefined),
}));

import * as api from './api';

const mockedSources = vi.mocked(api.listV2PyqSources);
const mockedKeys = vi.mocked(api.listV2PyqAnswerKeys);
const mockedCandidates = vi.mocked(api.listV2PyqExtractionCandidates);
const mockedCounts = vi.mocked(api.getV2PyqSourceCounts);
const mockedConflicts = vi.mocked(api.listV2PyqAnswerKeyConflicts);
const mockedAcquire = vi.mocked(api.acquireV2PyqSource);
const mockedPreviewKey = vi.mocked(api.previewV2PyqAnswerKey);
const mockedSaveKey = vi.mocked(api.saveV2PyqAnswerKey);
const mockedVerifyCandidate = vi.mocked(api.verifyV2PyqExtractionCandidate);

const SOURCE: V2PyqSource = {
  id: 10,
  examId: 2,
  year: 2022,
  paperCode: 'PH',
  sourceType: 'question_paper',
  authority: 'GATE Organising Institute',
  institution: 'IIT Kharagpur',
  title: 'GATE 2022 Physics (PH) Master Question Paper',
  sourceUrl: 'https://example.test/ph-2022.pdf',
  resourceId: 20,
  sha256: 'a'.repeat(64),
  sizeBytes: 100,
  pageCount: 42,
  publishedAt: null,
  acquiredAt: '2026-11-06T00:00:00.000Z',
  sessionLabel: '5',
  documentVersion: 'final',
  keyStatus: '',
  verificationStatus: 'acquired',
  verificationNotes: '',
  associatedSourceId: null,
  resourceFileName: 'ph-2022.pdf',
  resourceFilePath: 'C:\\data\\ph-2022.pdf',
  resourceFileStatus: 'ok',
  createdAt: '2026-11-06T00:00:00.000Z',
  updatedAt: '2026-11-06T00:00:00.000Z',
};

const KEY_SOURCE: V2PyqSource = {
  ...SOURCE,
  id: 11,
  sourceType: 'answer_key',
  title: 'GATE 2022 Physics (PH) Final Answer Key',
  sourceUrl: 'https://example.test/ph-2022-key.pdf',
  resourceFileName: 'ph-2022-key.pdf',
  keyStatus: 'final',
  associatedSourceId: 10,
};

const KEY: V2PyqAnswerKey = {
  id: 30,
  sourceId: 11,
  examId: 2,
  year: 2022,
  paperCode: 'PH',
  sessionLabel: '5',
  versionLabel: 'final-1',
  keyStatus: 'final',
  publishedAt: null,
  verifiedAt: '2026-11-06T00:00:00.000Z',
  verifiedBy: 'local reviewer',
  notes: '',
  supersedesKeyId: null,
  entryCount: 65,
  createdAt: '2026-11-06T00:00:00.000Z',
  updatedAt: '2026-11-06T00:00:00.000Z',
  sourceTitle: KEY_SOURCE.title,
  sourceUrl: KEY_SOURCE.sourceUrl,
  entries: [],
};

const CANDIDATE: V2ExtractionCandidate = {
  id: 40,
  extractionRunId: 50,
  sourceId: 10,
  sourceTitle: SOURCE.title,
  sourceUrl: SOURCE.sourceUrl,
  answerKeyId: 30,
  answerKeyVersion: 'final-1',
  answerKeyEntryId: 60,
  examId: 2,
  year: 2022,
  paperCode: 'PH',
  sessionLabel: '5',
  pageNumber: 1,
  questionNumber: '1',
  sectionCode: 'gate-ga',
  questionTypeCode: 'MCQ',
  statement: 'Synthetic extracted question for source review.',
  options: [
    { key: 'A', text: 'Alpha' },
    { key: 'B', text: 'Beta' },
    { key: 'C', text: 'Gamma' },
    { key: 'D', text: 'Delta' },
  ],
  correctAnswer: 'A',
  correctRange: '',
  maxMarks: 1,
  negativeMarks: -1 / 3,
  unansweredMarks: 0,
  numericTolerance: 0,
  difficulty: 3,
  syllabusNodeId: null,
  syllabusNodeTitle: null,
  conceptIds: [],
  reviewStatus: 'extracted',
  reviewNotes: '',
  reviewedAt: null,
  reviewedBy: '',
  verifiedQuestionId: null,
  rawText: 'Q.1 Synthetic extracted question.',
  dedupeKey: 'gate-physics-2022-5-1',
  createdAt: '2026-11-06T00:00:00.000Z',
  updatedAt: '2026-11-06T00:00:00.000Z',
};

const COUNTS: V2SourceCounts = {
  sources: 2,
  answerKeys: 1,
  keyEntries: 65,
  extractionRuns: 1,
  candidates: 1,
  candidatesVerified: 0,
  candidatesPending: 1,
  candidatesRejected: 0,
  verifiedQuestions: 0,
  keyConflicts: 0,
};

const OPTIONS = {
  exam: {
    id: 2,
    code: 'gate-physics',
    name: 'GATE Physics',
    shortName: 'GATE PH',
    isActive: true,
  },
  patterns: [],
  sections: [
    { id: 4, code: 'gate-ga', title: 'General Aptitude', defaultQuestionTypeId: 2 },
    { id: 5, code: 'gate-ph', title: 'Physics (PH)', defaultQuestionTypeId: 2 },
  ],
  questionTypes: [],
  topics: [],
  concepts: [],
  resources: [],
} as unknown as V2PracticeOptions;

async function setup() {
  mockedSources.mockResolvedValue([SOURCE, KEY_SOURCE]);
  mockedKeys.mockResolvedValue([KEY]);
  mockedCandidates.mockResolvedValue([CANDIDATE]);
  mockedCounts.mockResolvedValue(COUNTS);
  mockedConflicts.mockResolvedValue([]);
  const refresh = vi.fn().mockResolvedValue(undefined);
  render(<SourcesPanel examId={2} options={OPTIONS} refresh={refresh} today="2026-11-06" />);
  await screen.findAllByText(/GATE 2022 Physics \(PH\) Master Question Paper/);
  return { refresh };
}

describe('SourcesPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the source register and approves a reviewed candidate', async () => {
    const { refresh } = await setup();
    const user = userEvent.setup();
    expect(screen.getByText('Official sources & extraction')).toBeInTheDocument();
    expect(screen.getByText('Synthetic extracted question for source review.')).toBeInTheDocument();
    await user.click(
      screen.getByRole('button', { name: /Synthetic extracted question for source review/ }),
    );
    await user.click(screen.getByRole('button', { name: 'Approve and create PYQ' }));
    await waitFor(() => expect(mockedVerifyCandidate).toHaveBeenCalledTimes(1));
    expect(mockedVerifyCandidate.mock.calls[0][0].candidateId).toBe(40);
    expect(refresh).toHaveBeenCalled();
  });

  it('acquires a managed PDF source with the official metadata', async () => {
    await setup();
    const user = userEvent.setup();
    await user.type(
      screen.getByLabelText('Official title'),
      'GATE 2022 Physics (PH) Master Question Paper',
    );
    await user.type(
      screen.getByPlaceholderText('C:\\path\\to\\official.pdf'),
      'C:\\sources\\ph-2022.pdf',
    );
    const pages = screen.getByLabelText('PDF pages');
    await user.clear(pages);
    await user.type(pages, '42');
    mockedAcquire.mockResolvedValue({ source: SOURCE, duplicate: false });
    await user.click(screen.getByRole('button', { name: 'Acquire PDF and register' }));
    await waitFor(() => expect(mockedAcquire).toHaveBeenCalledTimes(1));
    expect(mockedAcquire.mock.calls[0][0].filePath).toBe('C:\\sources\\ph-2022.pdf');
    expect(mockedAcquire.mock.calls[0][0].sourceType).toBe('question_paper');
    expect(mockedAcquire.mock.calls[0][0].pageCount).toBe(42);
  });

  it('previews and saves a final answer key', async () => {
    await setup();
    const user = userEvent.setup();
    const keySelect = screen.getByLabelText('Answer-key source');
    await user.selectOptions(keySelect, '11');
    await user.type(
      screen.getByLabelText('Answer-key text (row or cell-per-line layout)'),
      '1 5 MCQ GA A 1',
    );
    mockedPreviewKey.mockResolvedValue({
      entries: [],
      missingQuestionNumbers: [2, 3],
      duplicateQuestionNumbers: [],
      parsedCount: 1,
    });
    mockedSaveKey.mockResolvedValue(KEY);
    await user.click(screen.getByRole('button', { name: 'Preview parsed key' }));
    await waitFor(() => expect(mockedPreviewKey).toHaveBeenCalledTimes(1));
    expect(await screen.findByText(/Parsed 1 key entries/)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Save answer key' }));
    await waitFor(() => expect(mockedSaveKey).toHaveBeenCalledTimes(1));
    expect(mockedSaveKey.mock.calls[0][0].sourceId).toBe(11);
    expect(mockedSaveKey.mock.calls[0][0].verified).toBe(true);
  });
});
