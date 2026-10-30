# CSIR-NET Physics Tracker 1.2.0 — Stage 1 Notes

Status: **Stage 1 implementation — JSON data portability**
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

## Safety

- Import prefers a verified safety backup and a single transaction.
- A failed import rolls back all resource and core changes.
- Repeated batch protection remains active.
- No remote files are downloaded automatically, and no PDF files are packaged in the application.

## Verification

- TypeScript, ESLint with zero warnings, Prettier, Rustfmt, and Clippy with warnings denied passed.
- **182 frontend tests passed** across 26 files.
- **164 Rust tests passed**, with 1 ignored synthetic performance test.
- New tests cover format 1 compatibility, format 2 resource round trips, merge preservation, replace
  ordering, duplicate resource keys, missing local PDFs, checksum mismatches, and transactional
  rollback.
- Database schema remains **10**; no migration was added.
- The final 1.2.0 installer is not produced in this stage. The current public release remains 1.1.0.
