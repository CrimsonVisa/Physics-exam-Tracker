# CSIR-NET Physics Tracker

An offline-first Windows desktop study tracker for CSIR-UGC NET Physical Sciences.

**Phase 3 status:** the dashboard, persistent planning settings, exam countdown, and the
official-source Physical Sciences syllabus tracker are implemented. The syllabus is seeded with 182
topics across General Aptitude and the nine Physical Sciences units, and each topic can store a
personal status, priority, confidence rating, and notes. The study timer, dedicated PYQ and mock-test
tracking, revision scheduling, analytics, and backup/restore are not implemented yet. Navigation
entries for those modules are clearly marked as planned and are not interactive.

## Stack

- [Tauri 2](https://v2.tauri.app/) desktop shell with a Rust backend
- React 19 and TypeScript (strict)
- Vite 8
- Rust-managed SQLite through `rusqlite` (bundled SQLite) with versioned migrations
- Vitest and Testing Library
- ESLint and Prettier
- MIT license

## Prerequisites

- Windows 10 or 11 (64-bit) with the WebView2 runtime
- Node.js 22.12 or newer and npm
- Rust 1.95 or newer with the `x86_64-pc-windows-msvc` toolchain
- Visual Studio with the **Desktop development with C++** workload and a Windows SDK
- The [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for Windows

This project was validated with Node.js 26.7.0, npm 11.19.0, and Rust/Cargo 1.99.1. Rust 1.95 is the
minimum because `rusqlite_migration` 2.x requires it.

### PowerShell notes

If Cargo is installed but not on `PATH`, add it for the current PowerShell session:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
```

Use `npm.cmd` rather than `npm` if PowerShell's execution policy blocks the `npm.ps1` shim.

## Setup

```powershell
git clone https://github.com/CrimsonVisa/csir-net-tracker.git
cd csir-net-tracker
npm.cmd install
```

Both `package-lock.json` and `src-tauri/Cargo.lock` are committed so installs are reproducible.

## Development

Run the desktop app with the Vite dev server and Rust backend:

```powershell
npm.cmd run tauri:dev
```

To run only the frontend in a browser:

```powershell
npm.cmd run dev
```

Tauri commands are unavailable in a plain browser, so the dashboard shows a database-unavailable
error instead of settings. The desktop app is the supported development target.

## Data storage

The database is created outside the repository in Tauri's per-user application-data directory:

```text
%APPDATA%\com.crimsonvisa.csirnetphysics\csir-net-tracker.sqlite3
```

SQLite may also create `-wal` and `-shm` files alongside it. The directory is created on first launch
if needed. The schema is versioned with deterministic, append-only migrations (current schema version:
2), and the app enables foreign keys, WAL journaling, a busy timeout, and transactions for multi-step
writes. Upgrading a version-1 database preserves its saved settings and seeds the catalog without
creating any progress rows.

The initial single-row settings record uses:

- Exam date: `2026-12-19` — **provisional**; verify the actual subject date on the admit card.
- Preparation start date: the local first-run date, clamped to the exam date. No preparation history
  is fabricated.
- Daily study target: `240` minutes (4 hours).

Date-only values are stored as `YYYY-MM-DD` strings and validated in Rust as real calendar dates,
including leap years. The frontend uses one date-only utility and one reusable date input; it never
parses date-only strings through UTC timestamps.

## Syllabus catalog and progress

Migration `0002_syllabus.sql` adds four `STRICT` tables:

- `syllabus_sources` — official document metadata and the date it was checked.
- `syllabus_sections` — the ten units (General Aptitude plus the nine Physical Sciences units), with
  stable codes and display order.
- `syllabus_topics` — 182 catalog entries with stable codes, source references, category, ordering,
  and an archive flag.
- `syllabus_progress` — one optional row per topic containing the user's study record.

Catalog definitions and study progress are deliberately separate. Seeding upserts the built-in
catalog by stable code and never writes to `syllabus_progress`; the initial database therefore
contains no fabricated progress. If a future catalog removes a topic, the topic is archived rather
than deleted, and its existing progress row is preserved. Re-seeding the same catalog is idempotent.

The catalog uses only three categories: **General Aptitude**, **Core**, and **Advanced**. These are
the categories used by the syllabus document. They are distinct from the exam question paper's Parts
A, B, and C; the app labels that distinction explicitly and does not call Core "Part A" or Advanced
"Part B".

### Official sources

The catalog was checked on **29 October 2026** against:

- CSIR-HRDG, **Proposed Syllabus for Physical Sciences** —
  <https://csirhrdg.res.in/SiteContent/ManagedContent/ContentFiles/20201221140202824physicalscience_syllbus.pdf>
  (source page: <https://csirhrdg.res.in/Home/Index/1/Default/3485/78>)
- National Testing Agency, **Joint CSIR-UGC NET December 2026 Information Bulletin** —
  <https://cdnbbsr.s3waas.gov.in/s3efdf562ce2fb0ad460fd8e9d33e57f57/uploads/2026/10/202610071407707951.pdf>
  (source page: <https://csirnet.nta.nic.in/>)

The CSIR PDF's own title, **Proposed Syllabus for Physical Sciences**, is preserved in the app and in
this README. The six General Aptitude entries follow the bulletin's stated emphases: logical
reasoning, graphical analysis, analytical and numerical ability, quantitative comparison, series
formation, and puzzles.

Topic titles were transcribed from these documents with typographic normalization for spacing,
dashes, and symbols. They are not claimed to be a character-perfect OCR reproduction. No topic
weighting or guaranteed exam appearance is implied.

### Recorded progress

Each topic supports:

- Status: Not started, Learning, Revised, or Confident.
- Priority: Low, Normal, High, or High-Yield.
- Confidence: a whole number from 1 to 5.
- Notes: up to 2,000 characters, stored only in the local SQLite database.
- An updated timestamp shown by the editor after the first save.

Priority and **High-Yield** are personal planning labels. They are not official CSIR weightings.
Likewise, the summary percentage is a study-tracking approximation, not a marks prediction or an
official CSIR score. It gives every active topic equal weight and uses these status scores:

| Status      | Score |
| ----------- | ----: |
| Not started |     0 |
| Learning    |   0.5 |
| Revised     |  0.75 |
| Confident   |     1 |

The syllabus screen supports text search (topic title or code), section, category, status, and
priority filters. Search is debounced, archived topics are excluded, and filters are validated in
Rust. The UI includes loading, save-in-progress, validation, retry, and empty states.

## Typography note

Latin Modern is not currently bundled with the app. No font files or font packages were added in
Phase 3; the interface continues to use system fonts (Segoe UI for UI text and Georgia for display
headings). Adopting a self-hosted Latin Modern or comparable math-friendly face remains a UI polish
task for a later phase.

## Checks

```powershell
npm.cmd run typecheck    # strict TypeScript checking
npm.cmd run lint         # ESLint, zero warnings allowed
npm.cmd run format:check # Prettier
npm.cmd run test         # Vitest + Testing Library
npm.cmd run build        # production frontend bundle
npm.cmd run rust:fmt     # cargo fmt --check
npm.cmd run rust:clippy  # Clippy with warnings denied
npm.cmd run rust:test    # Rust unit tests
```

Run all JavaScript/TypeScript linting, formatting, tests, and Rust checks together:

```powershell
npm.cmd run check
```

`format` and `test:coverage` are also available as `npm.cmd run format` and
`npm.cmd run test:coverage`.

## Windows production build

```powershell
npm.cmd run tauri:build
```

The NSIS installer is written to:

```text
src-tauri\target\release\bundle\nsis\
```

The executable and bundle are generated from the product identity in `src-tauri/tauri.conf.json`:

- Product: CSIR-NET Physics Tracker
- Identifier: `com.crimsonvisa.csirnetphysics`
- Version: `0.1.0`

## Project structure

```text
src/                     React frontend and tests
  components/DateField.tsx   reusable accessible date input
  screens/SyllabusScreen.tsx syllabus catalog, filters, and topic editor
  lib/dateOnly.ts            timezone-independent date utilities
  lib/useTodayDateOnly.ts    local-date refresh for long-running windows
  lib/settingsApi.ts         typed settings command wrappers
  lib/syllabusApi.ts         typed syllabus command wrappers
  lib/commandError.ts        shared structured-error decoding
src-tauri/               Tauri configuration and Rust backend
  capabilities/          Tauri window permissions
  icons/                 Application icons
  migrations/            Append-only SQL schema migrations
  src/db.rs              App-data path, connection setup, managed state, migrations
  src/date_only.rs       Authoritative date parsing and formatting
  src/settings.rs        Settings validation, defaults, load, and update
  src/syllabus_catalog.rs Official-source catalog definitions
  src/syllabus.rs        Catalog seeding, search, progress, summaries, and validation
  src/error.rs           Serializable structured command errors
  src/lib.rs             Tauri builder, setup, and typed commands
  src/main.rs            Desktop entry point
index.html               Vite HTML entry
vite.config.ts           Vite and Vitest configuration
```

## License

MIT © 2026 CrimsonVisa. See [LICENSE](LICENSE).
