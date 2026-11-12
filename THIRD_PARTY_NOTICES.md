# Third-Party Notices

Physics Tracker application code is released under the MIT license. The project bundles or depends
on the following third-party assets and libraries under their respective licenses. The project's MIT
license does not replace those third-party licenses.

This file is a summary. The authoritative license text for each dependency is distributed with that
dependency and its package metadata. Dependency versions are pinned by `package-lock.json` and
`src-tauri/Cargo.lock`. Before distributing an installer, include the applicable third-party license
texts and notices with the distribution.

## Bundled fonts

All bundled fonts are distributed under the SIL Open Font License, Version 1.1. The font files are
used locally and are not downloaded at runtime.

| Font family                   | Source                               | License/text                                               |
| ----------------------------- | ------------------------------------ | ---------------------------------------------------------- |
| Computer Modern Unicode 0.7.0 | <https://cm-unicode.sourceforge.io/> | `src/assets/fonts/OFL-1.1.txt`, `public/fonts/OFL-1.1.txt` |
| Inter                         | Google Fonts                         | `src/assets/fonts/ui/inter/OFL.txt`                        |
| Manrope                       | Google Fonts                         | `src/assets/fonts/ui/manrope/OFL.txt`                      |
| IBM Plex Sans                 | Google Fonts                         | `src/assets/fonts/ui/ibm-plex-sans/OFL.txt`                |
| Plus Jakarta Sans             | Google Fonts                         | `src/assets/fonts/ui/plus-jakarta-sans/OFL.txt`            |
| JetBrains Mono                | Google Fonts                         | `src/assets/fonts/ui/jetbrains-mono/OFL.txt`               |

Additional attribution and source details are in [src/assets/fonts/README.md](src/assets/fonts/README.md)
and [src/assets/fonts/ui/README.md](src/assets/fonts/ui/README.md).

## JavaScript runtime dependencies

| Package                           | License           |
| --------------------------------- | ----------------- |
| `@tauri-apps/api`                 | Apache-2.0 OR MIT |
| `@tauri-apps/plugin-dialog`       | MIT OR Apache-2.0 |
| `@tauri-apps/plugin-notification` | MIT OR Apache-2.0 |
| `@tauri-apps/plugin-opener`       | MIT OR Apache-2.0 |
| `katex`                           | MIT               |
| `react`                           | MIT               |
| `react-dom`                       | MIT               |

## Rust runtime and build dependencies

| Crate                       | License           |
| --------------------------- | ----------------- |
| `chrono`                    | MIT OR Apache-2.0 |
| `regex`                     | MIT OR Apache-2.0 |
| `reqwest`                   | MIT OR Apache-2.0 |
| `rusqlite`                  | MIT               |
| `rusqlite_migration`        | Apache-2.0        |
| `scraper`                   | ISC               |
| `serde`                     | MIT OR Apache-2.0 |
| `serde_json`                | MIT OR Apache-2.0 |
| `sha2`                      | MIT OR Apache-2.0 |
| `tauri`                     | Apache-2.0 OR MIT |
| `tauri-build`               | Apache-2.0 OR MIT |
| `tauri-plugin-dialog`       | Apache-2.0 OR MIT |
| `tauri-plugin-notification` | Apache-2.0 OR MIT |
| `tauri-plugin-opener`       | Apache-2.0 OR MIT |
| `tempfile`                  | MIT OR Apache-2.0 |
| `url`                       | MIT OR Apache-2.0 |
| `uuid`                      | Apache-2.0 OR MIT |

## Additional third-party components

### MPL-2.0 Rust crates

These transitive Rust dependencies are licensed under MPL-2.0:

- `cssparser` 0.35.0 and 0.37.0 — <https://github.com/servo/rust-cssparser>
- `cssparser-macros` 0.6.1 and 0.7.1 — <https://github.com/servo/rust-cssparser>
- `dtoa-short` 0.3.5 — <https://github.com/upsuper/dtoa-short>
- `option-ext` 0.2.0 — <https://github.com/soc/option-ext>
- `selectors` 0.31.0 and 0.38.0 — <https://github.com/servo/stylo>

Preserve the MPL-2.0 notices, include the MPL-2.0 license text with binary distributions, and make
the corresponding source available. The audited `selectors` packages do not include a standalone
license file; their source files carry the standard MPL header and their Cargo metadata declares
MPL-2.0.

### Build-time-only npm components

The following packages are used by the development/build toolchain and are not part of the installed
application. If the toolchain itself is redistributed, preserve their notices:

- `lightningcss` and platform packages — MPL-2.0
- `caniuse-lite` — CC-BY-4.0, attribution required
- `mdn-data` — CC0-1.0
- `lru-cache` and `minimatch` — BlueOak-1.0.0

TypeScript is Apache-2.0 and includes `ThirdPartyNoticeText.txt`; it is a build-time dependency.

### Other notices

- `webpki-root-certs` — CDLA-Permissive-2.0; include its license and attribution if shipped.
- Tauri NSIS installer template — `src-tauri/nsis/installer.nsi` is derived from Tauri's NSIS
  template. Tauri is dual-licensed MIT/Apache-2.0; preserve the applicable Tauri license notices
  when distributing an installer.
- Apache-2.0 dependencies may include separate NOTICE files. Preserve them where present; no Cargo
  NOTICE files were found in the audited package set.

## Reference data and examination content

Constants and formula entries include citations to sources such as CODATA/NIST, the BIPM SI
brochure, NIST DLMF, the Particle Data Group, and physics textbooks. Those citations identify
sources; they do not by themselves grant redistribution rights.

The Physics Tracker PH monogram and application icons are original project assets. The application
does not bundle examination-authority logos. Examination names and syllabus references are used for
identification and study organization; they remain the property of their respective authorities.
