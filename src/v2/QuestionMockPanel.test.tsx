import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  V2PracticePattern,
  V2PyqQuestionSummary,
  V2QuestionMockCandidate,
  V2QuestionMockSession,
} from './api';
import { QuestionMockPanel } from './QuestionMockPanel';

vi.mock('./api', () => ({
  listV2QuestionMockCandidates: vi.fn(),
  startV2QuestionMock: vi.fn(),
  getV2QuestionMockSession: vi.fn(),
  saveV2QuestionMockState: vi.fn(),
  submitV2QuestionMock: vi.fn(),
}));

import * as api from './api';

const mockedCandidates = vi.mocked(api.listV2QuestionMockCandidates);
const mockedStart = vi.mocked(api.startV2QuestionMock);
const mockedSave = vi.mocked(api.saveV2QuestionMockState);
const mockedSubmit = vi.mocked(api.submitV2QuestionMock);

const TODAY = '2026-11-05';

const PATTERN: V2PracticePattern = {
  id: 1,
  patternCode: 'csir-net-physical-sciences-705',
  patternTitle: 'CSIR-UGC NET Physical Sciences (705)',
  variant: 'Physical Sciences',
  versionLabel: 'December 2026',
  verificationStatus: 'verified',
  effectiveFrom: null,
  durationMinutes: 180,
  totalMarks: 200,
  totalQuestions: 75,
  attemptLimit: 55,
  sourceTitle: 'Official bulletin',
  sourceUrl: 'https://example.test/bulletin.pdf',
  sourceReference: 'Chapter 3',
  verifiedOn: '2026-11-05',
  notes: '',
  sections: [
    {
      id: 10,
      sectionId: 3,
      code: 'csir-705-part-a',
      title: 'Part A',
      displayOrder: 1,
      questionCount: 1,
      maxAttempts: 15,
      maxMarks: 2,
      correctMarks: 2,
      incorrectMarks: -0.5,
      unansweredMarks: 0,
      questionTypes: ['MCQ'],
      markingVariants: [],
      numericTolerance: 0,
      partialRule: '',
      notes: '',
    },
  ],
};

const QUESTION: V2PyqQuestionSummary = {
  id: 10,
  examId: 1,
  year: 2026,
  sessionLabel: 'Test session',
  questionNumber: '1',
  sectionId: 3,
  sectionCode: 'csir-705-part-a',
  sectionTitle: 'Part A',
  questionTypeId: 1,
  questionTypeCode: 'MCQ',
  patternVersionId: 1,
  patternVersionLabel: 'December 2026',
  statement: 'Synthetic question-level fixture.',
  maxMarks: 2,
  negativeMarks: -0.5,
  unansweredMarks: 0,
  difficulty: 3,
  sourceTitle: 'Synthetic source',
  sourcePage: '1',
  sourceUrl: null,
  answerKeySource: 'Synthetic key',
  answerKeyVersion: '1',
  answerKeyVerified: true,
  verificationStatus: 'verified',
  numericTolerance: 0,
  correctRange: '',
  officialExplanation: '',
  thirdPartySolution: '',
  userNote: '',
  resourceId: null,
  sourceId: null,
  answerKeyId: null,
  isArchived: false,
  attemptCount: 0,
  conceptCount: 0,
  syllabusNodeId: 13,
  syllabusNodeTitle: 'Classical Mechanics',
  topicIds: [13],
  options: [
    { key: 'A', text: 'First choice' },
    { key: 'B', text: 'Second choice' },
    { key: 'C', text: 'Third choice' },
    { key: 'D', text: 'Fourth choice' },
  ],
};

const CANDIDATE: V2QuestionMockCandidate = {
  question: QUESTION,
  patternSectionId: 10,
  patternSectionCode: 'csir-705-part-a',
  patternSectionTitle: 'Part A',
};

const QUESTION_ITEM = {
  id: 100,
  questionId: 10,
  displayOrder: 1,
  sectionId: 3,
  patternSectionId: 10,
  sectionCode: 'csir-705-part-a',
  sectionTitle: 'Part A',
  sectionOrder: 1,
  questionTypeCode: 'MCQ',
  questionNumber: '1',
  topicId: 13,
  topicTitle: 'Classical Mechanics',
  statement: QUESTION.statement,
  options: QUESTION.options,
  selectedAnswer: '',
  visited: true,
  markedForReview: false,
  notEvaluated: false,
  responseStatus: 'unanswered',
  marksObtained: 0,
  maxMarks: 2,
  negativeMarks: -0.5,
  unansweredMarks: 0,
  numericTolerance: 0,
  timeSpentSeconds: 0,
  correctAnswer: null,
  correctRange: null,
  answerKeyVersion: '1',
  officialExplanation: '',
  thirdPartySolution: '',
  userNote: '',
  sourceTitle: 'Synthetic source',
  sourcePage: '1',
  sourceUrl: null,
  difficulty: 3,
  year: 2026,
  sessionLabel: 'Test session',
  concepts: [],
};

const SECTION = {
  id: 1,
  sectionId: 3,
  patternSectionId: 10,
  sectionCode: 'csir-705-part-a',
  sectionTitle: 'Part A',
  displayOrder: 1,
  maxAttempts: 15,
  questionCount: 1,
  answered: 0,
  marked: 0,
  notVisited: 0,
  correct: 0,
  incorrect: 0,
  unanswered: 1,
  maxMarks: 2,
  marksObtained: 0,
  timeSpentSeconds: 0,
  averageTimeSeconds: null,
  accuracyPercentage: null,
  correctMarks: 2,
  incorrectMarks: -0.5,
  unansweredMarks: 0,
};

const IN_PROGRESS: V2QuestionMockSession = {
  mockId: 1,
  examId: 1,
  patternVersionId: 1,
  patternVersionLabel: 'December 2026',
  patternTitle: 'CSIR-UGC NET Physical Sciences (705)',
  testName: 'Synthetic question-level mock',
  testDate: TODAY,
  durationMinutes: 180,
  testMode: 'custom',
  status: 'in_progress',
  mockKind: 'question_level',
  startedAt: '2026-11-05T10:00:00.000Z',
  deadlineAt: '2026-11-05T13:00:00.000Z',
  endedAt: null,
  serverNow: '2026-11-05T10:00:00.000Z',
  remainingSeconds: 3600,
  currentQuestionId: 100,
  lastActivityAt: '2026-11-05T10:00:00.000Z',
  submissionKind: '',
  maxMarks: 2,
  marksObtained: null,
  percentage: null,
  availableQuestions: 1,
  attemptedQuestions: 0,
  correctQuestions: 0,
  incorrectQuestions: 0,
  unansweredQuestions: 1,
  totalTimeSeconds: 0,
  averageTimeSeconds: 0,
  questions: [QUESTION_ITEM],
  sections: [SECTION],
  topics: [],
  compatibleAttempts: [],
  schemeSnapshotJson: '{}',
  notes: '',
  reviewObservations: '',
  isArchived: false,
};

const COMPLETED: V2QuestionMockSession = {
  ...IN_PROGRESS,
  status: 'completed',
  submissionKind: 'manual',
  remainingSeconds: null,
  marksObtained: 2,
  percentage: 100,
  attemptedQuestions: 1,
  correctQuestions: 1,
  unansweredQuestions: 0,
  questions: [
    {
      ...QUESTION_ITEM,
      selectedAnswer: 'B',
      responseStatus: 'correct',
      marksObtained: 2,
      timeSpentSeconds: 30,
      correctAnswer: 'B',
      officialExplanation: 'Synthetic official explanation.',
    },
  ],
  sections: [
    {
      ...SECTION,
      answered: 1,
      correct: 1,
      unanswered: 0,
      marksObtained: 2,
      timeSpentSeconds: 30,
      averageTimeSeconds: 30,
      accuracyPercentage: 100,
    },
  ],
  topics: [
    {
      topicId: 13,
      topicTitle: 'Classical Mechanics',
      questions: 1,
      correct: 1,
      incorrect: 0,
      unanswered: 0,
      accuracyPercentage: 100,
      averageTimeSeconds: 30,
    },
  ],
  compatibleAttempts: [
    {
      mockId: 1,
      testDate: TODAY,
      testName: 'Synthetic question-level mock',
      marksObtained: 2,
      maxMarks: 2,
      percentage: 100,
      current: true,
    },
  ],
};

async function setup() {
  mockedCandidates.mockResolvedValue([CANDIDATE]);
  mockedStart.mockResolvedValue(IN_PROGRESS);
  mockedSave.mockResolvedValue({
    saved: true,
    status: 'in_progress',
    autoSubmitted: false,
    serverNow: '2026-11-05T10:00:01.000Z',
    remainingSeconds: 3599,
    currentQuestionId: 100,
    answeredQuestions: 1,
    markedQuestions: 0,
    notVisitedQuestions: 0,
  });
  mockedSubmit.mockResolvedValue(COMPLETED);
  const refresh = vi.fn().mockResolvedValue(undefined);
  render(
    <QuestionMockPanel examId={1} mocks={[]} pattern={PATTERN} refresh={refresh} today={TODAY} />,
  );
  await screen.findByText(/Synthetic question-level fixture/);
  return { refresh };
}

describe('QuestionMockPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('starts a timed question-level mock and saves an answer with navigation state', async () => {
    const { refresh } = await setup();
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Select all listed' }));
    await user.click(screen.getByRole('button', { name: 'Start timed mock' }));
    expect(await screen.findByText(/Synthetic question-level mock/)).toBeInTheDocument();
    expect(mockedStart).toHaveBeenCalledTimes(1);
    expect(mockedStart.mock.calls[0][0].questionIds).toEqual([10]);

    await user.click(screen.getByRole('radio', { name: /Second choice/ }));
    await waitFor(() => expect(mockedSave).toHaveBeenCalled());
    expect(mockedSave.mock.calls[0][0].selectedAnswer).toBe('B');
    await user.click(screen.getByRole('button', { name: 'Mark for review' }));
    await waitFor(() =>
      expect(mockedSave.mock.calls.some((call) => call[0].markedForReview === true)).toBe(true),
    );
    await user.click(screen.getByRole('button', { name: 'Submit' }));
    await user.click(screen.getByRole('button', { name: 'Submit now' }));
    await waitFor(() => expect(mockedSubmit).toHaveBeenCalledTimes(1));
    expect(
      await screen.findByRole('heading', {
        level: 2,
        name: 'Synthetic question-level mock results',
      }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Synthetic official explanation/)).toBeInTheDocument();
    expect(screen.getByText(/Compatible pattern attempts/)).toBeInTheDocument();
    expect(mockedSubmit.mock.calls[0][0].automatic).toBe(false);
    expect(refresh).toHaveBeenCalled();
  });

  it('shows an empty state when no eligible questions exist', async () => {
    mockedCandidates.mockResolvedValue([]);
    render(
      <QuestionMockPanel
        examId={1}
        mocks={[]}
        pattern={PATTERN}
        refresh={vi.fn().mockResolvedValue(undefined)}
        today={TODAY}
      />,
    );
    expect(await screen.findByText('No eligible question-level questions')).toBeInTheDocument();
  });
});
