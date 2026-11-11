# Privacy and Data Handling

Physics Tracker is designed as a local-first desktop study tracker. It does not require an
account, does not include telemetry, and does not upload study records to a remote service.

## Data stored locally

The active application stores its SQLite database at:

```text
%APPDATA%\com.crimsonvisa.physics-tracker\physics-tracker.sqlite3
```

SQLite may create `-wal` and `-shm` sidecars next to the database.

The WebView2 profile is stored separately at:

```text
%LOCALAPPDATA%\com.crimsonvisa.physics-tracker\EBWebView
```

Backups default to:

```text
%APPDATA%\com.crimsonvisa.physics-tracker\backups
```

The database contains study records, syllabus progress, timer/session data, notes, resources,
flashcards, practice data, settings, and backup metadata. It does not contain passwords or remote
account credentials.

## Network use

Core study functionality is offline. The application may use the network when you explicitly:

- discover resources from supported source pages;
- open an external reference link;
- follow an official syllabus/source URL.

The application does not send study data, notes, practice answers, or database contents to those
sites. It also does not include advertising, analytics SDKs, crash-reporting services, or remote
accounts.

## Backups and exports

Backup and export files are unencrypted. Store them in a location you control and protect them
according to the sensitivity of your study records. Do not attach personal databases or backups to
public issues or pull requests.

## V1 separation

The active V2 application uses the identifier `com.crimsonvisa.physics-tracker`. It does not open
or modify the retired V1 database. V1 data remains in its own separate location.

## Uninstall behavior

The installer is configured not to recursively delete the user data directory. Review
`src-tauri/nsis/installer.nsi` and the release checklist before making deletion guarantees in a
release announcement. Backups and exports outside the application data folder are under your
control.

## Security reporting

Do not disclose vulnerability details, personal data, databases, or backups in a public issue.
Follow [SECURITY.md](../SECURITY.md).
