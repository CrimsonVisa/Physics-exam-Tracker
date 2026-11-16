import { useEffect, useState } from 'react';
import {
  getV2DashboardSections,
  resetV2DashboardSections,
  saveV2DashboardSections,
  type V2DatabaseStatus,
  type V2DashboardSection,
  type V2ExamSummary,
} from './api';
import {
  PerformanceSection,
  QuickStartSection,
  RecentlyStudiedSection,
  RevisionOverviewSection,
  TodayOverviewSection,
  TodayPlanSection,
} from './DashboardSections';
import { needsDailyData, useDailyDashboardData } from './dailyDashboardData';
import { ExamCountdownSection } from './ExamCountdownSection';
import { examDateDisplay } from './examDate';
import type { V2View } from './navigation';
import { StudySection } from './StudySection';

type CombinedDashboardProps = {
  status: V2DatabaseStatus;
  today: string;
  focusTimerRequest: number;
  onTimerFocused: (requestId: number) => void;
  onTimerUnavailable: (requestId: number) => void;
  onOpenExam: (examCode: string) => void;
  onNavigate: (view: V2View) => void;
};

/**
 * Mirrors the V2 default section list in src-tauri/src/v2/dashboard.rs and is
 * only used while the persisted preference list is still loading.
 */
const FALLBACK_SECTIONS: Array<[string, string]> = [
  ['quickstart', 'Quick start timer'],
  ['today', "Today's overview"],
  ['countdown', 'Exam countdown'],
  ['plan', "Today's study plan"],
  ['revision', 'Revision overview'],
  ['recent', 'Recently studied'],
  ['performance', 'Performance and weak areas'],
  ['overview', 'Database overview'],
  ['study', 'Study timer and sessions'],
  ['exams', 'Examination workspaces'],
  ['database', 'Database details'],
];

function ExamCard({
  exam,
  today,
  onOpen,
}: {
  exam: V2ExamSummary;
  today: string;
  onOpen: () => void;
}) {
  const date = examDateDisplay(exam.examDate, today);
  const actionLabel = exam.isActive ? 'Open workspace' : 'Open historical workspace';

  return (
    <article
      className={`v2-exam-card${exam.isActive ? '' : ' v2-exam-card-inactive'}`}
      data-exam-code={exam.code}
    >
      <div className="v2-exam-card-topline">
        <span
          className={`v2-status-badge ${exam.isActive ? 'v2-status-active' : 'v2-status-inactive'}`}
        >
          {exam.isActive ? 'Active' : 'Inactive'}
        </span>
        <span className="v2-exam-code">{exam.code}</span>
      </div>

      <div className="v2-exam-card-heading">
        <p className="v2-exam-short-name">{exam.shortName}</p>
        <h2>{exam.name}</h2>
        {exam.authority && <p className="v2-exam-authority">{exam.authority}</p>}
      </div>

      <dl className="v2-exam-date">
        <div>
          <dt>Exam date</dt>
          <dd className={`v2-date-${date.state}`}>{date.label}</dd>
        </div>
        <div>
          <dt>Date detail</dt>
          <dd>{date.detail}</dd>
        </div>
      </dl>

      <button className="secondary-button v2-open-button" onClick={onOpen} type="button">
        {actionLabel}
      </button>
    </article>
  );
}

export function CombinedDashboard({
  status,
  today,
  focusTimerRequest,
  onTimerFocused,
  onTimerUnavailable,
  onOpenExam,
  onNavigate,
}: CombinedDashboardProps) {
  const [sections, setSections] = useState<V2DashboardSection[] | null>(null);
  const [sectionsReady, setSectionsReady] = useState(false);
  const [layoutBusy, setLayoutBusy] = useState(false);
  const [layoutError, setLayoutError] = useState<string | null>(null);

  const orderedSections =
    sections ??
    FALLBACK_SECTIONS.map(([sectionKey, title], displayOrder) => ({
      sectionKey,
      title,
      visible: true,
      displayOrder,
    }));
  const studySection = sections?.find((section) => section.sectionKey === 'study') ?? null;
  const daily = useDailyDashboardData(
    today,
    needsDailyData(orderedSections.map((section) => section.sectionKey)),
  );

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const loaded = await getV2DashboardSections();
        if (!cancelled) {
          setSections(loaded);
        }
      } catch {
        // The default fixed dashboard remains usable if preferences fail to load.
      } finally {
        if (!cancelled) {
          setSectionsReady(true);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!sectionsReady || !sections || focusTimerRequest === 0) {
      return;
    }
    if (!studySection || !studySection.visible) {
      onTimerUnavailable(focusTimerRequest);
    }
  }, [focusTimerRequest, onTimerUnavailable, sections, sectionsReady, studySection]);

  async function toggleSection(key: string) {
    if (!sections) {
      return;
    }
    setSections(
      sections.map((section) =>
        section.sectionKey === key ? { ...section, visible: !section.visible } : section,
      ),
    );
  }

  async function saveLayout() {
    if (!sections) {
      return;
    }
    setLayoutBusy(true);
    setLayoutError(null);
    try {
      setSections(
        await saveV2DashboardSections({
          examId: null,
          sections: sections.map((section) => ({
            sectionKey: section.sectionKey,
            visible: section.visible,
            displayOrder: section.displayOrder,
          })),
        }),
      );
    } catch (error) {
      setLayoutError(
        typeof error === 'object' && error !== null && 'message' in error
          ? String((error as { message: unknown }).message)
          : 'Dashboard preferences could not be saved.',
      );
    } finally {
      setLayoutBusy(false);
    }
  }

  async function resetLayout() {
    setLayoutBusy(true);
    setLayoutError(null);
    try {
      setSections(await resetV2DashboardSections());
    } catch {
      setLayoutError('Dashboard preferences could not be reset.');
    } finally {
      setLayoutBusy(false);
    }
  }

  function moveSection(key: string, offset: -1 | 1) {
    if (!sections) {
      return;
    }
    const index = sections.findIndex((section) => section.sectionKey === key);
    const target = index + offset;
    if (index < 0 || target < 0 || target >= sections.length) {
      return;
    }
    const next = [...sections];
    const [moved] = next.splice(index, 1);
    next.splice(target, 0, moved);
    setSections(next.map((section, order) => ({ ...section, displayOrder: order })));
  }

  function renderSection(key: string) {
    if (key === 'quickstart') {
      return (
        <QuickStartSection exams={status.exams} key={key} sessions={daily.sessions} today={today} />
      );
    }
    if (key === 'today') {
      return <TodayOverviewSection data={daily} key={key} onNavigate={onNavigate} today={today} />;
    }
    if (key === 'countdown') {
      return (
        <ExamCountdownSection
          exams={status.exams}
          key={key}
          onOpenExam={onOpenExam}
          onOpenSettings={() => onNavigate({ kind: 'settings' })}
          today={today}
        />
      );
    }
    if (key === 'plan') {
      return (
        <TodayPlanSection data={daily} exams={status.exams} key={key} onNavigate={onNavigate} />
      );
    }
    if (key === 'revision') {
      return <RevisionOverviewSection data={daily} key={key} onNavigate={onNavigate} />;
    }
    if (key === 'recent') {
      return (
        <RecentlyStudiedSection
          data={daily}
          exams={status.exams}
          key={key}
          onNavigate={onNavigate}
        />
      );
    }
    if (key === 'performance') {
      return <PerformanceSection exams={status.exams} key={key} onNavigate={onNavigate} />;
    }
    if (key === 'overview') {
      return (
        <section aria-label="Database overview" className="v2-overview-strip" key={key}>
          <div>
            <span className="v2-overview-value">{activeCount}</span>
            <span className="v2-overview-label">Active exams</span>
          </div>
          <div>
            <span className="v2-overview-value">{inactiveCount}</span>
            <span className="v2-overview-label">Inactive exams</span>
          </div>
          <div>
            <span className="v2-overview-value">{datesSet}</span>
            <span className="v2-overview-label">Exam dates set</span>
          </div>
          <div>
            <span className="v2-overview-value">Schema {status.schemaVersion}</span>
            <span className="v2-overview-label">Healthy local database</span>
          </div>
        </section>
      );
    }
    if (key === 'study') {
      return (
        <StudySection
          exams={status.exams}
          focusRequest={sectionsReady ? focusTimerRequest : 0}
          key={key}
          onTimerFocused={onTimerFocused}
          today={today}
        />
      );
    }
    if (key === 'exams') {
      return status.exams.length === 0 ? (
        <section className="v2-empty-panel" aria-live="polite" key={key}>
          <h2>No exam workspaces configured</h2>
          <p>The database did not return any exam records. Retry the database check.</p>
        </section>
      ) : (
        <section aria-labelledby="exam-workspaces-heading" className="v2-exam-grid" key={key}>
          <div className="v2-section-heading">
            <div>
              <p className="eyebrow">Workspaces</p>
              <h2 id="exam-workspaces-heading">Examinations</h2>
            </div>
            <p>{status.exams.length} configured workspaces</p>
          </div>
          {status.exams.map((exam) => (
            <ExamCard
              exam={exam}
              key={exam.code}
              onOpen={() => onOpenExam(exam.code)}
              today={today}
            />
          ))}
        </section>
      );
    }
    if (key === 'database') {
      return (
        <section aria-labelledby="database-status-heading" className="v2-database-panel" key={key}>
          <div className="v2-section-heading">
            <div>
              <p className="eyebrow">Local storage</p>
              <h2 id="database-status-heading">Database</h2>
            </div>
            <span className="v2-health-pill">Healthy</span>
          </div>
          <dl className="v2-database-facts">
            <div>
              <dt>Application identifier</dt>
              <dd>{status.appIdentifier}</dd>
            </div>
            <div>
              <dt>Schema</dt>
              <dd>
                {status.schemaVersion} / {status.expectedSchemaVersion}
              </dd>
            </div>
            <div>
              <dt>Exam records</dt>
              <dd>
                {status.examCount} total · {status.activeExamCount} active
              </dd>
            </div>
            <div>
              <dt>Integrity</dt>
              <dd>{status.integrityCheck === 'ok' ? 'Verified' : 'Unavailable'}</dd>
            </div>
          </dl>
          {status.databasePath && <p className="v2-path">{status.databasePath}</p>}
        </section>
      );
    }
    return null;
  }

  const activeCount = status.exams.filter((exam) => exam.isActive).length;
  const datesSet = status.exams.filter((exam) => exam.examDate !== null).length;
  const inactiveCount = status.exams.length - activeCount;

  return (
    <div className="page-container v2-dashboard">
      <header className="page-header">
        <p className="eyebrow">Physics Tracker · dashboard</p>
        <h1>Combined dashboard</h1>
        <p className="page-lede">
          Four independent examination workspaces share one local Physics database. Dates and
          workspace details shown here come directly from the verified database.
        </p>
      </header>

      {orderedSections.map((section) =>
        section.visible ? renderSection(section.sectionKey) : null,
      )}

      <details className="v2-concept-card">
        <summary>Customize dashboard</summary>
        <div className="v2-settings-form v2-dashboard-settings-form">
          {(sections ?? []).map((section, index) => (
            <div className="v2-dashboard-layout-row" key={section.sectionKey}>
              <label className="v2-checkbox-field">
                <input
                  checked={section.visible}
                  onChange={() => void toggleSection(section.sectionKey)}
                  type="checkbox"
                />
                <span>{section.title}</span>
              </label>
              <div className="v2-dashboard-order-actions">
                <button
                  aria-label={`Move ${section.title} up`}
                  className="secondary-button"
                  disabled={layoutBusy || index === 0}
                  onClick={() => moveSection(section.sectionKey, -1)}
                  type="button"
                >
                  Up
                </button>
                <button
                  aria-label={`Move ${section.title} down`}
                  className="secondary-button"
                  disabled={layoutBusy || index === (sections?.length ?? 1) - 1}
                  onClick={() => moveSection(section.sectionKey, 1)}
                  type="button"
                >
                  Down
                </button>
              </div>
            </div>
          ))}
          <button
            className="primary-button"
            disabled={layoutBusy || !sections}
            onClick={() => void saveLayout()}
            type="button"
          >
            Save dashboard layout
          </button>
          <button
            className="secondary-button"
            disabled={layoutBusy}
            onClick={() => void resetLayout()}
            type="button"
          >
            Restore defaults
          </button>
        </div>
        {layoutError && <p className="v2-study-error">{layoutError}</p>}
      </details>
    </div>
  );
}
