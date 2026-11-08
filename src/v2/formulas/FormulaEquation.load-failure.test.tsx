import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('../katex', () => ({
  loadKatex: () => Promise.reject(new Error('KaTeX chunk failed')),
}));

import { MathText } from '../MathText';
import { FormulaEquation } from './FormulaEquation';

describe('KaTeX load failure handling', () => {
  it('keeps the formula plain-text fallback without an unhandled rejection', async () => {
    render(<FormulaEquation latex={String.raw`E=mc^2`} plainText="E = m c^2" />);

    await waitFor(() =>
      expect(screen.getByTitle('Formula rendering unavailable')).toHaveTextContent('E = m c^2'),
    );
  });

  it('keeps the MathText fallback and reports that rendering is unavailable', async () => {
    render(<MathText text={'Energy $E=mc^2$'} />);

    await waitFor(() =>
      expect(screen.getByTitle('LaTeX rendering unavailable')).toHaveTextContent('$E=mc^2$'),
    );
  });
});
