# Security Policy

## Supported versions

Security fixes are provided for the current Physics Tracker 2 release line. Unsupported or
historical branches may receive no fixes.

## Reporting a vulnerability

Do not open a public issue containing vulnerability details, personal data, credentials, database
files, or backups.

Report vulnerabilities through GitHub's private vulnerability reporting for this repository when
available, or contact the repository maintainer through their GitHub profile. Include:

- affected version;
- Windows version;
- a minimal description of the issue;
- reproduction steps using disposable data;
- impact assessment;
- any suggested mitigation.

Never include a real study database, backup, `.env` file, access token, password, or private key.

## Scope

Relevant areas include:

- local database handling and migrations;
- backup, restore, import, and export paths;
- local file opening and resource handling;
- Tauri command validation, capabilities, and CSP;
- dependency vulnerabilities;
- accidental data disclosure;
- installer and update integrity.

The application currently has no remote account system, telemetry, updater, or code-signing
certificate. Do not assume those mechanisms are present.

## Response

Reports are reviewed on a best-effort basis. There is no guaranteed response SLA. Please allow time
for triage before public disclosure, and coordinate disclosure with the maintainer.
