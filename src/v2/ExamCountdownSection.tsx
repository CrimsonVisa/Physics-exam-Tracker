import type { V2ExamSummary } from './api';
import { examDateDisplay, type ExamCountdownUrgency, type ExamDateDisplay } from './examDate';

type ExamCountdownSectionProps = {
  exams: V2ExamSummary[];
  today: string;
  onOpenExam: (examCode: string) => void;
  onOpenSettings: () => void;
};

const URGENCY_LABELS: Record<ExamCountdownUrgency, string> = {
  far: 'Scheduled',
  approaching: 'Approaching',
  imminent: 'Soon',
  today: 'Today',
  passed: 'Passed',
  'not-set': 'Date not set',
  invalid: 'Invalid date',
};

type CountdownEntry = {
  exam: V2ExamSummary;
  display: ExamDateDisplay;
};

function stateRank(display: ExamDateDisplay): number {
  switch (display.state) {
    case 'upcoming':
    case 'exam-day':
      return 0;
    case 'passed':
      return 1;
    case 'invalid':
      return 2;
    default:
      return 3;
  }
}

function countdownEntries(exams: V2ExamSummary[], today: string): CountdownEntry[] {
  return exams
    .filter((exam) => exam.isActive)
    .map((exam) => ({ exam, display: examDateDisplay(exam.examDate, today) }))
    .sort((left, right) => {
      const rank = stateRank(left.display) - stateRank(right.display);
      if (rank !== 0) {
        return rank;
      }
      const dateOrder = (left.exam.examDate ?? '').localeCompare(right.exam.examDate ?? '');
      return dateOrder !== 0 ? dateOrder : left.exam.displayOrder - right.exam.displayOrder;
    });
}

function accessibleLabel(exam: V2ExamSummary, display: ExamDateDisplay): string {
  if (display.state === 'upcoming') {
    return `${exam.name}: ${display.daysRemaining === 1 ? '1 day' : `${display.daysRemaining} days`} remaining. Exam on ${display.detail}.`;
  }
  if (display.state === 'exam-day') {
    return `${exam.name}: examination today, ${display.detail}.`;
  }
  if (display.state === 'passed') {
    return `${exam.name}: examination date passed. ${display.detail}.`;
  }
  if (display.state === 'not-set') {
    return `${exam.name}: examination date not set. Add it in Settings.`;
  }
  return `${exam.name}: invalid examination date. Correct it in Settings.`;
}

function CountdownValue({ display }: { display: ExamDateDisplay }) {
  if (display.state === 'upcoming') {
    return (
      <span className="v2-countdown-main">
        <strong>{display.daysRemaining}</strong>
        <span>{display.daysRemaining === 1 ? 'day remaining' : 'days remaining'}</span>
      </span>
    );
  }
  if (display.state === 'exam-day') {
    return (
      <span className="v2-countdown-main">
        <strong>Today</strong>
        <span>Examination day</span>
      </span>
    );
  }
  if (display.state === 'passed') {
    return (
      <span className="v2-countdown-main">
        <strong>Passed</strong>
        <span>
          {display.daysSinceExam} day{display.daysSinceExam === 1 ? '' : 's'} ago
        </span>
      </span>
    );
  }
  return (
    <span className="v2-countdown-main">
      <strong>{display.state === 'invalid' ? 'Invalid' : 'Not set'}</strong>
      <span>
        {display.state === 'invalid'
          ? 'Check the saved date in Settings'
          : 'Add the exam date in Settings'}
      </span>
    </span>
  );
}

/** Compact countdown for every active examination with a saved date. */
export function ExamCountdownSection({
  exams,
  today,
  onOpenExam,
  onOpenSettings,
}: ExamCountdownSectionProps) {
  const entries = countdownEntries(exams, today);
  const activeCount = entries.length;
  const datedCount = entries.filter(
    (entry) => entry.display.state !== 'not-set' && entry.display.state !== 'invalid',
  ).length;

  return (
    <section aria-label="Exam countdown" className="v2-concept-card v2-exam-countdown">
      <div className="v2-section-heading">
        <div>
          <p className="eyebrow">Examinations</p>
          <h2>Exam countdown</h2>
        </div>
        <p>
          {activeCount === 0
            ? 'No active examinations'
            : `${activeCount} active · ${datedCount} with dates`}
        </p>
      </div>

      {activeCount === 0 ? (
        <p className="v2-empty-copy">
          No active examinations are configured. Activate an examination in Settings to see its
          countdown.
        </p>
      ) : (
        <ul className="v2-exam-countdown-list">
          {entries.map(({ exam, display }) => {
            const needsSettings = display.state === 'not-set' || display.state === 'invalid';
            return (
              <li key={exam.code}>
                <button
                  aria-label={accessibleLabel(exam, display)}
                  className="v2-exam-countdown-card"
                  data-urgency={display.urgency}
                  onClick={() => (needsSettings ? onOpenSettings() : onOpenExam(exam.code))}
                  type="button"
                >
                  <span className="v2-exam-countdown-topline">
                    <span className="v2-exam-countdown-code">{exam.shortName}</span>
                    <span className="v2-exam-countdown-urgency">
                      {URGENCY_LABELS[display.urgency]}
                    </span>
                  </span>
                  <span className="v2-exam-countdown-name">{exam.name}</span>
                  <CountdownValue display={display} />
                  <span className="v2-exam-countdown-date">
                    {exam.examDate ? display.detail : 'No examination date configured'}
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
