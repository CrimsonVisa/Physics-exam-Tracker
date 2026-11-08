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
});
