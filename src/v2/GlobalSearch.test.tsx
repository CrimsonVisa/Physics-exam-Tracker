import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2ExamSummary, V2GlobalSearchResult } from './api';
import type { V2View } from './navigation';
import { GlobalSearch } from './GlobalSearch';

vi.mock('./api', () => ({
  globalSearchV2: vi.fn(),
}));

import * as api from './api';

const mockedSearch = vi.mocked(api.globalSearchV2);

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
    name: 'GATE Physics',
    shortName: 'GATE PH',
    authority: '',
    officialUrl: '',
    displayOrder: 2,
    isActive: true,
    examDate: null,
  },
];

function results(overrides: Partial<V2GlobalSearchResult> = {}): V2GlobalSearchResult {
  return {
    query: 'vector',
    examId: null,
    syllabus: [
      {
        nodeId: 3,
        examId: 1,
        examCode: 'csir-net-physics',
        examShortName: 'CSIR-NET',
        examIsActive: true,
        title: 'Vector calculus',
        nodeCode: 'T1',
        nodeKind: 'topic',
        breadcrumb: 'Mechanics',
        progressStatus: 'revised',
      },
    ],
    concepts: [
      {
        conceptId: 7,
        slug: 'kinematics',
        title: 'Kinematics',
        domain: 'Classical Mechanics',
        masteryScore: 65,
        evidenceCount: 3,
        examCodes: ['csir-net-physics'],
      },
    ],
    pyqs: [
      {
        questionId: 8,
        examId: 1,
        examCode: 'csir-net-physics',
        examShortName: 'CSIR-NET',
        year: 2026,
        sessionLabel: 'December',
        questionNumber: 'Q1',
        sectionTitle: 'Part A',
        statementSnippet: 'A vector question about motion.',
      },
    ],
    resources: [
      {
        resourceId: 9,
        title: 'Vector notes',
        resourceType: 'notes',
        subjectArea: 'Mathematical Methods',
        examCodes: ['gate-physics'],
      },
    ],
    flashcards: [],
    ...overrides,
  };
}

function renderSearch(
  overrides: Partial<{
    result: V2GlobalSearchResult;
    error: Error;
    onNavigate: (view: V2View) => void;
  }> = {},
) {
  const onClose = vi.fn();
  const onNavigate: (view: V2View) => void = overrides.onNavigate ?? vi.fn();
  if (overrides.error) {
    mockedSearch.mockRejectedValue(overrides.error);
  } else {
    mockedSearch.mockResolvedValue(overrides.result ?? results());
  }
  render(<GlobalSearch exams={EXAMS} onClose={onClose} onNavigate={onNavigate} />);
  return { onClose, onNavigate };
}

async function typeQuery(query: string) {
  const input = await screen.findByRole('combobox', { name: 'Search Physics Tracker' });
  input.focus();
  await userEvent.setup().type(input, query);
  return input;
}

describe('GlobalSearch', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('waits for two characters, debounces, and queries once per settled input', async () => {
    renderSearch();
    const input = await screen.findByRole('combobox', { name: 'Search Physics Tracker' });
    input.focus();
    await userEvent.setup().type(input, 'v');
    expect(mockedSearch).not.toHaveBeenCalled();
    expect(screen.getByText(/Type at least two characters/)).toBeInTheDocument();

    await userEvent.setup().type(input, 'ector');
    await waitFor(() =>
      expect(mockedSearch).toHaveBeenCalledWith({
        query: 'vector',
        examId: null,
        limitPerCategory: 6,
      }),
    );
    expect(mockedSearch).toHaveBeenCalledTimes(1);
  });

  it('groups results with workspace identity and useful secondary text', async () => {
    renderSearch();
    await typeQuery('vector');
    const firstOption = await screen.findByRole('option', { name: /Vector calculus/ });
    expect(firstOption.textContent).toContain('Mechanics · Topic · CSIR-NET · revised');
    for (const group of [
      'Syllabus topics and units',
      'Concepts',
      'Previous-year questions',
      'Resources',
    ]) {
      expect(screen.getByText(group)).toBeInTheDocument();
    }
    const options = screen.getAllByRole('option').map((option) => option.textContent ?? '');
    expect(options.some((text) => text.includes('Mastery 65.0 / 100'))).toBe(true);
    expect(options.some((text) => text.includes('A vector question about motion'))).toBe(true);
    expect(options.some((text) => text.includes('CSIR-NET'))).toBe(true);
    expect(screen.getByText('4 results')).toBeInTheDocument();
  });

  it('navigates with arrow keys and Enter to the exact workspaces', async () => {
    const { onNavigate } = renderSearch();
    const input = await typeQuery('vector');
    await screen.findByRole('option', { name: /Vector calculus/ });

    expect(input).toHaveAttribute('aria-activedescendant', 'v2-search-option-0');
    await userEvent.keyboard('{ArrowDown}{ArrowDown}{Enter}');
    expect(onNavigate).toHaveBeenCalledWith({
      kind: 'practice',
      examCode: 'csir-net-physics',
      questionId: 8,
    });
  });

  it('wraps upward navigation to the last result group', async () => {
    const { onNavigate } = renderSearch();
    const input = await typeQuery('vector');
    await screen.findByRole('option', { name: /Vector calculus/ });
    expect(input).toHaveAttribute('aria-activedescendant', 'v2-search-option-0');
    await userEvent.keyboard('{ArrowUp}{Enter}');
    expect(onNavigate).toHaveBeenCalledWith({
      kind: 'resources',
      query: 'Vector notes',
    });
  });

  it('opens shared concepts regardless of the search scope', async () => {
    const { onNavigate } = renderSearch();
    await typeQuery('vector');
    await screen.findByText('Kinematics');
    await userEvent.keyboard('{ArrowDown}{Enter}');
    expect(onNavigate).toHaveBeenCalledWith({
      kind: 'concepts',
      examCode: null,
      conceptSlug: 'kinematics',
    });

    const scope = screen.getByRole('combobox', { name: 'Exam scope' });
    mockedSearch.mockResolvedValue(results());
    await userEvent.setup().selectOptions(scope, '2');
    await waitFor(() =>
      expect(mockedSearch).toHaveBeenLastCalledWith({
        query: 'vector',
        examId: 2,
        limitPerCategory: 6,
      }),
    );
  });

  it('navigates to a flashcard result and labels shared cards', async () => {
    const { onNavigate } = renderSearch({
      result: results({
        flashcards: [
          {
            flashcardId: 21,
            frontSnippet: 'Vector field definition',
            backSnippet: 'A vector assigned to each point in space.',
            subjectArea: 'Mathematical Methods',
            examCodes: [],
          },
        ],
      }),
    });
    await typeQuery('vector');
    const option = await screen.findByRole('option', { name: /Vector field definition/ });
    expect(option.textContent).toContain('Shared');
    await userEvent.setup().click(option);
    expect(onNavigate).toHaveBeenCalledWith({ kind: 'flashcards', flashcardId: 21 });
  });

  it('shows the no-match empty state and closes on Escape', async () => {
    const { onClose } = renderSearch({
      result: results({ syllabus: [], concepts: [], pyqs: [], resources: [] }),
    });
    await typeQuery('zzzz');
    expect(await screen.findByText('No matches for “zzzz”.')).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalled();
  });

  it('reports backend errors with a working retry', async () => {
    renderSearch({
      error: Object.assign(new Error('Search backend offline.'), {
        code: 'command_failed',
      }),
    });
    await typeQuery('gamma');
    expect(await screen.findByRole('alert')).toHaveTextContent('Search backend offline.');
    mockedSearch.mockResolvedValue(results());
    await userEvent.setup().click(screen.getByRole('button', { name: 'Retry search' }));
    expect(await screen.findByRole('option', { name: /Vector calculus/ })).toBeInTheDocument();
  });
});
