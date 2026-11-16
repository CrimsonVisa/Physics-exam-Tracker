import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { MathText } from './MathText';

describe('MathText', () => {
  it('renders plain text and escaped dollar signs', () => {
    render(<MathText text={'A literal \\$5 and plain text.'} />);
    expect(screen.getByText(/A literal \$5 and plain text/)).toBeInTheDocument();
    expect(document.querySelector('.katex')).toBeNull();
  });

  it('renders inline and display LaTeX through KaTeX', async () => {
    render(<MathText text={'Energy $E=mc^2$ and $$\\int_0^1 x dx$$'} />);
    await waitFor(() => expect(document.querySelector('.katex')).not.toBeNull());
    expect(document.querySelector('.v2-math-inline .katex')).not.toBeNull();
    expect(document.querySelector('.v2-math-display .katex')).not.toBeNull();
    expect(document.querySelector('.katex-mathml')).not.toBeNull();
  });

  it('never interprets user content as raw HTML', async () => {
    render(<MathText text={'<img src="x" onerror="alert(1)" />'} />);
    expect(document.querySelector('img')).toBeNull();
    expect(screen.getByText(/<img src="x"/)).toBeInTheDocument();
  });

  it('supports explicit inline and display delimiter pairs', async () => {
    render(<MathText text={String.raw`Inline \(a^2+b^2=c^2\) and display \[\int_0^1 x\,dx\].`} />);

    await waitFor(() => expect(document.querySelectorAll('.katex').length).toBe(2));
    expect(document.querySelector('.v2-math-inline .katex')).not.toBeNull();
    expect(document.querySelector('.v2-math-display .katex')).not.toBeNull();
  });

  it('renders nested fractions, Greek letters, sums, limits and matrices', async () => {
    const text = String.raw`Maxwell: $\nabla \times \mathbf{B} =
      \mu_0 \mathbf{J} + \mu_0 \epsilon_0 \frac{\partial \mathbf{E}}{\partial t}$.
      $$Z = \sum_{n=1}^{\infty} \frac{1}{n^\alpha} \quad
      \lim_{x \to 0} \frac{\sin x}{x} = 1$$`;
    render(<MathText text={text} />);

    await waitFor(() => expect(document.querySelectorAll('.katex').length).toBe(2));
    expect(document.querySelector('.v2-math-inline .katex')).not.toBeNull();
    expect(document.querySelector('.v2-math-display .katex')).not.toBeNull();
  });

  it('leaves ordinary text, escaped dollars and unclosed delimiters untouched', async () => {
    render(<MathText text={String.raw`Price \$5 and an unclosed $E=mc^2`} />);

    expect(screen.getByText(/Price \$5 and an unclosed \$E=mc\^2/)).toBeInTheDocument();
    expect(document.querySelector('.katex')).toBeNull();
  });

  it('falls back to source text for invalid LaTeX', async () => {
    render(<MathText text={String.raw`Broken $\frac{$ expression`} />);

    await waitFor(() =>
      expect(screen.getByTitle('LaTeX rendering unavailable')).toHaveTextContent(
        String.raw`$\frac{$`,
      ),
    );
    expect(document.querySelector('.katex')).toBeNull();
  });

  it('renders an empty string safely', () => {
    const { container } = render(<MathText text="" />);
    expect(container.querySelector('.v2-math-text')).not.toBeNull();
    expect(container.querySelector('.katex')).toBeNull();
  });
});
