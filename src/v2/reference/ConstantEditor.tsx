import { useState, type FormEvent } from 'react';
import { toCommandError } from '../../lib/commandError';
import { saveV2CustomConstant, type V2CustomConstant, type V2CustomConstantInput } from '../api';
import { DialogShell } from '../DialogShell';
import { MathPreview } from '../math/MathPreview';
import { CONSTANT_CATEGORIES } from '../reference/catalog';

type ConstantEditorProps = {
  constant?: V2CustomConstant | null;
  onClose: () => void;
  onSaved: (constant: V2CustomConstant) => void;
};

type FormState = {
  name: string;
  symbol: string;
  valueText: string;
  unit: string;
  category: string;
  uncertaintyText: string;
  precisionText: string;
  source: string;
  description: string;
  notes: string;
};

function formFrom(constant: V2CustomConstant | null | undefined): FormState {
  return {
    name: constant?.name ?? '',
    symbol: constant?.symbol ?? '',
    valueText: constant?.valueText ?? '',
    unit: constant?.unit ?? '',
    category: constant?.category ?? 'User-defined',
    uncertaintyText: constant?.uncertaintyText ?? '',
    precisionText: constant?.precisionText ?? '',
    source: constant?.source ?? '',
    description: constant?.description ?? '',
    notes: constant?.notes ?? '',
  };
}

export function ConstantEditor({ constant, onClose, onSaved }: ConstantEditorProps) {
  const [form, setForm] = useState<FormState>(() => formFrom(constant));
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  function update<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  function fieldProps(key: keyof FormState) {
    const message = fieldErrors[key];
    return {
      'aria-describedby': message ? `constant-${key}-error` : undefined,
      'aria-invalid': message ? true : undefined,
    } as const;
  }

  function fieldError(key: keyof FormState) {
    const message = fieldErrors[key];
    return message ? (
      <span className="v2-field-error" id={`constant-${key}-error`} role="alert">
        {message}
      </span>
    ) : null;
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) {
      return;
    }
    setBusy(true);
    setFieldErrors({});
    setFormError(null);
    const input: V2CustomConstantInput = {
      id: constant?.id ?? null,
      ...form,
    };
    try {
      const saved = await saveV2CustomConstant(input);
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
      ariaLabel={constant ? 'Edit custom constant' : 'Add custom constant'}
      className="v2-reference-dialog"
      onClose={onClose}
    >
      <form className="v2-reference-editor" onSubmit={submit}>
        <header className="v2-reference-editor-header v2-dialog-header">
          <div>
            <p className="eyebrow">Custom reference</p>
            <h2>{constant ? 'Edit constant' : 'Add constant'}</h2>
          </div>
          <button
            aria-label="Close constant editor"
            className="v2-dialog-close"
            onClick={onClose}
            type="button"
          >
            Close
          </button>
        </header>

        <div className="v2-reference-editor-body v2-dialog-body">
          {formError ? (
            <p className="v2-inline-error" role="alert">
              {formError}
            </p>
          ) : null}

          <div className="v2-reference-editor-grid">
            <label className="v2-field v2-reference-field-wide">
              <span>Full name</span>
              <input
                {...fieldProps('name')}
                autoFocus
                maxLength={300}
                onChange={(event) => update('name', event.target.value)}
                required
                value={form.name}
              />
              {fieldError('name')}
            </label>
            <label className="v2-field">
              <span>Symbol</span>
              <input
                {...fieldProps('symbol')}
                aria-label="Symbol"
                maxLength={80}
                onChange={(event) => update('symbol', event.target.value)}
                required
                value={form.symbol}
              />
              <span className="v2-field-hint">
                LaTeX is supported: \hbar, \alpha, \epsilon_0, \mu_0, \sigma. Plain symbols such as
                c, G and h also work.
              </span>
              <MathPreview compact label="Symbol preview" source={form.symbol} />
              {fieldError('symbol')}
            </label>
            <label className="v2-field">
              <span>Unit</span>
              <input
                {...fieldProps('unit')}
                aria-label="Unit"
                maxLength={120}
                onChange={(event) => update('unit', event.target.value)}
                placeholder="e.g. J s, m s^-1, or blank"
                value={form.unit}
              />
              <span className="v2-field-hint">
                {String.raw`Unit symbols accept LaTeX, for example \mathrm{J\,s} or \mathrm{m\,s^{-1}}.`}
              </span>
              <MathPreview compact label="Unit preview" source={form.unit} />
              {fieldError('unit')}
            </label>
            <label className="v2-field v2-reference-field-wide">
              <span>Numerical value</span>
              <input
                {...fieldProps('valueText')}
                className="v2-reference-value-input"
                maxLength={200}
                onChange={(event) => update('valueText', event.target.value)}
                placeholder="e.g. 6.62607015e-34"
                required
                value={form.valueText}
              />
              <span className="v2-field-hint">
                Stored and displayed exactly as entered. Scientific notation is preserved.
              </span>
              {fieldError('valueText')}
            </label>
            <label className="v2-field">
              <span>Category</span>
              <input
                {...fieldProps('category')}
                list="constant-category-options"
                maxLength={120}
                onChange={(event) => update('category', event.target.value)}
                required
                value={form.category}
              />
              <datalist id="constant-category-options">
                {CONSTANT_CATEGORIES.map((category) => (
                  <option key={category} value={category} />
                ))}
              </datalist>
              {fieldError('category')}
            </label>
            <label className="v2-field">
              <span>Precision or significant figures</span>
              <input
                {...fieldProps('precisionText')}
                maxLength={120}
                onChange={(event) => update('precisionText', event.target.value)}
                placeholder="Optional, e.g. 9 significant figures"
                value={form.precisionText}
              />
              {fieldError('precisionText')}
            </label>
            <label className="v2-field v2-reference-field-wide">
              <span>Uncertainty</span>
              <input
                {...fieldProps('uncertaintyText')}
                maxLength={200}
                onChange={(event) => update('uncertaintyText', event.target.value)}
                placeholder="Optional, e.g. 0.00000001 or exact"
                value={form.uncertaintyText}
              />
              {fieldError('uncertaintyText')}
            </label>
            <label className="v2-field v2-reference-field-wide">
              <span>Source or reference</span>
              <input
                {...fieldProps('source')}
                maxLength={1000}
                onChange={(event) => update('source', event.target.value)}
                placeholder="Optional source, textbook, dataset or note"
                value={form.source}
              />
              {fieldError('source')}
            </label>
            <label className="v2-field v2-reference-field-wide">
              <span>Description</span>
              <textarea
                {...fieldProps('description')}
                aria-label="Description"
                maxLength={6000}
                onChange={(event) => update('description', event.target.value)}
                required
                rows={3}
                value={form.description}
              />
              <span className="v2-field-hint">
                Plain text is fine. Wrap inline mathematics in $...$ or \(...\), and display
                mathematics in $$...$$ or \[...\].
              </span>
              <MathPreview label="Mathematical preview" mode="text" source={form.description} />
              {fieldError('description')}
            </label>
            <label className="v2-field v2-reference-field-wide">
              <span>Notes</span>
              <textarea
                {...fieldProps('notes')}
                aria-label="Notes"
                maxLength={6000}
                onChange={(event) => update('notes', event.target.value)}
                placeholder="Optional usage notes, conventions or caveats"
                rows={2}
                value={form.notes}
              />
              <MathPreview label="Mathematical preview" mode="text" source={form.notes} />
              {fieldError('notes')}
            </label>
          </div>
        </div>

        <footer className="v2-reference-editor-actions v2-dialog-footer">
          <button className="secondary-button" disabled={busy} onClick={onClose} type="button">
            Cancel
          </button>
          <button className="primary-button" disabled={busy} type="submit">
            {busy ? 'Saving…' : constant ? 'Save changes' : 'Add constant'}
          </button>
        </footer>
      </form>
    </DialogShell>
  );
}
