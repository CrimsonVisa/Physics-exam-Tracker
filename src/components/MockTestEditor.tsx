import { type FormEvent, useState } from 'react';
import { DateField } from './DateField';
import { toCommandError } from '../lib/commandError';
import { parseDateOnly } from '../lib/dateOnly';
import {
  createMockTest,
  updateMockTest,
  type MockPartInput,
  type MockPartResult,
  type MockTest,
  type MockTestInput,
} from '../lib/mockTestApi';

type PartKey = 'partA' | 'partB' | 'partC';

type PartForm = {
  questions: string;
  correct: string;
  incorrect: string;
  unanswered: string;
  correctMarks: string;
  incorrectMarks: string;
  unansweredMarks: string;
};

type MockForm = {
  testName: string;
  testDate: string;
  durationMinutes: string;
  maxMarks: string;
  marksObtained: string;
  partA: PartForm;
  partB: PartForm;
  partC: PartForm;
  notes: string;
  reviewObservations: string;
};

type MockFormField =
  | 'testName'
  | 'testDate'
  | 'durationMinutes'
  | 'maxMarks'
  | 'marksObtained'
  | 'partA'
  | 'partB'
  | 'partC'
  | 'notes'
  | 'reviewObservations';

type MockTestEditorProps = {
  test: MockTest | null;
  today: string;
  onSaved: (test: MockTest, wasEditing: boolean) => void;
  onCancel: () => void;
};

const MAX_NOTES_CHARS = 2_000;
const PART_KEYS: PartKey[] = ['partA', 'partB', 'partC'];
const PART_LABELS: Record<PartKey, string> = { partA: 'A', partB: 'B', partC: 'C' };

function blankPart(): PartForm {
  return {
    questions: '0',
    correct: '0',
    incorrect: '0',
    unanswered: '0',
    correctMarks: '',
    incorrectMarks: '',
    unansweredMarks: '',
  };
}

function partFromResult(result: MockPartResult): PartForm {
  return {
    questions: String(result.questions),
    correct: String(result.correct),
    incorrect: String(result.incorrect),
    unanswered: String(result.unanswered),
    correctMarks: String(result.correctMarks),
    incorrectMarks: String(result.incorrectMarks),
    unansweredMarks: String(result.unansweredMarks),
  };
}

function formFromTest(test: MockTest | null, today: string): MockForm {
  if (!test) {
    return {
      testName: '',
      testDate: today,
      durationMinutes: '',
      maxMarks: '',
      marksObtained: '',
      partA: blankPart(),
      partB: blankPart(),
      partC: blankPart(),
      notes: '',
      reviewObservations: '',
    };
  }

  return {
    testName: test.testName,
    testDate: test.testDate,
    durationMinutes: String(test.durationMinutes),
    maxMarks: String(test.maxMarks),
    marksObtained: test.marksObtained === null ? '' : String(test.marksObtained),
    partA: partFromResult(test.partA),
    partB: partFromResult(test.partB),
    partC: partFromResult(test.partC),
    notes: test.notes,
    reviewObservations: test.reviewObservations,
  };
}

function parseWholeNumber(value: string): number | null {
  const trimmed = value.trim();
  if (!/^\d+$/.test(trimmed)) {
    return null;
  }
  return Number(trimmed);
}

function parseOptionalNumber(value: string): number | null | 'invalid' {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : 'invalid';
}

const MARKS_TOLERANCE_FACTOR = 1e-9;
const PERCENTAGE_TOLERANCE = 1e-9;

function marksTolerance(value: number): number {
  return Math.max(1, Math.abs(value)) * MARKS_TOLERANCE_FACTOR;
}

function partMaximum(input: MockPartInput): number {
  return input.questions * (input.correctMarks ?? 0);
}

function partScore(input: MockPartInput): number {
  return (
    input.correct * (input.correctMarks ?? 0) +
    input.incorrect * (input.incorrectMarks ?? 0) +
    input.unanswered * (input.unansweredMarks ?? 0)
  );
}

function savedConfigurationIssues(test: MockTest): string[] {
  const issues: string[] = [];
  const parts = [
    ['Part A', test.partA],
    ['Part B', test.partB],
    ['Part C', test.partC],
  ] as const;
  const highestPossible = parts.reduce(
    (total, [, part]) => total + part.questions * part.correctMarks,
    0,
  );

  if (test.maxMarks + marksTolerance(highestPossible) < highestPossible) {
    issues.push(
      `Maximum marks (${formatScore(test.maxMarks)}) do not cover the highest possible score (${formatScore(highestPossible)}).`,
    );
  }
  if (test.computedPercentage > 100 + PERCENTAGE_TOLERANCE) {
    issues.push('The calculated percentage is above 100%.');
  }
  for (const [label, part] of parts) {
    if (part.incorrectMarks > 0) {
      issues.push(`${label} has positive incorrect-answer marks.`);
    }
    if (part.unansweredMarks > 0) {
      issues.push(`${label} has positive unanswered marks.`);
    }
    if (part.correct + part.incorrect + part.unanswered !== part.questions) {
      issues.push(`${label} counts do not add up to its question total.`);
    }
  }

  return issues.length === 0
    ? ['This saved marking configuration would be rejected by the current rules.']
    : issues;
}

function validatePart(part: PartForm): {
  input: MockPartInput | null;
  error: string | null;
} {
  const questions = parseWholeNumber(part.questions);
  const correct = parseWholeNumber(part.correct);
  const incorrect = parseWholeNumber(part.incorrect);
  const unanswered = parseWholeNumber(part.unanswered);

  if (
    questions === null ||
    correct === null ||
    incorrect === null ||
    unanswered === null ||
    questions > 10_000 ||
    correct > 10_000 ||
    incorrect > 10_000 ||
    unanswered > 10_000
  ) {
    return {
      input: null,
      error: 'Use whole question counts from 0 to 10000.',
    };
  }

  if (correct + incorrect + unanswered !== questions) {
    return {
      input: null,
      error: 'Correct + incorrect + unanswered must equal the question count.',
    };
  }

  const correctMarks = parseOptionalNumber(part.correctMarks);
  if (questions > 0 && correctMarks === null) {
    return { input: null, error: 'Enter the marks awarded for a correct answer.' };
  }
  if (
    correctMarks === 'invalid' ||
    (correctMarks !== null && (correctMarks < 0 || correctMarks > 1_000))
  ) {
    return { input: null, error: 'Correct-answer marks must be between 0 and 1000.' };
  }

  const incorrectMarks = parseOptionalNumber(part.incorrectMarks);
  if (
    incorrectMarks === 'invalid' ||
    (incorrectMarks !== null && (incorrectMarks < -1_000 || incorrectMarks > 0))
  ) {
    return {
      input: null,
      error: 'Incorrect-answer marks must be zero or negative. Use 0 for no penalty.',
    };
  }

  const unansweredMarks = parseOptionalNumber(part.unansweredMarks);
  if (
    unansweredMarks === 'invalid' ||
    (unansweredMarks !== null && (unansweredMarks < -1_000 || unansweredMarks > 0))
  ) {
    return {
      input: null,
      error: 'Unanswered marks must be zero or negative. Use 0 for no penalty.',
    };
  }

  return {
    error: null,
    input: {
      questions,
      correct,
      incorrect,
      unanswered,
      correctMarks: typeof correctMarks === 'number' ? correctMarks : null,
      incorrectMarks: typeof incorrectMarks === 'number' ? incorrectMarks : null,
      unansweredMarks: typeof unansweredMarks === 'number' ? unansweredMarks : null,
    },
  };
}

function validateForm(form: MockForm): {
  input: MockTestInput | null;
  errors: Partial<Record<MockFormField, string>>;
} {
  const errors: Partial<Record<MockFormField, string>> = {};
  const name = form.testName.trim();
  if (!name) {
    errors.testName = 'Enter a test name.';
  } else if (name.length > 200) {
    errors.testName = 'Test name must be 200 characters or fewer.';
  }

  if (!parseDateOnly(form.testDate)) {
    errors.testDate = 'Choose a real calendar date.';
  }

  const duration = parseWholeNumber(form.durationMinutes);
  if (duration === null || duration < 1 || duration > 1_440) {
    errors.durationMinutes = 'Enter a duration from 1 to 1440 minutes.';
  }

  const maxMarks = parseOptionalNumber(form.maxMarks);
  if (maxMarks === null || maxMarks === 'invalid' || maxMarks <= 0 || maxMarks > 100_000) {
    errors.maxMarks = 'Enter maximum marks greater than 0 and at most 100000.';
  }

  const marksObtained = parseOptionalNumber(form.marksObtained);
  if (
    marksObtained === 'invalid' ||
    (marksObtained !== null && (marksObtained < -100_000 || marksObtained > 100_000))
  ) {
    errors.marksObtained = 'Enter marks between -100000 and 100000, or leave blank.';
  } else if (
    typeof marksObtained === 'number' &&
    typeof maxMarks === 'number' &&
    marksObtained > maxMarks
  ) {
    errors.marksObtained = 'Reported marks cannot exceed maximum marks.';
  }

  if (form.notes.length > MAX_NOTES_CHARS) {
    errors.notes = `Notes must be ${MAX_NOTES_CHARS} characters or fewer.`;
  }
  if (form.reviewObservations.length > MAX_NOTES_CHARS) {
    errors.reviewObservations = `Review observations must be ${MAX_NOTES_CHARS} characters or fewer.`;
  }

  const partResults = {
    partA: validatePart(form.partA),
    partB: validatePart(form.partB),
    partC: validatePart(form.partC),
  };

  for (const key of PART_KEYS) {
    if (partResults[key].error) {
      errors[key] = partResults[key].error ?? undefined;
    }
  }

  if (
    !errors.partA &&
    !errors.partB &&
    !errors.partC &&
    (partResults.partA.input?.questions ?? 0) +
      (partResults.partB.input?.questions ?? 0) +
      (partResults.partC.input?.questions ?? 0) ===
      0
  ) {
    errors.partA = 'A mock test must contain at least one question across Parts A, B and C.';
  }

  if (!errors.partA && !errors.partB && !errors.partC && typeof maxMarks === 'number') {
    const highestPossible = PART_KEYS.reduce(
      (total, key) => total + partMaximum(partResults[key].input!),
      0,
    );
    if (maxMarks + marksTolerance(highestPossible) < highestPossible) {
      errors.maxMarks = `Maximum marks must be at least the highest possible score from this marking configuration (${formatScore(highestPossible)}).`;
    } else {
      const computedMarks = PART_KEYS.reduce(
        (total, key) => total + partScore(partResults[key].input!),
        0,
      );
      if (
        computedMarks > maxMarks + marksTolerance(maxMarks) ||
        (maxMarks > 0 && (computedMarks / maxMarks) * 100 > 100 + PERCENTAGE_TOLERANCE)
      ) {
        errors.maxMarks =
          'The calculated score cannot exceed maximum marks. Review maximum marks or the marking configuration.';
      }
    }
  }

  if (Object.keys(errors).length > 0) {
    return { input: null, errors };
  }

  return {
    errors,
    input: {
      testName: name,
      testDate: form.testDate,
      durationMinutes: duration!,
      maxMarks: maxMarks as number,
      marksObtained: typeof marksObtained === 'number' ? marksObtained : null,
      partA: partResults.partA.input!,
      partB: partResults.partB.input!,
      partC: partResults.partC.input!,
      notes: form.notes,
      reviewObservations: form.reviewObservations,
    },
  };
}

function partCalculation(part: PartForm): {
  questions: number;
  attempted: number;
  correct: number;
  incorrect: number;
  unanswered: number;
  marks: number;
  maximumMarks: number;
} {
  const questions = parseWholeNumber(part.questions) ?? 0;
  const correct = parseWholeNumber(part.correct) ?? 0;
  const incorrect = parseWholeNumber(part.incorrect) ?? 0;
  const unanswered = parseWholeNumber(part.unanswered) ?? 0;
  const correctMarks = parseOptionalNumber(part.correctMarks);
  const incorrectMarks = parseOptionalNumber(part.incorrectMarks);
  const unansweredMarks = parseOptionalNumber(part.unansweredMarks);
  const marks =
    correct * (typeof correctMarks === 'number' ? correctMarks : 0) +
    incorrect * (typeof incorrectMarks === 'number' ? incorrectMarks : 0) +
    unanswered * (typeof unansweredMarks === 'number' ? unansweredMarks : 0);

  return {
    questions,
    attempted: correct + incorrect,
    correct,
    incorrect,
    unanswered,
    marks,
    maximumMarks: questions * (typeof correctMarks === 'number' ? correctMarks : 0),
  };
}

function formatScore(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(2);
}

export function MockTestEditor({ test, today, onSaved, onCancel }: MockTestEditorProps) {
  const [form, setForm] = useState<MockForm>(() => formFromTest(test, today));
  const [errors, setErrors] = useState<Partial<Record<MockFormField, string>>>({});
  const [isSaving, setIsSaving] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);

  function clearError(field: MockFormField) {
    setErrors((current) => {
      if (!current[field]) {
        return current;
      }
      const next = { ...current };
      delete next[field];
      return next;
    });
    setFeedback(null);
  }

  function handleField(field: keyof MockForm, value: string) {
    setForm((current) => ({ ...current, [field]: value }));
    clearError(field as MockFormField);
  }

  function handlePart(key: PartKey, field: keyof PartForm, value: string) {
    setForm((current) => ({
      ...current,
      [key]: { ...current[key], [field]: value },
    }));
    clearError(key);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (isSaving) {
      return;
    }

    setFeedback(null);
    const validation = validateForm(form);
    setErrors(validation.errors);

    if (!validation.input) {
      return;
    }

    setIsSaving(true);

    try {
      const saved =
        test === null
          ? await createMockTest(validation.input)
          : await updateMockTest(test.id, validation.input);
      onSaved(saved, test !== null);
    } catch (error: unknown) {
      const commandError = toCommandError(error);
      const field = commandError.field as MockFormField | undefined;

      if (
        field &&
        [
          'testName',
          'testDate',
          'durationMinutes',
          'maxMarks',
          'marksObtained',
          'partA',
          'partB',
          'partC',
          'notes',
          'reviewObservations',
        ].includes(field)
      ) {
        setErrors({ [field]: commandError.message });
      }
      setFeedback(commandError.message);
    } finally {
      setIsSaving(false);
    }
  }

  const calculations = PART_KEYS.map((key) => ({
    key,
    ...partCalculation(form[key]),
  }));
  const totalMarks = calculations.reduce((total, part) => total + part.marks, 0);
  const totalQuestions = calculations.reduce((total, part) => total + part.questions, 0);
  const highestPossibleMarks = calculations.reduce((total, part) => total + part.maximumMarks, 0);
  const maxMarksValue = parseOptionalNumber(form.maxMarks);
  const maxMarksInsufficient =
    typeof maxMarksValue === 'number' &&
    maxMarksValue + marksTolerance(highestPossibleMarks) < highestPossibleMarks;
  const computedPercentage =
    typeof maxMarksValue === 'number' && maxMarksValue > 0
      ? (totalMarks / maxMarksValue) * 100
      : null;
  const impossibleScore =
    computedPercentage !== null &&
    (!Number.isFinite(computedPercentage) || computedPercentage > 100 + PERCENTAGE_TOLERANCE);

  return (
    <form className="planner-form mock-form" onSubmit={handleSubmit} noValidate>
      {test && !test.configurationValid && (
        <div className="save-feedback save-feedback-warning" role="status">
          <p>Saved marking configuration needs review before this record can be saved again.</p>
          <ul>
            {savedConfigurationIssues(test).map((issue) => (
              <li key={issue}>{issue}</li>
            ))}
          </ul>
        </div>
      )}

      <div className="planner-form-row">
        <div className={`form-field${errors.testName ? ' form-field-invalid' : ''}`}>
          <label htmlFor="mock-name">Test name</label>
          <input
            id="mock-name"
            value={form.testName}
            maxLength={200}
            onChange={(event) => handleField('testName', event.target.value)}
            aria-invalid={errors.testName ? true : undefined}
            disabled={isSaving}
            required
          />
          {errors.testName && (
            <p className="field-error" role="alert">
              {errors.testName}
            </p>
          )}
        </div>

        <DateField
          id="mock-date"
          label="Test date"
          value={form.testDate}
          onChange={(value) => handleField('testDate', value)}
          error={errors.testDate}
          disabled={isSaving}
        />
      </div>

      <div className="planner-form-row">
        <div className={`form-field${errors.durationMinutes ? ' form-field-invalid' : ''}`}>
          <label htmlFor="mock-duration">Duration (minutes)</label>
          <input
            id="mock-duration"
            type="number"
            min={1}
            max={1440}
            step={1}
            value={form.durationMinutes}
            onChange={(event) => handleField('durationMinutes', event.target.value)}
            aria-invalid={errors.durationMinutes ? true : undefined}
            disabled={isSaving}
            required
          />
          {errors.durationMinutes && (
            <p className="field-error" role="alert">
              {errors.durationMinutes}
            </p>
          )}
        </div>

        <div
          className={`form-field${errors.maxMarks || maxMarksInsufficient ? ' form-field-invalid' : ''}`}
        >
          <label htmlFor="mock-max-marks">Maximum marks</label>
          <input
            id="mock-max-marks"
            type="number"
            min={0}
            step="0.01"
            value={form.maxMarks}
            onChange={(event) => handleField('maxMarks', event.target.value)}
            aria-invalid={errors.maxMarks || maxMarksInsufficient ? true : undefined}
            aria-describedby="mock-max-marks-hint"
            disabled={isSaving}
            required
          />
          <p className="field-hint" id="mock-max-marks-hint">
            Highest possible score from this marking configuration:{' '}
            {formatScore(highestPossibleMarks)}.
          </p>
          {(errors.maxMarks || maxMarksInsufficient) && (
            <p className="field-error" role="alert">
              {errors.maxMarks ??
                `Maximum marks must be at least ${formatScore(highestPossibleMarks)}.`}
            </p>
          )}
        </div>
      </div>

      <div className={`form-field${errors.marksObtained ? ' form-field-invalid' : ''}`}>
        <label htmlFor="mock-reported-marks">Reported score (optional)</label>
        <input
          id="mock-reported-marks"
          type="number"
          step="0.01"
          value={form.marksObtained}
          onChange={(event) => handleField('marksObtained', event.target.value)}
          aria-invalid={errors.marksObtained ? true : undefined}
          disabled={isSaving}
        />
        <p className="field-hint">
          Analytics use the score calculated from your saved part counts and marking configuration.
          Keep the reported score for comparison if you have an official result.
        </p>
        {errors.marksObtained && (
          <p className="field-error" role="alert">
            {errors.marksObtained}
          </p>
        )}
      </div>

      <div className="mock-part-grid">
        {PART_KEYS.map((key) => (
          <fieldset
            className={`mock-part-card${errors[key] ? ' form-field-invalid' : ''}`}
            key={key}
          >
            <legend>Part {PART_LABELS[key]}</legend>
            <div className="mock-part-counts">
              <div className="form-field">
                <label htmlFor={`mock-${key}-questions`}>Questions</label>
                <input
                  id={`mock-${key}-questions`}
                  type="number"
                  min={0}
                  max={10000}
                  step={1}
                  value={form[key].questions}
                  onChange={(event) => handlePart(key, 'questions', event.target.value)}
                  disabled={isSaving}
                />
              </div>
              <div className="form-field">
                <label htmlFor={`mock-${key}-correct`}>Correct</label>
                <input
                  id={`mock-${key}-correct`}
                  type="number"
                  min={0}
                  max={10000}
                  step={1}
                  value={form[key].correct}
                  onChange={(event) => handlePart(key, 'correct', event.target.value)}
                  disabled={isSaving}
                />
              </div>
              <div className="form-field">
                <label htmlFor={`mock-${key}-incorrect`}>Incorrect</label>
                <input
                  id={`mock-${key}-incorrect`}
                  type="number"
                  min={0}
                  max={10000}
                  step={1}
                  value={form[key].incorrect}
                  onChange={(event) => handlePart(key, 'incorrect', event.target.value)}
                  disabled={isSaving}
                />
              </div>
              <div className="form-field">
                <label htmlFor={`mock-${key}-unanswered`}>Unanswered</label>
                <input
                  id={`mock-${key}-unanswered`}
                  type="number"
                  min={0}
                  max={10000}
                  step={1}
                  value={form[key].unanswered}
                  onChange={(event) => handlePart(key, 'unanswered', event.target.value)}
                  disabled={isSaving}
                />
              </div>
            </div>

            <div className="mock-marking-grid">
              <div className="form-field">
                <label htmlFor={`mock-${key}-correct-marks`}>Correct marks</label>
                <input
                  id={`mock-${key}-correct-marks`}
                  type="number"
                  min={0}
                  step="0.01"
                  value={form[key].correctMarks}
                  onChange={(event) => handlePart(key, 'correctMarks', event.target.value)}
                  disabled={isSaving}
                />
              </div>
              <div className="form-field">
                <label htmlFor={`mock-${key}-incorrect-marks`}>Incorrect marks</label>
                <input
                  id={`mock-${key}-incorrect-marks`}
                  type="number"
                  max={0}
                  step="0.01"
                  value={form[key].incorrectMarks}
                  onChange={(event) => handlePart(key, 'incorrectMarks', event.target.value)}
                  disabled={isSaving}
                />
              </div>
              <div className="form-field">
                <label htmlFor={`mock-${key}-unanswered-marks`}>Unanswered marks</label>
                <input
                  id={`mock-${key}-unanswered-marks`}
                  type="number"
                  max={0}
                  step="0.01"
                  value={form[key].unansweredMarks}
                  onChange={(event) => handlePart(key, 'unansweredMarks', event.target.value)}
                  disabled={isSaving}
                />
              </div>
            </div>

            <p className="field-hint">
              {calculations.find((part) => part.key === key)?.correct ?? 0} correct ·{' '}
              {calculations.find((part) => part.key === key)?.incorrect ?? 0} incorrect ·{' '}
              {calculations.find((part) => part.key === key)?.unanswered ?? 0} unanswered ·{' '}
              {formatScore(calculations.find((part) => part.key === key)?.marks ?? 0)} calculated
              marks
            </p>
            {errors[key] && (
              <p className="field-error" role="alert">
                {errors[key]}
              </p>
            )}
          </fieldset>
        ))}
      </div>

      <p className="field-hint">
        Enter marking values for each part. Use 0 for incorrect or unanswered if the test has no
        penalty. Leave correct marks blank only for parts with no questions. The score preview is
        calculated from these values, not from a built-in universal scheme.
      </p>

      <section className="mock-score-preview" aria-label="Calculated mock score">
        <div>
          <span>Calculated score</span>
          <strong>
            {formatScore(totalMarks)}
            {typeof maxMarksValue === 'number' && maxMarksValue > 0
              ? ` / ${formatScore(maxMarksValue)}`
              : ''}
          </strong>
        </div>
        <div>
          <span>Calculated percentage</span>
          <strong>
            {computedPercentage === null
              ? 'Set maximum marks'
              : `${computedPercentage.toFixed(2)}%`}
          </strong>
        </div>
        <div>
          <span>Total questions</span>
          <strong>{totalQuestions}</strong>
        </div>
      </section>

      {impossibleScore && (
        <p className="save-feedback save-feedback-error" role="alert">
          The calculated score is above maximum marks. Review maximum marks or the marking
          configuration before saving.
        </p>
      )}

      <div className={`form-field${errors.notes ? ' form-field-invalid' : ''}`}>
        <label htmlFor="mock-notes">Notes (optional)</label>
        <textarea
          id="mock-notes"
          className="planner-textarea"
          value={form.notes}
          maxLength={MAX_NOTES_CHARS}
          onChange={(event) => handleField('notes', event.target.value)}
          disabled={isSaving}
        />
        <p className="field-hint">
          {form.notes.length}/{MAX_NOTES_CHARS} characters.
        </p>
        {errors.notes && (
          <p className="field-error" role="alert">
            {errors.notes}
          </p>
        )}
      </div>

      <div className={`form-field${errors.reviewObservations ? ' form-field-invalid' : ''}`}>
        <label htmlFor="mock-review">Review observations (optional)</label>
        <textarea
          id="mock-review"
          className="planner-textarea"
          value={form.reviewObservations}
          maxLength={MAX_NOTES_CHARS}
          onChange={(event) => handleField('reviewObservations', event.target.value)}
          disabled={isSaving}
        />
        <p className="field-hint">
          {form.reviewObservations.length}/{MAX_NOTES_CHARS} characters.
        </p>
        {errors.reviewObservations && (
          <p className="field-error" role="alert">
            {errors.reviewObservations}
          </p>
        )}
      </div>

      <div className="form-footer">
        <button type="submit" className="primary-button" disabled={isSaving}>
          {isSaving ? 'Saving…' : test === null ? 'Add mock test' : 'Save mock test'}
        </button>
        <button type="button" className="secondary-button" onClick={onCancel} disabled={isSaving}>
          {test === null ? 'Close' : 'Cancel edit'}
        </button>
      </div>

      {feedback && (
        <p className="save-feedback save-feedback-error" role="alert">
          {feedback}
        </p>
      )}
    </form>
  );
}
