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
src/v2/                    Active React application and tests
src/components/            Shared components used by the active shell
src/lib/                   Frontend utilities and API wrappers
src-tauri/src/v2/          Active Rust backend, schema, seeds, and tests
src-tauri/schema.sql       V2 schema
src-tauri/migrations/      Historical migration files retained for compatibility
src-tauri/capabilities/    Tauri permissions
src-tauri/icons/           Application icons
docs/                      Architecture, features, privacy, release, and audit docs
tools/schema-validation/   Standalone schema specification and validation script
```

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
