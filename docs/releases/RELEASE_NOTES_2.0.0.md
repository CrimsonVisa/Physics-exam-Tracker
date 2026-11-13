# Physics Tracker 2.0.0 — Release Notes

**Status:** Public release preparation. The final tagged release will be published manually through
GitHub Releases after the draft is reviewed, the generated notes are replaced with this document,
and the attached installer and checksum are verified.

## Overview

Physics Tracker 2.0.0 is the first public release of the four-examination Physics Tracker
workspace. It replaces the earlier CSIR-only product presentation with a single application for:

- CSIR-UGC NET Physical Sciences
- GATE Physics (PH)
- TIFR Graduate School Physics
- JEST Physics

The application remains offline-first. Study records stay in a local SQLite database.

## Highlights

- Combined dashboard and four independent examination workspaces.
- Global focus/break timer with recovery and session allocation.
- Official-source syllabus trees and concept mastery tracking.
- Formula, terms, units, and physical-constant reference libraries.
- PYQ practice, verified mock tests, scoring, and practice analytics.
- Flashcards, scientific calculator, planner, and adaptive revision planning.
- Sticky notes, resource catalogue, and managed local PDF copies.
- Accessible analytics charts with data tables.
- Verified backup/restore, CSV export, and versioned JSON portability.
- Dark/light themes and adjustable typography.

## Data compatibility

- Application identifier: `com.crimsonvisa.physics-tracker`
- Database: `%APPDATA%\com.crimsonvisa.physics-tracker\physics-tracker.sqlite3`
- WebView2 profile: `%LOCALAPPDATA%\com.crimsonvisa.physics-tracker\EBWebView`
- Current schema version: 5

Physics Tracker 1.x and 2.x use separate application identities and data directories. Existing 1.x
data under the legacy `com.crimsonvisa.csirnetphysics` identity is not imported automatically into
2.x. Installing 2.x does not delete 1.x data; the versions can coexist.

## License

Physics Tracker 2.0.0 is distributed under the MIT license. See [LICENSE](../../LICENSE).
The NSIS installer displays the MIT license. Installed copies are under
`%LOCALAPPDATA%\Physics Tracker\legal\`: `LICENSE`, `THIRD_PARTY_NOTICES.md`,
`THIRD_PARTY_COVERAGE.md`, `licenses\`, `notices\`, and the bundled font license texts in `fonts\`.

Third-party components remain under their respective licenses; see
[THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md).

## Installation

The release artifact is a per-user NSIS installer:

```text
Physics Tracker_2.0.0_x64-setup.exe
```

Windows 10/11 x64 and the Microsoft Edge WebView2 runtime are required. If WebView2 is absent, the
installer downloads Microsoft's bootstrapper, which requires an internet connection. Offline
WebView2 installation is not supported by this build. The current installer is unsigned;
SmartScreen may display a warning. Verify the published SHA-256 checksum before running the
installer.

## Known limitations

- Windows-only.
- Installer is currently unsigned.
- No automatic updater.
- Backup and export files are unencrypted.
- Public screenshots and final release artifacts are added after isolated visual verification.

## Verification

The release should be built from a clean tagged commit after the full quality gate passes:

```powershell
npm.cmd run check
npm.cmd run tauri:build
```

See [docs/RELEASE.md](../RELEASE.md) for the release checklist and artifact verification steps.
