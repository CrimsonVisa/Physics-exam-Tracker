# Bundled Computer Modern Unicode fonts

These fonts are bundled locally so the application has consistent LaTeX-style typography without
any network request.

## Sources

- **Computer Modern Unicode (CMU) 0.7.0**, converted from the original Computer Modern METAFONT
  sources by Andrey V. Panov:
  <https://cm-unicode.sourceforge.io/>
- The OpenType files are distributed through CTAN:
  <https://ctan.org/pkg/cm-unicode>

The font files are unmodified. Only the serif, sans-serif, and typewriter variants used by the
interface are included.

## Included files

- CMU Serif: Roman, Bold Extended, Italic, Bold Extended Italic
- CMU Sans Serif: Medium, Bold Extended, Oblique, Bold Extended Oblique
- CMU Typewriter Text: Regular, Bold, Bold Italic

## License

Computer Modern Unicode fonts are released under the **SIL Open Font License, Version 1.1 (OFL)**.
The license permits bundling, embedding, and redistribution with software when the copyright and
license notices are preserved. The full text is in [`OFL-1.1.txt`](OFL-1.1.txt).

The Tauri installer maps [`public/fonts/OFL-1.1.txt`](../../../public/fonts/OFL-1.1.txt) to
`legal/fonts/CMU-OFL-1.1.txt` inside the installed application bundle. `legal/fonts/` is not a
directory in the source repository. This source-tree copy is retained for documentation and audit
traceability; both copies are byte-identical.

Copyright (C) original METAFONT authors, including Donald Ervin Knuth and the EC, LH, CB Greek,
TIPA, Vietnamese, and CMBright contributors; Unicode adaptation Copyright (C) 2003-2009
Andrey V. Panov, with Reserved Font Family Name "Computer Modern Unicode fonts".

The fonts are distributed without warranty.
