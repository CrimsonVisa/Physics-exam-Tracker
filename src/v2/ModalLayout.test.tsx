import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { CustomFormulaEditor } from './formulas/CustomFormulaEditor';
import { ConstantEditor } from './reference/ConstantEditor';
import { CustomUnitEditor } from './terms/CustomUnitEditor';

function requireElement(element: Element | null, label: string): HTMLElement {
  if (!(element instanceof HTMLElement)) {
    throw new Error(`Expected ${label} to be an HTML element`);
  }
  return element;
}

const editorCases = [
  {
    name: 'Add custom constant',
    closeLabel: 'Close constant editor',
    firstField: 'Full name',
    lastField: 'Notes',
    primaryAction: 'Add constant',
    renderEditor: () => render(<ConstantEditor onClose={vi.fn()} onSaved={vi.fn()} />),
  },
  {
    name: 'Add custom formula',
    closeLabel: 'Close formula editor',
    firstField: 'Formula name',
    lastField: 'Notes',
    primaryAction: 'Add formula',
    renderEditor: () => render(<CustomFormulaEditor onClose={vi.fn()} onSaved={vi.fn()} />),
  },
  {
    name: 'Add custom unit',
    closeLabel: 'Close unit editor',
    firstField: 'Unit name',
    lastField: 'Notes',
    primaryAction: 'Add unit',
    renderEditor: () => render(<CustomUnitEditor onClose={vi.fn()} onSaved={vi.fn()} />),
  },
];

describe('reference editor modal layout', () => {
  it.each(editorCases)(
    'keeps the $name header and actions outside the scrollable body',
    ({ name, closeLabel, firstField, lastField, primaryAction, renderEditor }) => {
      renderEditor();

      const dialog = screen.getByRole('dialog', { name });
      const header = requireElement(dialog.querySelector('.v2-dialog-header'), 'dialog header');
      const body = requireElement(dialog.querySelector('.v2-dialog-body'), 'dialog body');
      const footer = requireElement(dialog.querySelector('.v2-dialog-footer'), 'dialog footer');

      expect(header).toContainElement(screen.getByRole('button', { name: closeLabel }));
      expect(body).toContainElement(screen.getByLabelText(firstField));
      expect(body).toContainElement(screen.getByLabelText(lastField));
      expect(body).not.toContainElement(screen.getByRole('button', { name: primaryAction }));
      expect(footer).toContainElement(screen.getByRole('button', { name: 'Cancel' }));
      expect(footer).toContainElement(screen.getByRole('button', { name: primaryAction }));
    },
  );

  it('keeps the actions accessible when the formula variable list is long', async () => {
    const user = userEvent.setup();
    render(<CustomFormulaEditor onClose={vi.fn()} onSaved={vi.fn()} />);

    const body = requireElement(document.querySelector('.v2-dialog-body'), 'dialog body');
    for (let index = 0; index < 12; index += 1) {
      await user.click(screen.getByRole('button', { name: 'Add variable' }));
    }

    expect(body.querySelectorAll('.v2-reference-variable-row')).toHaveLength(12);
    expect(body).toContainElement(screen.getByLabelText('Variable 12 unit'));

    const footer = requireElement(document.querySelector('.v2-dialog-footer'), 'dialog footer');
    expect(footer).toContainElement(screen.getByRole('button', { name: 'Cancel' }));
    expect(footer).toContainElement(screen.getByRole('button', { name: 'Add formula' }));
    expect(screen.getByRole('button', { name: 'Close formula editor' })).toBeEnabled();
  });
});

describe('shared modal constraints', () => {
  const dialogCss = readFileSync(join(process.cwd(), 'src/v2/global-search.css'), 'utf8');
  const referenceCss = readFileSync(join(process.cwd(), 'src/v2/reference.css'), 'utf8');

  it('keeps the dialog within the viewport with a single scrolling body', () => {
    expect(dialogCss).toMatch(/\.v2-dialog-backdrop\s*{[^}]*align-items:\s*center;/s);
    expect(dialogCss).toMatch(/\.v2-dialog-backdrop\s*{[^}]*overflow:\s*hidden;/s);
    expect(dialogCss).toMatch(/\.v2-dialog-body\s*{[^}]*min-height:\s*0;/s);
    expect(dialogCss).toMatch(/\.v2-dialog-body\s*{[^}]*overflow-y:\s*auto;/s);
    expect(dialogCss).toMatch(/\.v2-dialog-body\s*{[^}]*overscroll-behavior:\s*contain;/s);
    expect(dialogCss).toMatch(
      /\.v2-dialog-header,\s*\.v2-dialog-footer\s*{[^}]*flex:\s*0 0 auto;/s,
    );
  });

  it('uses viewport-relative heights, fixed actions and responsive gutters', () => {
    expect(referenceCss).toMatch(/\.v2-reference-dialog\s*{[^}]*max-height:\s*calc\(100dvh/s);
    expect(referenceCss).toMatch(/\.v2-reference-dialog\s*{[^}]*overflow:\s*hidden;/s);
    expect(referenceCss).toMatch(/\.v2-reference-editor\s*{[^}]*flex-direction:\s*column;/s);
    expect(referenceCss).toMatch(
      /\.v2-reference-editor-actions\s*{[^}]*border-top:\s*1px solid var\(--line\);/s,
    );
    expect(dialogCss).toMatch(
      /@media \(max-width: 700px\)[\s\S]*?\.v2-dialog-backdrop\s*{[^}]*--v2-dialog-gutter:\s*10px;/s,
    );
    expect(referenceCss).toMatch(
      /@media \(max-width: 680px\)[\s\S]*?\.v2-reference-editor-body\s*{[^}]*padding:\s*14px 16px;/s,
    );
  });
});
