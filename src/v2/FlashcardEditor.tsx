import { useEffect, useState, type FormEvent } from 'react';
import { toCommandError } from '../lib/commandError';
import {
  getV2ExamSyllabus,
  saveV2Flashcard,
  searchV2Concepts,
  type V2ConceptSummary,
  type V2ExamSyllabus,
  type V2ExamSummary,
  type V2FlashcardDetail,
  type V2FlashcardExamLinkInput,
} from './api';
import { DialogShell } from './DialogShell';
import { MathText } from './MathText';

type ExamSelection = {
  selected: boolean;
  nodeId: number | null;
};

type FlashcardEditorProps = {
  exams: V2ExamSummary[];
  subjects: string[];
  today: string;
  initialExamCode?: string | null;
  initialNodeId?: number | null;
  flashcard?: V2FlashcardDetail | null;
  onClose: () => void;
  onSaved: (detail: V2FlashcardDetail) => void;
};

function initialExamSelections(
  exams: V2ExamSummary[],
  detail: V2FlashcardDetail | null,
  initialExamCode: string | null,
  initialNodeId: number | null,
): Record<number, ExamSelection> {
  const selections: Record<number, ExamSelection> = {};
  for (const exam of exams) {
    selections[exam.id] = { selected: false, nodeId: null };
  }
  if (detail) {
    for (const link of detail.flashcard.examLinks) {
      selections[link.examId] = {
        selected: true,
        nodeId: link.syllabusNodeId,
      };
    }
  } else if (initialExamCode) {
    const exam = exams.find((item) => item.code === initialExamCode);
    if (exam) {
      selections[exam.id] = { selected: true, nodeId: initialNodeId };
    }
  }
  return selections;
}

export function FlashcardEditor({
  exams,
  subjects,
  today,
  initialExamCode = null,
  initialNodeId = null,
  flashcard = null,
  onClose,
  onSaved,
}: FlashcardEditorProps) {
  const [front, setFront] = useState(flashcard?.front ?? '');
  const [back, setBack] = useState(flashcard?.back ?? '');
  const [subject, setSubject] = useState(flashcard?.flashcard.subjectArea ?? '');
  const [conceptId, setConceptId] = useState<number | null>(flashcard?.flashcard.conceptId ?? null);
  const [conceptTitle, setConceptTitle] = useState(flashcard?.flashcard.conceptTitle ?? '');
  const [conceptQuery, setConceptQuery] = useState('');
  const [conceptOptions, setConceptOptions] = useState<V2ConceptSummary[]>([]);
  const [selections, setSelections] = useState<Record<number, ExamSelection>>(() =>
    initialExamSelections(exams, flashcard, initialExamCode, initialNodeId),
  );
  const [syllabi, setSyllabi] = useState<Record<number, V2ExamSyllabus>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const selectedIds = exams
      .filter((exam) => selections[exam.id]?.selected && !syllabi[exam.id])
      .map((exam) => exam.id);
    if (selectedIds.length === 0) {
      return undefined;
    }
    void (async () => {
      try {
        const loaded = await Promise.all(selectedIds.map((examId) => getV2ExamSyllabus(examId)));
        if (cancelled) {
          return;
        }
        setSyllabi((current) => {
          const next = { ...current };
          for (const syllabus of loaded) {
            next[syllabus.examId] = syllabus;
          }
          return next;
        });
      } catch (loadError) {
        if (!cancelled) {
          setError(toCommandError(loadError).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [exams, selections, syllabi]);

  useEffect(() => {
    let cancelled = false;
    const timer = window.setTimeout(
      () => {
        const query = conceptQuery.trim();
        if (!query) {
          setConceptOptions([]);
          return;
        }
        void (async () => {
          try {
            const result = await searchV2Concepts({ query, limit: 6, includeArchived: false });
            if (!cancelled) {
              setConceptOptions(result.concepts);
            }
          } catch {
            if (!cancelled) {
              setConceptOptions([]);
            }
          }
        })();
      },
      conceptQuery.trim() ? 220 : 0,
    );
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [conceptQuery]);

  function toggleExam(examId: number, selected: boolean) {
    setSelections((current) => ({
      ...current,
      [examId]: {
        selected,
        nodeId: selected ? (current[examId]?.nodeId ?? null) : null,
      },
    }));
  }

  function setNode(examId: number, nodeId: number | null) {
    setSelections((current) => ({
      ...current,
      [examId]: {
        selected: true,
        nodeId,
      },
    }));
    if (!subject.trim() && nodeId !== null) {
      const node = syllabi[examId]?.nodes.find((item) => item.id === nodeId);
      if (node) {
        setSubject(node.category || node.title);
      }
    }
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    if (!front.trim() || !back.trim() || !subject.trim()) {
      setError('Front, back and subject are required.');
      return;
    }
    setBusy(true);
    try {
      const examLinks: V2FlashcardExamLinkInput[] = exams
        .filter((exam) => selections[exam.id]?.selected)
        .map((exam) => ({
          examId: exam.id,
          syllabusNodeId: selections[exam.id]?.nodeId ?? null,
        }));
      const detail = await saveV2Flashcard({
        id: flashcard?.flashcard.id ?? null,
        front,
        back,
        subjectArea: subject,
        conceptId,
        examLinks,
        today,
      });
      onSaved(detail);
      onClose();
    } catch (saveError) {
      setError(toCommandError(saveError).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <DialogShell
      ariaLabel={flashcard ? 'Edit flashcard' : 'New flashcard'}
      className="v2-flashcard-editor-dialog"
      onClose={onClose}
    >
      <form className="v2-flashcard-editor" onSubmit={(event) => void submit(event)}>
        <header className="v2-flashcard-editor-head">
          <div>
            <p className="eyebrow">Flashcard</p>
            <h2>{flashcard ? 'Edit card' : 'New card'}</h2>
          </div>
          <button
            aria-label="Close flashcard editor"
            className="secondary-button"
            onClick={onClose}
            type="button"
          >
            Cancel
          </button>
        </header>

        <label className="v2-field">
          <span>Front</span>
          <textarea
            data-autofocus
            maxLength={20000}
            onChange={(event) => setFront(event.target.value)}
            placeholder="Prompt or question. Use $...$ for inline LaTeX and $$...$$ for display math."
            rows={3}
            value={front}
          />
        </label>
        {front.trim() && (
          <div className="v2-flashcard-preview">
            <MathText text={front} />
          </div>
        )}

        <label className="v2-field">
          <span>Back</span>
          <textarea
            maxLength={20000}
            onChange={(event) => setBack(event.target.value)}
            placeholder="Answer or explanation."
            rows={3}
            value={back}
          />
        </label>
        {back.trim() && (
          <div className="v2-flashcard-preview">
            <MathText text={back} />
          </div>
        )}

        <label className="v2-field">
          <span>Physics subject</span>
          <input
            list="v2-flashcard-subjects"
            maxLength={200}
            onChange={(event) => setSubject(event.target.value)}
            placeholder="e.g. Classical Mechanics"
            value={subject}
          />
          <datalist id="v2-flashcard-subjects">
            {subjects.map((item) => (
              <option key={item} value={item} />
            ))}
          </datalist>
        </label>

        <fieldset className="v2-flashcard-fieldset">
          <legend>Optional shared concept</legend>
          {conceptId !== null ? (
            <div className="v2-flashcard-selected-concept">
              <span>{conceptTitle}</span>
              <button
                className="secondary-button"
                onClick={() => {
                  setConceptId(null);
                  setConceptTitle('');
                }}
                type="button"
              >
                Remove
              </button>
            </div>
          ) : (
            <>
              <label className="v2-field">
                <span>Search concepts</span>
                <input
                  aria-label="Search shared concepts"
                  onChange={(event) => setConceptQuery(event.target.value)}
                  placeholder="Search the shared concept library"
                  value={conceptQuery}
                />
              </label>
              {conceptOptions.length > 0 && (
                <ul className="v2-flashcard-concept-options">
                  {conceptOptions.map((concept) => (
                    <li key={concept.id}>
                      <button
                        className="link-button"
                        onClick={() => {
                          setConceptId(concept.id);
                          setConceptTitle(concept.title);
                          setConceptQuery('');
                          setConceptOptions([]);
                        }}
                        type="button"
                      >
                        {concept.title}
                        <span> · {concept.domain}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </>
          )}
        </fieldset>

        <fieldset className="v2-flashcard-fieldset">
          <legend>Examinations and syllabus topic</legend>
          <p className="v2-empty-copy">
            Leave every examination unchecked to share this card across all four examinations.
          </p>
          {exams.map((exam) => {
            const selection = selections[exam.id];
            const syllabus = syllabi[exam.id];
            return (
              <div className="v2-flashcard-exam-link" key={exam.id}>
                <label>
                  <input
                    aria-label={`Include ${exam.shortName}`}
                    checked={selection?.selected ?? false}
                    onChange={(event) => toggleExam(exam.id, event.target.checked)}
                    type="checkbox"
                  />
                  <span>{exam.shortName}</span>
                </label>
                {selection?.selected && (
                  <select
                    aria-label={`Syllabus topic for ${exam.shortName}`}
                    onChange={(event) =>
                      setNode(exam.id, event.target.value ? Number(event.target.value) : null)
                    }
                    value={selection.nodeId ?? ''}
                  >
                    <option value="">No syllabus topic</option>
                    {(syllabus?.nodes ?? []).map((node) => (
                      <option key={node.id} value={node.id}>
                        {node.nodePath.join(' › ')} · {node.title}
                      </option>
                    ))}
                  </select>
                )}
              </div>
            );
          })}
        </fieldset>

        {error && (
          <p className="v2-study-error" role="alert">
            {error}
          </p>
        )}

        <div className="v2-form-actions">
          <button className="secondary-button" onClick={onClose} type="button">
            Cancel
          </button>
          <button className="primary-button" disabled={busy} type="submit">
            {busy ? 'Saving…' : 'Save card'}
          </button>
        </div>
      </form>
    </DialogShell>
  );
}
