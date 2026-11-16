import { useEffect, useState } from 'react';
import { loadKatex, type KatexRenderer } from '../katex';

export type KatexState = {
  katex: KatexRenderer | null;
  loadFailed: boolean;
};

/**
 * Load the shared KaTeX renderer once per mounted mathematical component.
 * The underlying module promise is cached in `katex.ts`, so multiple
 * expressions share a single network/chunk request.
 */
export function useKatex(): KatexState {
  const [katex, setKatex] = useState<KatexRenderer | null>(null);
  const [loadFailed, setLoadFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void loadKatex()
      .then((loaded) => {
        if (!cancelled) {
          setKatex(() => loaded);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setLoadFailed(true);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return { katex, loadFailed };
}
