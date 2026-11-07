import { useCallback, useEffect, useMemo, useState } from 'react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { openPath, openUrl } from '@tauri-apps/plugin-opener';
import { toCommandError } from '../lib/commandError';
import {
  acquireV2PyqSource,
  extractV2PyqSource,
  getV2PyqSourceCounts,
  listV2PyqAnswerKeyConflicts,
  listV2PyqAnswerKeys,
  listV2PyqExtractionCandidates,
  listV2PyqSources,
  previewV2PyqAnswerKey,
  reconcileV2PyqSource,
  rejectV2PyqExtractionCandidate,
  saveV2PyqAnswerKey,
  saveV2PyqSource,
  setV2PyqSourceStatus,
  updateV2PyqExtractionCandidate,
  verifyV2PyqExtractionCandidate,
  type V2AnswerKeyConflict,
  type V2AnswerKeyPreview,
  type V2ExtractionCandidate,
  type V2PracticeOptions,
  type V2PyqAnswerKey,
  type V2PyqSource,
  type V2QuestionOption,
  type V2SourceCounts,
} from './api';

type SourcesPanelProps = {
  examId: number;
  today: string;
  options: V2PracticeOptions;
  refresh: () => Promise<void>;
};

function optionText(options: V2QuestionOption[]): string {
  return options.map((option) => `${option.key}) ${option.text}`).join('\n');
}

function parseOptions(text: string): V2QuestionOption[] {
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line, index) => {
      const match = line.match(/^([A-Za-z0-9]{1,8})[).:-]\s*(.+)$/);
      if (match) {
        return { key: match[1].toUpperCase(), text: match[2].trim() };
      }
      return { key: String.fromCharCode(65 + index), text: line };
    });
}

function shortHash(hash: string | null): string {
  if (!hash) return 'No hash';
  return `${hash.slice(0, 12)}…${hash.slice(-8)}`;
}

function sourceTypeLabel(value: string): string {
  if (value === 'question_paper') return 'Question paper';
  if (value === 'answer_key') return 'Answer key';
  return 'Supporting';
}

function statusLabel(value: string): string {
  return value.replaceAll('_', ' ');
}

export function SourcesPanel({ examId, today, options, refresh }: SourcesPanelProps) {
  const [sources, setSources] = useState<V2PyqSource[]>([]);
  const [answerKeys, setAnswerKeys] = useState<V2PyqAnswerKey[]>([]);
  const [candidates, setCandidates] = useState<V2ExtractionCandidate[]>([]);
  const [conflicts, setConflicts] = useState<V2AnswerKeyConflict[]>([]);
  const [counts, setCounts] = useState<V2SourceCounts | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const [year, setYear] = useState(Number(today.slice(0, 4)) || 2026);
  const [sourceType, setSourceType] = useState<'question_paper' | 'answer_key' | 'supporting'>(
    'question_paper',
  );
  const [title, setTitle] = useState('');
  const [authority, setAuthority] = useState('GATE Organising Institute');
  const [institution, setInstitution] = useState('');
  const [sourceUrl, setSourceUrl] = useState('');
  const [sessionLabel, setSessionLabel] = useState('');
  const [pageCount, setPageCount] = useState('40');
  const [documentVersion, setDocumentVersion] = useState('final');
  const [keyStatus, setKeyStatus] = useState('final');
  const [publishedAt, setPublishedAt] = useState('');
  const [associatedSourceId, setAssociatedSourceId] = useState('');
  const [filePath, setFilePath] = useState('');

  const [keySourceId, setKeySourceId] = useState('');
  const [keyVersionLabel, setKeyVersionLabel] = useState('final-1');
  const [keyPublishedAt, setKeyPublishedAt] = useState('');
  const [keyVerified, setKeyVerified] = useState(true);
  const [keyVerifiedBy, setKeyVerifiedBy] = useState('local reviewer');
  const [keyNotes, setKeyNotes] = useState('');
  const [keyText, setKeyText] = useState('');
  const [keyPreview, setKeyPreview] = useState<V2AnswerKeyPreview | null>(null);

  const [paperSourceId, setPaperSourceId] = useState('');
  const [extractionKeyId, setExtractionKeyId] = useState('');
  const [extractionMethod, setExtractionMethod] = useState('pdf_text');
  const [extractionText, setExtractionText] = useState('');
  const [extractionNotes, setExtractionNotes] = useState('');

  const [filterYear, setFilterYear] = useState('');
  const [filterStatus, setFilterStatus] = useState('needs_review');
  const [selectedCandidateId, setSelectedCandidateId] = useState<number | null>(null);
  const [editStatement, setEditStatement] = useState('');
  const [editOptions, setEditOptions] = useState('');
  const [editCorrectAnswer, setEditCorrectAnswer] = useState('');
  const [editCorrectRange, setEditCorrectRange] = useState('');
  const [editMaxMarks, setEditMaxMarks] = useState('1');
  const [editNegativeMarks, setEditNegativeMarks] = useState('0');
  const [editTolerance, setEditTolerance] = useState('0');
  const [editDifficulty, setEditDifficulty] = useState('3');
  const [editSection, setEditSection] = useState('gate-ph');
  const [editType, setEditType] = useState('MCQ');
  const [editTopic, setEditTopic] = useState('');
  const [editConcepts, setEditConcepts] = useState<number[]>([]);
  const [editNotes, setEditNotes] = useState('');

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [nextSources, nextKeys, nextCandidates, nextCounts, nextConflicts] = await Promise.all([
        listV2PyqSources({ examId }),
        listV2PyqAnswerKeys(examId, null),
        listV2PyqExtractionCandidates({
          examId,
          year: filterYear ? Number(filterYear) : null,
          reviewStatus: filterStatus || null,
          limit: 300,
        }),
        getV2PyqSourceCounts(),
        listV2PyqAnswerKeyConflicts(examId),
      ]);
      setSources(nextSources);
      setAnswerKeys(nextKeys);
      setCandidates(nextCandidates);
      setCounts(nextCounts);
      setConflicts(nextConflicts);
    } catch (caught) {
      setError(toCommandError(caught).message);
    } finally {
      setLoading(false);
    }
  }, [examId, filterStatus, filterYear]);

  useEffect(() => {
    let cancelled = false;
    void Promise.resolve().then(() => {
      if (!cancelled) void load();
    });
    return () => {
      cancelled = true;
    };
  }, [load]);

  const paperSources = useMemo(
    () => sources.filter((source) => source.sourceType === 'question_paper'),
    [sources],
  );
  const keySources = useMemo(
    () => sources.filter((source) => source.sourceType === 'answer_key'),
    [sources],
  );
  const topics = useMemo(
    () => options.topics.filter((topic) => topic.nodeKind === 'topic'),
    [options.topics],
  );
  const sections = useMemo(
    () => options.sections.filter((section) => section.code.startsWith('gate-')),
    [options.sections],
  );
  const selectedCandidate = useMemo(
    () => candidates.find((candidate) => candidate.id === selectedCandidateId) ?? null,
    [candidates, selectedCandidateId],
  );

  function chooseCandidate(candidate: V2ExtractionCandidate) {
    setSelectedCandidateId(candidate.id);
    setEditStatement(candidate.statement);
    setEditOptions(optionText(candidate.options));
    setEditCorrectAnswer(candidate.correctAnswer);
    setEditCorrectRange(candidate.correctRange);
    setEditMaxMarks(String(candidate.maxMarks ?? 1));
    setEditNegativeMarks(String(candidate.negativeMarks));
    setEditTolerance(String(candidate.numericTolerance));
    setEditDifficulty(String(candidate.difficulty));
    setEditSection(candidate.sectionCode || 'gate-ph');
    setEditType(candidate.questionTypeCode || 'MCQ');
    setEditTopic(candidate.syllabusNodeId ? String(candidate.syllabusNodeId) : '');
    setEditConcepts(candidate.conceptIds);
    setEditNotes(candidate.reviewNotes);
  }

  async function runAction(action: () => Promise<string>) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      setNotice(await action());
      await load();
      await refresh();
    } catch (caught) {
      setError(toCommandError(caught).message);
    } finally {
      setBusy(false);
    }
  }

  async function pickPdf() {
    const selected = await openDialog({
      multiple: false,
      directory: false,
      filters: [{ name: 'PDF documents', extensions: ['pdf'] }],
    });
    if (typeof selected === 'string') {
      setFilePath(selected);
    }
  }

  function buildSourceInput() {
    return {
      examId,
      year,
      paperCode: 'PH',
      sourceType,
      authority,
      institution,
      title,
      sourceUrl: sourceUrl.trim() || null,
      pageCount: pageCount ? Number(pageCount) : null,
      publishedAt: publishedAt || null,
      sessionLabel,
      documentVersion,
      keyStatus: sourceType === 'answer_key' ? keyStatus : '',
      verificationStatus: null,
      verificationNotes: '',
      associatedSourceId: associatedSourceId ? Number(associatedSourceId) : null,
    };
  }

  return (
    <section className="v2-practice-panel" aria-label="Official PYQ sources and extraction">
      <div className="v2-section-heading">
        <div>
          <h2>Official sources & extraction</h2>
          <p>
            Register official GATE Physics papers and final answer keys, import managed PDFs,
            extract page-referenced questions, reconcile keys, and approve only reviewed PYQs.
          </p>
        </div>
      </div>
      {counts && (
        <div className="v2-practice-stats">
          <div>
            <strong>{counts.sources}</strong>
            <span>Sources</span>
          </div>
          <div>
            <strong>{counts.answerKeys}</strong>
            <span>Answer keys</span>
          </div>
          <div>
            <strong>{counts.candidatesPending}</strong>
            <span>Pending review</span>
          </div>
          <div>
            <strong>{counts.candidatesVerified}</strong>
            <span>Verified candidates</span>
          </div>
          <div>
            <strong>{counts.verifiedQuestions}</strong>
            <span>Verified PYQs</span>
          </div>
          <div>
            <strong>{counts.keyConflicts}</strong>
            <span>Key conflicts</span>
          </div>
        </div>
      )}
      {error && (
        <p className="v2-inline-error" role="alert">
          {error}
        </p>
      )}
      {notice && <p className="v2-success-copy">{notice}</p>}

      <div className="v2-source-grid">
        <section className="v2-practice-card">
          <h3>1. Acquire or register a source</h3>
          <div className="v2-practice-form-grid">
            <label className="v2-field">
              <span>Year</span>
              <input
                min={1900}
                onChange={(event) => setYear(Number(event.target.value))}
                type="number"
                value={year}
              />
            </label>
            <label className="v2-field">
              <span>Source type</span>
              <select
                onChange={(event) =>
                  setSourceType(
                    event.target.value as 'question_paper' | 'answer_key' | 'supporting',
                  )
                }
                value={sourceType}
              >
                <option value="question_paper">Question paper</option>
                <option value="answer_key">Answer key</option>
                <option value="supporting">Supporting document</option>
              </select>
            </label>
            <label className="v2-field v2-field-wide">
              <span>Official title</span>
              <input
                onChange={(event) => setTitle(event.target.value)}
                placeholder="GATE 2022 Physics (PH) Master Question Paper"
                value={title}
              />
            </label>
            <label className="v2-field">
              <span>Authority</span>
              <input onChange={(event) => setAuthority(event.target.value)} value={authority} />
            </label>
            <label className="v2-field">
              <span>Institution</span>
              <input
                onChange={(event) => setInstitution(event.target.value)}
                placeholder="IIT Kharagpur"
                value={institution}
              />
            </label>
            <label className="v2-field v2-field-wide">
              <span>Official URL</span>
              <input
                onChange={(event) => setSourceUrl(event.target.value)}
                placeholder="https://gate2026.iitg.ac.in/doc/download/..."
                value={sourceUrl}
              />
            </label>
            <label className="v2-field">
              <span>Session label</span>
              <input
                onChange={(event) => setSessionLabel(event.target.value)}
                placeholder="5"
                value={sessionLabel}
              />
            </label>
            <label className="v2-field">
              <span>PDF pages</span>
              <input
                min={1}
                onChange={(event) => setPageCount(event.target.value)}
                type="number"
                value={pageCount}
              />
            </label>
            <label className="v2-field">
              <span>Document version</span>
              <input
                onChange={(event) => setDocumentVersion(event.target.value)}
                value={documentVersion}
              />
            </label>
            <label className="v2-field">
              <span>Published date</span>
              <input
                onChange={(event) => setPublishedAt(event.target.value)}
                type="date"
                value={publishedAt}
              />
            </label>
            {sourceType === 'answer_key' && (
              <label className="v2-field">
                <span>Key status</span>
                <select onChange={(event) => setKeyStatus(event.target.value)} value={keyStatus}>
                  <option value="final">Final</option>
                  <option value="provisional">Provisional</option>
                  <option value="revised">Revised</option>
                  <option value="withdrawn">Withdrawn</option>
                </select>
              </label>
            )}
            {sourceType === 'answer_key' && (
              <label className="v2-field">
                <span>Associated question paper</span>
                <select
                  onChange={(event) => setAssociatedSourceId(event.target.value)}
                  value={associatedSourceId}
                >
                  <option value="">Not associated</option>
                  {paperSources.map((source) => (
                    <option key={source.id} value={source.id}>
                      {source.year} · {source.title}
                    </option>
                  ))}
                </select>
              </label>
            )}
          </div>
          <div className="v2-source-file-row">
            <input
              onChange={(event) => setFilePath(event.target.value)}
              placeholder="C:\path\to\official.pdf"
              value={filePath}
            />
            <button className="secondary-button" onClick={() => void pickPdf()} type="button">
              Browse PDF
            </button>
          </div>
          <div className="v2-form-actions">
            <button
              className="primary-button"
              disabled={busy || !title.trim() || !filePath.trim()}
              onClick={() =>
                void runAction(async () => {
                  const result = await acquireV2PyqSource({
                    ...buildSourceInput(),
                    filePath,
                    storageMode: 'managed',
                  });
                  return result.duplicate
                    ? `This document is already registered as source ${result.source.id}.`
                    : `Registered source ${result.source.id} with SHA-256 ${shortHash(result.source.sha256)}.`;
                })
              }
              type="button"
            >
              {busy ? 'Working…' : 'Acquire PDF and register'}
            </button>
            <button
              className="secondary-button"
              disabled={busy || !title.trim()}
              onClick={() =>
                void runAction(async () => {
                  const source = await saveV2PyqSource(buildSourceInput());
                  return `Registered source ${source.id} without a managed file.`;
                })
              }
              type="button"
            >
              Register metadata only
            </button>
          </div>
        </section>

        <section className="v2-practice-card">
          <h3>2. Import a final answer key</h3>
          <div className="v2-practice-form-grid">
            <label className="v2-field">
              <span>Answer-key source</span>
              <select onChange={(event) => setKeySourceId(event.target.value)} value={keySourceId}>
                <option value="">Choose a source</option>
                {keySources.map((source) => (
                  <option key={source.id} value={source.id}>
                    {source.year} · {source.title} · {statusLabel(source.verificationStatus)}
                  </option>
                ))}
              </select>
            </label>
            <label className="v2-field">
              <span>Version label</span>
              <input
                onChange={(event) => setKeyVersionLabel(event.target.value)}
                value={keyVersionLabel}
              />
            </label>
            <label className="v2-field">
              <span>Published date</span>
              <input
                onChange={(event) => setKeyPublishedAt(event.target.value)}
                type="date"
                value={keyPublishedAt}
              />
            </label>
            <label className="v2-field v2-field-check">
              <span>Verified final key</span>
              <input
                checked={keyVerified}
                onChange={(event) => setKeyVerified(event.target.checked)}
                type="checkbox"
              />
            </label>
            <label className="v2-field">
              <span>Verified by</span>
              <input
                onChange={(event) => setKeyVerifiedBy(event.target.value)}
                value={keyVerifiedBy}
              />
            </label>
          </div>
          <label className="v2-field">
            <span>Answer-key text (row or cell-per-line layout)</span>
            <textarea
              onChange={(event) => setKeyText(event.target.value)}
              placeholder={'Q. No. Session Question Type Section Key/Range Mark\n1 5 MCQ GA D 1'}
              rows={7}
              value={keyText}
            />
          </label>
          <label className="v2-field">
            <span>Review notes</span>
            <input onChange={(event) => setKeyNotes(event.target.value)} value={keyNotes} />
          </label>
          <div className="v2-form-actions">
            <button
              className="secondary-button"
              disabled={busy || !keyText.trim()}
              onClick={() =>
                void runAction(async () => {
                  const preview = await previewV2PyqAnswerKey(keyText);
                  setKeyPreview(preview);
                  return `Parsed ${preview.parsedCount} key entries; ${preview.missingQuestionNumbers.length} question numbers are missing.`;
                })
              }
              type="button"
            >
              Preview parsed key
            </button>
            <button
              className="primary-button"
              disabled={busy || !keySourceId || !keyText.trim() || !keyVersionLabel.trim()}
              onClick={() =>
                void runAction(async () => {
                  const key = await saveV2PyqAnswerKey({
                    sourceId: Number(keySourceId),
                    versionLabel: keyVersionLabel,
                    keyStatus: 'final',
                    publishedAt: keyPublishedAt || null,
                    verified: keyVerified,
                    verifiedBy: keyVerifiedBy,
                    notes: keyNotes,
                    text: keyText,
                  });
                  return `Saved answer key ${key.id} with ${key.entryCount} entries.`;
                })
              }
              type="button"
            >
              Save answer key
            </button>
          </div>
          {keyPreview && (
            <p className="v2-empty-copy">
              Parsed {keyPreview.parsedCount} entries. Missing:{' '}
              {keyPreview.missingQuestionNumbers.length === 0
                ? 'none'
                : keyPreview.missingQuestionNumbers.join(', ')}
            </p>
          )}
        </section>

        <section className="v2-practice-card">
          <h3>3. Extract page-referenced questions</h3>
          <div className="v2-practice-form-grid">
            <label className="v2-field">
              <span>Question-paper source</span>
              <select
                onChange={(event) => setPaperSourceId(event.target.value)}
                value={paperSourceId}
              >
                <option value="">Choose a source</option>
                {paperSources.map((source) => (
                  <option key={source.id} value={source.id}>
                    {source.year} · {source.title}
                  </option>
                ))}
              </select>
            </label>
            <label className="v2-field">
              <span>Answer key</span>
              <select
                onChange={(event) => setExtractionKeyId(event.target.value)}
                value={extractionKeyId}
              >
                <option value="">Auto-associate final key</option>
                {answerKeys.map((key) => (
                  <option key={key.id} value={key.id}>
                    {key.year} · {key.versionLabel} · {key.keyStatus}
                  </option>
                ))}
              </select>
            </label>
            <label className="v2-field">
              <span>Extraction method</span>
              <select
                onChange={(event) => setExtractionMethod(event.target.value)}
                value={extractionMethod}
              >
                <option value="pdf_text">PDF text layer</option>
                <option value="ocr">OCR-assisted</option>
                <option value="manual">Manual transcription</option>
                <option value="assisted">Assisted import</option>
              </select>
            </label>
          </div>
          <label className="v2-field">
            <span>Page-delimited extracted text</span>
            <textarea
              onChange={(event) => setExtractionText(event.target.value)}
              placeholder={'===== PAGE 1 =====\nQ.1 ...\n(A) ...\n(B) ...'}
              rows={8}
              value={extractionText}
            />
          </label>
          <label className="v2-field">
            <span>Extraction notes</span>
            <input
              onChange={(event) => setExtractionNotes(event.target.value)}
              value={extractionNotes}
            />
          </label>
          <div className="v2-form-actions">
            <button
              className="primary-button"
              disabled={busy || !paperSourceId || !extractionText.trim()}
              onClick={() =>
                void runAction(async () => {
                  const result = await extractV2PyqSource({
                    sourceId: Number(paperSourceId),
                    text: extractionText,
                    method: extractionMethod,
                    answerKeyId: extractionKeyId ? Number(extractionKeyId) : null,
                    notes: extractionNotes,
                  });
                  return `Extracted ${result.extracted} questions: ${result.clean} ready for review, ${result.needsReview} flagged, ${result.missingKey} without a key.`;
                })
              }
              type="button"
            >
              Run extraction
            </button>
            <button
              className="secondary-button"
              disabled={busy || !paperSourceId || !extractionKeyId}
              onClick={() =>
                void runAction(async () => {
                  const count = await reconcileV2PyqSource(
                    Number(paperSourceId),
                    Number(extractionKeyId),
                  );
                  return `Reconciled ${count} pending candidates with the selected key.`;
                })
              }
              type="button"
            >
              Reconcile pending candidates
            </button>
          </div>
        </section>

        <section className="v2-practice-card">
          <h3>4. Verify source documents</h3>
          {sources.length === 0 ? (
            <p className="v2-empty-copy">No official sources registered yet.</p>
          ) : (
            <ul className="v2-source-list">
              {sources.map((source) => (
                <li key={source.id}>
                  <div className="v2-source-head">
                    <strong>
                      {source.year} · {sourceTypeLabel(source.sourceType)}
                    </strong>
                    <span
                      className={`v2-source-status v2-source-status-${source.verificationStatus}`}
                    >
                      {statusLabel(source.verificationStatus)}
                    </span>
                  </div>
                  <p>{source.title}</p>
                  <p className="v2-empty-copy">
                    {source.authority}
                    {source.institution ? ` · ${source.institution}` : ''} ·{' '}
                    {source.pageCount ?? '?'} pages · {shortHash(source.sha256)}
                  </p>
                  <p className="v2-empty-copy">
                    {source.resourceFileName ?? 'No managed PDF linked'}
                  </p>
                  <div className="v2-form-actions">
                    {source.sourceUrl && (
                      <button
                        className="secondary-button"
                        onClick={() => void openUrl(source.sourceUrl!)}
                        type="button"
                      >
                        Open official URL
                      </button>
                    )}
                    {source.resourceFilePath && (
                      <button
                        className="secondary-button"
                        onClick={() => void openPath(source.resourceFilePath!)}
                        type="button"
                      >
                        Open managed PDF
                      </button>
                    )}
                    {source.verificationStatus !== 'verified' && (
                      <button
                        className="secondary-button"
                        disabled={busy || !source.sha256}
                        onClick={() =>
                          void runAction(async () => {
                            await setV2PyqSourceStatus({
                              sourceId: source.id,
                              verificationStatus: 'verified',
                              verificationNotes:
                                'Official document identity, managed file hash, and page count checked by local reviewer.',
                            });
                            return `Source ${source.id} marked verified.`;
                          })
                        }
                        type="button"
                      >
                        Mark source verified
                      </button>
                    )}
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>

      <section className="v2-practice-card">
        <div className="v2-section-heading">
          <div>
            <h3>5. Review extracted questions</h3>
            <p className="v2-empty-copy">
              Approve only after the paper source and answer key are verified. Approval creates a
              verified PYQ and makes it available to question-level mocks when the pattern and
              choices are valid.
            </p>
          </div>
          <div className="v2-source-filters">
            <label className="v2-field">
              <span>Year</span>
              <input
                onChange={(event) => setFilterYear(event.target.value)}
                placeholder="All"
                value={filterYear}
              />
            </label>
            <label className="v2-field">
              <span>Status</span>
              <select
                onChange={(event) => setFilterStatus(event.target.value)}
                value={filterStatus}
              >
                <option value="">All</option>
                <option value="extracted">Extracted</option>
                <option value="needs_review">Needs review</option>
                <option value="verified">Verified</option>
                <option value="rejected">Rejected</option>
              </select>
            </label>
          </div>
        </div>
        {loading ? (
          <p role="status">Loading official sources…</p>
        ) : candidates.length === 0 ? (
          <div className="v2-empty-panel">
            <h3>No extraction candidates in this filter</h3>
            <p>Run extraction on a registered question paper to populate the review queue.</p>
          </div>
        ) : (
          <div className="v2-source-review-layout">
            <ul className="v2-source-candidate-list">
              {candidates.map((candidate) => (
                <li key={candidate.id}>
                  <button
                    className={`v2-source-candidate${selectedCandidateId === candidate.id ? ' v2-source-candidate-active' : ''}`}
                    onClick={() => chooseCandidate(candidate)}
                    type="button"
                  >
                    <span>
                      <strong>
                        {candidate.year} · Q{candidate.questionNumber} ·{' '}
                        {candidate.questionTypeCode}
                      </strong>
                      <span>
                        {candidate.sectionCode} · {candidate.maxMarks ?? '?'} marks · p.{' '}
                        {candidate.pageNumber ?? '?'}
                      </span>
                      <span>{candidate.reviewStatus.replaceAll('_', ' ')}</span>
                    </span>
                    <span>{candidate.statement.slice(0, 140)}</span>
                  </button>
                </li>
              ))}
            </ul>
            <div className="v2-source-candidate-detail">
              {selectedCandidate ? (
                <>
                  <div className="v2-section-heading">
                    <div>
                      <h4>
                        {selectedCandidate.year} · Q{selectedCandidate.questionNumber} ·{' '}
                        {selectedCandidate.sourceTitle}
                      </h4>
                      <p className="v2-empty-copy">
                        Page {selectedCandidate.pageNumber ?? '?'} ·{' '}
                        {selectedCandidate.answerKeyVersion ?? 'No key version'} ·{' '}
                        {selectedCandidate.reviewNotes || 'No review notes'}
                      </p>
                    </div>
                    <span className="v2-source-status">
                      {selectedCandidate.reviewStatus.replaceAll('_', ' ')}
                    </span>
                  </div>
                  <label className="v2-field">
                    <span>Question statement</span>
                    <textarea
                      onChange={(event) => setEditStatement(event.target.value)}
                      rows={6}
                      value={editStatement}
                    />
                  </label>
                  <label className="v2-field">
                    <span>Answer choices (one per line; optional A) prefix)</span>
                    <textarea
                      onChange={(event) => setEditOptions(event.target.value)}
                      rows={6}
                      value={editOptions}
                    />
                  </label>
                  <div className="v2-practice-form-grid">
                    <label className="v2-field">
                      <span>Correct answer</span>
                      <input
                        onChange={(event) => setEditCorrectAnswer(event.target.value)}
                        value={editCorrectAnswer}
                      />
                    </label>
                    <label className="v2-field">
                      <span>Correct range</span>
                      <input
                        onChange={(event) => setEditCorrectRange(event.target.value)}
                        value={editCorrectRange}
                      />
                    </label>
                    <label className="v2-field">
                      <span>Max marks</span>
                      <input
                        min={0.1}
                        onChange={(event) => setEditMaxMarks(event.target.value)}
                        step="0.1"
                        type="number"
                        value={editMaxMarks}
                      />
                    </label>
                    <label className="v2-field">
                      <span>Negative marks</span>
                      <input
                        onChange={(event) => setEditNegativeMarks(event.target.value)}
                        step="0.1"
                        type="number"
                        value={editNegativeMarks}
                      />
                    </label>
                    <label className="v2-field">
                      <span>Numeric tolerance</span>
                      <input
                        min={0}
                        onChange={(event) => setEditTolerance(event.target.value)}
                        step="0.001"
                        type="number"
                        value={editTolerance}
                      />
                    </label>
                    <label className="v2-field">
                      <span>Difficulty</span>
                      <input
                        max={5}
                        min={1}
                        onChange={(event) => setEditDifficulty(event.target.value)}
                        type="number"
                        value={editDifficulty}
                      />
                    </label>
                    <label className="v2-field">
                      <span>Section</span>
                      <select
                        onChange={(event) => setEditSection(event.target.value)}
                        value={editSection}
                      >
                        {sections.map((section) => (
                          <option key={section.id} value={section.code}>
                            {section.title}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label className="v2-field">
                      <span>Question type</span>
                      <select
                        onChange={(event) => setEditType(event.target.value)}
                        value={editType}
                      >
                        <option value="MCQ">MCQ</option>
                        <option value="MSQ">MSQ</option>
                        <option value="NAT">NAT</option>
                      </select>
                    </label>
                    <label className="v2-field">
                      <span>Syllabus topic</span>
                      <select
                        onChange={(event) => setEditTopic(event.target.value)}
                        value={editTopic}
                      >
                        <option value="">Unmapped</option>
                        {topics.map((topic) => (
                          <option key={topic.id} value={topic.id}>
                            {topic.category} · {topic.title}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label className="v2-field">
                      <span>Shared concepts</span>
                      <select
                        multiple
                        onChange={(event) =>
                          setEditConcepts(
                            Array.from(event.target.selectedOptions).map((option) =>
                              Number(option.value),
                            ),
                          )
                        }
                        size={5}
                        value={editConcepts.map(String)}
                      >
                        {options.concepts.map((concept) => (
                          <option key={concept.id} value={concept.id}>
                            {concept.title}
                          </option>
                        ))}
                      </select>
                    </label>
                  </div>
                  <label className="v2-field">
                    <span>Review notes</span>
                    <input
                      onChange={(event) => setEditNotes(event.target.value)}
                      value={editNotes}
                    />
                  </label>
                  <div className="v2-form-actions">
                    <button
                      className="secondary-button"
                      disabled={busy}
                      onClick={() =>
                        void runAction(async () => {
                          await updateV2PyqExtractionCandidate({
                            candidateId: selectedCandidate.id,
                            statement: editStatement,
                            options: parseOptions(editOptions),
                            correctAnswer: editCorrectAnswer,
                            correctRange: editCorrectRange,
                            maxMarks: Number(editMaxMarks),
                            negativeMarks: Number(editNegativeMarks),
                            numericTolerance: Number(editTolerance),
                            difficulty: Number(editDifficulty),
                            sectionCode: editSection,
                            questionTypeCode: editType,
                            syllabusNodeId: editTopic ? Number(editTopic) : null,
                            conceptIds: editConcepts,
                            reviewNotes: editNotes,
                            reviewedBy: 'local reviewer',
                          });
                          return 'Review edits saved. The candidate remains pending approval.';
                        })
                      }
                      type="button"
                    >
                      Save review edits
                    </button>
                    <button
                      className="primary-button"
                      disabled={busy}
                      onClick={() =>
                        void runAction(async () => {
                          await verifyV2PyqExtractionCandidate({
                            candidateId: selectedCandidate.id,
                            reviewedBy: 'local reviewer',
                            reviewNotes:
                              editNotes ||
                              'Approved after checking the official page, key and question type.',
                          });
                          return 'Candidate approved and verified PYQ created.';
                        })
                      }
                      type="button"
                    >
                      Approve and create PYQ
                    </button>
                    <button
                      className="secondary-button"
                      disabled={busy}
                      onClick={() =>
                        void runAction(async () => {
                          await rejectV2PyqExtractionCandidate({
                            candidateId: selectedCandidate.id,
                            reviewedBy: 'local reviewer',
                            reviewNotes: editNotes || 'Rejected during source review.',
                          });
                          return 'Candidate rejected.';
                        })
                      }
                      type="button"
                    >
                      Reject
                    </button>
                  </div>
                </>
              ) : (
                <p className="v2-empty-copy">Select a candidate to review it.</p>
              )}
            </div>
          </div>
        )}
      </section>

      {conflicts.length > 0 && (
        <section className="v2-practice-card">
          <h3>6. Revised key conflicts</h3>
          <ul className="v2-source-list">
            {conflicts.map((conflict) => (
              <li key={conflict.id}>
                <strong>Question {conflict.questionId}</strong>
                <p>
                  {conflict.oldKeyValue} → {conflict.newKeyValue} · {conflict.status}
                </p>
                <p className="v2-empty-copy">{conflict.notes}</p>
              </li>
            ))}
          </ul>
        </section>
      )}
    </section>
  );
}
