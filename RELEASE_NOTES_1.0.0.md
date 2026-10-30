# CSIR-NET Physics Tracker 1.0.0 — Release Notes

Release date: 30 October 2026

## Overview

CSIR-NET Physics Tracker 1.0.0 is the first stable Windows release of the offline-first study tracker
for CSIR-UGC NET Physical Sciences.

The application provides one local workspace for syllabus progress, focused study sessions, planning,
PYQ practice, mock tests, revision scheduling, analytics, and verified data portability.

## Highlights

- Official-source Physical Sciences syllabus catalog with 182 topics.
- Persistent focus timer with restart/sleep recovery and append-only study sessions.
- Checklist tasks, planned study blocks, month/week/day calendar, and exam-anchored rescheduling.
- PYQ bank and mock-test history with separate performance analytics.
- Configurable spaced-repetition revision scheduler with append-only outcome history.
- Preparation dashboard with study streaks, evidence-backed insights, and custom SQLite-backed layout.
- Study, performance, topic-intelligence, and timeline analytics.
- Verified full-database backup and restore with manifests, checksums, WAL-safe snapshots, automatic
  retention, and safety backups.
- Ten CSV dataset exports and complete versioned JSON export/import.
- Repeat-safe JSON merge batches that prevent the same export from being appended twice.
- Long-running backup, restore, export, and import work runs off the UI thread.
- Accessible focus states, skip-to-content navigation, labelled forms, and offline-only assets.

## Exam date

The default exam date of **19 December 2026** is provisional. It is not presented as an official
confirmed examination date. Users can change it in Overview settings, and the countdown uses the saved
value. Date changes do not happen silently.

## Data compatibility

- Database schema version: **9**
- Upgrades from earlier schemas run through append-only migrations.
- A verified pre-migration snapshot is created first. If it fails, migration is aborted.
- Older compatible SQLite backups upgrade through the normal migration process after restore.
- JSON exports from schema 8 or 9 are accepted. New exports include an import batch identifier.
- Existing study records, settings, dashboard layout, timer state, and revision history are preserved.

## Backup warning

Backup and export files are **not encrypted**. They contain sensitive personal study data. Store them
in a private folder or encrypted removable drive. A backup requires its adjacent
`.sqlite3.manifest.json` file for restore.

## Release artifacts

- Installer: `CSIR-NET Physics Tracker_1.0.0_x64-setup.exe`
- Installer size: 3,748,081 bytes (3.57 MiB)
- Installer SHA-256: `DB7E552020A21F11D6405B902F5F661B0078F80889778F0FE11C4408B7187259`
- Standalone executable SHA-256: `A5045544EF3C6593D57191A4B00B7DD68AEDFBDF64031A7B54DB1D2960B19A98`
- Installer mode: current user, no Administrator access required
- Install location: `%LOCALAPPDATA%\CSIR-NET Physics Tracker`

## Verification summary

- TypeScript, ESLint with zero warnings, Prettier, production frontend build, Rustfmt, Clippy with
  warnings denied, and Rust tests pass.
- Test results: 176 frontend tests and 149 Rust tests passed; 1 synthetic performance test is ignored
  by default and was run separately.
- Production Tauri release build completed and the executable reports version 1.0.0.
- Release executable launched successfully.
- NSIS silent install, launch, uninstall, reinstall, second launch, and second uninstall completed.
- The existing schema-9 database, settings, 182-topic catalog, and 9-section dashboard layout survived
  install, uninstall, and reinstall.
- The real local database was upgraded from schema 8 to 9 with all configured dates and records intact;
  a verified pre-migration schema-8 snapshot was created automatically.
- Fresh first-run initialization created a schema-9 database with provisional default settings, 182
  topics, and 9 dashboard sections in an isolated empty data profile.
- A separate physical clean Windows VM was not available; fresh-install verification is
  application-level plus installer install/uninstall, not a second-machine test.

## Performance audit

An ignored Rust audit creates a temporary database with synthetic data only: 2,000 study sessions,
2,000 PYQ attempts, 500 tasks, 200 planned blocks, 100 mock tests, and 100 revision schedules across
182 topics. It never touches the real user database. Representative debug-build timings on the
development machine were:

- Preparation dashboard: about 28 ms
- Study analytics: about 14 ms
- Performance analytics: about 17 ms
- Filtered PYQ search: about 10 ms
- Calendar month: about 13 ms
- Revision dashboard: about 5 ms
- Complete JSON export: about 518 ms for 2.3 MB
- Verified SQLite backup: about 107 ms for 1.3 MB
- History load of 2,000 sessions: about 14 ms

These are local synthetic measurements on a debug build, not release-mode guarantees. No production
data was used.

## Known limitations

- No built-in encryption or password protection for backups/exports.
- Automatic backups run only while the app is running, not as a Windows background service.
- Cancellation is not exposed for database replacement/restore; safety snapshots and rollback protect
  against failure.
- JSON merge does not import missing syllabus catalog topics or overwrite installation-local backup,
  timer, revision, or dashboard configuration.
- Mapped network drives cannot always be distinguished from local drives; use a genuinely local or
  removable-drive folder.
- Restore requires the adjacent manifest.
- No cloud synchronization or multi-device merge service.
