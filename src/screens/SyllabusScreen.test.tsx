import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SyllabusScreen from './SyllabusScreen';
import type {
  SyllabusCatalog,
  SyllabusFilter,
  SyllabusSummary,
  SyllabusTopic,
  TopicProgress,
  TopicProgressInput,
} from '../lib/syllabusApi';
import type { RevisionSummary } from '../lib/revisionApi';

const { invokeMock, openUrlMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  openUrlMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: openUrlMock,
}));

const CATALOG: SyllabusCatalog = {
  sources: [
    {
      code: 'csir-physical-sciences',
      title: 'Proposed Syllabus for Physical Sciences',
      publisher: 'CSIR Human Resource Development Group',
      url: 'https://csirhrdg.res.in/example-syllabus.pdf',
      landingUrl: 'https://csirhrdg.res.in/example-page',
      verifiedOn: '2026-10-29',
      notes: 'Official CSIR-HRDG PDF with Core and Advanced categories.',
    },
    {
      code: 'nta-csir-net-dec-2026',
      title: 'Joint CSIR-UGC NET December 2026 Information Bulletin',
      publisher: 'National Testing Agency',
      url: 'https://example.gov.in/bulletin.pdf',
      landingUrl: 'https://csirnet.nta.nic.in/',
      verifiedOn: '2026-10-29',
      notes: 'Official NTA bulletin with General Aptitude emphases.',
    },
  ],
  sections: [
    {
      code: 'GA',
      title: 'General Aptitude',
      sourceCode: 'nta-csir-net-dec-2026',
      sourceReference: 'NTA December 2026 bulletin — General Aptitude',
      displayOrder: 1,
    },
    {
      code: 'MMP',
      title: 'Mathematical Methods of Physics',
      sourceCode: 'csir-physical-sciences',
      sourceReference: 'CSIR syllabus PDF — Core and Advanced',
      displayOrder: 2,
    },
  ],
};

const DEFAULT_PROGRESS: TopicProgress = {
  status: 'Not started',
  priority: 'Normal',
  confidence: 3,
  notes: '',
  updatedAt: null,
  recorded: false,
};

const GA_TOPIC: SyllabusTopic = {
  id: 1,
  code: 'GA-01',
  sectionCode: 'GA',
  sectionTitle: 'General Aptitude',
  title: 'Logical reasoning',
  category: 'General Aptitude',
  sourceCode: 'nta-csir-net-dec-2026',
  sourceTitle: 'Joint CSIR-UGC NET December 2026 Information Bulletin',
  sourceUrl: 'https://example.gov.in/bulletin.pdf',
  sourceReference: 'NTA December 2026 bulletin — General Aptitude',
  progress: DEFAULT_PROGRESS,
};

const MMP_TOPIC: SyllabusTopic = {
  id: 2,
  code: 'MMP-C01',
  sectionCode: 'MMP',
  sectionTitle: 'Mathematical Methods of Physics',
  title: 'Dimensional analysis',
  category: 'Core',
  sourceCode: 'csir-physical-sciences',
  sourceTitle: 'Proposed Syllabus for Physical Sciences',
  sourceUrl: 'https://csirhrdg.res.in/example-syllabus.pdf',
  sourceReference: 'CSIR syllabus PDF — Core',
  progress: DEFAULT_PROGRESS,
};

const TOPICS = [GA_TOPIC, MMP_TOPIC];

const SUMMARY: SyllabusSummary = {
  totalTopics: 2,
  statusCounts: {
    notStarted: 2,
    learning: 0,
    revised: 0,
    confident: 0,
  },
  approximateProgress: 0,
  sections: [
    {
      code: 'GA',
      title: 'General Aptitude',
      totalTopics: 1,
      statusCounts: { notStarted: 1, learning: 0, revised: 0, confident: 0 },
      approximateProgress: 0,
    },
    {
      code: 'MMP',
      title: 'Mathematical Methods of Physics',
      totalTopics: 1,
      statusCounts: { notStarted: 1, learning: 0, revised: 0, confident: 0 },
      approximateProgress: 0,
    },
  ],
};

const UPDATED_SUMMARY: SyllabusSummary = {
  ...SUMMARY,
  statusCounts: { notStarted: 1, learning: 1, revised: 0, confident: 0 },
  approximateProgress: 25,
  sections: [
    {
      ...SUMMARY.sections[0],
      statusCounts: { notStarted: 0, learning: 1, revised: 0, confident: 0 },
      approximateProgress: 50,
    },
    SUMMARY.sections[1],
  ],
};

type MockOptions = {
  catalog?: SyllabusCatalog;
  summary?: SyllabusSummary | ((call: number) => SyllabusSummary);
  topics?: SyllabusTopic[] | ((filter: SyllabusFilter) => SyllabusTopic[]);
  revisions?: RevisionSummary[];
  failFirstCatalog?: boolean;
};

function installMock({
  catalog = CATALOG,
  summary = SUMMARY,
  topics = TOPICS,
  revisions = [],
  failFirstCatalog = false,
}: MockOptions = {}) {
  let summaryCalls = 0;
  let catalogFailed = false;

  invokeMock.mockImplementation((command: string, args?: unknown) => {
    if (command === 'get_syllabus_catalog') {
      if (failFirstCatalog && !catalogFailed) {
        catalogFailed = true;
        return Promise.reject(new Error('boom'));
      }
      return Promise.resolve(catalog);
    }

    if (command === 'get_syllabus_summary') {
      summaryCalls += 1;
      return Promise.resolve(typeof summary === 'function' ? summary(summaryCalls) : summary);
    }

    if (command === 'search_syllabus_topics') {
      const filter = (args as { filter: SyllabusFilter }).filter;
      return Promise.resolve(typeof topics === 'function' ? topics(filter) : topics);
    }

    if (command === 'get_revision_summaries') {
      return Promise.resolve(revisions);
    }

    return Promise.reject(new Error(`Unexpected command: ${command}`));
  });
}

describe('SyllabusScreen', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    openUrlMock.mockReset();
    openUrlMock.mockResolvedValue(undefined);
  });

  it('shows a loading state while the catalog and progress commands are pending', () => {
    invokeMock.mockReturnValue(new Promise(() => {}));

    render(<SyllabusScreen />);

    expect(screen.getByText('Loading syllabus…')).toBeInTheDocument();
    expect(screen.queryByLabelText('Search topics')).not.toBeInTheDocument();
  });

  it('renders the summary, filters, topic details, formula, priority note, and sources', async () => {
    installMock({ summary: UPDATED_SUMMARY, topics: [GA_TOPIC] });

    render(<SyllabusScreen />);

    expect(await screen.findByText('25.0%')).toBeInTheDocument();
    expect(screen.getByText('2 active topics')).toBeInTheDocument();
    expect(screen.getByText(/every active topic has equal weight/)).toBeInTheDocument();
    expect(screen.getByText(/Learning 0.5, Revised 0.75 and Confident 1/)).toBeInTheDocument();
    expect(
      screen.getByText(/not an official CSIR score or a marks prediction/),
    ).toBeInTheDocument();
    expect(screen.getByText(/High-Yield are your own planning labels/)).toBeInTheDocument();

    expect(screen.getByLabelText('Search topics')).toBeInTheDocument();
    expect(screen.getByLabelText('Filter by section')).toBeInTheDocument();
    expect(screen.getByLabelText('Filter by category')).toBeInTheDocument();
    expect(screen.getByLabelText('Filter by status')).toBeInTheDocument();
    expect(screen.getByLabelText('Filter by priority')).toBeInTheDocument();

    const topicButton = await screen.findByRole('button', { name: /GA-01/ });
    expect(topicButton).toHaveAttribute('aria-pressed', 'true');
    expect(
      screen.getByRole('heading', { name: 'Logical reasoning', level: 2 }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Status')).toHaveValue('Not started');
    expect(screen.getByLabelText('Priority')).toHaveValue('Normal');
    expect(screen.getByLabelText('Confidence (1–5)')).toHaveValue('3');

    expect(
      screen.getByRole('heading', { name: 'Proposed Syllabus for Physical Sciences' }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Those categories are separate from the exam question paper/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/General Aptitude topics use the NTA December 2026 bulletin/),
    ).toBeInTheDocument();
  });

  it('opens official sources through the system opener and reports opener failures', async () => {
    installMock();

    render(<SyllabusScreen />);

    const [officialDocument] = await screen.findAllByRole('link', { name: 'Official document' });
    await userEvent.click(officialDocument);
    expect(openUrlMock).toHaveBeenCalledWith(CATALOG.sources[0].url);

    openUrlMock.mockRejectedValueOnce(new Error('opener unavailable'));
    const [sourcePage] = screen.getAllByRole('link', { name: 'Source page' });
    await userEvent.click(sourcePage);
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Could not open this link in your browser.',
    );
  });

  it('shows an empty state when no topics match', async () => {
    installMock({
      summary: {
        ...SUMMARY,
        totalTopics: 0,
        statusCounts: { notStarted: 0, learning: 0, revised: 0, confident: 0 },
        sections: SUMMARY.sections.map((section) => ({
          ...section,
          totalTopics: 0,
          statusCounts: { notStarted: 0, learning: 0, revised: 0, confident: 0 },
          approximateProgress: 0,
        })),
      },
      topics: [],
    });

    render(<SyllabusScreen />);

    expect(await screen.findByText('No topics match these filters')).toBeInTheDocument();
    expect(screen.getByText('0 active topics')).toBeInTheDocument();
  });

  it('shows a load error and retries with fresh command calls', async () => {
    installMock({ failFirstCatalog: true });

    render(<SyllabusScreen />);

    expect(await screen.findByText('Syllabus could not be loaded')).toBeInTheDocument();
    expect(
      screen.getByText('The local database is unavailable. Open the desktop app and try again.'),
    ).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));

    expect(await screen.findByRole('button', { name: /GA-01/ })).toBeInTheDocument();
    expect(
      invokeMock.mock.calls.filter(([command]) => command === 'get_syllabus_catalog'),
    ).toHaveLength(2);
  });

  it('passes section and search filters to the backend search command', async () => {
    installMock({
      topics: (filter) => {
        if (filter.query === 'quantum') {
          return [];
        }
        if (filter.sectionCode === 'MMP') {
          return [MMP_TOPIC];
        }
        return TOPICS;
      },
    });

    render(<SyllabusScreen />);

    await screen.findByRole('button', { name: /GA-01/ });
    fireEvent.change(screen.getByLabelText('Filter by section'), { target: { value: 'MMP' } });

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('search_syllabus_topics', {
        filter: { sectionCode: 'MMP' },
      }),
    );
    expect(await screen.findByRole('button', { name: /MMP-C01/ })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /GA-01/ })).not.toBeInTheDocument();

    await userEvent.type(screen.getByLabelText('Search topics'), 'quantum');

    await waitFor(
      () =>
        expect(invokeMock).toHaveBeenCalledWith('search_syllabus_topics', {
          filter: { sectionCode: 'MMP', query: 'quantum' },
        }),
      { timeout: 2000 },
    );
    expect(await screen.findByText('No topics match these filters')).toBeInTheDocument();
  });

  it('shows revision status and next due date without changing syllabus progress', async () => {
    installMock({
      topics: [GA_TOPIC],
      revisions: [
        {
          topicId: GA_TOPIC.id,
          status: 'active',
          dueState: 'due_today',
          nextRevisionDate: '2026-10-30',
          completedRevisions: 2,
          masteryEstimate: 50,
        },
      ],
    });

    render(<SyllabusScreen />);

    expect((await screen.findAllByText('Revision due today')).length).toBeGreaterThan(0);
    expect(screen.getByText('Next: 30 October 2026')).toBeInTheDocument();
    expect(screen.getByText(/2 completed · 50.0% revision mastery/)).toBeInTheDocument();
    expect(screen.getByLabelText('Status')).toHaveValue('Not started');
  });

  it('edits and saves progress with a typed payload and refreshes the summary', async () => {
    installMock({
      summary: (call) => (call === 1 ? SUMMARY : UPDATED_SUMMARY),
      topics: [GA_TOPIC],
    });

    invokeMock.mockImplementation((command: string, args?: unknown) => {
      if (command === 'get_syllabus_catalog') {
        return Promise.resolve(CATALOG);
      }
      if (command === 'search_syllabus_topics') {
        return Promise.resolve([GA_TOPIC]);
      }
      if (command === 'get_syllabus_summary') {
        return Promise.resolve(
          invokeMock.mock.calls.filter(([name]) => name === 'get_syllabus_summary').length > 1
            ? UPDATED_SUMMARY
            : SUMMARY,
        );
      }
      if (command === 'update_topic_progress') {
        const { progress } = args as {
          topicId: number;
          progress: TopicProgressInput;
        };
        return Promise.resolve({
          ...progress,
          updatedAt: '2026-10-29T12:00:00.000Z',
          recorded: true,
        });
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<SyllabusScreen />);

    await screen.findByRole('button', { name: /GA-01/ });
    fireEvent.change(screen.getByLabelText('Status'), { target: { value: 'Learning' } });
    fireEvent.change(screen.getByLabelText('Priority'), { target: { value: 'High-Yield' } });
    fireEvent.change(screen.getByLabelText('Confidence (1–5)'), { target: { value: '4' } });
    fireEvent.change(screen.getByLabelText('Notes'), {
      target: { value: 'Practice puzzles daily.' },
    });

    expect(screen.getByText('Unsaved changes')).toBeInTheDocument();
    expect(
      screen.getByText('No saved progress yet. Defaults are shown until you save.'),
    ).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Save progress' }));

    expect(await screen.findByText('Progress saved for GA-01.')).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith('update_topic_progress', {
      topicId: 1,
      progress: {
        status: 'Learning',
        priority: 'High-Yield',
        confidence: 4,
        notes: 'Practice puzzles daily.',
      },
    });
    expect(await screen.findByText('25.0%')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save progress' })).toBeDisabled();
    expect(screen.getByText('All changes saved')).toBeInTheDocument();
  });

  it('shows a save-in-progress state and disables the editor', async () => {
    let resolveUpdate!: (progress: TopicProgress) => void;
    const pendingUpdate = new Promise<TopicProgress>((resolve) => {
      resolveUpdate = resolve;
    });

    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_syllabus_catalog') {
        return Promise.resolve(CATALOG);
      }
      if (command === 'get_syllabus_summary') {
        return Promise.resolve(SUMMARY);
      }
      if (command === 'search_syllabus_topics') {
        return Promise.resolve([GA_TOPIC]);
      }
      if (command === 'update_topic_progress') {
        return pendingUpdate;
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<SyllabusScreen />);

    await screen.findByRole('button', { name: /GA-01/ });
    fireEvent.change(screen.getByLabelText('Status'), { target: { value: 'Learning' } });
    await userEvent.click(screen.getByRole('button', { name: 'Save progress' }));

    expect(screen.getByRole('button', { name: 'Saving…' })).toBeDisabled();
    expect(screen.getByLabelText('Status')).toBeDisabled();
    expect(screen.getByText('Writing to SQLite…')).toBeInTheDocument();

    await act(async () => {
      resolveUpdate({
        status: 'Learning',
        priority: 'Normal',
        confidence: 3,
        notes: '',
        updatedAt: '2026-10-29T12:00:00.000Z',
        recorded: true,
      });
      await pendingUpdate;
    });

    expect(await screen.findByText('Progress saved for GA-01.')).toBeInTheDocument();
    expect(screen.getByLabelText('Status')).toBeEnabled();
  });

  it('maps a structured save error to the confidence field', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_syllabus_catalog') {
        return Promise.resolve(CATALOG);
      }
      if (command === 'get_syllabus_summary') {
        return Promise.resolve(SUMMARY);
      }
      if (command === 'search_syllabus_topics') {
        return Promise.resolve([GA_TOPIC]);
      }
      if (command === 'update_topic_progress') {
        return Promise.reject({
          code: 'validation_error',
          message: 'Confidence must be a whole number from 1 to 5.',
          field: 'confidence',
        });
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<SyllabusScreen />);

    await screen.findByRole('button', { name: /GA-01/ });
    fireEvent.change(screen.getByLabelText('Status'), { target: { value: 'Learning' } });
    fireEvent.change(screen.getByLabelText('Confidence (1–5)'), { target: { value: '5' } });
    await userEvent.click(screen.getByRole('button', { name: 'Save progress' }));

    const alerts = await screen.findAllByRole('alert');
    expect(alerts.length).toBeGreaterThanOrEqual(2);
    for (const alert of alerts) {
      expect(alert).toHaveTextContent('Confidence must be a whole number from 1 to 5.');
    }
    expect(screen.getByLabelText('Confidence (1–5)')).toHaveAttribute('aria-invalid', 'true');
  });
});
