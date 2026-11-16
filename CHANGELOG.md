# Changelog

All notable changes to Physics Tracker are recorded here. The project follows semantic versioning
for the application release line.

## [2.1.0] - 2026-11-17

Physics Tracker 2.1 is a quality and consistency release for the four-examination experience. See
the [v2.1.0 release notes](docs/releases/RELEASE_NOTES_2.1.0.md).

### Added

- Exam countdown section for active examinations, including upcoming, exam-day, passed, and
  date-not-set states.
- Native Windows notifications when a focus timer completes, with duplicate-notification
  suppression and the existing notification setting respected.
- Multi-exam session associations and shared topic allocations, crediting each associated
  examination with the full session duration while counting overall time once.
- Managed PDF open, attach, relink, replace, missing-file detection, and removal workflows.
- Reference-library detail and editor views for formulas, built-in constants, custom constants,
  units, and conversion summaries.

### Changed

- Reference tabs now load their libraries on demand while preserving tab state.
- Reference editor dialogs keep their headers and primary actions fixed around a scrollable body.
- KaTeX and mixed-text rendering were improved for formulas, constants, units, notes, and
  descriptions.
- Dashboard summaries and section rendering were refined, including exam-date countdowns and
  consistent once-per-session study totals.
- Revision scheduling now exposes due work, daily plans, deferrals, completion history, and
  adaptive intervals more clearly.
- Timer completion and recovery paths keep the displayed, persisted, and credited examination sets
  aligned.

### Fixed

- Removing an examination association now removes only that examination's topic allocations while
  preserving retained topics, session duration, and overall study time.
- Retained historical exam associations preserve their exact credited seconds across repeated
  edits; new active associations receive the full session duration and inactive exams receive no
  new credit.
- Running timers reject topic allocations outside the persisted examination selection in both the
  frontend and backend.
- Deactivated examinations are excluded from running timer snapshots and completion credit without
  mutating historical session allocations.
- Legacy in-progress timer selection backfill is idempotent and no longer resurrects deliberately
  removed associations.

### Security

- The application remains local-first with no telemetry or remote account.
- Backup and export files remain unencrypted and must be protected by the user.
- Managed PDF copies are validated locally; SQLite backups still exclude PDF bytes.

## [2.0.0] - 2026-11-14

Initial Physics Tracker 2.0 release. See the
[v2.0.0 release notes](docs/releases/RELEASE_NOTES_2.0.0.md).

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
- Backup and export files are unencrypted.
- SQLite backups do not include managed PDF/resource files; those files must be preserved
  separately.

## [1.2.0] - 2026-10-31

Historical V1 release line. See
[docs/releases/RELEASE_NOTES_1.2.0.md](docs/releases/RELEASE_NOTES_1.2.0.md).

## [1.1.0] - 2026-10-30

Historical V1 release line. See
[docs/releases/RELEASE_NOTES_1.1.0.md](docs/releases/RELEASE_NOTES_1.1.0.md).

## [1.0.0] - 2026-10-30

Historical V1 release line. See
[docs/releases/RELEASE_NOTES_1.0.0.md](docs/releases/RELEASE_NOTES_1.0.0.md).
