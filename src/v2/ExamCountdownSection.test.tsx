import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { addDaysDateOnly, formatDateLong } from '../lib/dateOnly';
import type { V2ExamSummary } from './api';
import { ExamCountdownSection } from './ExamCountdownSection';

const TODAY = '2026-11-04';

function exam(overrides: Partial<V2ExamSummary> = {}): V2ExamSummary {
  return {
    id: 1,
    code: 'csir-net-physics',
    name: 'CSIR-UGC NET Physical Sciences',
    shortName: 'CSIR-NET',
    authority: 'CSIR-HRDG / NTA',
    officialUrl: 'https://example.com',
    displayOrder: 1,
    isActive: true,
    examDate: null,
    ...overrides,
  };
}

function dateAfter(days: number): string {
  const value = addDaysDateOnly(TODAY, days);
  if (!value) {
    throw new Error(`Could not calculate ${days} days from ${TODAY}`);
  }
  return value;
}

function renderSection(exams: V2ExamSummary[]) {
  const onOpenExam = vi.fn();
  const onOpenSettings = vi.fn();
  const view = render(
    <ExamCountdownSection
      exams={exams}
      onOpenExam={onOpenExam}
      onOpenSettings={onOpenSettings}
      today={TODAY}
    />,
  );
  return { ...view, onOpenExam, onOpenSettings };
}

describe('ExamCountdownSection', () => {
  it('shows multiple active exams with actual calendar-day values and dates', () => {
    const exams = [
      exam({
        code: 'csir-net-physics',
        name: 'CSIR-UGC NET Physical Sciences',
        shortName: 'CSIR-NET',
        examDate: dateAfter(45),
      }),
      exam({
        id: 2,
        code: 'gate-physics',
        name: 'GATE Physics',
        shortName: 'GATE',
        displayOrder: 2,
        examDate: dateAfter(82),
      }),
      exam({
        id: 4,
        code: 'jest-physics',
        name: 'JEST Physics',
        shortName: 'JEST',
        displayOrder: 4,
        examDate: dateAfter(110),
      }),
    ];

    renderSection(exams);

    for (const [days, name, shortName] of [
      [45, 'CSIR-UGC NET Physical Sciences', 'CSIR-NET'],
      [82, 'GATE Physics', 'GATE'],
      [110, 'JEST Physics', 'JEST'],
    ] as const) {
      const card = screen.getByRole('button', { name: new RegExp(name) });
      expect(within(card).getByText(String(days))).toBeInTheDocument();
      expect(within(card).getByText(shortName)).toBeInTheDocument();
      expect(within(card).getByText('Scheduled')).toBeInTheDocument();
      expect(within(card).getByText(formatDateLong(dateAfter(days)) ?? '')).toBeInTheDocument();
    }

    const cards = screen.getAllByRole('button');
    expect(cards[0]).toHaveAccessibleName(/CSIR-UGC NET Physical Sciences/);
    expect(cards[1]).toHaveAccessibleName(/GATE Physics/);
    expect(cards[2]).toHaveAccessibleName(/JEST Physics/);
  });

  it('uses the configured urgency categories', () => {
    const cases = [
      { name: 'Far exam', days: 31, urgency: 'far', badge: 'Scheduled' },
      { name: 'Approaching exam', days: 30, urgency: 'approaching', badge: 'Approaching' },
      { name: 'Soon exam', days: 7, urgency: 'imminent', badge: 'Soon' },
      { name: 'Today exam', days: 0, urgency: 'today', badge: 'Today' },
      { name: 'Past exam', days: -1, urgency: 'passed', badge: 'Passed' },
    ];
    const exams = cases.map((entry, index) =>
      exam({
        id: index + 1,
        code: `exam-${index}`,
        name: entry.name,
        shortName: `E${index}`,
        displayOrder: index,
        examDate: dateAfter(entry.days),
      }),
    );

    renderSection(exams);

    for (const entry of cases) {
      const card = screen.getByRole('button', { name: new RegExp(entry.name) });
      expect(card).toHaveAttribute('data-urgency', entry.urgency);
      expect(card.querySelector('.v2-exam-countdown-urgency')).toHaveTextContent(entry.badge);
    }
  });

  it('indicates examination day and passed dates without negative countdowns', () => {
    renderSection([
      exam({ id: 1, name: 'Today exam', examDate: TODAY }),
      exam({ id: 2, code: 'past', name: 'Past exam', examDate: dateAfter(-3) }),
    ]);

    const todayCard = screen.getByRole('button', { name: /Today exam/ });
    expect(todayCard.querySelector('.v2-countdown-main strong')).toHaveTextContent('Today');
    expect(within(todayCard).getByText('Examination day')).toBeInTheDocument();

    const pastCard = screen.getByRole('button', { name: /Past exam/ });
    expect(pastCard.querySelector('.v2-countdown-main strong')).toHaveTextContent('Passed');
    expect(within(pastCard).getByText('3 days ago')).toBeInTheDocument();
    expect(within(pastCard).queryByText(/-3 days remaining/)).not.toBeInTheDocument();
  });

  it('handles missing and invalid dates through Settings', async () => {
    const user = userEvent.setup();
    const { onOpenExam, onOpenSettings } = renderSection([
      exam({ id: 1, name: 'No date exam', examDate: null }),
      exam({ id: 2, code: 'invalid', name: 'Invalid date exam', examDate: '2026-02-30' }),
    ]);

    const missing = screen.getByRole('button', { name: /No date exam/ });
    expect(within(missing).getByText('Not set')).toBeInTheDocument();
    expect(within(missing).getByText('Add the exam date in Settings')).toBeInTheDocument();

    const invalid = screen.getByRole('button', { name: /Invalid date exam/ });
    expect(within(invalid).getByText('Invalid')).toBeInTheDocument();
    expect(within(invalid).getByText('Check the saved date in Settings')).toBeInTheDocument();

    await user.click(missing);
    await user.click(invalid);
    expect(onOpenSettings).toHaveBeenCalledTimes(2);
    expect(onOpenExam).not.toHaveBeenCalled();
  });

  it('opens the exam workspace for a dated active exam', async () => {
    const user = userEvent.setup();
    const { onOpenExam } = renderSection([
      exam({ code: 'gate-physics', name: 'GATE Physics', examDate: dateAfter(45) }),
    ]);

    await user.click(screen.getByRole('button', { name: /GATE Physics/ }));
    expect(onOpenExam).toHaveBeenCalledWith('gate-physics');
  });

  it('excludes inactive exams and explains when none are active', () => {
    const { rerender } = render(
      <ExamCountdownSection
        exams={[exam({ name: 'Inactive exam', isActive: false, examDate: dateAfter(20) })]}
        onOpenExam={vi.fn()}
        onOpenSettings={vi.fn()}
        today={TODAY}
      />,
    );

    expect(screen.queryByText('Inactive exam')).not.toBeInTheDocument();
    expect(screen.getByText(/No active examinations are configured/)).toBeInTheDocument();

    rerender(
      <ExamCountdownSection
        exams={[
          exam({ name: 'Active exam', examDate: dateAfter(20) }),
          exam({ id: 2, code: 'inactive', name: 'Inactive exam', isActive: false }),
        ]}
        onOpenExam={vi.fn()}
        onOpenSettings={vi.fn()}
        today={TODAY}
      />,
    );
    expect(screen.getByText('Active exam')).toBeInTheDocument();
    expect(screen.queryByText('Inactive exam')).not.toBeInTheDocument();
  });

  it('uses a responsive grid and theme tokens for urgency styling', () => {
    const css = readFileSync(join(process.cwd(), 'src/v2/v2.css'), 'utf8');
    expect(css).toMatch(
      /\.v2-exam-countdown-list\s*{[^}]*grid-template-columns:\s*repeat\(auto-fit, minmax\(220px, 1fr\)\);/s,
    );
    expect(css).toMatch(
      /\.v2-exam-countdown-card\[data-urgency='approaching'\]\s*{[^}]*--warning-border/s,
    );
    expect(css).toMatch(
      /\.v2-exam-countdown-card\[data-urgency='imminent'\][\s\S]*?--error-border/s,
    );
    expect(css).toMatch(/\.v2-exam-countdown-card\[data-urgency='passed'\]\s*{[^}]*--line-strong/s);
  });

  it('updates when the saved exam date changes', () => {
    const first = exam({ name: 'GATE Physics', examDate: dateAfter(45) });
    const { rerender } = render(
      <ExamCountdownSection
        exams={[first]}
        onOpenExam={vi.fn()}
        onOpenSettings={vi.fn()}
        today={TODAY}
      />,
    );
    expect(screen.getByText('45')).toBeInTheDocument();

    rerender(
      <ExamCountdownSection
        exams={[{ ...first, examDate: dateAfter(60) }]}
        onOpenExam={vi.fn()}
        onOpenSettings={vi.fn()}
        today={TODAY}
      />,
    );
    expect(screen.getByText('60')).toBeInTheDocument();
    expect(screen.queryByText('45')).not.toBeInTheDocument();
  });
});
