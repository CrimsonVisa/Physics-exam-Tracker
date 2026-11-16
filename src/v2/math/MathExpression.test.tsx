import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { MathExpression } from './MathExpression';
import { MathPreview } from './MathPreview';

describe('MathExpression', () => {
  it('renders common physics notation, fractions and matrices through KaTeX', async () => {
    const latex = String.raw`\frac{\partial^2 \psi}{\partial x^2} =
      \sum_{n=1}^{\infty} \alpha_n \hbar \int_0^1 \mu_0 \sigma \,
      \begin{pmatrix} a & b \\ c & d \end{pmatrix}`;
    const { container } = render(<MathExpression display latex={latex} plainText="physics" />);

    await waitFor(() => expect(container.querySelector('.katex')).not.toBeNull());
    expect(container.querySelector('.katex-mathml')).not.toBeNull();
    expect(container.querySelector('[data-latex-source]')).toHaveAttribute(
      'data-latex-source',
      latex,
    );
    expect(container.querySelector('[role="math"]')).toHaveAttribute('aria-label', 'physics');
  });

  it('accepts complete explicit delimiter pairs in dedicated math fields', async () => {
    const inlineSource = '$E=mc^2$';
    const displaySource = String.raw`\[\int_0^1 x\,dx\]`;
    const { container } = render(
      <div>
        <MathExpression latex={inlineSource} />
        <MathExpression latex={displaySource} />
      </div>,
    );

    await waitFor(() => expect(container.querySelectorAll('.katex').length).toBe(2));
    expect(container.querySelector('.v2-math-expression-inline .katex')).not.toBeNull();
    expect(container.querySelector('.v2-math-expression-display .katex')).not.toBeNull();
    expect(
      [...container.querySelectorAll('[data-latex-source]')].map((element) =>
        element.getAttribute('data-latex-source'),
      ),
    ).toEqual([inlineSource, displaySource]);
  });

  it('renders ordinary symbols and Greek letters', async () => {
    const { container } = render(
      <div>
        <MathExpression latex="c" />
        <MathExpression latex="G" />
        <MathExpression latex="h" />
        <MathExpression latex={String.raw`\hbar`} />
        <MathExpression latex={String.raw`\alpha`} />
        <MathExpression latex={String.raw`\epsilon_0`} />
        <MathExpression latex={String.raw`\mu_0`} />
        <MathExpression latex={String.raw`\sigma`} />
      </div>,
    );

    await waitFor(() => expect(container.querySelectorAll('.katex').length).toBe(8));
    for (const expression of container.querySelectorAll('[data-latex-source]')) {
      expect(expression.getAttribute('data-latex-source')).toBeTruthy();
    }
  });

  it('falls back to readable text for invalid LaTeX without interpreting HTML', async () => {
    const { container } = render(
      <MathExpression
        display
        latex={String.raw`\frac{`}
        plainText="<img src=x onerror=alert(1)>"
      />,
    );

    await waitFor(() =>
      expect(screen.getByTitle('LaTeX could not be rendered')).toHaveTextContent(
        '<img src=x onerror=alert(1)>',
      ),
    );
    expect(container.querySelector('img')).toBeNull();
    expect(container.querySelector('.katex')).toBeNull();
  });

  it('renders an empty expression without crashing', () => {
    const { container } = render(<MathExpression latex="   " />);
    expect(container.querySelector('.v2-math-expression-empty')).not.toBeNull();
    expect(container.textContent).toBe('');
  });

  it('shows only explicit math in text-mode previews', async () => {
    const { rerender } = render(
      <MathPreview label="Preview" mode="text" source="Ordinary plain text" />,
    );
    expect(screen.queryByText('Preview')).not.toBeInTheDocument();

    rerender(<MathPreview label="Preview" mode="text" source={'Energy $E=mc^2$'} />);
    await waitFor(() => expect(document.querySelector('.v2-math-preview .katex')).not.toBeNull());
    expect(screen.getByText('Preview')).toBeInTheDocument();
  });

  it('shows expression previews and an empty-state prompt', () => {
    render(
      <MathPreview display emptyLabel="Enter an expression" label="Equation preview" source="" />,
    );
    expect(screen.getByText('Equation preview')).toBeInTheDocument();
    expect(screen.getByText('Enter an expression')).toBeInTheDocument();
  });

  it('keeps display math horizontally scrollable and theme-aware', () => {
    const css = readFileSync(join(process.cwd(), 'src/v2/math.css'), 'utf8');
    expect(css).toMatch(
      /\.v2-math-expression-display,[\s\S]*?\.v2-math-display\s*{[^}]*overflow-x:\s*auto;/,
    );
    expect(css).toMatch(/\.v2-math-preview\s*{[^}]*background:\s*var\(--surface-soft\);/);
    expect(css).toMatch(/@media \(max-width: 680px\)/);
    expect(css).not.toMatch(/#fff(?:fff)?|#000(?:000)?/i);
  });
});
