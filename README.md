# CSIR-NET Physics Tracker

An offline-first Windows desktop study tracker for CSIR-UGC NET Physical Sciences.

**Phase 2 status:** the dashboard, persistent planning settings, and exam countdown are implemented.
The study timer, syllabus checklist, PYQ and mock-test tracking, revisions, and backup/restore are not
implemented yet. Navigation entries for those modules are clearly marked as planned and are not
interactive.

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
if needed. The schema is versioned with deterministic migrations, and the app enables foreign keys,
WAL journaling, a busy timeout, and transactions for multi-step writes.

The initial single-row settings record uses:

- Exam date: `2026-12-19` — **provisional**; verify the actual subject date on the admit card.
- Preparation start date: the local first-run date, clamped to the exam date. No preparation history
  is fabricated.
- Daily study target: `240` minutes (4 hours).

Date-only values are stored as `YYYY-MM-DD` strings and validated in Rust as real calendar dates,
including leap years. The frontend uses one date-only utility and one reusable date input; it never
parses date-only strings through UTC timestamps.

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
  lib/dateOnly.ts            timezone-independent date utilities
  lib/useTodayDateOnly.ts    local-date refresh for long-running windows
  lib/settingsApi.ts         typed Tauri command wrappers
src-tauri/               Tauri configuration and Rust backend
  capabilities/          Tauri window permissions
  icons/                 Application icons
  migrations/            Versioned SQL schema migrations
  src/db.rs              App-data path, connection setup, managed state, migrations
  src/date_only.rs       Authoritative date parsing and formatting
  src/settings.rs        Settings validation, defaults, load, and update
  src/error.rs           Serializable structured command errors
  src/lib.rs             Tauri builder, setup, and typed commands
  src/main.rs            Desktop entry point
index.html               Vite HTML entry
vite.config.ts           Vite and Vitest configuration
```

## License

MIT © 2026 CrimsonVisa. See [LICENSE](LICENSE).
