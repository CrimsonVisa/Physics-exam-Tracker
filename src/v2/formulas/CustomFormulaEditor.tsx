import { useState, type FormEvent } from 'react';
import { toCommandError } from '../../lib/commandError';
import {
  saveV2CustomFormula,
  type V2CustomFormula,
  type V2CustomFormulaInput,
  type V2CustomFormulaVariableInput,
} from '../api';
import { DialogShell } from '../DialogShell';
import { FORMULA_LIBRARY } from './data';
import { FORMULA_SUBJECTS } from './taxonomy';

type CustomFormulaEditorProps = {
  formula?: V2CustomFormula | null;
  onClose: () => void;
  onSaved: (formula: V2CustomFormula) => void;
};

type FormState = {
  name: string;
  latex: string;
  plainText: string;
  category: string;
  description: string;
  notes: string;
  source: string;
};

function formFrom(formula: V2CustomFormula | null | undefined): FormState {
  return {
    name: formula?.name ?? '',
    latex: formula?.latex ?? '',
    plainText: formula?.plainText ?? '',
    category: formula?.category ?? '',
    description: formula?.description ?? '',
    notes: formula?.notes ?? '',
    source: formula?.source ?? '',
  };
}

function variablesFrom(
  formula: V2CustomFormula | null | undefined,
): V2CustomFormulaVariableInput[] {
  return (
    formula?.variables.map((variable) => ({
      symbol: variable.symbol,
      meaning: variable.meaning,
      unit: variable.unit,
    })) ?? []
  );
}

export function CustomFormulaEditor({ formula, onClose, onSaved }: CustomFormulaEditorProps) {
  const [form, setForm] = useState<FormState>(() => formFrom(formula));
  const [variables, setVariables] = useState<V2CustomFormulaVariableInput[]>(() =>
    variablesFrom(formula),
  );
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  function update<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  function updateVariable(index: number, key: keyof V2CustomFormulaVariableInput, value: string) {
    setVariables((current) =>
      current.map((variable, position) =>
        position === index ? { ...variable, [key]: value } : variable,
      ),
    );
  }

  function fieldProps(key: keyof FormState) {
    const message = fieldErrors[key];
    return {
      'aria-describedby': message ? `formula-${key}-error` : undefined,
      'aria-invalid': message ? true : undefined,
    } as const;
  }

  function fieldError(key: keyof FormState) {
    const message = fieldErrors[key];
    return message ? (
      <span className="v2-field-error" id={`formula-${key}-error`} role="alert">
        {message}
      </span>
    ) : null;
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) {
      return;
    }
    const builtInName = FORMULA_LIBRARY.entries.find(
      (entry) => entry.title.trim().toLowerCase() === form.name.trim().toLowerCase(),
    );
    if (builtInName) {
      setFieldErrors({
        name: `The built-in formula "${builtInName.title}" already uses this name. Choose a different name for your custom formula.`,
      });
      setFormError(null);
      return;
    }
    setBusy(true);
    setFieldErrors({});
    setFormError(null);
    const input: V2CustomFormulaInput = {
      id: formula?.id ?? null,
      ...form,
      variables,
    };
    try {
      const saved = await saveV2CustomFormula(input);
      onSaved(saved);
    } catch (error) {
      const commandError = toCommandError(error);
      if (commandError.field && commandError.field in form) {
        setFieldErrors({ [commandError.field]: commandError.message });
      } else {
        setFormError(commandError.message);
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <DialogShell
      ariaLabel={formula ? 'Edit custom formula' : 'Add custom formula'}
      className="v2-reference-dialog"
      onClose={onClose}
    >
      <form className="v2-reference-editor" onSubmit={submit}>
        <header className="v2-reference-editor-header">
          <div>
            <p className="eyebrow">Custom reference</p>
            <h2>{formula ? 'Edit formula' : 'Add formula'}</h2>
          </div>
          <button
            aria-label="Close formula editor"
            className="v2-dialog-close"
            onClick={onClose}
            type="button"
          >
            Close
          </button>
        </header>

        {formError ? (
          <p className="v2-inline-error" role="alert">
            {formError}
          </p>
        ) : null}

        <div className="v2-reference-editor-grid">
          <label className="v2-field v2-reference-field-wide">
            <span>Formula name</span>
            <input
              {...fieldProps('name')}
              autoFocus
              maxLength={200}
              onChange={(event) => update('name', event.target.value)}
              required
              value={form.name}
            />
            {fieldError('name')}
          </label>
          <label className="v2-field">
            <span>Category</span>
            <input
              {...fieldProps('category')}
              list="formula-category-options"
              maxLength={120}
              onChange={(event) => update('category', event.target.value)}
              placeholder="e.g. Quantum mechanics"
              required
              value={form.category}
            />
            <datalist id="formula-category-options">
              {FORMULA_SUBJECTS.map((subject) => (
                <option key={subject.id} value={subject.label} />
              ))}
            </datalist>
            {fieldError('category')}
          </label>
          <span className="v2-reference-field-wide v2-field-hint">
            Expressions are stored as reference text. Physics Tracker never evaluates or parses them
            automatically.
          </span>
          <label className="v2-field">
            <span>LaTeX expression</span>
            <textarea
              {...fieldProps('latex')}
              className="v2-reference-expression-input"
              maxLength={20000}
              onChange={(event) => update('latex', event.target.value)}
              placeholder={String.raw`e.g. E = mc^2`}
              required
              rows={3}
              value={form.latex}
            />
            {fieldError('latex')}
          </label>
          <label className="v2-field">
            <span>Plain-text expression</span>
            <textarea
              {...fieldProps('plainText')}
              className="v2-reference-expression-input"
              maxLength={20000}
              onChange={(event) => update('plainText', event.target.value)}
              placeholder="e.g. E = m c^2"
              required
              rows={3}
              value={form.plainText}
            />
            {fieldError('plainText')}
          </label>
          <label className="v2-field v2-reference-field-wide">
            <span>Description</span>
            <textarea
              {...fieldProps('description')}
              maxLength={6000}
              onChange={(event) => update('description', event.target.value)}
              required
              rows={3}
              value={form.description}
            />
            {fieldError('description')}
          </label>
        </div>

        <section aria-label="Formula variables" className="v2-reference-variable-editor">
          <div className="v2-reference-variable-editor-header">
            <div>
              <h3>Variables and meanings</h3>
              <p>Define each symbol so the formula remains understandable without evaluation.</p>
            </div>
            <button
              className="secondary-button"
              onClick={() =>
                setVariables((current) => [...current, { symbol: '', meaning: '', unit: '' }])
              }
              type="button"
            >
              Add variable
            </button>
          </div>
          {variables.length === 0 ? (
            <p className="v2-empty-copy">No variables recorded yet.</p>
          ) : (
            <ol className="v2-reference-variable-list">
              {variables.map((variable, index) => (
                <li className="v2-reference-variable-row" key={index}>
                  <label className="v2-field">
                    <span>Symbol</span>
                    <input
                      aria-label={`Variable ${index + 1} symbol`}
                      maxLength={80}
                      onChange={(event) => updateVariable(index, 'symbol', event.target.value)}
                      required
                      value={variable.symbol}
                    />
                  </label>
                  <label className="v2-field">
                    <span>Meaning</span>
                    <input
                      aria-label={`Variable ${index + 1} meaning`}
                      maxLength={500}
                      onChange={(event) => updateVariable(index, 'meaning', event.target.value)}
                      required
                      value={variable.meaning}
                    />
                  </label>
                  <label className="v2-field">
                    <span>Unit</span>
                    <input
                      aria-label={`Variable ${index + 1} unit`}
                      maxLength={120}
                      onChange={(event) => updateVariable(index, 'unit', event.target.value)}
                      placeholder="Optional"
                      value={variable.unit}
                    />
                  </label>
                  <button
                    aria-label={`Remove variable ${index + 1}`}
                    className="secondary-button v2-reference-remove-variable"
                    onClick={() =>
                      setVariables((current) => current.filter((_, position) => position !== index))
                    }
                    type="button"
                  >
                    Remove
                  </button>
                </li>
              ))}
            </ol>
          )}
        </section>

        <div className="v2-reference-editor-grid">
          <label className="v2-field v2-reference-field-wide">
            <span>Source or reference</span>
            <input
              {...fieldProps('source')}
              maxLength={1000}
              onChange={(event) => update('source', event.target.value)}
              placeholder="Optional textbook, paper or note"
              value={form.source}
            />
            {fieldError('source')}
          </label>
          <label className="v2-field v2-reference-field-wide">
            <span>Notes</span>
            <textarea
              {...fieldProps('notes')}
              maxLength={6000}
              onChange={(event) => update('notes', event.target.value)}
              placeholder="Optional assumptions, conventions or caveats"
              rows={2}
              value={form.notes}
            />
            {fieldError('notes')}
          </label>
        </div>

        <footer className="v2-reference-editor-actions">
          <button className="secondary-button" disabled={busy} onClick={onClose} type="button">
            Cancel
          </button>
          <button className="primary-button" disabled={busy} type="submit">
            {busy ? 'Saving…' : formula ? 'Save changes' : 'Add formula'}
          </button>
        </footer>
      </form>
    </DialogShell>
  );
}
