# Changelog

All notable changes to Physics Tracker are recorded here. The project follows semantic versioning
for the application release line.

## [2.0.0] - 2026-11-11

Initial Physics Tracker 2.0 public release preparation.

### Added

- Four independent examination workspaces: CSIR-UGC NET, GATE Physics, TIFR GS, and JEST.
- V2 SQLite data layer with exam profiles, syllabus sources, concept mappings, mastery evidence,
  timer/session allocations, revision plans, practice data, reference libraries, notes, resources,
  and backup metadata.
- Combined dashboard with configurable section visibility and order.
- Global study timer with focus/break modes, checkpoint recovery, session history, and allocations.
- Official-source syllabus trees, progress tracking, concept search, and mastery evidence.
- Formula, term, unit, and built-in physical-constant reference libraries with KaTeX equations.
- PYQ practice, verified mock-test patterns, answer keys, scoring, and practice analytics.
- Flashcards with review scheduling and active-recall review.
- Scientific calculator with memory, angle modes, keyboard support, and session history.
- Checklist, planned study blocks, calendar, exam-date rescheduling, and revision planning.
- Sticky notes with quick access, autosave, pinning, colors, and syllabus associations.
- Resource catalogue with managed local PDF copies and progress tracking.
- Accessible SVG analytics charts with equivalent data tables.
- Verified backup/restore, automatic retention, CSV export, and versioned JSON portability.
- Dark/light themes, selectable typography, focus management, reduced-motion support, and accessible
  dialogs.

### Changed

- Public product presentation is now **Physics Tracker**; the internal application version is
  `2.0.0` and the identifier is `com.crimsonvisa.physics-tracker`.
- Active user data is stored under the Physics Tracker application identity and kept separate from
  retired V1 data.

### Security

- Local-first storage with no telemetry or remote account.
- Tauri capabilities and CSP are configured for the desktop application.
- Backup and export files remain unencrypted and must be protected by the user.

### Known limitations

- Windows-only; the installer targets NSIS per-user installation.
- Builds are currently unsigned.
- No automatic updater is configured.
- Public screenshots, code signing, and release artifacts are still being prepared.

## [1.2.0] - 2026-10-31

Historical V1 release line. See
[docs/releases/RELEASE_NOTES_1.2.0.md](docs/releases/RELEASE_NOTES_1.2.0.md).

## [1.1.0] - 2026-10-30

Historical V1 release line. See
[docs/releases/RELEASE_NOTES_1.1.0.md](docs/releases/RELEASE_NOTES_1.1.0.md).

## [1.0.0] - 2026-10-30

Historical V1 release line. See
[docs/releases/RELEASE_NOTES_1.0.0.md](docs/releases/RELEASE_NOTES_1.0.0.md).
