import { useCallback, useEffect, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import { BrandLockup } from '../components/BrandMark';
import { useAppShortcuts } from '../lib/appShortcuts';
import {
  applyTheme,
  getStoredSidebarMode,
  getStoredTheme,
  storeSidebarMode,
  storeTheme,
  type Theme,
} from '../lib/uiPreferences';
import { useTodayDateOnly } from '../lib/useTodayDateOnly';
import {
  initializeV2Database,
  getV2DatabaseStatus,
  type V2DatabaseStatus,
  type V2ExamSummary,
} from './api';
import { CombinedDashboard } from './CombinedDashboard';
import { ConceptsScreen } from './ConceptsScreen';
import { DatabaseStateScreen } from './DatabaseStateScreen';
import { ExamWorkspace } from './ExamWorkspace';
import { FlashcardsScreen } from './FlashcardsScreen';
import { BackupScreen } from './BackupScreen';
import { AnalyticsScreen } from './AnalyticsScreen';
import { GlobalSearch } from './GlobalSearch';
import { PlannerScreen } from './PlannerScreen';
import { PracticeScreen } from './PracticeScreen';
import { ResourcesScreen } from './ResourcesScreen';
import { RevisionScreen } from './RevisionScreen';
import { SettingsScreen } from './SettingsScreen';
import { ShortcutsDialog } from './ShortcutsDialog';
import { SyllabusScreen } from './SyllabusScreen';
import type { V2View } from './navigation';
import { V2Sidebar, type V2SidebarMode } from './V2Sidebar';

type V2AppProps = {
  today?: string;
};

/** Workspace accent identity for the current view. */
function viewWorkspace(view: V2View): string {
  switch (view.kind) {
    case 'dashboard':
      return 'dashboard';
    case 'exam':
      return 'exam';
    case 'concepts':
      return 'concepts';
    case 'flashcards':
      return 'flashcards';
    case 'syllabus':
      return 'syllabus';
    case 'practice':
      return 'practice';
    case 'planner':
      return 'planner';
    case 'revision':
      return 'revision';
    case 'analytics':
      return 'analytics';
    case 'resources':
      return 'resources';
    case 'backup':
      return 'backup';
    case 'settings':
      return 'settings';
    default:
      return 'dashboard';
  }
}

type BootstrapErrorProps = {
  message: string;
  busy: boolean;
  onRetry: () => void;
};

function BootstrapError({ message, busy, onRetry }: BootstrapErrorProps) {
  return (
    <main className="v2-database-state">
      <div className="v2-database-state-card" role="alert">
        <BrandLockup subtitle="Scientific study workspace" />
        <p className="eyebrow">Physics Tracker · V2 database</p>
        <h1>Physics Tracker could not read its V2 database</h1>
        <p>{message}</p>
        <p>
          The shell does not fall back to V1 data. Check the V2 application-data folder and retry.
        </p>
        <div className="v2-database-actions">
          <button className="primary-button" disabled={busy} onClick={onRetry} type="button">
            {busy ? 'Checking…' : 'Retry'}
          </button>
        </div>
      </div>
    </main>
  );
}

export default function V2App({ today: fixedToday }: V2AppProps) {
  const today = useTodayDateOnly(fixedToday);
  const [theme, setTheme] = useState<Theme>(() => getStoredTheme());
  const [status, setStatus] = useState<V2DatabaseStatus | null>(null);
  const [sidebarMode, setSidebarMode] = useState<V2SidebarMode>(() =>
    getStoredSidebarMode() === 'collapsed' ? 'collapsed' : 'expanded',
  );
  const [view, setView] = useState<V2View>({ kind: 'dashboard' });
  const [busy, setBusy] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);

  const fetchStatus = useCallback(async (allowInitialize: boolean) => {
    let next = await getV2DatabaseStatus();
    if (allowInitialize && next.state === 'notInitialized') {
      next = await initializeV2Database();
    }
    return next;
  }, []);

  const applyStatus = useCallback((next: V2DatabaseStatus) => {
    setStatus(next);
    if (next.state !== 'healthy') {
      setSearchOpen(false);
      setShortcutsOpen(false);
      setView({ kind: 'dashboard' });
    } else {
      setView((current) => {
        if (current.kind === 'exam') {
          return next.exams.some((exam) => exam.code === current.examCode)
            ? current
            : { kind: 'dashboard' };
        }
        if (
          current.kind === 'concepts' ||
          current.kind === 'syllabus' ||
          current.kind === 'practice'
        ) {
          return current.examCode === null ||
            next.exams.some((exam) => exam.code === current.examCode)
            ? current
            : { kind: current.kind, examCode: null };
        }
        if (
          current.kind === 'planner' ||
          current.kind === 'revision' ||
          current.kind === 'analytics' ||
          current.kind === 'resources' ||
          current.kind === 'backup' ||
          current.kind === 'settings'
        ) {
          return current;
        }
        return { kind: 'dashboard' };
      });
    }
  }, []);

  const loadStatus = useCallback(
    async (allowInitialize: boolean) => {
      setBusy(true);
      setLoadError(null);

      try {
        applyStatus(await fetchStatus(allowInitialize));
      } catch (error) {
        setLoadError(toCommandError(error).message);
      } finally {
        setBusy(false);
      }
    },
    [applyStatus, fetchStatus],
  );

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const next = await fetchStatus(true);
        if (!cancelled) {
          applyStatus(next);
        }
      } catch (error) {
        if (!cancelled) {
          setLoadError(toCommandError(error).message);
        }
      } finally {
        if (!cancelled) {
          setBusy(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [applyStatus, fetchStatus]);

  useEffect(() => {
    applyTheme(theme);
    storeTheme(theme);
  }, [theme]);

  useEffect(() => {
    storeSidebarMode(sidebarMode);
  }, [sidebarMode]);

  const toggleSidebarCollapse = useCallback(() => {
    setSidebarMode((current) => (current === 'collapsed' ? 'expanded' : 'collapsed'));
  }, []);

  useAppShortcuts(
    {
      onOpenSearch: () => {
        setShortcutsOpen(false);
        setSearchOpen(true);
      },
      onOpenShortcuts: () => {
        setSearchOpen(false);
        setShortcutsOpen(true);
      },
      onOpenExam: (examCode) => {
        if (searchOpen || shortcutsOpen) {
          return;
        }
        if (status?.exams.some((exam) => exam.code === examCode)) {
          setView({ kind: 'exam', examCode });
        }
      },
    },
    status?.state === 'healthy',
  );

  if (busy && status === null) {
    return (
      <main className="v2-database-state" aria-busy="true">
        <div className="v2-database-state-card" role="status">
          <BrandLockup subtitle="Scientific study workspace" />
          <p className="eyebrow">Physics Tracker · V2 database</p>
          <h1>Opening the V2 database</h1>
          <p>Checking the local database foundation and the four examination records.</p>
        </div>
      </main>
    );
  }

  if (status === null) {
    return (
      <BootstrapError
        busy={busy}
        message={loadError ?? 'The V2 database did not return a status.'}
        onRetry={() => void loadStatus(true)}
      />
    );
  }

  if (status.state !== 'healthy') {
    return (
      <DatabaseStateScreen
        busy={busy}
        loadError={loadError}
        onInitialize={() => void loadStatus(true)}
        onRetry={() => void loadStatus(false)}
        status={status}
      />
    );
  }

  const selectedExam: V2ExamSummary | null =
    view.kind === 'exam'
      ? (status.exams.find((exam) => exam.code === view.examCode) ?? null)
      : null;

  function openExam(examCode: string) {
    if (status?.exams.some((exam) => exam.code === examCode)) {
      setView({ kind: 'exam', examCode });
    }
  }

  function content(currentStatus: V2DatabaseStatus) {
    if (view.kind === 'exam' && !selectedExam) {
      return (
        <div className="page-container">
          <section className="v2-empty-panel" role="alert">
            <h1>Exam workspace unavailable</h1>
            <p>The selected exam code is not present in the V2 database.</p>
            <button
              className="secondary-button"
              onClick={() => setView({ kind: 'dashboard' })}
              type="button"
            >
              Return to combined dashboard
            </button>
          </section>
        </div>
      );
    }

    if (view.kind === 'concepts') {
      return (
        <ConceptsScreen
          exams={currentStatus.exams}
          initialConceptSlug={view.conceptSlug ?? null}
          initialExamCode={view.examCode}
          key={`${view.examCode ?? 'combined'}:${view.conceptSlug ?? ''}`}
          onOpenFlashcards={(conceptSlug, conceptTitle) =>
            setView({ kind: 'flashcards', conceptSlug, conceptTitle })
          }
          today={today}
        />
      );
    }

    if (view.kind === 'syllabus') {
      return (
        <SyllabusScreen
          exams={currentStatus.exams}
          initialExamCode={view.examCode}
          initialFocusNodeId={view.focusNodeId ?? null}
          key={`${view.examCode ?? 'combined'}:${view.focusNodeId ?? ''}`}
          onOpenFlashcards={(examCode, nodeId, nodeTitle) =>
            setView({ kind: 'flashcards', examCode, nodeId, nodeTitle })
          }
        />
      );
    }

    if (view.kind === 'flashcards') {
      return (
        <FlashcardsScreen
          exams={currentStatus.exams}
          initialConceptSlug={view.conceptSlug ?? null}
          initialConceptTitle={view.conceptTitle ?? null}
          initialExamCode={view.examCode ?? null}
          initialFlashcardId={view.flashcardId ?? null}
          initialNodeId={view.nodeId ?? null}
          initialNodeTitle={view.nodeTitle ?? null}
          initialQuery={view.query ?? null}
          key={`${view.examCode ?? ''}:${view.nodeId ?? ''}:${view.conceptSlug ?? ''}:${
            view.flashcardId ?? ''
          }:${view.query ?? ''}`}
          today={today}
        />
      );
    }

    if (view.kind === 'practice') {
      return (
        <PracticeScreen
          exams={currentStatus.exams}
          initialExamCode={view.examCode}
          initialQuestionId={view.questionId ?? null}
          key={`${view.examCode ?? 'practice'}:${view.questionId ?? ''}`}
          today={today}
        />
      );
    }

    if (view.kind === 'planner') {
      return <PlannerScreen exams={currentStatus.exams} today={today} />;
    }

    if (view.kind === 'revision') {
      return <RevisionScreen exams={currentStatus.exams} today={today} />;
    }

    if (view.kind === 'analytics') {
      return <AnalyticsScreen exams={currentStatus.exams} today={today} />;
    }

    if (view.kind === 'resources') {
      return (
        <ResourcesScreen
          exams={currentStatus.exams}
          initialQuery={view.query ?? null}
          key={view.query ?? 'library'}
        />
      );
    }

    if (view.kind === 'backup') {
      return <BackupScreen onChanged={() => void loadStatus(false)} />;
    }

    if (view.kind === 'settings') {
      return <SettingsScreen onChanged={() => void loadStatus(false)} />;
    }

    if (selectedExam) {
      return (
        <ExamWorkspace
          exam={selectedExam}
          exams={currentStatus.exams}
          onBack={() => setView({ kind: 'dashboard' })}
          onOpenConcepts={(examCode) => setView({ kind: 'concepts', examCode })}
          onOpenSyllabus={(examCode) => setView({ kind: 'syllabus', examCode })}
          today={today}
        />
      );
    }

    return (
      <CombinedDashboard
        onNavigate={setView}
        onOpenExam={openExam}
        status={currentStatus}
        today={today}
      />
    );
  }

  return (
    <div
      className="app-frame v2-app-frame"
      data-exam={view.kind === 'exam' ? view.examCode : undefined}
      data-sidebar-mode={sidebarMode}
      data-workspace={viewWorkspace(view)}
    >
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>
      <V2Sidebar
        exams={status.exams}
        mode={sidebarMode}
        onNavigate={setView}
        onOpenSearch={() => {
          setShortcutsOpen(false);
          setSearchOpen(true);
        }}
        onOpenShortcuts={() => {
          setSearchOpen(false);
          setShortcutsOpen(true);
        }}
        onThemeChange={setTheme}
        onToggleCollapse={toggleSidebarCollapse}
        theme={theme}
        view={view}
      />
      <div className="main-column">
        <main className="main-area" id="main-content" tabIndex={-1}>
          {loadError && (
            <p className="v2-inline-error" role="alert">
              {loadError}
            </p>
          )}
          {content(status)}
        </main>
      </div>
      {searchOpen && status.state === 'healthy' && (
        <GlobalSearch
          exams={status.exams}
          onClose={() => setSearchOpen(false)}
          onNavigate={(nextView) => {
            setSearchOpen(false);
            setView(nextView);
          }}
        />
      )}
      {shortcutsOpen && <ShortcutsDialog onClose={() => setShortcutsOpen(false)} />}
    </div>
  );
}
