# Architecture

Physics Tracker is a single-user Windows desktop application built with Tauri 2. The active
application is the version 2 code under `src/v2` and `src-tauri/src/v2`.

## Runtime layers

```text
React 19 + TypeScript (src/v2)
        │
        │ Tauri IPC commands (`v2_*`)
        ▼
Rust backend (src-tauri/src/v2)
        │
        ├── SQLite database (rusqlite, schema version 5)
        ├── local file and backup operations
        └── resource discovery and external references
```

The frontend does not access SQLite directly. All persistent operations pass through typed Tauri
commands, which validate input, enforce transactions, and return structured errors.

## Application shell

- Entry point: `src/main.tsx`
- Shell: `src/v2/App.tsx`
- Navigation model: `src/v2/navigation.ts`
- Sidebar: `src/v2/V2Sidebar.tsx`
- Theme and typography preferences: `src/lib/uiPreferences.ts`, `src/lib/appearance.ts`

The shell initializes database status, loads the four examination profiles, and renders one of the
dashboard, exam workspace, reference, practice, planning, analytics, resource, backup, notes, or
settings screens.

## Backend modules

| Area                                         | Module                                                  |
| -------------------------------------------- | ------------------------------------------------------- |
| Database initialization and connection state | `src-tauri/src/v2/mod.rs`                               |
| Tauri command layer                          | `src-tauri/src/v2/commands.rs`                          |
| Examination profiles and settings            | `src-tauri/src/v2/admin.rs`                             |
| Study timer and sessions                     | `src-tauri/src/v2/study.rs`                             |
| Syllabus preparation and progress            | `src-tauri/src/v2/syllabus.rs`                          |
| Concepts, mappings, and mastery              | `src-tauri/src/v2/concepts.rs`, `mastery.rs`            |
| Flashcards                                   | `src-tauri/src/v2/flashcards/`                          |
| Practice, PYQs, mocks, and sources           | `src-tauri/src/v2/practice/`                            |
| Reference libraries                          | `src-tauri/src/v2/reference.rs`, `builtin_constants.rs` |
| Analytics and charts                         | `src-tauri/src/v2/analytics.rs`, `analytics_charts.rs`  |
| Planner                                      | `src-tauri/src/v2/planner.rs`                           |
| Revision and daily plans                     | `src-tauri/src/v2/revision.rs`, `revision_plan.rs`      |
| Sticky notes                                 | `src-tauri/src/v2/sticky_notes.rs`                      |
| Resources                                    | `src-tauri/src/v2/resources.rs`                         |
| Backup, restore, and portability             | `src-tauri/src/v2/backup.rs`                            |
| Full-text search                             | `src-tauri/src/v2/search.rs`                            |

## Data model

The current V2 schema is in `src-tauri/src/v2/schema.sql`. Upgrade scripts are applied only to an
existing compatible V2 database. The schema includes:

- examination profiles and syllabus sources/nodes;
- concept mappings, progress, mastery evidence, and replay runs;
- timer state, study sessions, and exam/topic allocations;
- checklist tasks, planned blocks, and calendar data;
- revision schedules, daily plans, deferrals, and history;
- PYQs, mock tests, sources, answer keys, and practice analytics;
- flashcards and review schedules;
- formulas, units, constants, and built-in constant catalogues;
- sticky notes, resources, and file metadata;
- backup settings and portability import batches.

The production database identity is `com.crimsonvisa.physics-tracker`. User data is stored under the
active application-data directory; the retired V1 database is not opened by this application.
Debug builds use the isolated identity `com.crimsonvisa.physics-tracker.dev` for the database,
backups, resources, settings, and WebView2 profile, so `tauri dev` cannot open production storage.

## Timer lifecycle

The timer is persisted in SQLite and derives elapsed time from timestamps, not from a webview
interval alone.

- The frontend timer provider reads and mutates the backend snapshot.
- A heartbeat thread checkpoints a running timer even when the window is minimized.
- Restart, sleep, and stale-heartbeat recovery pause the timer and exclude unverified time.
- Break timers never create study sessions.
- Timer state is the single source of truth for the Study screens and the global timer indicator.

## Backup and portability

- Backups are SQLite snapshots with a manifest containing schema, application version, checksums,
  and database identity.
- Restore operations create a safety copy and validate a temporary candidate before replacing the
  live database.
- CSV exports and versioned JSON exports are explicit user actions.
- Backup and export files are not encrypted by the application.

## Frontend styling

The UI is split across the shared design layer and feature styles:

- `src/styles.css` — active global base stylesheet imported by `src/main.tsx`; it contains shared base
  rules and retained legacy V1 rules. A future split should separate active/shared styles from
  V1-only selectors;
- `src/v2/v2.css` — V2 screen and component styles;
- `src/v2/design-system.css` — V2 tokens and shared controls;
- `src/v2/workspaces.css` — workspace and exam identity rules;
- feature styles such as `charts.css`, `flashcards.css`, `formulas.css`, `terms.css`,
  `reference.css`, `notes.css`, `calculator.css`, and `global-search.css`.

All application colors, typography, and surfaces are local CSS; there are no remote UI assets.

## Security boundaries

- Tauri capabilities are defined in `src-tauri/capabilities/default.json`.
- The production Content Security Policy restricts scripts, styles, images, and IPC access.
- External URLs are opened through the system opener only on explicit user action.
- Local managed resources are opened from the application's managed resource folder.
- No telemetry, analytics SDK, advertising, or remote account is integrated.
- The application currently has no updater and no code-signing configuration.

## Legacy V1 material

The active V2 frontend entry is `src/main.tsx`, which renders `src/v2/App.tsx`. The retained V1
frontend (`src/App.tsx`, `src/screens/`, and most of `src/components/` and `src/lib/`) is not loaded
by that entry point. Active V2 imports only `BrandMark.tsx`, `NavIcon.tsx`, and `ThemeSwitch.tsx`
from `src/components`, plus the shared utilities `appearance`, `appShortcuts`, `commandError`,
`dateOnly`, `navigation`, `studyTime`, `timerEvents`, `uiPreferences`, and `useTodayDateOnly` from
`src/lib`.

The active backend is `src-tauri/src/v2`. `src-tauri/src/main.rs` is the active binary entry point
calling `csir_net_tracker_lib::run()`, and `src-tauri/src/lib.rs` contains the Tauri setup and
registers only `v2_*` commands. `error.rs` and `app_paths.rs` provide shared error and path helpers.
`performance_audit.rs` is a test-only, ignored performance audit, not V1 runtime code. The remaining
root Rust modules are marked `#[allow(dead_code)]` and retained as V1 code; their commands are not
registered in the V2 `tauri::generate_handler!` list. V1 migrations remain under
`src-tauri/migrations`.

V1 source code and migrations remain in the repository for historical compatibility. V1 application
data is kept separate and is not opened by the V2 shell. Do not delete V1 code or migrations without
a separate migration and product decision.
