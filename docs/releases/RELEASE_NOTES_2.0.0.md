# Physics Tracker 2.0.0 — Release Notes

## Overview

Physics Tracker 2.0.0 introduces the four-examination Physics Tracker workspace in the first release.
It replaces the earlier CSIR-only product presentation with a single application for:

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

## Screenshots

See the [README screenshot gallery](https://github.com/CrimsonVisa/Physics-exam-Tracker/blob/main/README.md#screenshots)
for screenshots captured from Physics Tracker 2.0.0 with disposable demo data. The gallery uses the
dark theme.

## Download and install

The v2.0.0 release is an unpublished draft, and the repository is currently private. The links below
will not be publicly available until the release is published.

[GitHub Releases — Physics Tracker v2.0.0](https://github.com/CrimsonVisa/Physics-exam-Tracker/releases/tag/v2.0.0)

The release will include the per-user NSIS installer:

[Physics.Tracker_2.0.0_x64-setup.exe](https://github.com/CrimsonVisa/Physics-exam-Tracker/releases/download/v2.0.0/Physics.Tracker_2.0.0_x64-setup.exe)

**SHA-256:**

```text
A14816F743002C6BCF09CAC8F2203B9617B1609AC486E673AE741141D3FC4814
```

After publication, verify the downloaded file from PowerShell before running it:

```powershell
Get-FileHash ".\Physics.Tracker_2.0.0_x64-setup.exe" -Algorithm SHA256
```

The reported hash must match the value above. GitHub displays the asset name with periods in place of
spaces; the attached `.sha256` file may reference the original build filename. Verify the downloaded
file using the SHA-256 value above.

### Installation

1. After the release is published, download `Physics.Tracker_2.0.0_x64-setup.exe` from the GitHub
   release.
2. Verify the SHA-256 checksum.
3. Run the installer. It installs for the current Windows user and does not require administrator
   rights.
4. Launch Physics Tracker from the Start Menu or desktop shortcut.

The installer is **unsigned**, so Windows SmartScreen may display a warning. Verify the checksum
before choosing to run the installer.

Only the per-user NSIS installer will be distributed. No portable or standalone build will be
provided.

## Requirements

- Windows 10 or Windows 11, 64-bit
- Microsoft Edge WebView2 runtime

If WebView2 is absent, the installer downloads Microsoft's bootstrapper, which requires an internet
connection. Offline WebView2 installation is not supported by this build.

## First launch

No account or sign-in is required. The combined dashboard opens and the local database is created or
opened automatically. Use **Settings** to set the preparation start date and daily study target, and
to configure the examination profiles and exam dates.

## Data compatibility

- Application identifier: `com.crimsonvisa.physics-tracker`
- Database: `%APPDATA%\com.crimsonvisa.physics-tracker\physics-tracker.sqlite3`
- Managed resources: `%APPDATA%\com.crimsonvisa.physics-tracker\resources`
- WebView2 profile: `%LOCALAPPDATA%\com.crimsonvisa.physics-tracker\EBWebView`
- Current schema version: 5

Physics Tracker 1.x and 2.x use separate application identities and data directories. Existing 1.x
data under the legacy `com.crimsonvisa.csirnetphysics` identity is not imported automatically into
2.x. Installing 2.x does not delete 1.x data; the versions can coexist.

## Backups and resources

Backups are SQLite database snapshots with manifests and checksums. They do not contain the bytes of
managed PDF/resource files. To preserve those files, copy the `resources` directory separately while
the application is closed, and keep it with your backups.

Backups and exports are unencrypted. Store them in a location you control and protect them according
to the sensitivity of your study records.

## License

Physics Tracker 2.0.0 is distributed under the MIT license. See
[LICENSE](https://github.com/CrimsonVisa/Physics-exam-Tracker/blob/v2.0.0/LICENSE).

The NSIS installer displays the MIT license. Installed copies include the legal resources under
`%LOCALAPPDATA%\Physics Tracker\legal\`: `LICENSE`, `THIRD_PARTY_NOTICES.md`,
`THIRD_PARTY_COVERAGE.md`, `licenses\`, `notices\`, and the bundled font license texts in `fonts\`.

Third-party components remain under their respective licenses; see
[THIRD_PARTY_NOTICES.md](https://github.com/CrimsonVisa/Physics-exam-Tracker/blob/v2.0.0/THIRD_PARTY_NOTICES.md).

## Known limitations

- Windows-only.
- Installer is unsigned.
- No automatic updater.
- Backup and export files are unencrypted.
- SQLite backups do not include managed PDF/resource files.
- No portable or standalone build will be provided.

## Support

- Bug reports and feature requests: [GitHub Issues](https://github.com/CrimsonVisa/Physics-exam-Tracker/issues)
- Security reports: follow
  [SECURITY.md](https://github.com/CrimsonVisa/Physics-exam-Tracker/blob/v2.0.0/SECURITY.md)

Do not attach databases, backups, personal study data, exam PDFs, credentials, or private keys to a
public issue.
