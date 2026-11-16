# Development Guide

## Prerequisites

- Windows 10 or Windows 11, 64-bit
- Node.js 22.12 or newer
- Rust 1.95 or newer with the `x86_64-pc-windows-msvc` toolchain
- Visual Studio with the **Desktop development with C++** workload and a Windows SDK
- Microsoft Edge WebView2 runtime

The project was validated with Node.js 26.7.0, npm 11.19.0, and Rust/Cargo 1.99.1. The declared
minimums are Node.js 22.12 and Rust 1.95.

## Setup

```powershell
npm.cmd ci
```

Use `npm.cmd` rather than `npm` if PowerShell's execution policy blocks the `npm.ps1` shim.

## Run the desktop application

```powershell
npm.cmd run tauri:dev
```

The Vite frontend alone can be run with:

```powershell
npm.cmd run dev
```

Tauri commands are unavailable in a plain browser. The desktop application is the supported
development target.

## Development data isolation

`npm run tauri:dev` uses `src-tauri/tauri.dev.conf.json` and a dedicated development identity:

```text
com.crimsonvisa.physics-tracker.dev
```

Debug builds resolve all writable application storage beneath that identity. Release builds keep
the existing production identity and locations unchanged.

| Storage                          | Development (`tauri dev`)                                               | Production / release                                                |
| -------------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------------- |
| SQLite database                  | `%APPDATA%\com.crimsonvisa.physics-tracker.dev\physics-tracker.sqlite3` | `%APPDATA%\com.crimsonvisa.physics-tracker\physics-tracker.sqlite3` |
| SQLite WAL/SHM                   | Next to the development database                                        | Next to the production database                                     |
| Backups and recovery files       | `%APPDATA%\com.crimsonvisa.physics-tracker.dev\backups`                 | `%APPDATA%\com.crimsonvisa.physics-tracker\backups`                 |
| Managed resources and PDFs       | `%APPDATA%\com.crimsonvisa.physics-tracker.dev\resources`               | `%APPDATA%\com.crimsonvisa.physics-tracker\resources`               |
| Settings                         | Development SQLite database                                             | Production SQLite database                                          |
| WebView2 profile and web storage | `%LOCALAPPDATA%\com.crimsonvisa.physics-tracker.dev\EBWebView`          | `%LOCALAPPDATA%\com.crimsonvisa.physics-tracker\EBWebView`          |

The debug/release separation is compile-time (`cfg(debug_assertions)`) and is verified before the
database is opened. Starting a debug build with the production Tauri identifier fails safely instead
of opening production storage. Do not run `tauri dev` without the development configuration file.

Development backups may not be configured inside the production V2 application-data folder. Exported
CSV/JSON files and manually selected backup destinations are explicit user actions, not application
storage.

## Quality checks

```powershell
npm.cmd run typecheck
npm.cmd run lint
npm.cmd run format:check
npm.cmd run test
npm.cmd run rust:fmt
npm.cmd run rust:clippy
npm.cmd run rust:test
```

Run the full gate:

```powershell
npm.cmd run check
```

## Production build

```powershell
npm.cmd run tauri:build
```

The NSIS installer is written under:

```text
src-tauri\target\release\bundle\nsis\
```

Do not commit build output. `dist/` and `src-tauri/target/` are ignored.

## Data safety during development

- Tests create temporary databases and temporary resource folders.
- Do not point development tooling at the live per-user database.
- Do not run migrations or preparation against a real user database while developing.
- Do not commit databases, WAL/SHM files, backups, exports, or local environment files.
- The retired V1 application and its data are separate from V2. Do not use V1 data for development.

## Repository layout

```text
src/main.tsx                    V2 entry point
src/v2/                         Active React application and tests
src/App.tsx, src/screens/       Retained V1 frontend, not loaded by src/main.tsx
src/components/                 Shared active components plus retained V1 components
src/lib/                        Shared active utilities plus retained V1 API wrappers
src-tauri/src/v2/               Active Rust backend, schema, seeds, and tests
src-tauri/src/*.rs              Tauri entry, shared helpers, and retained V1 modules
src-tauri/src/v2/schema.sql     Current V2 schema
src-tauri/migrations/           Historical V1 migrations retained for compatibility
src-tauri/capabilities/         Tauri permissions
src-tauri/nsis/                 NSIS installer template
src-tauri/icons/                Application icons
legal/                          MIT and third-party licence bundle
docs/                           Architecture, features, privacy, release, and historical audit docs
tools/schema-validation/        Historical initial-schema specification/validator
```

Active V2 imports `BrandMark.tsx`, `NavIcon.tsx`, and `ThemeSwitch.tsx` from `src/components`. Most
other components in that folder are V1-only. Active V2 also uses selected utilities from `src/lib`;
the remaining V1 API wrappers are retained for historical compatibility and are not loaded by
`src/main.tsx`. V1 backend root modules and migrations remain for historical compatibility; do not
delete them without a separate migration and product decision.

## Code conventions

- TypeScript is strict. Do not introduce `any` without a clear boundary.
- Rust code must pass `cargo fmt` and Clippy with warnings denied.
- Keep scientific data, calculations, and persisted formats unchanged unless explicitly changing
  application behavior.
- Prefer explicit tests for state transitions, persistence, backup/restore, and destructive actions.
- Keep user-facing copy separate from internal identifiers and technical paths.
- Do not add network, telemetry, credentials, or external services without a reviewed design.

## Adding a feature

1. Identify the active V2 screen and backend module.
2. Add or update typed frontend API wrappers in `src/v2/api.ts`.
3. Add backend validation and transactions before persistence.
4. Add tests for success, validation failure, and persistence/reopen behavior.
5. Update the feature guide and changelog if behavior changes.
6. Run the full quality gate.

## Troubleshooting

- **Tauri commands unavailable:** run the desktop app with `npm run tauri:dev`, not a browser.
- **WebView2 missing:** install the Microsoft Edge WebView2 runtime.
- **Rust linker errors:** confirm the MSVC C++ workload and Windows SDK are installed.
- **Formatting failures:** run `npm run format` only for intended files, or fix the listed files.
- **Database state errors:** use a disposable database for development; do not modify a live user
  database to clear development errors.

### MSVC temporary-file errors (`LNK1104`, `D8050`)

If the error names a transient file under `%TEMP%` and normal temp writes work, use a dedicated
existing temp directory for that build session:

```powershell
New-Item -ItemType Directory -Force src-tauri\target\build-temp | Out-Null
$env:TMP = (Resolve-Path src-tauri\target\build-temp).Path
$env:TEMP = $env:TMP
npm.cmd run tauri:build
```

This is a local environment workaround, not project configuration. Investigate antivirus or
temp-directory policy rather than committing a workaround.
