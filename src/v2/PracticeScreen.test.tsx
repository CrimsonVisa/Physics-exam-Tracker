import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  V2ExamSummary,
  V2MockTest,
  V2PracticeAnalytics,
  V2PracticeOptions,
  V2PyqQuestionDetail,
} from './api';
import { PracticeScreen } from './PracticeScreen';

vi.mock('./api', () => ({
  getV2PracticeOptions: vi.fn(),
  listV2PyqQuestions: vi.fn(),
  listV2MockTests: vi.fn(),
  getV2PracticeAnalytics: vi.fn(),
  getV2PyqQuestionDetail: vi.fn(),
  saveV2PyqQuestion: vi.fn(),
  archiveV2PyqQuestion: vi.fn(),
  recordV2PyqAttempt: vi.fn(),
  previewV2PyqImport: vi.fn(),
  applyV2PyqImport: vi.fn(),
  saveV2MockTest: vi.fn(),
  setV2MockTestStatus: vi.fn(),
  archiveV2MockTest: vi.fn(),
  listV2QuestionMockCandidates: vi.fn(),
  startV2QuestionMock: vi.fn(),
  getV2QuestionMockSession: vi.fn(),
  saveV2QuestionMockState: vi.fn(),
  submitV2QuestionMock: vi.fn(),
}));

vi.mock('./SourcesPanel', () => ({
  SourcesPanel: () => <div>Official sources panel</div>,
}));

import * as api from './api';

const mockedOptions = vi.mocked(api.getV2PracticeOptions);
const mockedQuestions = vi.mocked(api.listV2PyqQuestions);
const mockedMocks = vi.mocked(api.listV2MockTests);
const mockedAnalytics = vi.mocked(api.getV2PracticeAnalytics);
const mockedSaveQuestion = vi.mocked(api.saveV2PyqQuestion);
const mockedSaveMock = vi.mocked(api.saveV2MockTest);
const mockedQuestionCandidates = vi.mocked(api.listV2QuestionMockCandidates);

const TODAY = '2026-11-05';

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
  {
    id: 2,
    code: 'gate-physics',
    name: 'GATE Physics (PH)',
    shortName: 'GATE PH',
    authority: 'IIT',
    officialUrl: 'https://gate.iitm.ac.in/',
    displayOrder: 2,
    isActive: true,
    examDate: null,
  },
  {
    id: 3,
    code: 'tifr-gs-physics',
    name: 'TIFR Graduate School Physics',
    shortName: 'TIFR GS',
    authority: 'TIFR',
    officialUrl: 'https://www.tifr.res.in/academics/',
    displayOrder: 3,
    isActive: true,
    examDate: null,
  },
  {
    id: 4,
    code: 'jest-physics',
    name: 'JEST Physics',
    shortName: 'JEST',
    authority: 'JEST Consortium',
    officialUrl: 'https://www.jest.org.in/',
    displayOrder: 4,
    isActive: true,
    examDate: null,
  },
];

const OPTIONS: V2PracticeOptions = {
  exam: {
    id: 1,
    code: 'csir-net-physics',
    name: 'CSIR-UGC NET Physical Sciences',
    shortName: 'CSIR-NET',
    isActive: true,
  },
  patterns: [
    {
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
      notes: 'Verified.',
      sections: [
        {
          id: 10,
          sectionId: 3,
          code: 'csir-705-part-a',
          title: 'Part A',
          displayOrder: 1,
          questionCount: 20,
          maxAttempts: 15,
          maxMarks: 30,
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
    },
    {
      id: 2,
      patternCode: 'csir-net-physical-sciences-705',
      patternTitle: 'CSIR-UGC NET Physical Sciences (705)',
      variant: 'Physical Sciences',
      versionLabel: 'Next session',
      verificationStatus: 'pending',
      effectiveFrom: null,
      durationMinutes: null,
      totalMarks: 0,
      totalQuestions: 0,
      attemptLimit: null,
      sourceTitle: '',
      sourceUrl: null,
      sourceReference: '',
      verifiedOn: null,
      notes: 'Pending.',
      sections: [],
    },
  ],
  sections: [{ id: 3, code: 'csir-705-part-a', title: 'Part A', defaultQuestionTypeId: 1 }],
  questionTypes: [{ id: 1, code: 'MCQ', title: 'Multiple Choice', answerFormat: 'single_option' }],
  topics: [
    {
      id: 13,
      code: 'csir-part-a-core-classical-mechanics',
      title: 'Classical Mechanics',
      nodeKind: 'topic',
      category: 'Core',
      parentId: 2,
      displayOrder: 1,
    },
  ],
  concepts: [
    {
      id: 1,
      slug: 'classical-mechanics',
      title: 'Classical Mechanics',
      domain: 'Classical Mechanics',
    },
  ],
  resources: [],
};

const DETAIL: V2PyqQuestionDetail = {
  question: {
    id: 8,
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
    statement: 'Synthetic test-only question.',
    maxMarks: 2,
    negativeMarks: -0.5,
    unansweredMarks: 0,
    difficulty: 3,
    sourceTitle: 'Synthetic fixture',
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
    conceptCount: 1,
    syllabusNodeId: 13,
    syllabusNodeTitle: 'Classical Mechanics',
    topicIds: [13],
    options: [
      { key: 'A', text: 'First choice' },
      { key: 'B', text: 'Second choice' },
      { key: 'C', text: 'Third choice' },
      { key: 'D', text: 'Fourth choice' },
    ],
  },
  correctAnswer: 'B',
  topics: [
    {
      nodeId: 13,
      nodeCode: 'csir-part-a-core-classical-mechanics',
      nodeTitle: 'Classical Mechanics',
      isPrimary: true,
    },
  ],
  concepts: [
    { conceptId: 1, slug: 'classical-mechanics', title: 'Classical Mechanics', isPrimary: true },
  ],
  attempts: [],
};

const ANALYTICS: V2PracticeAnalytics = {
  examId: 1,
  pyqAttempts: 1,
  distinctQuestions: 1,
  correct: 1,
  incorrect: 0,
  unanswered: 0,
  accuracyPercentage: 100,
  marksObtained: 2,
  maxMarksAttempted: 2,
  percentage: 100,
  timeSpentSeconds: 30,
  averageSecondsPerQuestion: 30,
  verifiedQuestions: 1,
  verifiedQuestionsInScope: 1,
  totalQuestionsInScope: 1,
  verificationCoveragePercentage: 100,
  byYear: [],
  bySection: [],
  byTopic: [],
  pyqTrend: [],
  mocks: {
    completedCount: 0,
    averagePercentage: null,
    bestPercentage: null,
    lastPercentage: null,
    byPattern: [],
    history: [],
    insufficientData: true,
  },
  insufficientPyqData: true,
};

const MOCK: V2MockTest = {
  id: 1,
  examId: 1,
  patternVersionId: 1,
  patternVersionLabel: 'December 2026',
  patternTitle: 'CSIR-UGC NET Physical Sciences (705)',
  focusNodeId: null,
  focusNodeTitle: null,
  testName: 'Full-length mock',
  testDate: TODAY,
  durationMinutes: 180,
  testMode: 'full_length',
  maxMarks: 30,
  marksObtained: 18.5,
  percentage: 61.667,
  status: 'completed',
  notes: '',
  reviewObservations: '',
  isArchived: false,
  availableQuestions: 15,
  attemptedQuestions: 13,
  correctQuestions: 10,
  incorrectQuestions: 3,
  unansweredQuestions: 2,
  startedAt: null,
  endedAt: null,
  schemeSnapshotJson: '{}',
  mockKind: 'aggregate',
  deadlineAt: null,
  currentQuestionId: null,
  lastActivityAt: null,
  submissionKind: '',
  sections: [],
  scoreHistory: [],
};

async function setup() {
  mockedOptions.mockResolvedValue(OPTIONS);
  mockedQuestions.mockResolvedValue([]);
  mockedMocks.mockResolvedValue([]);
  mockedAnalytics.mockResolvedValue(ANALYTICS);
  mockedQuestionCandidates.mockResolvedValue([]);
  render(<PracticeScreen exams={EXAMS} initialExamCode="csir-net-physics" today={TODAY} />);
  await screen.findAllByText(/December 2026/);
}

describe('PracticeScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows the verified pattern and empty PYQ state', async () => {
    await setup();
    expect(screen.getByText(/180 minutes/)).toBeInTheDocument();
    expect(screen.getByText('No PYQ questions match')).toBeInTheDocument();
    expect(screen.getByText(/No question content is stored/)).toBeInTheDocument();
  });

  it('saves a manually entered question and refreshes the bank', async () => {
    await setup();
    mockedSaveQuestion.mockResolvedValue(DETAIL);
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Add question' }));
    await user.type(screen.getByLabelText('Question number'), '1');
    await user.type(screen.getByLabelText('Question statement'), 'Synthetic test-only question.');
    await user.type(screen.getByLabelText('Correct answer'), 'B');
    await user.click(screen.getByRole('button', { name: 'Save question' }));
    await waitFor(() => expect(mockedSaveQuestion).toHaveBeenCalledTimes(1));
    expect(await screen.findByText(/Question saved/)).toBeInTheDocument();
  });

  it('disables pending patterns and lets a verified pattern create a mock', async () => {
    await setup();
    mockedSaveMock.mockResolvedValue(MOCK);
    const user = userEvent.setup();
    await user.click(screen.getByRole('tab', { name: 'Mock tests' }));
    await user.click(screen.getByRole('tab', { name: 'Aggregate result entry' }));
    expect(screen.getByRole('button', { name: /Next session/ })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: /December 2026/ }));
    await user.click(screen.getByRole('button', { name: 'Save planned mock' }));
    await waitFor(() => expect(mockedSaveMock).toHaveBeenCalledTimes(1));
    const input = mockedSaveMock.mock.calls[0][0];
    expect(input.patternVersionId).toBe(1);
    expect(input.sections[0].patternSectionId).toBe(10);
  });

  it('renders persisted analytics without inventing a trend', async () => {
    await setup();
    const user = userEvent.setup();
    await user.click(screen.getByRole('tab', { name: 'Practice analytics' }));
    expect(screen.getAllByText('100.00%').length).toBeGreaterThan(0);
    expect(screen.getByText(/Trend requires at least two attempts/)).toBeInTheDocument();
  });
});

describe('PYQ deep links', () => {
  it('opens the correct exam, PYQ tab and selected question from a search result', async () => {
    mockedOptions.mockResolvedValue(OPTIONS);
    mockedQuestions.mockResolvedValue([]);
    mockedMocks.mockResolvedValue([]);
    mockedAnalytics.mockResolvedValue(ANALYTICS);
    mockedQuestionCandidates.mockResolvedValue([]);
    vi.mocked(api.getV2PyqQuestionDetail).mockResolvedValue(DETAIL);

    render(
      <PracticeScreen
        exams={EXAMS}
        initialExamCode="csir-net-physics"
        initialQuestionId={8}
        today={TODAY}
      />,
    );

    await waitFor(() => expect(vi.mocked(api.getV2PyqQuestionDetail)).toHaveBeenCalledWith(1, 8));
    expect(await screen.findByText('Synthetic test-only question.')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'PYQ bank & attempts' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
  });
});
