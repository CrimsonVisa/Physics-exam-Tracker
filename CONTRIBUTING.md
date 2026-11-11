# Contributing to Physics Tracker

Thank you for considering a contribution.

## Before you start

- This repository targets the active Physics Tracker 2 application, not the retired V1 application.
- Do not use or modify a live user database for development.
- Do not include databases, WAL/SHM files, backups, exports, exam PDFs, personal study records, or
  secrets in issues, commits, or pull requests.
- For security vulnerabilities, follow [SECURITY.md](SECURITY.md) instead of opening a public issue.

## Development setup

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md). The short version:

```powershell
npm.cmd ci
npm.cmd run tauri:dev
```

## Making changes

1. Keep changes focused and explain the user-visible or technical purpose.
2. Preserve existing data formats, migrations, timer behavior, and backup guarantees unless the
   change explicitly requires a compatible migration.
3. Add or update tests for changed behavior.
4. Keep scientific data and reference content accurate; include sources when adding content.
5. Update the changelog and relevant docs for user-visible behavior.
6. Do not commit generated output, local configuration, databases, backups, or PDFs.

## Quality gate

Run all relevant checks before requesting review:

```powershell
npm.cmd run check
```

For UI changes, include screenshots captured with disposable test data. Do not expose personal
notes, exam dates, names, or other identifying study information.

## Pull requests

A good pull request includes:

- a concise summary;
- the motivation for the change;
- the tests or manual verification performed;
- data/privacy/security implications;
- screenshots for visible UI changes;
- confirmation that no generated or personal files are included.

By contributing, you agree that your contribution is licensed under the repository's MIT license.
