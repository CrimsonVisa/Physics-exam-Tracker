import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  V2ConceptDetail,
  V2ConceptMasterySummary,
  V2ConceptSearchResult,
  V2ConceptSummary,
  V2ConceptSyllabusMapping,
  V2ExamConceptCoverage,
  V2ExamSummary,
} from './api';
import { ConceptsScreen } from './ConceptsScreen';

vi.mock('./api', () => ({
  searchV2Concepts: vi.fn(),
  getV2ConceptDetail: vi.fn(),
  updateV2ConceptMapping: vi.fn(),
  updateV2ConceptMastery: vi.fn(),
  replayV2ConceptMastery: vi.fn(),
  verifyV2MasteryRun: vi.fn(),
}));

import * as api from './api';

const mockedSearch = vi.mocked(api.searchV2Concepts);
const mockedDetail = vi.mocked(api.getV2ConceptDetail);
const mockedMappingUpdate = vi.mocked(api.updateV2ConceptMapping);
const mockedUpdate = vi.mocked(api.updateV2ConceptMastery);
const mockedReplay = vi.mocked(api.replayV2ConceptMastery);
const mockedVerify = vi.mocked(api.verifyV2MasteryRun);

const TODAY = '2026-11-04';

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

function coverage(
  exam: V2ExamSummary,
  overrides: Partial<V2ExamConceptCoverage> = {},
): V2ExamConceptCoverage {
  return {
    examId: exam.id,
    examCode: exam.code,
    examName: exam.name,
    examShortName: exam.shortName,
    examIsActive: exam.isActive,
    configured: false,
    status: 'not_configured',
    contentVersionLabel: null,
    contentReviewStatus: null,
    contentVerifiedOn: null,
    nodeCount: 0,
    topicNodeCount: 0,
    revisedNodeCount: 0,
    mappedConceptCount: 0,
    verifiedMappedConceptCount: 0,
    ...overrides,
  };
}

function concept(overrides: Partial<V2ConceptSummary> = {}): V2ConceptSummary {
  return {
    id: 7,
    slug: 'kinematics',
    title: 'Kinematics',
    description: 'Description of motion using position, velocity, and acceleration.',
    domain: 'Classical Mechanics',
    parentConceptId: 1,
    parentSlug: 'classical-mechanics',
    parentTitle: 'Classical Mechanics',
    difficulty: 1,
    isArchived: false,
    mastery: null,
    examCodes: [],
    verifiedExamCodes: [],
    ...overrides,
  };
}

function searchResult(
  concepts: V2ConceptSummary[],
  coverages = EXAMS.map((exam) => coverage(exam)),
): V2ConceptSearchResult {
  return {
    concepts,
    total: concepts.length,
    domains: ['Classical Mechanics', 'Quantum Mechanics'],
    examCoverage: coverages,
  };
}

function mastery(overrides: Partial<V2ConceptMasterySummary> = {}): V2ConceptMasterySummary {
  return {
    conceptId: 7,
    masteryScore: 65,
    confidence: 2,
    evidenceCount: 1,
    lastEvidenceAt: '2026-11-04T10:00:00.000Z',
    lastReviewedOn: TODAY,
    lastRecalculatedAt: '2026-11-04T10:00:00.000Z',
    lastRecalcRunId: 1,
    algorithmVersion: '1.0',
    ...overrides,
  };
}

function mapping(overrides: Partial<V2ConceptSyllabusMapping> = {}): V2ConceptSyllabusMapping {
  return {
    examId: 1,
    examCode: 'csir-net-physics',
    examName: 'CSIR-UGC NET Physical Sciences',
    examShortName: 'CSIR-NET',
    examIsActive: true,
    nodeId: 12,
    nodeCode: 'T1',
    nodeTitle: 'Kinematics Topic',
    nodeKind: 'topic',
    nodeCategory: 'Mechanics',
    nodeDisplayOrder: 1,
    nodeIsArchived: false,
    nodePath: ['Mechanics Unit', 'Kinematics Topic'],
    conceptId: 5,
    weight: 2,
    isPrimary: true,
    notes: 'Curated relationship',
    status: 'proposed',
    origin: 'curated',
    sourceReference: 'https://example.com/syllabus',
    provenance: 'Curated crosswalk from official wording; review required.',
    contentVersionId: 1,
    reviewedAt: null,
    reviewedBy: '',
    updatedAt: '2026-11-04T00:00:00Z',
    sourceTitle: 'Test syllabus source',
    sourcePublisher: 'Test publisher',
    sourceUrl: null,
    sourceVerifiedOn: '2026-11-04',
    contentVersionLabel: 'v1',
    contentReviewStatus: 'verified',
    contentVerifiedOn: '2026-11-04',
    isApproved: false,
    isVerified: true,
    progressStatus: 'revised',
    ...overrides,
  };
}

function detail(
  baseConcept: V2ConceptSummary,
  overrides: Partial<V2ConceptDetail> = {},
): V2ConceptDetail {
  return {
    concept: baseConcept,
    children: [],
    mappings: [],
    evidence: [],
    masteryHistory: [],
    ...overrides,
  };
}

function evidence() {
  return {
    id: 11,
    sourceKind: 'manual',
    sourceId: null,
    sourceVersion: 1,
    examId: null,
    examCode: null,
    examShortName: null,
    localDate: TODAY,
    occurredAt: '2026-11-04T10:00:00.000Z',
    rawValue: { target_score: 65 },
    algorithmVersionId: 1,
    algorithmVersion: '1.0',
    notes: 'Manual check',
    createdAt: '2026-11-04T10:00:00.000Z',
    isVoided: false,
    voidReason: null,
    voidedAt: null,
    replacementEvidenceId: null,
  };
}

function run() {
  return {
    id: 3,
    reason: 'manual_override',
    algorithmVersionId: 1,
    algorithmVersion: '1.0',
    evidenceCount: 1,
    evidenceFingerprint: 'a'.repeat(64),
    resultingScore: 65,
    resultingConfidence: 2,
    startedAt: '2026-11-04T10:00:00.000Z',
    finishedAt: '2026-11-04T10:00:00.000Z',
    notes: '',
  };
}

async function renderLibrary(result = searchResult([concept()])) {
  mockedSearch.mockResolvedValue(result);
  render(<ConceptsScreen exams={EXAMS} today={TODAY} />);
  await screen.findByRole('heading', { level: 1, name: 'Concept library' });
}

describe('V2 concept library', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedSearch.mockResolvedValue(searchResult([concept()]));
  });

  it('browses concepts, shows missing mappings, and searches by exam', async () => {
    const user = userEvent.setup();
    await renderLibrary(searchResult([concept()]));

    expect(await screen.findByText('Kinematics')).toBeInTheDocument();
    expect(screen.getAllByText('No exam mappings').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('No mastery evidence').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Official syllabus not configured')).toHaveLength(4);

    const searchInput = screen.getByLabelText('Search concepts');
    await user.type(searchInput, 'quantum');
    await user.click(screen.getByRole('button', { name: 'Search' }));
    await waitFor(() =>
      expect(mockedSearch).toHaveBeenLastCalledWith(
        expect.objectContaining({ query: 'quantum', limit: 100 }),
      ),
    );

    await user.selectOptions(screen.getByLabelText('Examination'), 'gate-physics');
    await waitFor(() =>
      expect(mockedSearch).toHaveBeenLastCalledWith(
        expect.objectContaining({ query: 'quantum', examCode: 'gate-physics' }),
      ),
    );
  });

  it('opens a concept detail with verified mapping and shared mastery', async () => {
    const user = userEvent.setup();
    const mastered = concept({
      mastery: mastery(),
      examCodes: ['csir-net-physics'],
      verifiedExamCodes: ['csir-net-physics'],
    });
    mockedDetail.mockResolvedValue(
      detail(mastered, {
        mappings: [mapping()],
        evidence: [evidence()],
        masteryHistory: [run()],
      }),
    );
    await renderLibrary(searchResult([mastered]));

    await user.click(await screen.findByRole('button', { name: /Kinematics/ }));

    expect(
      await screen.findByRole('heading', { level: 1, name: 'Kinematics' }),
    ).toBeInTheDocument();
    expect(screen.getByText('65.0')).toBeInTheDocument();
    expect(screen.getByText(/Mechanics Unit → Kinematics Topic/)).toBeInTheDocument();
    expect(screen.getByText('Proposed', { selector: 'span' })).toBeInTheDocument();
    expect(screen.getByText('Syllabus content verified')).toBeInTheDocument();
    expect(screen.getByText('Manual override')).toBeInTheDocument();
    expect(screen.getByText('1 calculation')).toBeInTheDocument();

    mockedVerify.mockResolvedValue({
      runId: 3,
      matches: true,
      storedScore: 65,
      recomputedScore: 65,
      storedConfidence: 2,
      recomputedConfidence: 2,
      storedFingerprint: 'a'.repeat(64),
      recomputedFingerprint: 'a'.repeat(64),
      resultCount: 1,
      mismatches: [],
    });
    await user.click(screen.getByRole('button', { name: 'Verify replay' }));
    expect(
      await screen.findByText(
        'Replay verified: stored score, confidence, fingerprint, and ordered results match.',
      ),
    ).toBeInTheDocument();
  });

  it('reviews a proposed mapping without auto-approving it', async () => {
    const user = userEvent.setup();
    const proposed = mapping();
    mockedDetail.mockResolvedValue(detail(concept(), { mappings: [proposed] }));
    mockedMappingUpdate.mockResolvedValue(
      detail(concept(), {
        mappings: [
          mapping({
            status: 'approved',
            isApproved: true,
            reviewedAt: '2026-11-04T12:00:00Z',
            reviewedBy: 'local reviewer',
          }),
        ],
      }),
    );
    await renderLibrary();

    await user.click(await screen.findByRole('button', { name: /Kinematics/ }));
    expect(await screen.findByText('Proposed', { selector: 'span' })).toBeInTheDocument();
    await user.selectOptions(screen.getByLabelText('Mapping status'), 'approved');
    await user.click(screen.getByRole('button', { name: 'Save review' }));

    await waitFor(() =>
      expect(mockedMappingUpdate).toHaveBeenCalledWith({
        examId: 1,
        nodeId: 12,
        conceptId: 5,
        status: 'approved',
        notes: 'Curated relationship',
      }),
    );
    expect(
      await screen.findByText('Mapping approved. It is now eligible for revision planning.'),
    ).toBeInTheDocument();
  });

  it('distinguishes missing evidence from low mastery', async () => {
    const user = userEvent.setup();
    mockedDetail.mockResolvedValue(detail(concept()));
    await renderLibrary();

    await user.click(await screen.findByRole('button', { name: /Kinematics/ }));

    expect(await screen.findByText('No mastery evidence yet')).toBeInTheDocument();
    expect(
      screen.getByText(
        'Absence of evidence is not the same as low mastery. No score is fabricated for this concept until explicit evidence is recorded.',
      ),
    ).toBeInTheDocument();
    expect(screen.queryByText('0.0')).not.toBeInTheDocument();
  });

  it('shows an unverified syllabus mapping explicitly', async () => {
    const user = userEvent.setup();
    const unverified = concept({
      examCodes: ['gate-physics'],
      verifiedExamCodes: [],
    });
    mockedDetail.mockResolvedValue(
      detail(unverified, {
        mappings: [
          mapping({
            examId: 2,
            examCode: 'gate-physics',
            examShortName: 'GATE PH',
            contentReviewStatus: 'draft',
            contentVerifiedOn: null,
            isVerified: false,
          }),
        ],
      }),
    );
    await renderLibrary(searchResult([unverified]));
    await user.click(await screen.findByRole('button', { name: /Kinematics/ }));

    expect(await screen.findByText('Proposed', { selector: 'span' })).toBeInTheDocument();
    expect(screen.getByText(/content v1 \(draft\)/)).toBeInTheDocument();
    expect(screen.getByText('Syllabus content unverified')).toBeInTheDocument();
    expect(
      screen.queryByText(/No exam-specific syllabus mapping is configured for this concept/),
    ).not.toBeInTheDocument();
  });

  it('records manual mastery evidence and refreshes the concept', async () => {
    const user = userEvent.setup();
    const initial = concept();
    const updated = concept({ mastery: mastery({ masteryScore: 72.5 }) });
    mockedDetail.mockResolvedValueOnce(detail(initial)).mockResolvedValue(
      detail(updated, {
        evidence: [evidence()],
        masteryHistory: [run()],
      }),
    );
    mockedUpdate.mockResolvedValue({
      conceptId: initial.id,
      runId: 3,
      mastery: mastery({ masteryScore: 72.5 }),
    });
    await renderLibrary();
    await user.click(await screen.findByRole('button', { name: /Kinematics/ }));

    await user.click(screen.getByText('Record manual mastery evidence'));
    const target = screen.getByLabelText('Manual mastery target (0–100)');
    await user.clear(target);
    await user.type(target, '72.5');
    await user.click(screen.getByRole('button', { name: 'Record manual evidence' }));

    await waitFor(() =>
      expect(mockedUpdate).toHaveBeenCalledWith({
        conceptId: 7,
        targetScore: 72.5,
        localDate: TODAY,
        notes: '',
      }),
    );
    expect(
      await screen.findByText(
        'Manual evidence recorded. Shared mastery was recalculated from the full active evidence set.',
      ),
    ).toBeInTheDocument();
    expect(screen.getByText('72.5')).toBeInTheDocument();
  });

  it('recalculates mastery from evidence', async () => {
    const user = userEvent.setup();
    mockedDetail.mockResolvedValue(detail(concept({ mastery: mastery() })));
    mockedReplay.mockResolvedValue({
      conceptId: 7,
      runId: 4,
      mastery: mastery({ masteryScore: 66, lastRecalcRunId: 4 }),
    });
    await renderLibrary();
    await user.click(await screen.findByRole('button', { name: /Kinematics/ }));

    await user.click(screen.getByRole('button', { name: 'Recalculate from evidence' }));
    await waitFor(() => expect(mockedReplay).toHaveBeenCalledWith({ conceptId: 7 }));
    expect(
      await screen.findByText('Mastery was recalculated from the current active evidence.'),
    ).toBeInTheDocument();
  });

  it('shows an empty state when no concepts match', async () => {
    await renderLibrary(searchResult([]));

    expect(
      await screen.findByText(
        'No concepts match these filters. Clear the search or choose a different examination.',
      ),
    ).toBeInTheDocument();
  });

  it('shows an understandable library error', async () => {
    mockedSearch.mockRejectedValue({
      code: 'database_error',
      message: 'The concept library is unavailable.',
    });
    render(<ConceptsScreen exams={EXAMS} today={TODAY} />);

    expect(await screen.findByText('The concept library is unavailable.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Retry concept search' })).toBeInTheDocument();
  });
});

describe('Concept deep links', () => {
  it('opens a shared concept directly from a search result regardless of exam scope', async () => {
    const user = userEvent.setup();
    const onOpenFlashcards = vi.fn();
    const shared = concept();
    mockedSearch.mockResolvedValue(searchResult([shared]));
    mockedDetail.mockResolvedValue(detail(shared));

    render(
      <ConceptsScreen
        exams={EXAMS}
        initialConceptSlug={shared.slug}
        initialExamCode={null}
        onOpenFlashcards={onOpenFlashcards}
        today={TODAY}
      />,
    );

    expect(
      await screen.findByRole('heading', { level: 1, name: shared.title }),
    ).toBeInTheDocument();
    expect(mockedDetail).toHaveBeenCalledWith({ slug: shared.slug });
    expect(screen.getByRole('button', { name: /Back to concept library/ })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: `Open flashcards for ${shared.title}` }));
    expect(onOpenFlashcards).toHaveBeenCalledWith(shared.slug, shared.title);
  });
});
