import { useState, type FormEvent } from 'react';
import { toCommandError } from '../../lib/commandError';
import {
  saveV2CustomUnit,
  type V2CustomUnit,
  type V2CustomUnitConversionKind,
  type V2CustomUnitInput,
  type V2CustomUnitSiClassification,
} from '../api';
import { DialogShell } from '../DialogShell';
import { MathPreview } from '../math/MathPreview';
import { TERM_LIBRARY } from './data';
import {
  CONVERSION_KIND_HINTS,
  CONVERSION_KIND_LABELS,
  SI_CLASSIFICATION_LABELS,
} from '../reference/catalog';

type CustomUnitEditorProps = {
  unit?: V2CustomUnit | null;
  onClose: () => void;
  onSaved: (unit: V2CustomUnit) => void;
};

type FormState = {
  name: string;
  symbol: string;
  quantity: string;
  dimension: string;
  siClassification: V2CustomUnitSiClassification;
  conversionKind: V2CustomUnitConversionKind;
  conversionFactor: string;
  conversionReferenceUnit: string;
  conversionNote: string;
  description: string;
  notes: string;
  source: string;
};

function formFrom(unit: V2CustomUnit | null | undefined): FormState {
  return {
    name: unit?.name ?? '',
    symbol: unit?.symbol ?? '',
    quantity: unit?.quantity ?? '',
    dimension: unit?.dimension ?? '',
    siClassification: unit?.siClassification ?? 'non_si',
    conversionKind: unit?.conversionKind ?? 'none',
    conversionFactor: unit?.conversionFactor ?? '',
    conversionReferenceUnit: unit?.conversionReferenceUnit ?? '',
    conversionNote: unit?.conversionNote ?? '',
    description: unit?.description ?? '',
    notes: unit?.notes ?? '',
    source: unit?.source ?? '',
  };
}

export function CustomUnitEditor({ unit, onClose, onSaved }: CustomUnitEditorProps) {
  const [form, setForm] = useState<FormState>(() => formFrom(unit));
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const builtInUnit = form.name.trim()
    ? (TERM_LIBRARY.entries.find(
        (entry) =>
          (entry.kind === 'base-unit' || entry.kind === 'derived-unit') &&
          entry.term.trim().toLowerCase() === form.name.trim().toLowerCase(),
      ) ?? null)
    : null;
  const builtInSymbol = form.symbol.trim()
    ? (TERM_LIBRARY.entries.find(
        (entry) =>
          (entry.kind === 'base-unit' || entry.kind === 'derived-unit') &&
          entry.siUnit?.trim().toLowerCase() === form.symbol.trim().toLowerCase(),
      ) ?? null)
    : null;

  function update<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  function setConversionKind(kind: V2CustomUnitConversionKind) {
    setForm((current) => ({
      ...current,
      conversionKind: kind,
      conversionFactor: kind === 'scalar' ? current.conversionFactor : '',
      conversionReferenceUnit: kind === 'none' ? '' : current.conversionReferenceUnit,
      conversionNote: kind === 'contextual' ? current.conversionNote : '',
    }));
  }

  function fieldProps(key: keyof FormState) {
    const message = fieldErrors[key];
    return {
      'aria-describedby': message ? `unit-${key}-error` : undefined,
      'aria-invalid': message ? true : undefined,
    } as const;
  }

  function fieldError(key: keyof FormState) {
    const message = fieldErrors[key];
    return message ? (
      <span className="v2-field-error" id={`unit-${key}-error`} role="alert">
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
    const input: V2CustomUnitInput = {
      id: unit?.id ?? null,
      ...form,
    };
    try {
      const saved = await saveV2CustomUnit(input);
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
      ariaLabel={unit ? 'Edit custom unit' : 'Add custom unit'}
      className="v2-reference-dialog"
      onClose={onClose}
    >
      <form className="v2-reference-editor" onSubmit={submit}>
        <header className="v2-reference-editor-header v2-dialog-header">
          <div>
            <p className="eyebrow">Custom reference</p>
            <h2>{unit ? 'Edit unit' : 'Add unit'}</h2>
          </div>
          <button
            aria-label="Close unit editor"
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
            <label className="v2-field">
              <span>Unit name</span>
              <input
                {...fieldProps('name')}
                autoFocus
                maxLength={200}
                onChange={(event) => update('name', event.target.value)}
                required
                value={form.name}
              />
              {builtInUnit && !fieldErrors.name ? (
                <span className="v2-field-hint" role="status">
                  A built-in unit already uses this name. Your custom entry will be stored
                  separately and labelled User-defined.
                </span>
              ) : null}
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
                {String.raw`LaTeX is supported: \mu_0, \Omega, \mathrm{kg\,m\,s^{-2}}. Plain symbols such as m or s also work.`}
              </span>
              <MathPreview compact label="Symbol preview" source={form.symbol} />
              {builtInSymbol && !fieldErrors.symbol ? (
                <span className="v2-field-hint" role="status">
                  A built-in unit already uses this symbol. Your custom entry will be stored
                  separately and labelled User-defined.
                </span>
              ) : null}
              {fieldError('symbol')}
            </label>
            <label className="v2-field">
              <span>Physical quantity</span>
              <input
                {...fieldProps('quantity')}
                maxLength={200}
                onChange={(event) => update('quantity', event.target.value)}
                placeholder="e.g. Length, Energy, Magnetic flux"
                required
                value={form.quantity}
              />
              {fieldError('quantity')}
            </label>
            <label className="v2-field">
              <span>Physical dimension</span>
              <input
                {...fieldProps('dimension')}
                aria-label="Physical dimension"
                maxLength={200}
                onChange={(event) => update('dimension', event.target.value)}
                placeholder="e.g. L, M L^2 T^-2"
                value={form.dimension}
              />
              <span className="v2-field-hint">
                {String.raw`LaTeX is supported, for example \mathsf{M}\mathsf{L}^{2}\mathsf{T}^{-2}.`}
              </span>
              <MathPreview compact label="Dimension preview" source={form.dimension} />
              {fieldError('dimension')}
            </label>
            <label className="v2-field">
              <span>SI classification</span>
              <select
                {...fieldProps('siClassification')}
                onChange={(event) =>
                  update('siClassification', event.target.value as V2CustomUnitSiClassification)
                }
                value={form.siClassification}
              >
                {Object.entries(SI_CLASSIFICATION_LABELS).map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
              {fieldError('siClassification')}
            </label>
            <label className="v2-field">
              <span>Conversion kind</span>
              <select
                {...fieldProps('conversionKind')}
                onChange={(event) =>
                  setConversionKind(event.target.value as V2CustomUnitConversionKind)
                }
                value={form.conversionKind}
              >
                {Object.entries(CONVERSION_KIND_LABELS).map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
              <span className="v2-field-hint">{CONVERSION_KIND_HINTS[form.conversionKind]}</span>
              {fieldError('conversionKind')}
            </label>

            {form.conversionKind === 'scalar' ? (
              <>
                <label className="v2-field">
                  <span>Conversion factor</span>
                  <input
                    {...fieldProps('conversionFactor')}
                    maxLength={200}
                    onChange={(event) => update('conversionFactor', event.target.value)}
                    placeholder="e.g. 0.3048 or 1.602e-19"
                    required
                    value={form.conversionFactor}
                  />
                  {fieldError('conversionFactor')}
                </label>
                <label className="v2-field">
                  <span>Reference unit</span>
                  <input
                    {...fieldProps('conversionReferenceUnit')}
                    aria-label="Reference unit"
                    maxLength={120}
                    onChange={(event) => update('conversionReferenceUnit', event.target.value)}
                    placeholder="e.g. m"
                    required
                    value={form.conversionReferenceUnit}
                  />
                  <MathPreview
                    compact
                    label="Reference unit preview"
                    source={form.conversionReferenceUnit}
                  />
                  {fieldError('conversionReferenceUnit')}
                </label>
              </>
            ) : null}

            {form.conversionKind === 'contextual' ? (
              <>
                <label className="v2-field">
                  <span>Reference unit</span>
                  <input
                    {...fieldProps('conversionReferenceUnit')}
                    aria-label="Reference unit"
                    maxLength={120}
                    onChange={(event) => update('conversionReferenceUnit', event.target.value)}
                    placeholder="Optional reference unit"
                    value={form.conversionReferenceUnit}
                  />
                  <MathPreview
                    compact
                    label="Reference unit preview"
                    source={form.conversionReferenceUnit}
                  />
                  {fieldError('conversionReferenceUnit')}
                </label>
                <label className="v2-field v2-reference-field-wide">
                  <span>Conversion context</span>
                  <textarea
                    {...fieldProps('conversionNote')}
                    aria-label="Conversion context"
                    maxLength={2000}
                    onChange={(event) => update('conversionNote', event.target.value)}
                    placeholder="Describe the convention, material, temperature or context required."
                    required
                    rows={2}
                    value={form.conversionNote}
                  />
                  <MathPreview
                    label="Mathematical preview"
                    mode="text"
                    source={form.conversionNote}
                  />
                  {fieldError('conversionNote')}
                </label>
              </>
            ) : null}

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
              <span>Source or reference</span>
              <input
                {...fieldProps('source')}
                maxLength={1000}
                onChange={(event) => update('source', event.target.value)}
                placeholder="Optional standard, textbook or note"
                value={form.source}
              />
              {fieldError('source')}
            </label>
            <label className="v2-field v2-reference-field-wide">
              <span>Notes</span>
              <textarea
                {...fieldProps('notes')}
                aria-label="Notes"
                maxLength={6000}
                onChange={(event) => update('notes', event.target.value)}
                placeholder="Optional usage notes or caveats"
                rows={2}
                value={form.notes}
              />
              <MathPreview label="Mathematical preview" mode="text" source={form.notes} />
              {fieldError('notes')}
            </label>
          </div>

          <p className="v2-reference-disclaimer">
            Conversion details are reference information only. Physics Tracker does not run an
            automatic conversion engine or alter official SI definitions.
          </p>
        </div>

        <footer className="v2-reference-editor-actions v2-dialog-footer">
          <button className="secondary-button" disabled={busy} onClick={onClose} type="button">
            Cancel
          </button>
          <button className="primary-button" disabled={busy} type="submit">
            {busy ? 'Saving…' : unit ? 'Save changes' : 'Add unit'}
          </button>
        </footer>
      </form>
    </DialogShell>
  );
}
