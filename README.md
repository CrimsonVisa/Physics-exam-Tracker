# Physics Tracker

[![CI](https://github.com/CrimsonVisa/Physics-exam-Tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/CrimsonVisa/Physics-exam-Tracker/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%2F11%20x64-informational)](docs/DEVELOPMENT.md)
[![Version: 2.0.0](https://img.shields.io/badge/version-2.0.0-informational)](CHANGELOG.md)

Physics Tracker is a local-first Windows desktop study app for four physics entrance examinations.
It keeps syllabus progress, focused study sessions, planning, practice, revision, reference
material, notes, and analytics in one place.

Use it to decide what to study next, record focused sessions, practise previous-year questions, and
see which topics need another pass before the exam. The current application version is **2.0.0**.

Physics Tracker is an independent study tool. It is not affiliated with or endorsed by any
examination authority.

## Supported examinations

| Examination                    | Code               |
| ------------------------------ | ------------------ |
| CSIR-UGC NET Physical Sciences | `csir-net-physics` |
| GATE Physics (PH)              | `gate-physics`     |
| JEST Physics                   | `jest-physics`     |
| TIFR GS Physics                | `tifr-gs-physics`  |

Each examination has its own workspace, while notes, reference material, and the local database are
shared across the application.

## Key features

### Plan and focus

- Combined dashboard with configurable sections, today's study summary, and four exam cards.
- Focus, short-break, and long-break timers with pause/resume, recovery after sleep or restart, and
  session history.
- Manual study sessions and time allocation across examinations and syllabus topics.
- Checklist tasks, planned study blocks, calendar views, exam dates, and rescheduling.
- Spaced-repetition revision schedules, daily revision plans, workload limits, and deferrals.

### Work through the syllabus

- Syllabus trees based on official exam sources, with source and version metadata.
- Topic progress, priority, confidence, notes, search, and completion filters.
- Shared concept library with exam mappings, mastery evidence, and review history.
- Clear separation between shared concept mastery and exam-specific syllabus completion.

### Practise and revise

- Previous-year question bank with source, year, section, topic, and verification metadata.
- Question-level practice with MCQ, MSQ, and numeric-answer support.
- Mock-test workflows with paper patterns, answer keys, scoring, and practice analytics.
- Flashcards with review scheduling, active recall, due queues, and retention history.
- Scientific calculator with memory, previous-answer reuse, angle modes, keyboard support, and
  session history.

### Reference material and notes

- Formula reference with equations, symbols, assumptions, conventions, and syllabus links.
- Terms and SI units reference with aliases, notation, dimensions, and conversion information.
- Built-in physical constants with values, uncertainty, precision, and source metadata.
- User-managed formulas, units, constants, and searchable local notes.
- Sticky notes with quick capture, pinning, colours, search, and optional exam/topic links.
- Local resource catalogue with external links, study progress, and managed PDF copies.

### See progress and protect your data

- Study trends, consistency, practice outcomes, syllabus completion, and weak-topic summaries.
- Accessible charts with equivalent data tables.
- Verified SQLite backup and restore with manifests and checksums.
- Automatic backup retention, CSV exports, and versioned JSON export/import.
- Dark and light themes, adjustable typography, keyboard navigation, focus states, and reduced-motion
  support.

## Screenshots

> **Screenshots pending.** No genuine application screenshots are stored in this repository yet.
> Verified screenshots captured with disposable study data are still needed. No fabricated or generic
> application images are included.

Suggested future captures: the combined dashboard, study timer, syllabus workspace, practice
analytics, and revision planner.

## Installation

### Download

No public 2.0.0 installer has been published yet. When the 2.0.0 release is published, use the
per-user NSIS installer from
[GitHub Releases](https://github.com/CrimsonVisa/Physics-exam-Tracker/releases) and verify the
published SHA-256 checksum before running it.

The installer targets Windows 10 or Windows 11, 64-bit, and requires the Microsoft Edge WebView2
runtime. The current build configuration is unsigned, so Windows SmartScreen may display a warning.

If WebView2 is not already installed, the installer uses Microsoft's download bootstrapper, so an
internet connection is required for that step. This build does not include an offline WebView2
installer.

### License and notices

The NSIS installer displays the MIT license before installation. The installed application includes
the MIT license at `%LOCALAPPDATA%\Physics Tracker\legal\LICENSE`, the third-party notice summary at
`%LOCALAPPDATA%\Physics Tracker\legal\THIRD_PARTY_NOTICES.md`, the complete package coverage report at
`%LOCALAPPDATA%\Physics Tracker\legal\THIRD_PARTY_COVERAGE.md`, and the full license texts and font
notices under `%LOCALAPPDATA%\Physics Tracker\legal\licenses\`, `legal\notices\`, and `legal\fonts\`.

### Upgrading from 1.x

Physics Tracker 2.x uses the application identity `com.crimsonvisa.physics-tracker` and a separate
data directory. Existing Physics Tracker 1.x data is kept under its legacy
`com.crimsonvisa.csirnetphysics` identity and is **not imported automatically** into 2.x. Installing
2.x does not delete the 1.x data. The two versions can coexist; retain the 1.x installation or a copy
of its data if you need those records.

### Build from source

Source builds require:

- Windows 10 or Windows 11, 64-bit
- Microsoft Edge WebView2 runtime
- Node.js 22.12 or newer
- Rust 1.95 or newer with the `x86_64-pc-windows-msvc` toolchain
- Visual Studio with the **Desktop development with C++** workload and a Windows SDK

```powershell
git clone https://github.com/CrimsonVisa/Physics-exam-Tracker.git
cd Physics-exam-Tracker
npm ci
npm run tauri:dev
```

For a production build:

```powershell
npm run tauri:build
```

If PowerShell blocks the npm shim, use `npm.cmd` instead of `npm`. The full development and release
instructions are in [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) and [docs/RELEASE.md](docs/RELEASE.md).

## Data storage and privacy

Study data stays on the local Windows profile:

```text
%APPDATA%\com.crimsonvisa.physics-tracker\physics-tracker.sqlite3
```

SQLite may create `-wal` and `-shm` sidecars next to the database. The WebView2 profile is stored
separately at:

```text
%LOCALAPPDATA%\com.crimsonvisa.physics-tracker\EBWebView
```

Backups default to:

```text
%APPDATA%\com.crimsonvisa.physics-tracker\backups
```

Core study features work offline. The application may use the network when you ask it to discover
resources, open an official source, or follow an external reference link.

- No account, telemetry, advertising, or remote study-data upload is required.
- Backup and export files are unencrypted; store them somewhere you control.
- The active application does not open the retired V1 database.
- 2.x does not automatically import or delete 1.x data.

See [docs/PRIVACY.md](docs/PRIVACY.md) for the full privacy statement.

## Technology stack

- **Desktop shell:** Tauri 2
- **Frontend:** React 19, TypeScript, Vite
- **Backend:** Rust
- **Storage:** SQLite through `rusqlite`
- **Mathematics:** KaTeX
- **Testing:** Vitest and Rust tests

## Development setup

Install the prerequisites listed under [Build from source](#build-from-source), then run:

```powershell
npm.cmd ci
npm.cmd run tauri:dev
```

Useful commands:

```powershell
npm.cmd run dev          # Vite frontend only; Tauri commands are unavailable in a browser
npm.cmd run check        # TypeScript, ESLint, Prettier, frontend tests, Rust format, Clippy, Rust tests
npm.cmd run tauri:build  # Production desktop build
```

Development rules and data-safety guidance are in [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).
Do not use a live user database, personal study data, or exam PDFs during development.

## Project structure

```text
src/main.tsx                  Active V2 frontend entry point
src/v2/                       Active React application
src/components/               Shared active components plus retained V1 components
src/lib/                      Shared active utilities plus retained V1 API wrappers
src-tauri/src/v2/             Active Rust backend, schema, and seed data
src-tauri/src/*.rs            Tauri entry, shared helpers, and retained V1 Rust modules
src-tauri/capabilities/       Tauri permissions
src-tauri/nsis/               Installer template
legal/                        MIT and third-party licence bundle
docs/                         Architecture, features, privacy, development, and release docs
tools/schema-validation/      Historical initial-schema specification and validator
```

The active V2 frontend is under `src/v2`, and `src-tauri/src/v2` is the active Rust backend. The
`src/components` and `src/lib` folders contain shared modules used by the active shell together with
retained V1 components and API wrappers.

Legacy V1 frontend, backend, and migrations remain in the repository for historical compatibility.
They are not part of the active V2 startup path; do not delete them without a separate migration and
product decision.

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

- Report security issues through [SECURITY.md](SECURITY.md), not a public issue.
- Follow the [Code of Conduct](CODE_OF_CONDUCT.md).
- Do not include databases, backups, personal study records, exam PDFs, or secrets in issues or pull
  requests.
- Run `npm.cmd run check` before requesting review.

## License

Physics Tracker is released under the **MIT License**. See [LICENSE](LICENSE).

Bundled fonts and third-party dependencies remain under their own licenses; see
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
