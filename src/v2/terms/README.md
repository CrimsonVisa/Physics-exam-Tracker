# Physics Terms & SI Units Data Guide

This directory contains the curated, read-only Physics Terms & SI Units reference for V2.
The corrected library contains 123 entries across the approved 15 categories. Three entries were
added to keep absolute/relative electromagnetic quantities and angular/ordinary wavenumber
distinct.

## Adding or editing a term

1. Add the entry to the matching subject module under `data/`.
2. Use a stable, lowercase kebab-case `id`. Never reuse or rename an existing ID.
3. Keep terms, aliases and definitions unique after Unicode normalization.
4. Choose a `kind`: base unit, derived unit, quantity, concept, symbol or notation.
5. Units and quantities require their applicable SI unit and dimensional formula. Concepts,
   symbols and notation may omit fields that are not physically applicable.
6. Use the base-dimension notation `M, L, T, I, Θ, N, J` and the existing SI conventions.
7. Cite textbook/reference provenance with edition and locator where available.
8. `data/shared.ts` applies an entry-specific chapter-check map keyed by
   `entryId + citation + locator`. Do not reuse the Formula Reference checked set, and do not
   verify one term merely because another term uses the same chapter. A chapter or
   table-of-contents locator is still chapter-level checking, not definition-level verification.
9. Only add an entry-specific checked source key after checking that exact term against the cited
   source. Otherwise leave the locator unverified.
10. Syllabus links are optional. Use exact node codes and titles, and never force an unsupported
    examination mapping. TIFR uses broad-area nodes only and never `explicit` relevance.
11. An `explicit` link means the node title names the term or an exact, unambiguous synonym. Every
    explicit link must be added to `explicitAllowlist.ts` after review. Standard subtopics are
    implicit.
12. Do not claim JEST coverage for atomic/molecular, nuclear/particle or condensed-matter entries
    without explicit official syllabus support.
13. Write definitions, conditions and conventions independently. Do not copy textbook paragraphs.
14. Run `npm exec -- vitest run src/v2/terms` and the full frontend suite.

## Validation

`validation.ts` checks the supported dimensional grammar, SI unit expressions, dimensional and
rendered-unit equivalence, correct SI unit symbols and capitalization for base/derived units,
KaTeX fields (including notation variants), known source citations, entry-specific source metadata,
exact syllabus codes/titles, the explicit-link allowlist, TIFR/JEST restrictions and related-ID
integrity.

The SI grammar supports base and named derived units, SI prefixes, products separated by spaces or
`*`/`·`, simple quotients with `/`, and integer or fractional powers. `kg` is exact-only; prefixes
are applied to `g`, not `kg`. Rendered units are canonicalized into unit/exponent tokens before
comparison, so `m/s^2` and `m s^-2` compare equal. The grammar intentionally does not support full
mathematical expressions, parentheses, sums, arbitrary non-SI units or general algebra.

## Dates and source status

`TERM_LIBRARY_LAST_REVIEWED` is the library compilation/review date. It is not a per-source
verification date. Source status is recorded separately on each source entry as
`chapter-checked` or `unverified`.

## Scientific review

Every term still needs human review after automated validation. Check the distinction between units
and quantities, dimensional consistency, symbol conventions, SI capitalization, conditions,
notation variants and syllabus relevance. Automated tests cannot establish physics correctness.
