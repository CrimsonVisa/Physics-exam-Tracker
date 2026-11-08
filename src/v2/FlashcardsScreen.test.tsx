import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  V2ExamSyllabus,
  V2ExamSummary,
  V2FlashcardDetail,
  V2FlashcardLibrary,
  V2FlashcardSchedule,
  V2FlashcardStats,
  V2FlashcardSummary,
} from './api';
import { FlashcardsScreen } from './FlashcardsScreen';

vi.mock('./api', () => ({
  listV2Flashcards: vi.fn(),
  getV2FlashcardStats: vi.fn(),
  getV2ExamSyllabus: vi.fn(),
  getV2FlashcardDetail: vi.fn(),
  setV2FlashcardArchived: vi.fn(),
  saveV2Flashcard: vi.fn(),
  searchV2Concepts: vi.fn(),
  getV2FlashcardReviewQueue: vi.fn(),
  reviewV2Flashcard: vi.fn(),
}));

import * as api from './api';

const mockedList = vi.mocked(api.listV2Flashcards);
const mockedStats = vi.mocked(api.getV2FlashcardStats);
const mockedSyllabus = vi.mocked(api.getV2ExamSyllabus);
const mockedDetail = vi.mocked(api.getV2FlashcardDetail);
const mockedArchive = vi.mocked(api.setV2FlashcardArchived);
const mockedSave = vi.mocked(api.saveV2Flashcard);
const mockedConcepts = vi.mocked(api.searchV2Concepts);

const TODAY = '2026-11-08';

const EXAMS: V2ExamSummary[] = [
  {
    id: 1,
    code: 'csir-net-physics',
    name: 'CSIR-UGC NET Physical Sciences',
    shortName: 'CSIR-NET',
    authority: '',
    officialUrl: '',
    displayOrder: 1,
    isActive: true,
    examDate: null,
  },
  {
    id: 2,
    code: 'gate-physics',
    name: 'GATE Physics (PH)',
    shortName: 'GATE PH',
    authority: '',
    officialUrl: '',
    displayOrder: 2,
    isActive: true,
    examDate: null,
  },
];

const SCHEDULE: V2FlashcardSchedule = {
  dueOn: TODAY,
  intervalDays: 0,
  easeFactor: 2.5,
  reviewCount: 0,
  lapseCount: 0,
  lastRating: null,
  lastReviewedOn: null,
  lastReviewedAt: null,
  algorithmVersion: 1,
};

function summary(overrides: Partial<V2FlashcardSummary> = {}): V2FlashcardSummary {
  return {
    id: 1,
    frontPreview: 'Explain the gradient.',
    backPreview: 'It points toward steepest increase.',
    subjectArea: 'Mathematical Methods',
    conceptId: null,
    conceptTitle: null,
    isArchived: false,
    archivedAt: null,
    schedule: SCHEDULE,
    examLinks: [],
    createdAt: '2026-11-08T00:00:00Z',
    updatedAt: '2026-11-08T00:00:00Z',
    ...overrides,
  };
}

function detail(overrides: Partial<V2FlashcardDetail> = {}): V2FlashcardDetail {
  return {
    flashcard: summary(),
    front: 'Explain the gradient.',
    back: 'It points toward steepest increase.',
    reviews: [],
    ...overrides,
  };
}

const LIBRARY: V2FlashcardLibrary = {
  cards: [summary()],
  total: 1,
  subjects: ['Mathematical Methods'],
};

const STATS: V2FlashcardStats = {
  totalCards: 1,
  dueCards: 1,
  reviewedCards: 0,
  newCards: 1,
  totalReviews: 0,
  reviewsLast30Days: 0,
  successfulLast30Days: 0,
  retentionPercent: null,
};

const SYLLABUS: V2ExamSyllabus = {
  examId: 1,
  examCode: 'csir-net-physics',
  examName: 'CSIR-UGC NET Physical Sciences',
  examIsActive: true,
  configured: true,
  contentVersionLabel: 'official',
  contentReviewStatus: 'verified',
  summary: { nodeCount: 2, topicCount: 1, revisedOrConfidentCount: 0, mappedConceptCount: 0 },
  nodes: [
    {
      id: 10,
      examId: 1,
      parentId: null,
      code: 'U1',
      title: 'Mathematical Methods',
      nodeKind: 'unit',
      category: 'Mathematical Methods',
      displayOrder: 1,
      readinessWeight: 1,
      isArchived: false,
      sourceTitle: 'Official',
      sourceVerifiedOn: TODAY,
      sourceUrl: null,
      sourceReference: 'ref',
      contentVersionLabel: 'official',
      contentReviewStatus: 'verified',
      progressStatus: 'not_started',
      priority: 'normal',
      confidence: 3,
      notes: '',
      nodePath: ['Mathematical Methods'],
    },
    {
      id: 11,
      examId: 1,
      parentId: 10,
      code: 'T1',
      title: 'Vector calculus',
      nodeKind: 'topic',
      category: 'Mathematical Methods',
      displayOrder: 1,
      readinessWeight: 1,
      isArchived: false,
      sourceTitle: 'Official',
      sourceVerifiedOn: TODAY,
      sourceUrl: null,
      sourceReference: 'ref',
      contentVersionLabel: 'official',
      contentReviewStatus: 'verified',
      progressStatus: 'not_started',
      priority: 'normal',
      confidence: 3,
      notes: '',
      nodePath: ['Mathematical Methods', 'Vector calculus'],
    },
  ],
};

describe('FlashcardsScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedList.mockResolvedValue(LIBRARY);
    mockedStats.mockResolvedValue(STATS);
    mockedSyllabus.mockResolvedValue(SYLLABUS);
    mockedDetail.mockResolvedValue(detail());
    mockedArchive.mockResolvedValue(detail({ flashcard: summary({ isArchived: true }) }));
  });

  it('loads the library and stats with the local review date', async () => {
    render(<FlashcardsScreen exams={EXAMS} today={TODAY} />);

    expect(await screen.findByText('Explain the gradient.')).toBeInTheDocument();
    expect(screen.getByText('Total cards')).toBeInTheDocument();
    expect(screen.getByText('Due today')).toBeInTheDocument();
    await waitFor(() =>
      expect(mockedList).toHaveBeenCalledWith(
        expect.objectContaining({ today: TODAY, examId: null }),
      ),
    );
    expect(mockedStats).toHaveBeenCalledWith(expect.objectContaining({ today: TODAY }));
  });

  it('creates a card through the editor', async () => {
    const user = userEvent.setup();
    mockedSave.mockResolvedValue(
      detail({
        front: 'New question',
        back: 'New answer',
        flashcard: summary({
          frontPreview: 'New question',
          backPreview: 'New answer',
          subjectArea: 'Electromagnetism',
        }),
      }),
    );
    render(<FlashcardsScreen exams={EXAMS} today={TODAY} />);
    await screen.findByText('Explain the gradient.');

    await user.click(screen.getByRole('button', { name: 'New card' }));
    expect(screen.getByRole('dialog', { name: 'New flashcard' })).toBeInTheDocument();

    await user.type(screen.getByLabelText('Front'), 'New question');
    await user.type(screen.getByLabelText('Back'), 'New answer');
    await user.type(screen.getByLabelText('Physics subject'), 'Electromagnetism');
    await user.click(screen.getByRole('button', { name: 'Save card' }));

    await waitFor(() =>
      expect(mockedSave).toHaveBeenCalledWith(
        expect.objectContaining({
          id: null,
          front: 'New question',
          back: 'New answer',
          subjectArea: 'Electromagnetism',
          examLinks: [],
          today: TODAY,
        }),
      ),
    );
    expect(await screen.findByText('New question')).toBeInTheDocument();
  });

  it('opens a deep-linked card and archives it with confirmation', async () => {
    const user = userEvent.setup();
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);
    render(<FlashcardsScreen exams={EXAMS} initialFlashcardId={1} today={TODAY} />);

    expect((await screen.findAllByText('Explain the gradient.')).length).toBeGreaterThanOrEqual(2);
    expect(mockedDetail).toHaveBeenCalledWith(1);
    await user.click(screen.getByRole('button', { name: 'Archive card' }));
    expect(confirm).toHaveBeenCalled();
    await waitFor(() =>
      expect(mockedArchive).toHaveBeenCalledWith({ flashcardId: 1, archived: true }),
    );
    confirm.mockRestore();
  });

  it('saves a shared concept and an examination link from the editor', async () => {
    const user = userEvent.setup();
    const concept = {
      id: 7,
      slug: 'kinematics',
      title: 'Kinematics',
      description: '',
      domain: 'Classical Mechanics',
      parentConceptId: null,
      parentSlug: null,
      parentTitle: null,
      difficulty: 2,
      isArchived: false,
      mastery: null,
      examCodes: [],
      verifiedExamCodes: [],
    };
    mockedConcepts.mockResolvedValue({
      concepts: [concept],
      total: 1,
      domains: ['Classical Mechanics'],
      examCoverage: [],
    });
    mockedSave.mockResolvedValue(detail());
    render(<FlashcardsScreen exams={EXAMS} today={TODAY} />);
    await screen.findByText('Explain the gradient.');

    await user.click(screen.getByRole('button', { name: 'New card' }));
    await user.type(screen.getByLabelText('Front'), 'Concept prompt');
    await user.type(screen.getByLabelText('Back'), 'Concept answer');
    await user.type(screen.getByLabelText('Physics subject'), 'Classical Mechanics');
    await user.type(screen.getByLabelText('Search shared concepts'), 'kine');
    await waitFor(() => expect(mockedConcepts).toHaveBeenCalled());
    await user.click(await screen.findByRole('button', { name: /Kinematics/ }));
    await user.click(screen.getByLabelText('Include GATE PH'));
    await user.click(screen.getByRole('button', { name: 'Save card' }));

    await waitFor(() =>
      expect(mockedSave).toHaveBeenCalledWith(
        expect.objectContaining({
          conceptId: 7,
          examLinks: [{ examId: 2, syllabusNodeId: null }],
        }),
      ),
    );
  });

  it('applies concept and syllabus deep-link filters', async () => {
    render(
      <FlashcardsScreen
        exams={EXAMS}
        initialConceptSlug="kinematics"
        initialConceptTitle="Kinematics"
        initialExamCode="csir-net-physics"
        initialNodeId={11}
        initialNodeTitle="Vector calculus"
        today={TODAY}
      />,
    );

    expect(await screen.findByText('Concept: Kinematics')).toBeInTheDocument();
    expect(screen.getByText('Syllabus: Vector calculus')).toBeInTheDocument();
    await waitFor(() =>
      expect(mockedList).toHaveBeenCalledWith(
        expect.objectContaining({
          examId: 1,
          nodeId: 11,
          conceptSlug: 'kinematics',
        }),
      ),
    );
  });
});
