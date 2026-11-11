import { lazy, Suspense, useCallback, useEffect, useRef, useState } from 'react';
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
import { FormulaRouteErrorBoundary } from './FormulaRouteErrorBoundary';
import { CalculatorScreen } from './CalculatorScreen';
import { CalculatorSessionProvider } from './calculator/CalculatorSessionProvider';
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
import { GlobalTimerIndicator } from './timer/GlobalTimerIndicator';
import { NotesPanel } from './notes/NotesPanel';
import { NotesProvider } from './notes/NotesProvider';
import { NotesScreen } from './notes/NotesScreen';
import { useNotes } from './notes/notesContext';
import { TimerProvider } from './timer/TimerProvider';
import { V2Sidebar, type V2SidebarMode } from './V2Sidebar';
import { retainViewAfterStatus } from './viewStatus';

const FormulaReferenceScreen = lazy(() =>
  import('./formulas/FormulaReferenceScreen').then((module) => ({
    default: module.FormulaReferenceScreen,
  })),
);

const TermReferenceScreen = lazy(() =>
  import('./terms/TermReferenceScreen').then((module) => ({
    default: module.TermReferenceScreen,
  })),
);

type V2AppProps = {
  today?: string;
};

function timerViewKey(view: V2View): string {
  return view.kind === 'exam' ? `exam:${view.examCode}` : view.kind;
}

// eslint-disable-next-line react-refresh/only-export-components
export function acknowledgeTimerFocusRequest(current: number, requestId: number): number {
  return current === requestId ? 0 : current;
}

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
    case 'calculator':
      return 'calculator';
    case 'formulas':
      return 'formulas';
    case 'terms':
      return 'terms';
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
    case 'notes':
      return 'notes';
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
        <p className="eyebrow">Physics Tracker · database</p>
        <h1>Physics Tracker could not read its database</h1>
        <p>{message}</p>
        <p>
          The app does not fall back to legacy data. Check the application-data folder and retry.
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

export default function V2App(props: V2AppProps) {
  return (
    <CalculatorSessionProvider>
      <NotesProvider>
        <V2AppShell {...props} />
      </NotesProvider>
    </CalculatorSessionProvider>
  );
}

function V2AppShell({ today: fixedToday }: V2AppProps) {
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
  const [timerFocusRequest, setTimerFocusRequest] = useState(0);
  const [timerNotice, setTimerNotice] = useState<string | null>(null);
  const { openPanel, closePanel, panelOpen } = useNotes();
  const [notesSelectionId, setNotesSelectionId] = useState<number | null>(null);
  const timerFocusSequenceRef = useRef(0);
  const timerFocusTargetRef = useRef<string | null>(null);

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
      setView((current) => retainViewAfterStatus(current, next.exams));
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

  const handleTimerFocused = useCallback((requestId: number) => {
    setTimerFocusRequest((current) => acknowledgeTimerFocusRequest(current, requestId));
    setTimerNotice(null);
  }, []);

  const handleTimerUnavailable = useCallback((requestId: number) => {
    setTimerFocusRequest((current) => acknowledgeTimerFocusRequest(current, requestId));
    setTimerNotice(
      'The study timer section is hidden. Enable it under Customize dashboard to open its controls.',
    );
  }, []);

  useEffect(() => {
    if (timerFocusRequest === 0) {
      return;
    }
    const target = timerFocusTargetRef.current;
    if (!target) {
      return;
    }
    if (timerViewKey(view) !== target) {
      timerFocusTargetRef.current = null;
      setTimerFocusRequest(0);
      setTimerNotice(null);
    }
  }, [timerFocusRequest, view]);

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
      onOpenNotes: () => {
        setSearchOpen(false);
        setShortcutsOpen(false);
        openPanel();
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
          <p className="eyebrow">Physics Tracker · database</p>
          <h1>Opening the database</h1>
          <p>Checking the local database foundation and the four examination records.</p>
        </div>
      </main>
    );
  }

  if (status === null) {
    return (
      <BootstrapError
        busy={busy}
        message={loadError ?? 'The database did not return a status.'}
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
            <p>The selected exam code is not present in the database.</p>
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
          initialFocusNodeCode={view.focusNodeCode ?? null}
          initialFocusNodeId={view.focusNodeId ?? null}
          key={`${view.examCode ?? 'combined'}:${view.focusNodeId ?? ''}:${
            view.focusNodeCode ?? ''
          }`}
          onOpenFlashcards={(examCode, nodeId, nodeTitle) =>
            setView({ kind: 'flashcards', examCode, nodeId, nodeTitle })
          }
          onOpenFormulas={(examCode, nodeCode, nodeTitle) =>
            setView({ kind: 'formulas', examCode, nodeCode, nodeTitle })
          }
          onOpenTerms={(examCode, nodeCode, nodeTitle) =>
            setView({ kind: 'terms', examCode, nodeCode, nodeTitle })
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

    if (view.kind === 'calculator') {
      return <CalculatorScreen />;
    }

    if (view.kind === 'formulas') {
      return (
        <FormulaRouteErrorBoundary onBack={() => setView({ kind: 'dashboard' })}>
          <Suspense
            fallback={
              <div className="page-container" role="status">
                <p className="v2-empty-copy">Loading formula reference…</p>
              </div>
            }
          >
            <FormulaReferenceScreen
              exams={currentStatus.exams}
              initialExamCode={view.examCode ?? null}
              initialFormulaId={view.formulaId ?? null}
              initialNodeCode={view.nodeCode ?? null}
              initialNodeTitle={view.nodeTitle ?? null}
              initialQuery={view.query ?? null}
              initialSubject={view.subject ?? null}
              initialTopic={view.topic ?? null}
              key={`${view.examCode ?? 'combined'}:${view.formulaId ?? ''}:${
                view.nodeCode ?? ''
              }:${view.subject ?? ''}:${view.topic ?? ''}:${view.query ?? ''}`}
              onOpenSyllabus={(examCode, nodeCode) =>
                setView({ kind: 'syllabus', examCode, focusNodeCode: nodeCode })
              }
              onOpenTerms={(formulaId, formulaTitle) =>
                setView({ kind: 'terms', formulaId, formulaTitle })
              }
            />
          </Suspense>
        </FormulaRouteErrorBoundary>
      );
    }

    if (view.kind === 'terms') {
      return (
        <FormulaRouteErrorBoundary
          message="The terms and units reference could not be loaded. This does not affect your saved study data."
          onBack={() => setView({ kind: 'dashboard' })}
          title="Terms and units unavailable"
        >
          <Suspense
            fallback={
              <div className="page-container" role="status">
                <p className="v2-empty-copy">Loading terms and units…</p>
              </div>
            }
          >
            <TermReferenceScreen
              exams={currentStatus.exams}
              initialExamCode={view.examCode ?? null}
              initialFormulaId={view.formulaId ?? null}
              initialFormulaTitle={view.formulaTitle ?? null}
              initialKind={view.kindFilter ?? null}
              initialNodeCode={view.nodeCode ?? null}
              initialNodeTitle={view.nodeTitle ?? null}
              initialQuery={view.query ?? null}
              initialSubject={view.subject ?? null}
              initialTermId={view.termId ?? null}
              initialTopic={view.topic ?? null}
              key={`${view.examCode ?? 'combined'}:${view.termId ?? ''}:${
                view.formulaId ?? ''
              }:${view.nodeCode ?? ''}:${view.subject ?? ''}:${view.topic ?? ''}:${
                view.kindFilter ?? ''
              }:${view.query ?? ''}`}
              onOpenFormula={(formulaId) => setView({ kind: 'formulas', formulaId })}
              onOpenSyllabus={(examCode, nodeCode) =>
                setView({ kind: 'syllabus', examCode, focusNodeCode: nodeCode })
              }
            />
          </Suspense>
        </FormulaRouteErrorBoundary>
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

    if (view.kind === 'notes') {
      return (
        <NotesScreen
          exams={currentStatus.exams}
          initialSelectedNoteId={notesSelectionId}
          onSelectNote={setNotesSelectionId}
        />
      );
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
          focusTimerRequest={timerFocusRequest}
          onTimerFocused={handleTimerFocused}
          onBack={() => setView({ kind: 'dashboard' })}
          onOpenConcepts={(examCode) => setView({ kind: 'concepts', examCode })}
          onOpenFormulas={(examCode) => setView({ kind: 'formulas', examCode })}
          onOpenTerms={(examCode) => setView({ kind: 'terms', examCode })}
          onOpenSyllabus={(examCode) => setView({ kind: 'syllabus', examCode })}
          today={today}
        />
      );
    }

    return (
      <CombinedDashboard
        focusTimerRequest={timerFocusRequest}
        onTimerFocused={handleTimerFocused}
        onTimerUnavailable={handleTimerUnavailable}
        onNavigate={setView}
        onOpenExam={openExam}
        status={currentStatus}
        today={today}
      />
    );
  }

  function openTimer(examIds: number[]) {
    setTimerNotice(null);
    const targetExam = status?.exams.find((exam) => examIds.includes(exam.id)) ?? null;
    const headingExists = Boolean(document.getElementById('study-timer-heading'));
    const targetView: V2View = headingExists
      ? view
      : targetExam
        ? { kind: 'exam', examCode: targetExam.code }
        : { kind: 'dashboard' };
    timerFocusTargetRef.current = timerViewKey(targetView);
    if (!headingExists) {
      setView(targetView);
    }
    timerFocusSequenceRef.current += 1;
    setTimerFocusRequest(timerFocusSequenceRef.current);
  }

  return (
    <TimerProvider>
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
          onOpenNotes={openPanel}
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
            {timerNotice && (
              <p className="v2-global-timer-notice" role="status">
                {timerNotice}
              </p>
            )}
            {loadError && (
              <p className="v2-inline-error" role="alert">
                {loadError}
              </p>
            )}
            {content(status)}
          </main>
        </div>
        <GlobalTimerIndicator onOpenTimer={openTimer} />
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
        {panelOpen && status.state === 'healthy' && (
          <NotesPanel
            exams={status.exams}
            onOpenFullPage={() => {
              setView({ kind: 'notes' });
              closePanel();
            }}
            onSelectNote={setNotesSelectionId}
            selectedNoteId={notesSelectionId}
          />
        )}
      </div>
    </TimerProvider>
  );
}
