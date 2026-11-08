export type KatexRenderer = typeof import('katex').default;

let katexPromise: Promise<KatexRenderer> | null = null;

export function loadKatex(): Promise<KatexRenderer> {
  if (!katexPromise) {
    katexPromise = import('katex')
      .then((module) => module.default)
      .catch((error) => {
        katexPromise = null;
        throw error;
      });
  }
  return katexPromise;
}
