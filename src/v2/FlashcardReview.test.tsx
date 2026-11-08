import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2ExamSummary, V2FlashcardQueue } from './api';
import { FlashcardReview } from './FlashcardReview';

vi.mock('./api', () => ({
  getV2FlashcardReviewQueue: vi.fn(),
  reviewV2Flashcard: vi.fn(),
}));

import * as api from './api';

const mockedQueue = vi.mocked(api.getV2FlashcardReviewQueue);
const mockedReview = vi.mocked(api.reviewV2Flashcard);

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
];

function queue(): V2FlashcardQueue {
  return {
    remaining: 0,
    cards: [
      {
        id: 11,
        front: 'State the gradient meaning.',
        back: 'The direction of steepest increase.',
        subjectArea: 'Mathematical Methods',
        conceptTitle: null,
        schedule: {
          dueOn: TODAY,
          intervalDays: 0,
          easeFactor: 2.5,
          reviewCount: 0,
          lapseCount: 0,
          lastRating: null,
          lastReviewedOn: null,
          lastReviewedAt: null,
          algorithmVersion: 1,
        },
      },
      {
        id: 12,
        front: 'Second prompt',
        back: 'Second answer',
        subjectArea: 'Classical Mechanics',
        conceptTitle: 'Kinematics',
        schedule: {
          dueOn: TODAY,
          intervalDays: 0,
          easeFactor: 2.5,
          reviewCount: 0,
          lapseCount: 0,
          lastRating: null,
          lastReviewedOn: null,
          lastReviewedAt: null,
          algorithmVersion: 1,
        },
      },
    ],
  };
}

function renderReview() {
  const onBack = vi.fn();
  const onReviewed = vi.fn();
  render(
    <FlashcardReview
      conceptSlug={null}
      examId={null}
      exams={EXAMS}
      nodeId={null}
      onBack={onBack}
      onReviewed={onReviewed}
      subjectArea=""
      today={TODAY}
    />,
  );
  return { onBack, onReviewed };
}

describe('FlashcardReview', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedQueue.mockResolvedValue(queue());
    mockedReview.mockResolvedValue({
      review: {
        id: 1,
        reviewedOn: TODAY,
        reviewedAt: '2026-11-08T00:00:00Z',
        rating: 'good',
        scheduledDueOn: TODAY,
        nextDueOn: '2026-11-10',
        intervalBeforeDays: 0,
        intervalAfterDays: 2,
        easeBefore: 2.5,
        easeAfter: 2.5,
        algorithmVersion: 1,
      },
      schedule: {
        dueOn: '2026-11-10',
        intervalDays: 2,
        easeFactor: 2.5,
        reviewCount: 1,
        lapseCount: 0,
        lastRating: 'good',
        lastReviewedOn: TODAY,
        lastReviewedAt: '2026-11-08T00:00:00Z',
        algorithmVersion: 1,
      },
    });
  });

  it('reveals the answer, records a rating and advances through the queue', async () => {
    const user = userEvent.setup();
    const { onReviewed } = renderReview();

    expect(await screen.findByText('State the gradient meaning.')).toBeInTheDocument();
    expect(screen.getByText('Card 1 of 2')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Good/ })).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Reveal answer' }));
    expect(screen.getByText('The direction of steepest increase.')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /Good/ }));

    await waitFor(() =>
      expect(mockedReview).toHaveBeenCalledWith({
        flashcardId: 11,
        rating: 'good',
        today: TODAY,
      }),
    );
    expect(onReviewed).toHaveBeenCalledTimes(1);
    expect(await screen.findByText('Second prompt')).toBeInTheDocument();
    expect(screen.getByText('Card 2 of 2')).toBeInTheDocument();
  });

  it('shows the empty due state and switches to practice all', async () => {
    const user = userEvent.setup();
    mockedQueue.mockImplementation(async (input) =>
      input.dueOnly ? { remaining: 0, cards: [] } : queue(),
    );
    renderReview();

    expect(await screen.findByText('Nothing to review right now')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Practice all cards' }));

    await waitFor(() =>
      expect(mockedQueue).toHaveBeenLastCalledWith(
        expect.objectContaining({ dueOnly: false, today: TODAY }),
      ),
    );
    expect(await screen.findByText('State the gradient meaning.')).toBeInTheDocument();
  });
});
