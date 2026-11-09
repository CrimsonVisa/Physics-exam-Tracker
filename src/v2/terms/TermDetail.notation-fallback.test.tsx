import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('../katex', () => ({
  loadKatex: () => Promise.reject(new Error('KaTeX chunk failed')),
}));

import { TERM_LIBRARY } from './data';
import { TermDetail } from './TermDetail';

function renderEntry(entryId: string) {
  const entry = TERM_LIBRARY.entries.find((candidate) => candidate.id === entryId);
  expect(entry).toBeDefined();
  render(
    <TermDetail
      entries={TERM_LIBRARY.entries}
      entry={entry!}
      exams={[]}
      onBack={() => undefined}
      onOpenRelated={() => undefined}
    />,
  );
}

describe('TermDetail notation fallback', () => {
  it('uses plain-text notation when KaTeX cannot load', async () => {
    renderEntry('units-si-prefix');

    expect(await screen.findByText('μ')).toBeInTheDocument();
    expect(screen.queryByText(String.raw`\mathrm{\mu}`)).not.toBeInTheDocument();
  });

  it('preserves ket notation without raw LaTeX', async () => {
    renderEntry('quantum-mechanics-wavefunction');

    expect(await screen.findByText('|ψ⟩')).toBeInTheDocument();
    expect(screen.queryByText(String.raw`|\psi\rangle`)).not.toBeInTheDocument();
  });

  it('preserves braced function notation without raw LaTeX', async () => {
    renderEntry('mathematical-physics-fourier-transform');

    expect(await screen.findByText('F{f}(k)')).toBeInTheDocument();
    expect(screen.queryByText(String.raw`\mathcal{F}\{f\}(k)`)).not.toBeInTheDocument();
  });
});
