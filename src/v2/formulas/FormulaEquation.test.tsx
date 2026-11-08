import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { FormulaEquation } from './FormulaEquation';

describe('FormulaEquation', () => {
  it('renders inline and display KaTeX with MathML', async () => {
    const { container } = render(
      <div>
        <FormulaEquation latex={String.raw`E=mc^2`} plainText="E = m c^2" />
        <FormulaEquation
          display
          latex={String.raw`\int_0^1 x\,dx`}
          plainText="integral from 0 to 1 of x dx"
        />
      </div>,
    );

    await waitFor(() => expect(container.querySelectorAll('.katex').length).toBeGreaterThan(0));
    expect(container.querySelector('.v2-formula-equation-inline .katex')).not.toBeNull();
    expect(container.querySelector('.v2-formula-equation-display .katex')).not.toBeNull();
    expect(container.querySelectorAll('.katex-mathml').length).toBeGreaterThan(0);
  });

  it('shows the readable plain-text fallback for invalid LaTeX', async () => {
    render(
      <FormulaEquation display latex={String.raw`\frac{`} plainText="invalid formula fallback" />,
    );

    await waitFor(() =>
      expect(screen.getByTitle('Formula rendering unavailable')).toHaveTextContent(
        'invalid formula fallback',
      ),
    );
  });

  it('does not interpret raw HTML in the fallback', async () => {
    render(
      <FormulaEquation
        latex={String.raw`\notACommand{`}
        plainText="<img src=x onerror=alert(1)>"
      />,
    );

    await waitFor(() =>
      expect(screen.getByTitle('Formula rendering unavailable')).toHaveTextContent(
        '<img src=x onerror=alert(1)>',
      ),
    );
    expect(document.querySelector('img')).toBeNull();
  });
});
