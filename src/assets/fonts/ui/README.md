# Bundled UI fonts

All UI fonts are bundled locally so the application never depends on fonts installed on the
operating system and never makes a network request.

## Families and sources

All families are licensed under the SIL Open Font License 1.1 (OFL). The original `OFL.txt`
licence text is included next to each font file.

| Family                              | File                                            | Source                                                          |
| ----------------------------------- | ----------------------------------------------- | --------------------------------------------------------------- |
| Inter (variable, opsz/wght)         | `inter/InterVariable.ttf`                       | <https://github.com/google/fonts/tree/main/ofl/inter>           |
| Manrope (variable, wght)            | `manrope/ManropeVariable.ttf`                   | <https://github.com/google/fonts/tree/main/ofl/manrope>         |
| IBM Plex Sans (variable, wdth/wght) | `ibm-plex-sans/IBMPlexSansVariable.ttf`         | <https://github.com/google/fonts/tree/main/ofl/ibmplexsans>     |
| Plus Jakarta Sans (variable, wght)  | `plus-jakarta-sans/PlusJakartaSansVariable.ttf` | <https://github.com/google/fonts/tree/main/ofl/plusjakartasans> |
| JetBrains Mono (variable, wght)     | `jetbrains-mono/JetBrainsMonoVariable.ttf`      | <https://github.com/google/fonts/tree/main/ofl/jetbrainsmono>   |

The files are unmodified Google Fonts repository builds (TrueType variable fonts).

## Usage

- **Inter** is the default interface and display family.
- **Manrope**, **IBM Plex Sans** and **Plus Jakarta Sans** are optional interface families
  selectable in Settings → Appearance. The choice is stored as a local UI preference.
- **JetBrains Mono** renders scientific/numeric values (timer, scores, durations, identifiers).
- **CMU Serif** (in `src/assets/fonts/`) remains the mathematical-notation family so equations
  keep their LaTeX-style appearance.
