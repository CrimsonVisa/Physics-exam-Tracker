# Privacy and Data Handling

Physics Tracker is designed as a local-first desktop study tracker. It does not require an
account, does not include telemetry, and does not upload study records to a remote service.

## Data stored locally

The active application stores its SQLite database at:

```text
%APPDATA%\com.crimsonvisa.physics-tracker\physics-tracker.sqlite3
```

SQLite may create `-wal` and `-shm` sidecars next to the database.

Managed PDF/resource copies are stored separately under:

```text
%APPDATA%\com.crimsonvisa.physics-tracker\resources
```

The WebView2 profile is stored separately at:

```text
%LOCALAPPDATA%\com.crimsonvisa.physics-tracker\EBWebView
```

Backups default to:

```text
%APPDATA%\com.crimsonvisa.physics-tracker\backups
```

The database contains study records, syllabus progress, timer/session data, notes, resource
metadata, flashcards, practice data, settings, and backup metadata. It does not contain passwords or
remote account credentials.

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

SQLite backups contain the database records and metadata, but not the bytes of managed
PDF/resource files. To preserve those files, copy the `resources` directory separately while the
application is closed, and keep it with your backups.

## V1 separation

The active V2 application uses the identifier `com.crimsonvisa.physics-tracker`. It does not open
or modify the retired V1 database. V1 data remains in its own separate location.

## Uninstall behavior

Uninstalling Physics Tracker removes the installed application files, shortcuts, and per-user
uninstall registry entry. It does not recursively delete the V2 application-data directory.

The following remain on disk after uninstall unless you remove them manually:

- the V2 database, managed resources, and default backups under
  `%APPDATA%\com.crimsonvisa.physics-tracker`;
- the WebView2 profile under `%LOCALAPPDATA%\com.crimsonvisa.physics-tracker`;
- any backups or exports you saved in a custom location.

To remove all local data, uninstall the application and then delete those application-data folders
and any custom backup/export locations manually. Removing them permanently deletes your study
records and backups.

Installing or uninstalling 2.x does not import, modify, or delete 1.x data.

## Security reporting

Do not disclose vulnerability details, personal data, databases, or backups in a public issue.
Follow [SECURITY.md](../SECURITY.md).
