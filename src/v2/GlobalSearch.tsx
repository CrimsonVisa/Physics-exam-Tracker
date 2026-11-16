import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { toCommandError } from '../lib/commandError';
import { Icon } from '../components/NavIcon';
import { globalSearchV2, type V2ExamSummary, type V2GlobalSearchResult } from './api';
import { DialogShell } from './DialogShell';
import { MathText } from './MathText';
import { containsExplicitMath } from './math/render';
import type { V2View } from './navigation';

type GlobalSearchProps = {
  exams: V2ExamSummary[];
  onClose: () => void;
  onNavigate: (view: V2View) => void;
};

type SearchGroupId = 'syllabus' | 'concepts' | 'flashcards' | 'pyqs' | 'resources';

type SearchItem = {
  key: string;
  group: SearchGroupId;
  title: string;
  secondary: string;
  badge: string;
  view: V2View;
  index: number;
};

const GROUPS: Array<{ id: SearchGroupId; label: string; workspace: string }> = [
  { id: 'syllabus', label: 'Syllabus topics and units', workspace: 'syllabus' },
  { id: 'concepts', label: 'Concepts', workspace: 'concepts' },
  { id: 'flashcards', label: 'Flashcards', workspace: 'flashcards' },
  { id: 'pyqs', label: 'Previous-year questions', workspace: 'practice' },
  { id: 'resources', label: 'Resources', workspace: 'resources' },
];

const KIND_LABELS: Record<string, string> = {
  unit: 'Unit',
  section: 'Section',
  topic: 'Topic',
};

const RESOURCE_TYPE_LABELS: Record<string, string> = {
  book: 'Book',
  notes: 'Notes',
  video: 'Video',
  paper: 'Paper',
  website: 'Website',
  other: 'Other',
};

function examShortName(exams: V2ExamSummary[], code: string): string {
  return exams.find((exam) => exam.code === code)?.shortName ?? code;
}

function highlight(text: string, query: string): ReactNode {
  const needle = query.trim();
  if (needle.length < 2) {
    return text;
  }
  const index = text.toLowerCase().indexOf(needle.toLowerCase());
  if (index < 0) {
    return text;
  }
  return (
    <>
      {text.slice(0, index)}
      <mark>{text.slice(index, index + needle.length)}</mark>
      {text.slice(index + needle.length)}
    </>
  );
}

function SearchResultText({ text, query }: { text: string; query: string }) {
  if (containsExplicitMath(text)) {
    return <MathText text={text} />;
  }
  return <>{highlight(text, query)}</>;
}

function buildItems(result: V2GlobalSearchResult | null, exams: V2ExamSummary[]): SearchItem[] {
  if (!result) {
    return [];
  }
  const items: SearchItem[] = [];
  let index = 0;
  for (const hit of result.syllabus) {
    const kind = KIND_LABELS[hit.nodeKind] ?? hit.nodeKind;
    const progress =
      hit.progressStatus === 'not_started' ? '' : ` · ${hit.progressStatus.replace('_', ' ')}`;
    items.push({
      key: `syllabus-${hit.nodeId}`,
      group: 'syllabus',
      title: hit.title,
      secondary:
        [hit.breadcrumb, kind, `${hit.examShortName}${hit.examIsActive ? '' : ' (inactive)'}`]
          .filter(Boolean)
          .join(' · ') + progress,
      badge: kind,
      view: { kind: 'syllabus', examCode: hit.examCode, focusNodeId: hit.nodeId },
      index: index++,
    });
  }
  for (const hit of result.concepts) {
    const mastery =
      hit.masteryScore === null || hit.evidenceCount === 0
        ? 'No mastery evidence'
        : `Mastery ${hit.masteryScore.toFixed(1)} / 100`;
    items.push({
      key: `concept-${hit.conceptId}`,
      group: 'concepts',
      title: hit.title,
      secondary: [hit.domain, mastery].join(' · '),
      badge: hit.examCodes.length > 0 ? `${hit.examCodes.length} exam link(s)` : 'Shared',
      view: { kind: 'concepts', examCode: null, conceptSlug: hit.slug },
      index: index++,
    });
  }
  for (const hit of result.flashcards) {
    const examBadges = hit.examCodes.map((code) => examShortName(exams, code)).join(' · ');
    items.push({
      key: `flashcard-${hit.flashcardId}`,
      group: 'flashcards',
      title: hit.frontSnippet,
      secondary: [hit.subjectArea || 'No subject', hit.backSnippet].filter(Boolean).join(' · '),
      badge: examBadges || 'Shared',
      view: { kind: 'flashcards', flashcardId: hit.flashcardId },
      index: index++,
    });
  }
  for (const hit of result.pyqs) {
    items.push({
      key: `pyq-${hit.questionId}`,
      group: 'pyqs',
      title: `${hit.year} ${hit.sessionLabel} · ${hit.questionNumber}`,
      secondary: `${hit.sectionTitle} · ${hit.statementSnippet}`,
      badge: hit.examShortName,
      view: { kind: 'practice', examCode: hit.examCode, questionId: hit.questionId },
      index: index++,
    });
  }
  for (const hit of result.resources) {
    const examBadges = hit.examCodes.map((code) => examShortName(exams, code)).join(' · ');
    items.push({
      key: `resource-${hit.resourceId}`,
      group: 'resources',
      title: hit.title,
      secondary: [
        RESOURCE_TYPE_LABELS[hit.resourceType] ?? hit.resourceType,
        hit.subjectArea || 'No subject area',
      ].join(' · '),
      badge: examBadges || 'Shared',
      view: { kind: 'resources', query: hit.title },
      index: index++,
    });
  }
  return items;
}

export function GlobalSearch({ exams, onClose, onNavigate }: GlobalSearchProps) {
  const [query, setQuery] = useState('');
  const [scope, setScope] = useState('all');
  const [result, setResult] = useState<V2GlobalSearchResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeIndex, setActiveIndex] = useState(0);
  const [retryKey, setRetryKey] = useState(0);
  const sequence = useRef(0);

  const items = useMemo(() => buildItems(result, exams), [result, exams]);
  const trimmed = query.trim();
  const scopeExamId = scope === 'all' ? null : Number(scope);

  useEffect(() => {
    sequence.current += 1;
    const sequenceId = sequence.current;
    let timer: number | undefined;
    void (async () => {
      if (trimmed.length < 2) {
        setResult(null);
        setLoading(false);
        setError(null);
        return;
      }
      setLoading(true);
      setError(null);
      timer = window.setTimeout(() => {
        void (async () => {
          try {
            const next = await globalSearchV2({
              query: trimmed,
              examId: scopeExamId,
              limitPerCategory: 6,
            });
            if (sequence.current !== sequenceId) {
              return;
            }
            setResult(next);
            setActiveIndex(0);
          } catch (searchError) {
            if (sequence.current !== sequenceId) {
              return;
            }
            setResult(null);
            setError(toCommandError(searchError).message);
          } finally {
            if (sequence.current === sequenceId) {
              setLoading(false);
            }
          }
        })();
      }, 220);
    })();
    return () => {
      if (timer !== undefined) {
        window.clearTimeout(timer);
      }
    };
  }, [trimmed, scopeExamId, retryKey]);

  const grouped = GROUPS.map((group) => ({
    ...group,
    items: items.filter((item) => item.group === group.id),
  })).filter((group) => group.items.length > 0);

  function choose(item: SearchItem) {
    onNavigate(item.view);
  }

  function onInputKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.nativeEvent.isComposing) {
      return;
    }
    if (items.length === 0) {
      return;
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      setActiveIndex((index) => (index + 1) % items.length);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      setActiveIndex((index) => (index - 1 + items.length) % items.length);
    } else if (event.key === 'Home') {
      event.preventDefault();
      setActiveIndex(0);
    } else if (event.key === 'End') {
      event.preventDefault();
      setActiveIndex(items.length - 1);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      const item = items[Math.min(activeIndex, items.length - 1)];
      if (item) {
        choose(item);
      }
    }
  }

  const activeItem = items[Math.min(activeIndex, items.length - 1)] ?? null;
  const statusText = loading
    ? 'Searching…'
    : error
      ? 'Search unavailable'
      : trimmed.length < 2
        ? ''
        : items.length === 0
          ? 'No matches'
          : `${items.length} result${items.length === 1 ? '' : 's'}`;

  return (
    <DialogShell ariaLabel="Global search" className="v2-search-dialog" onClose={onClose}>
      <div className="v2-search-head">
        <Icon className="v2-search-icon" name="search" size={19} />
        <input
          aria-activedescendant={activeItem ? `v2-search-option-${activeItem.index}` : undefined}
          aria-autocomplete="list"
          aria-controls="v2-global-search-list"
          aria-expanded={items.length > 0}
          aria-label="Search Physics Tracker"
          className="v2-search-input"
          data-autofocus
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={onInputKeyDown}
          placeholder="Search syllabus, concepts, PYQs and resources…"
          role="combobox"
          type="text"
          value={query}
        />
        <span aria-hidden="true" className="v2-search-esc">
          <kbd>Esc</kbd>
        </span>
      </div>

      <div className="v2-search-scopebar">
        <label className="v2-search-scope">
          <span>Exam scope</span>
          <select
            aria-label="Exam scope"
            onChange={(event) => setScope(event.target.value)}
            value={scope}
          >
            <option value="all">All examinations</option>
            {exams.map((exam) => (
              <option key={exam.code} value={exam.id}>
                {exam.shortName}
              </option>
            ))}
          </select>
        </label>
        <p aria-live="polite" className="v2-search-status">
          {statusText}
        </p>
      </div>

      <div className="v2-search-body">
        {error ? (
          <div className="v2-search-empty" role="alert">
            <p>{error}</p>
            <button
              className="secondary-button"
              onClick={() => setRetryKey((key) => key + 1)}
              type="button"
            >
              Retry search
            </button>
          </div>
        ) : trimmed.length < 2 ? (
          <div className="v2-search-empty">
            <p>Type at least two characters to search across:</p>
            <ul className="v2-search-categories">
              <li>Exam syllabus units, sections and topics</li>
              <li>Shared concepts and mastery evidence</li>
              <li>Previous-year questions</li>
              <li>Study resources</li>
            </ul>
          </div>
        ) : loading && !result ? (
          <p className="v2-search-empty">Searching…</p>
        ) : items.length === 0 ? (
          <div className="v2-search-empty">
            <p>
              No matches for “{trimmed}”{scope !== 'all' ? ' in this examination' : ''}.
            </p>
            <p className="v2-empty-copy">
              Try a shorter term, a topic title, a question number or a resource type.
            </p>
          </div>
        ) : (
          <div
            aria-label="Search results"
            className="v2-search-results"
            id="v2-global-search-list"
            role="listbox"
          >
            {grouped.map((group) => (
              <div className="v2-search-group" data-workspace={group.workspace} key={group.id}>
                <p className="v2-search-group-label">
                  <span aria-hidden="true" className="v2-search-group-dot" />
                  {group.label}
                </p>
                {group.items.map((item) => {
                  const active = activeItem?.key === item.key;
                  return (
                    <button
                      aria-selected={active}
                      className={`v2-search-result${active ? ' is-active' : ''}`}
                      id={`v2-search-option-${item.index}`}
                      key={item.key}
                      onClick={() => choose(item)}
                      onMouseEnter={() => setActiveIndex(item.index)}
                      role="option"
                      tabIndex={-1}
                      type="button"
                    >
                      <span className="v2-search-result-copy">
                        <strong>
                          <SearchResultText query={trimmed} text={item.title} />
                        </strong>
                        <span>
                          <SearchResultText query={trimmed} text={item.secondary} />
                        </span>
                      </span>
                      <span className="v2-search-badge">{item.badge}</span>
                    </button>
                  );
                })}
              </div>
            ))}
          </div>
        )}
      </div>

      <footer className="v2-search-footer">
        <span>
          <kbd>↑</kbd>
          <kbd>↓</kbd> navigate · <kbd>Enter</kbd> open · <kbd>Esc</kbd> close
        </span>
        <span className="v2-search-footer-note">
          Scope only filters this search; it never changes your workspace.
        </span>
      </footer>
    </DialogShell>
  );
}
