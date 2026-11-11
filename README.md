# Physics Tracker

[![CI](https://github.com/CrimsonVisa/Physics-exam-Tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/CrimsonVisa/Physics-exam-Tracker/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%2F11%20x64-informational)](docs/DEVELOPMENT.md)
[![Version: 2.0.0](https://img.shields.io/badge/version-2.0.0-informational)](CHANGELOG.md)

Physics Tracker is an offline-first Windows desktop study workspace for four physics
examinations. It combines syllabus progress, focused study sessions, planning, practice,
revision scheduling, reference libraries, notes, analytics, and verified local backups in one
application.

The active application is **Physics Tracker 2.0.0**. It is built with Tauri 2, React 19,
TypeScript, Rust, and SQLite.

> **Release status:** the 2.0.0 public release is being prepared. Release artifacts are built
> from a clean tagged commit and published through GitHub Releases.

## Supported examinations

| Examination                    | Code               |
| ------------------------------ | ------------------ |
| CSIR-UGC NET Physical Sciences | `csir-net-physics` |
| GATE Physics (PH)              | `gate-physics`     |
| TIFR Graduate School Physics   | `tifr-gs-physics`  |
| JEST Physics                   | `jest-physics`     |

Physics Tracker is an independent study tool. It is not affiliated with or endorsed by the
examination authorities.

## Highlights

- **Study workspace:** shared focus timer, pause/resume, session history, manual sessions, and
  per-exam/topic study allocation.
- **Syllabus and concepts:** exam-specific official syllabus trees, progress tracking, source
  attribution, shared concept mappings, and mastery evidence.
- **Reference libraries:** formulas, terms and SI units, built-in physical constants, and
  user-managed formulas, units, and constants with KaTeX-rendered mathematics.
- **Practice:** previous-year questions, mock tests, verified paper patterns, answer keys,
  scoring, and practice analytics.
- **Planning:** checklist tasks, planned study blocks, calendar, exam dates, and rescheduling.
- **Revision:** spaced-repetition schedules, daily plans, deferrals, workload limits, and
  revision history.
- **Flashcards and calculator:** active-recall review plus a scientific calculator with memory,
  angle modes, and session history.
- **Sticky notes:** quick capture, pinning, color coding, search, and optional exam/subject/topic
  associations.
- **Analytics:** study trends, consistency, practice outcomes, syllabus completion, weak topics,
  and accessible chart data tables.
- **Resources and notes:** a local resource catalogue, managed PDF copies, and reference links.
- **Data safety:** verified backup/restore, automatic retention, CSV export, and versioned JSON
  export/import.
- **Accessibility:** keyboard navigation, focus management, reduced-motion support, light/dark
  themes, and adjustable typography.

## Installation

Public installers will be published through
[GitHub Releases](https://github.com/CrimsonVisa/Physics-exam-Tracker/releases).

Release builds produce a per-user NSIS installer:

```text
src-tauri\target\release\bundle\nsis\Physics Tracker_2.0.0_x64-setup.exe
```

The current build configuration is unsigned. Windows SmartScreen may warn until a trusted
code-signing certificate is configured. Verify downloaded artifacts against the published
SHA-256 checksum.

## Build from source

### Requirements

- Windows 10 or Windows 11, 64-bit
- Microsoft Edge WebView2 runtime
- Node.js 22.12 or newer
- Rust 1.95 or newer with the `x86_64-pc-windows-msvc` toolchain
- Visual Studio with the **Desktop development with C++** workload and a Windows SDK

### Commands

```powershell
git clone https://github.com/CrimsonVisa/Physics-exam-Tracker.git
cd Physics-exam-Tracker
npm ci
npm run tauri:dev
```

Production build:

```powershell
npm run tauri:build
```

Quality gate:

```powershell
npm run check
```

The full release checklist is in [docs/RELEASE.md](docs/RELEASE.md).

## Data and privacy

Physics Tracker keeps study data on the local Windows profile:

```text
%APPDATA%\com.crimsonvisa.physics-tracker\physics-tracker.sqlite3
```

SQLite may create `-wal` and `-shm` sidecars alongside the database. The WebView2 profile is
stored separately under:

```text
%LOCALAPPDATA%\com.crimsonvisa.physics-tracker\EBWebView
```

Backups default to:

```text
%APPDATA%\com.crimsonvisa.physics-tracker\backups
```

The active application does **not** open the retired V1 database. V1 data, if present, is kept
separate and untouched. See [docs/PRIVACY.md](docs/PRIVACY.md) for the full privacy statement.

- Core study functionality is offline.
- No telemetry, advertising, or remote account is required.
- Resource discovery and external reference links may use the network when you request them.
- Backup and export files are unencrypted; store them appropriately.
- The application does not access V1 data.

## Repository layout

```text
src/v2/                       Active React application
src-tauri/src/v2/             Active Rust backend, schema upgrades and seeds
src-tauri/src/v2/schema.sql   Current V2 database schema
docs/                         User, developer, privacy, release, and audit docs
tools/schema-validation/      Standalone schema specification and validation script
```

Legacy V1 source and migrations remain in the repository for historical compatibility. They are
not part of the active V2 startup path.

## Documentation

- [Feature guide](docs/FEATURES.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Development](docs/DEVELOPMENT.md)
- [Privacy](docs/PRIVACY.md)
- [Release process](docs/RELEASE.md)
- [Changelog](CHANGELOG.md)
- [2.0.0 release notes](docs/releases/RELEASE_NOTES_2.0.0.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Code of conduct](CODE_OF_CONDUCT.md)
- [Third-party notices](THIRD_PARTY_NOTICES.md)

## Contributing and security

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.
For security issues, follow [SECURITY.md](SECURITY.md) and do not open a public issue containing
vulnerability details, personal data, or database files.

## License

MIT. See [LICENSE](LICENSE). Bundled fonts and third-party dependencies are covered by their own
licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
