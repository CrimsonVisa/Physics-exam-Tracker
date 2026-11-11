# CSIR-NET Physics Tracker 1.2.0 — Stages 1-4 Notes

Status: **Stages 1-4 implementation — JSON portability, local PDF recovery, backup/restore improvements, and dark typography refinement**
Release date: pending final v1.2.0 completion

## Stage 1 scope

- Complete JSON export format **2** adds all 20 user tables, including:
  - `study_resources`
  - `resource_progress`
  - `resource_local_files`
  - `resource_discovery_runs`
- Existing format 1 exports from versions 1.0.0 and 1.1.0 remain importable.
- Resource metadata, original URLs, source page URLs, Drive IDs, availability, study status, notes,
  last-opened timestamps, local-file metadata, and discovery history are preserved.
- Merge matches resources by stable `resource_key` and preserves existing resource progress, personal
  notes, and local PDF references.
- Replace removes core and resource records in foreign-key order, inserts the validated export,
  restores append-only triggers, verifies relationships and counts, and commits in one transaction.
- Local PDF references are imported only when the path exists and the size and SHA-256 match. Missing
  or changed files are skipped and reported; catalogue metadata and study progress remain.
- Format 2 imports require schema 10. Format 1 imports continue to accept schema 8, 9, or 10.
- Database schema remains **10**; no new migration is required.

## Stage 2 scope — local PDF recovery

- Resource listing checks filesystem existence and stored size without rehashing every PDF.
- Clear states: **Online only**, **Offline copy**, **Local copy missing**, and **Local copy changed**.
- Missing or changed files retain the catalogue entry, study status, personal notes, and history.
- **Re-link moved file** validates the stored size and SHA-256 before updating the path.
- **Replace with another PDF** copies a new authorized PDF without overwriting or deleting the old
  file.
- Opening a local PDF re-checks the file immediately and shows a useful error when unavailable.
- Removing a local file reference keeps the catalogue entry and study history.
- Inaccessible directories and filesystem errors are handled without deleting metadata.
- Database schema remains **10**; no migration is required.

## Stage 3 scope — backup and restore improvements

- SQLite backup manifests now include record counts for all 16 core tables and the four resource
  tables: `study_resources`, `resource_progress`, `resource_local_files`, and
  `resource_discovery_runs`.
- Manifest format remains **1**; the new `recordCounts` field is optional, and manifests written
  before v1.2.0 remain readable. When counts are present, restore rejects a manifest whose counts do
  not match the database inside the file.
- Table counting is schema-aware, so older compatible backups without resource tables still preview
  and restore normally.
- Restore preview shows resource catalogue, resource study progress, local PDF reference, and
  discovery-history counts, plus local PDF availability on the current computer.
- The preview explains that SQLite backups preserve local PDF paths as references, not PDF contents,
  and warns when saved files are missing or changed.
- Full restore integration tests create multiple resources, progress and notes, discovery history,
  and local PDF metadata with temporary PDFs; back up the database; restore it into a separate
  disposable database; and verify identifiers, relationships, URLs, SHA-256, study data, integrity,
  and foreign keys.
- Cross-machine tests remove the original PDF directory before restore and verify that catalogue
  entries, progress, notes, and history remain, missing PDFs are not shown as **Offline copy**, no
  PDF is downloaded, no existing file is overwritten, and re-linking works afterwards.
- Backup files are verified to contain no PDF bytes. PDF files are not included in the application
  or installer.
- Database schema remains **10**; no migration is required.

## Stage 4 scope — typography and dark colour refinement

- Bundled the actual **Computer Modern Unicode (CMU) 0.7.0** family locally; no runtime font
  download and no generic serif substitute.
- Headings and reading content use **CMU Serif**; navigation and UI controls use **CMU Sans Serif**;
  code and technical text use **CMU Typewriter Text**.
- CMU Serif is the primary math face. Cambria Math and Segoe UI Symbol provide local fallback glyphs
  for mathematical operators that CMU does not contain, including integrals, summations, and
  partial derivatives. Subscripts and superscripts use real `<sub>`/`<sup>` markup.
- Applied the premium dark palette consistently: `#0B0E14` canvas, `#151B26` cards, `#202A39`
  elevated surfaces, `#63C7D9` primary accent, `#D6B779` secondary accent, `#E7EAF0` main text,
  `#9BAAC0` muted text, and restrained blue-grey borders.
- Existing page layouts, navigation dimensions, card structure, spacing, tables, components, and
  workflows are unchanged. This stage changes typography and colour only.
- Fonts are licensed under the SIL Open Font License, Version 1.1; the license is included at
  `src/assets/fonts/OFL-1.1.txt` and shipped under `public/fonts/OFL-1.1.txt`.
- Database schema remains **10**; no data or functional changes were made.

## Safety

- Import prefers a verified safety backup and a single transaction.
- A failed import rolls back all resource and core changes.
- Repeated batch protection remains active.
- No remote files are downloaded automatically, and no PDF files are packaged in the application.

## Verification

- TypeScript, ESLint with zero warnings, Prettier, Rustfmt, and Clippy with warnings denied passed.
- **184 frontend tests passed** across 26 files.
- **170 Rust tests passed**, with 1 ignored synthetic performance test.
- New tests cover format 1 compatibility, format 2 resource round trips, merge preservation, replace
  ordering, duplicate resource keys, missing local PDFs, checksum mismatches, transactional rollback,
  moved/deleted files, changed sizes, re-linking, replacement, removal failures, opening unavailable
  files, full resource backup/restore, manifest counts, PDF-byte exclusion, cross-machine restore,
  and legacy manifest compatibility.
- Stage 4 is visual-only; the existing test suites pass unchanged, and the production build emits
  the bundled CMU font assets locally.
- Database schema remains **10**; no migration was added.
- The final 1.2.0 installer is not produced in this stage. The current public release remains 1.1.0.
