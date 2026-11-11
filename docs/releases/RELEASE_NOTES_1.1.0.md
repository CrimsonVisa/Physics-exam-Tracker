# CSIR-NET Physics Tracker 1.1.0 — Release Notes

Release date: 30 October 2026

## Overview

CSIR-NET Physics Tracker 1.1.0 adds a Study Resources catalogue built from the material links
published by [csirnetphysics.in](https://www.csirnetphysics.in/), plus independent study tracking and
an optional local PDF library. All existing study, planning, analytics, backup, and portability
features remain in place.

The application stores resource metadata and user progress locally. It does not host, redistribute,
or claim ownership of third-party files.

## Study Resources

- New **Resources** navigation section with search and filters for resource type, syllabus unit,
  source website, availability, and study status.
- Ten resource categories: handwritten notes, classroom notes, assignments, formula sheets,
  previous-year papers, important topics, recommended books, NPTEL courses, YouTube playlists, and
  other educational resources.
- Manual **Refresh catalogue** reads six known source pages and their public resource data feeds.
  Requests are paced at least 1.25 seconds apart, stop on HTTP 429, and never run on startup.
- Resources are cached in SQLite. Failed or blocked refreshes preserve all previously discovered
  links and user progress.
- A successful source refresh marks missing links as **Removed from source** rather than deleting
  them.
- Original Google Drive file and folder URLs are preserved. The app extracts IDs where possible and
  opens original links in the system browser. It does not request Google credentials or bypass
  sharing limits.
- Manual registration is available when discovery is blocked or a user has another authorized link.
- Independent study status (Not started, Studying, Completed, Skipped), personal notes, and
  last-opened timestamps are stored separately from syllabus progress and revision history.
- Authorized PDFs can be imported into a user-selected local library. The app validates the PDF
  header, enforces a 200 MB limit, avoids overwriting, records SHA-256, supports opening in the
  default reader, and allows removing the local copy without deleting the resource or study history.
- Online-only links are labelled as requiring internet. **Offline copy** is shown only when a
  verified local file record exists.

## Live catalogue verification

A manual refresh against the live source pages on 30 October 2026 returned:

- 1,300 resources
- 1,173 Google Drive links
- 158 note/assignment/formula resources
- 777 previous-year question-paper resources
- 167 recommended-book resources
- 51 NPTEL courses
- 75 YouTube playlists
- 1 important-topics source page
- 71 other educational resources, including syllabus downloads and cut-off material

All nine Physical Sciences units and General Aptitude are represented. Exact and conservative
topic-title matches are mapped to existing syllabus topic codes; full multi-unit papers use
**All Units** and unclear entries use **Unmapped**.

## Data compatibility

- Database schema version: **10**
- Migration `0010_study_resources.sql` adds resource metadata, independent study progress, local PDF
  records, and discovery-run history.
- Existing schema-9 data is preserved. A verified pre-migration snapshot is still created first and
  the upgrade aborts if that snapshot fails.
- SQLite full backups include resource metadata, progress, and local-file records, but do not copy the
  PDF files themselves.
- JSON export continues to cover the core study tables; it does not currently include the resource
  catalogue or machine-specific local PDF paths.
- JSON import accepts schema 8, 9, or 10 exports.

## Release artifacts

- Installer: `CSIR-NET Physics Tracker_1.1.0_x64-setup.exe`
- Installer size: 5,901,285 bytes (5.63 MiB)
- Installer SHA-256: `378839FD7A9D558E8257D955D94964972D85B4B7ECD336151475D8AF0785BD2E`
- Standalone executable SHA-256: `48942C7C25012F874A81D984B0FF4726B14BA2F0E010449E7E2E7D412B574C0C`
- Installer mode: current user, no Administrator access required
- Install path: `%LOCALAPPDATA%\CSIR-NET Physics Tracker`

## Verification summary

- TypeScript, ESLint with zero warnings, Prettier, and the production frontend build passed.
- Rustfmt, Clippy with warnings denied, and the complete Rust test suite passed.
- Test results: 182 frontend tests and 158 Rust tests passed; 1 synthetic performance test remains
  ignored by default.
- The Tauri 1.1.0 release build and NSIS installer completed. The executable reports version 1.1.0.
- The real schema-9 database upgraded to schema 10 with a verified pre-migration snapshot. Settings,
  182 syllabus topics, and the existing study records were preserved.
- A live manual refresh discovered 1,300 resources and 1,173 Google Drive links with all six source
  pages readable and no HTTP 429 response.
- Silent install, launch, uninstall, reinstall, second launch, and second uninstall completed. The
  schema-10 database and all 1,300 resource rows survived every step.

## Known limitations

- Discovery depends on the source website's HTML and embedded public data feeds; if those change, the
  refresh reports a warning and preserves the existing catalogue.
- Google Drive availability is not probed for every link. The app does not claim a remote file is
  available offline or downloadable until the user has successfully imported a local copy.
- Folder links open in the browser; folder contents are not enumerated.
- The app never downloads a whole collection automatically.
- Resource notes and local PDF records are separate from syllabus and revision data by design.
- Backup and export files remain unencrypted.
