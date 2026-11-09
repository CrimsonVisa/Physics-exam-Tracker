import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2ExamSummary, V2ExamSyllabus } from './api';
import { SyllabusScreen } from './SyllabusScreen';

vi.mock('./api', () => ({
  getV2ExamSyllabus: vi.fn(),
  updateV2SyllabusNodeProgress: vi.fn(),
}));

import * as api from './api';

const mockedSyllabus = vi.mocked(api.getV2ExamSyllabus);
const mockedUpdate = vi.mocked(api.updateV2SyllabusNodeProgress);

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

const EMPTY: V2ExamSyllabus = {
  examId: 1,
  examCode: 'csir-net-physics',
  examName: 'CSIR-UGC NET Physical Sciences',
  examIsActive: true,
  configured: false,
  contentVersionLabel: null,
  contentReviewStatus: null,
  nodes: [],
  summary: { nodeCount: 0, topicCount: 0, revisedOrConfidentCount: 0, mappedConceptCount: 0 },
};

const CONFIGURED: V2ExamSyllabus = {
  ...EMPTY,
  configured: true,
  contentVersionLabel: 'v1',
  contentReviewStatus: 'verified',
  nodes: [
    {
      id: 1,
      examId: 1,
      parentId: null,
      code: 'S1',
      title: 'Mechanics',
      nodeKind: 'unit',
      category: 'Mechanics',
      displayOrder: 1,
      readinessWeight: 1,
      isArchived: false,
      sourceTitle: 'Official source',
      sourceVerifiedOn: '2026-11-04',
      sourceUrl: 'https://example.com/syllabus.pdf',
      sourceReference: 'Official source reference',
      contentVersionLabel: 'v1',
      contentReviewStatus: 'verified',
      progressStatus: 'not_started',
      priority: 'normal',
      confidence: 3,
      notes: '',
      nodePath: ['Mechanics'],
    },
    {
      id: 2,
      examId: 1,
      parentId: 1,
      code: 'T1',
      title: 'Kinematics',
      nodeKind: 'topic',
      category: 'Mechanics',
      displayOrder: 1,
      readinessWeight: 1,
      isArchived: false,
      sourceTitle: 'Official source',
      sourceVerifiedOn: '2026-11-04',
      sourceUrl: 'https://example.com/syllabus.pdf',
      sourceReference: 'Official source reference',
      contentVersionLabel: 'v1',
      contentReviewStatus: 'verified',
      progressStatus: 'not_started',
      priority: 'normal',
      confidence: 3,
      notes: '',
      nodePath: ['Mechanics', 'Kinematics'],
    },
  ],
  summary: { nodeCount: 2, topicCount: 1, revisedOrConfidentCount: 0, mappedConceptCount: 0 },
};

describe('V2 syllabus coverage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows the explicit unconfigured state', async () => {
    mockedSyllabus.mockResolvedValue(EMPTY);
    render(<SyllabusScreen exams={EXAMS} />);

    expect(await screen.findByText('Official syllabus not configured')).toBeInTheDocument();
  });

  it('shows configured nodes and updates progress', async () => {
    const user = userEvent.setup();
    mockedSyllabus.mockResolvedValue(CONFIGURED);
    mockedUpdate.mockResolvedValue({
      ...CONFIGURED,
      nodes: [{ ...CONFIGURED.nodes[0], progressStatus: 'revised' }],
      summary: { ...CONFIGURED.summary, revisedOrConfidentCount: 1 },
    });
    render(<SyllabusScreen exams={EXAMS} />);

    expect(await screen.findByText('Kinematics')).toBeInTheDocument();
    expect(screen.getAllByText(/Official source \(2026-11-04\)/).length).toBeGreaterThan(0);
    await user.selectOptions(screen.getByLabelText('Progress for Kinematics'), 'revised');
    await waitFor(() =>
      expect(mockedUpdate).toHaveBeenCalledWith({
        examId: 1,
        nodeId: 2,
        status: 'revised',
        priority: 'normal',
        confidence: 3,
        notes: '',
      }),
    );
  });

  it('searches, filters, and collapses the syllabus hierarchy', async () => {
    const user = userEvent.setup();
    mockedSyllabus.mockResolvedValue(CONFIGURED);
    render(<SyllabusScreen exams={EXAMS} />);

    expect(await screen.findByLabelText('Progress for Kinematics')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Collapse all' }));
    expect(screen.queryByLabelText('Progress for Kinematics')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Expand all' }));
    expect(await screen.findByLabelText('Progress for Kinematics')).toBeInTheDocument();

    const search = screen.getByLabelText('Search syllabus');
    await user.type(search, 'Kinematics');
    expect(screen.getByText('1 matching node.')).toBeInTheDocument();
    expect(screen.getByLabelText('Progress for Kinematics')).toBeInTheDocument();

    await user.clear(search);
    await user.selectOptions(screen.getByLabelText('Completion filter'), 'revised');
    expect(screen.getByText('0 matching nodes.')).toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText('Completion filter'), 'not_started');
    expect(screen.getByText('2 matching nodes.')).toBeInTheDocument();
  });
});

describe('Syllabus deep links', () => {
  it('expands ancestors, scrolls to and focuses the exact syllabus node', async () => {
    mockedSyllabus.mockResolvedValue(CONFIGURED);
    render(
      <SyllabusScreen exams={EXAMS} initialExamCode="csir-net-physics" initialFocusNodeId={2} />,
    );

    const node = await waitFor(() => {
      const element = document.getElementById('v2-syllabus-node-2');
      expect(element).not.toBeNull();
      return element as HTMLElement;
    });
    await waitFor(() => expect(node).toHaveFocus());
    expect(node).toHaveClass('v2-syllabus-node-focused');
    expect(screen.getByText('Kinematics')).toBeInTheDocument();
  });

  it('navigates to flashcards for a topic', async () => {
    const user = userEvent.setup();
    const onOpenFlashcards = vi.fn();
    mockedSyllabus.mockResolvedValue(CONFIGURED);
    render(
      <SyllabusScreen
        exams={EXAMS}
        initialExamCode="csir-net-physics"
        onOpenFlashcards={onOpenFlashcards}
      />,
    );

    await user.click(await screen.findByRole('button', { name: 'Open flashcards for Kinematics' }));
    expect(onOpenFlashcards).toHaveBeenCalledWith('csir-net-physics', 2, 'Kinematics');
  });

  it('navigates to formulas for a topic using the stable node code', async () => {
    const user = userEvent.setup();
    const onOpenFormulas = vi.fn();
    mockedSyllabus.mockResolvedValue(CONFIGURED);
    render(
      <SyllabusScreen
        exams={EXAMS}
        initialExamCode="csir-net-physics"
        onOpenFormulas={onOpenFormulas}
      />,
    );

    await user.click(await screen.findByRole('button', { name: 'Open formulas for Kinematics' }));
    expect(onOpenFormulas).toHaveBeenCalledWith('csir-net-physics', 'T1', 'Kinematics');
  });

  it('navigates to terms for a topic using the stable node code', async () => {
    const user = userEvent.setup();
    const onOpenTerms = vi.fn();
    mockedSyllabus.mockResolvedValue(CONFIGURED);
    render(
      <SyllabusScreen exams={EXAMS} initialExamCode="csir-net-physics" onOpenTerms={onOpenTerms} />,
    );

    await user.click(await screen.findByRole('button', { name: 'Open terms for Kinematics' }));
    expect(onOpenTerms).toHaveBeenCalledWith('csir-net-physics', 'T1', 'Kinematics');
  });

  it('resolves a syllabus deep link by stable node code', async () => {
    mockedSyllabus.mockResolvedValue(CONFIGURED);
    render(
      <SyllabusScreen exams={EXAMS} initialExamCode="csir-net-physics" initialFocusNodeCode="T1" />,
    );

    const node = await waitFor(() => {
      const element = document.getElementById('v2-syllabus-node-2');
      expect(element).not.toBeNull();
      return element as HTMLElement;
    });
    await waitFor(() => expect(node).toHaveFocus());
    expect(node).toHaveClass('v2-syllabus-node-focused');
  });
});
