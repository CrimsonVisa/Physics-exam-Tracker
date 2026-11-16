# Physics Tracker 2.1.0 — Release Notes

## Overview

Physics Tracker 2.1.0 is a quality release for the four-examination V2 workspace. It improves the
reference library and mathematical rendering, strengthens dashboard and revision workflows, adds
native Windows timer notifications and fuller managed-PDF handling, and closes important
multi-exam timer and historical-credit inconsistencies.

The application remains offline-first. Study records stay in a local SQLite database.

## Highlights

- Reference-library and equation-rendering improvements.
- Dashboard refinements and an examination countdown section.
- Clearer revision scheduling, daily plans, deferrals, and completion history.
- Native Windows notifications when a focus timer completes.
- Managed PDF open, attach, relink, replace, missing-file detection, and removal.
- Multi-exam study-time allocation with once-per-session overall accounting.
- Consistent running-timer associations and preserved historical exam credit.

## Reference library and mathematical rendering

- Formula, built-in constant, custom constant, unit, and conversion views are easier to browse and
  inspect.
- Reference tabs load their libraries on demand while preserving tab state.
- Shared editor dialogs keep their headers and primary actions fixed around a scrollable body, so
  long formula variable lists do not hide the save controls.
- KaTeX rendering was improved for inline and display mathematics, mixed text, formulas, constants,
  units, notes, and descriptions.
- Unit conversion summaries render symbols, factors, and contextual conversion notes consistently.

## Dashboard and examination countdown

- The dashboard includes an examination countdown for active examinations, with days remaining,
  exam-day, passed, invalid-date, and date-not-set states.
- Countdown cards prioritise upcoming examinations and link to the examination workspace or
  Settings when a date needs attention.
- Dashboard section ordering and summaries remain local-first and count recorded study time once.

## Revision scheduling

- Due and overdue revision work, today’s plan, upcoming revisions, awaiting-approval items, and
  recent completion history are presented together.
- Daily plans, deferrals, workload limits, completion, and adaptive intervals remain integrated
  with the planner and calendar.
- Mapping approval is required before proposed concepts become active revision work.

## Windows timer notifications

- Focus-session completion can display a native Windows notification when notifications are
  enabled.
- Repeated completion delivery is suppressed so a single completed session does not generate
  duplicate notices.
- Break timers do not create study sessions or completion notifications.

## PDF resource management

- Managed PDF copies can be imported, opened, attached, relinked, replaced, and removed from the
  resource catalogue.
- Missing, changed, truncated, unreadable, or oversized PDFs are reported without deleting the
  resource record.
- The original source file is not modified, and managed paths are validated so file operations
  remain inside the managed resource folder.
- SQLite backups still exclude PDF bytes; preserve the managed `resources` directory separately.

## Multi-exam study-time accounting

- One study session can be associated with multiple examinations.
- Each associated examination receives the full session duration for examination totals, while
  overall actual study time counts the session once.
- Topic allocations remain a detailed breakdown of the same recorded session.
- Historical split allocations retain their original credited seconds.

## Timer and historical-credit consistency

- Removing an examination association now deletes only that examination’s topic allocations.
  Retained topics, session duration, session identity, and overall study time are preserved.
- Retained historical exam associations keep their exact credited seconds across repeated edits.
  New active associations receive full-session credit, and inactive exams receive no new credit.
- Running timers lock their examination association set. Frontend and backend validation reject
  allocations outside the persisted selection.
- Completion credits only the persisted timer associations; examinations inferred from topic rows
  or legacy active-exam state are not silently added.
- Deactivated examinations are excluded from running timer snapshots and completion credit while
  historical session allocations remain intact.
- Legacy in-progress timer selection backfill is idempotent and does not resurrect deliberately
  removed associations.

## Download and install

The v2.1.0 release is distributed through GitHub Releases:

[GitHub Releases — Physics Tracker v2.1.0](https://github.com/CrimsonVisa/Physics-exam-Tracker/releases/tag/v2.1.0)

The release asset is the per-user NSIS installer:

[Download Physics.Tracker_2.1.0_x64-setup.exe](https://github.com/CrimsonVisa/Physics-exam-Tracker/releases/download/v2.1.0/Physics.Tracker_2.1.0_x64-setup.exe)

The local build filename is `Physics Tracker_2.1.0_x64-setup.exe`.

**SHA-256:**

```text
15E0117D3C1161E5241ADC0FE43AF7638E34D8A6316BDCA3696605B83376429A
```

Verify the downloaded file from PowerShell before running it:

```powershell
Get-FileHash ".\Physics.Tracker_2.1.0_x64-setup.exe" -Algorithm SHA256
```

The reported hash must match the value above. GitHub displays the asset name with periods in place
of spaces; verify the downloaded file using the SHA-256 value above.

### Installation

1. Download `Physics.Tracker_2.1.0_x64-setup.exe` from the GitHub release.
2. Verify the SHA-256 checksum.
3. Run the installer. It installs for the current Windows user and does not require administrator
   rights.
4. Launch Physics Tracker from the Start Menu or desktop shortcut.

The installer is **unsigned**, so Windows SmartScreen may display a warning. Verify the checksum
before choosing to run the installer.

Only the per-user NSIS installer is distributed. No portable or standalone build is provided.

## Requirements

- Windows 10 or Windows 11, 64-bit
- Microsoft Edge WebView2 runtime

If WebView2 is absent, the installer downloads Microsoft’s bootstrapper, which requires an internet
connection. Offline WebView2 installation is not supported by this build.

## First launch and data compatibility

- Application identifier: `com.crimsonvisa.physics-tracker`
- Database: `%APPDATA%\com.crimsonvisa.physics-tracker\physics-tracker.sqlite3`
- Managed resources: `%APPDATA%\com.crimsonvisa.physics-tracker\resources`
- WebView2 profile: `%LOCALAPPDATA%\com.crimsonvisa.physics-tracker\EBWebView`
- Current schema version: 5

Existing 2.0.0 data upgrades additively. Examination records, study sessions, topic allocations,
and historical split credits are preserved. The production database rehearsal verified schema 5,
integrity, foreign keys, historical credits, and overall study time.

Physics Tracker 1.x and 2.x use separate application identities and data directories. Installing
2.1.0 does not delete 1.x data, and the uninstaller intentionally preserves application data.

## Backups and resources

Backups are SQLite database snapshots with manifests and checksums. They do not contain the bytes of
managed PDF/resource files. To preserve those files, copy the `resources` directory separately while
the application is closed and keep it with your backups.

Backups and exports are unencrypted. Store them in a location you control and protect them according
to the sensitivity of your study records.

## Known limitations

- Windows-only.
- Installer is unsigned.
- No automatic updater.
- Backup and export files are unencrypted.
- SQLite backups do not include managed PDF/resource files.
- No portable or standalone build is provided.

## License

Physics Tracker 2.1.0 is distributed under the MIT license. See
[LICENSE](https://github.com/CrimsonVisa/Physics-exam-Tracker/blob/v2.1.0/LICENSE).

The NSIS installer displays the MIT license. Installed copies include the legal resources under
`%LOCALAPPDATA%\Physics Tracker\legal\`: `LICENSE`, `THIRD_PARTY_NOTICES.md`,
`THIRD_PARTY_COVERAGE.md`, `licenses\`, `notices\`, and the bundled font license texts in `fonts\`.

Third-party components remain under their respective licenses; see
[THIRD_PARTY_NOTICES.md](https://github.com/CrimsonVisa/Physics-exam-Tracker/blob/v2.1.0/THIRD_PARTY_NOTICES.md).

## Support

- Bug reports and feature requests: [GitHub Issues](https://github.com/CrimsonVisa/Physics-exam-Tracker/issues)
- Security reports: follow
  [SECURITY.md](https://github.com/CrimsonVisa/Physics-exam-Tracker/blob/v2.1.0/SECURITY.md)

Do not attach databases, backups, personal study data, exam PDFs, credentials, or private keys to a
public issue.
