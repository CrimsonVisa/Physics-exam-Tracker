# Formula Reference Data Guide

This directory contains the curated, read-only Physics Formula Reference for V2.

## Adding or editing a formula

1. Add the entry to the matching subject module under `data/`.
2. Use a stable, lowercase kebab-case `id`. Never reuse or rename an existing ID.
3. Supply at least one symbol, condition, source, and syllabus link.
4. Each syllabus link references exactly one stable syllabus node code and carries its own
   `explicit`, `implicit`, or `uncertain` relevance.
5. TIFR links must use one of the eight broad-area topic nodes in
   `src-tauri/src/v2/syllabus_seed.json`.
6. Do not add JEST links for atomic/molecular, nuclear/particle, or condensed-matter entries
   unless a future official JEST syllabus explicitly supports them.
7. An `uncertain` link must include a note explaining the uncertainty.
8. Cite textbook/reference provenance with edition and locator where available.
9. Use `locatorCheck: 'chapter-checked'` only when the cited chapter or section was inspected.
   This is a chapter-level check; it does not establish that the exact formula content was
   verified. Use `unverified` otherwise. Omitting the locator produces “locator not provided.”
10. Write summaries, conditions, and conventions independently. Do not copy textbook paragraphs.
11. Run `npm exec -- vitest run src/v2/formulas` and the full frontend suite.

## Scientific review

Every formula still needs human review after automated validation. Check notation, dimensions,
sign conventions, coordinate conventions, operator ordering, boundary conditions, approximation
validity, exactness, and syllabus relevance. Automated tests cannot establish physics correctness.
