# Third-Party Notices

Physics Tracker application code is released under the MIT license. The project bundles or depends
on the following third-party assets and libraries under their respective licenses.

This file is a summary. The authoritative license text for each dependency is distributed with that
dependency and its package metadata. Dependency versions are pinned by `package-lock.json` and
`src-tauri/Cargo.lock`.

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

## Branding and examination content

The Physics Tracker PH monogram and application icons are original project assets. The application
does not bundle examination-authority logos. Examination names and syllabus references are used for
identification and study organization; they remain the property of their respective authorities.
