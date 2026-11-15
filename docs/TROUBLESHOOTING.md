# Troubleshooting

This guide covers common installation, startup, backup, restore, and resource-file problems in
Physics Tracker 2.0.0.

Before changing anything, confirm:

- you are using the per-user installer `Physics.Tracker_2.0.0_x64-setup.exe`;
- you are on Windows 10 or Windows 11, 64-bit;
- Microsoft Edge WebView2 is installed;
- you know where your data is stored:

```text
%APPDATA%\com.crimsonvisa.physics-tracker\physics-tracker.sqlite3
%APPDATA%\com.crimsonvisa.physics-tracker\resources
%APPDATA%\com.crimsonvisa.physics-tracker\backups
%LOCALAPPDATA%\com.crimsonvisa.physics-tracker\EBWebView
```

Do not delete or replace the database manually. Use the in-app backup and restore workflow.

## The installer is blocked or SmartScreen warns

The v2.0.0 installer is unsigned, so Windows SmartScreen may warn about an unknown publisher.

1. Verify the SHA-256 of the downloaded GitHub release asset:

   ```powershell
   Get-FileHash ".\Physics.Tracker_2.0.0_x64-setup.exe" -Algorithm SHA256
   ```

2. Compare it with the canonical checksum for the GitHub release asset:

   ```text
   A14816F743002C6BCF09CAC8F2203B9617B1609AC486E673AE741141D3FC4814
   ```

3. Only continue if the checksum matches. If Windows or security software still blocks the file,
   use your security software's normal trust/allow flow; do not disable security protection globally.

The GitHub release asset uses the dot-separated filename. The space-separated local build output is
not the GitHub release asset.

The installer is per-user and does not require administrator rights.

## WebView2 is missing or fails to install

Physics Tracker requires Microsoft Edge WebView2.

- If WebView2 is absent, the installer downloads Microsoft's bootstrapper. An internet connection is
  required for that step.
- This build does not include an offline WebView2 installer.
- If the bootstrapper cannot complete, install the WebView2 Runtime manually from Microsoft:

  <https://developer.microsoft.com/microsoft-edge/webview2/>

Then run the Physics Tracker installer again.

## Physics Tracker does not open

1. Launch Physics Tracker from the Start Menu or desktop shortcut.
2. Confirm Microsoft Edge WebView2 is installed.
3. Re-run the installer if the application files are missing. Reinstalling does not remove the V2
   database, backups, managed resources, or WebView2 profile.
4. If the application still does not open, report the problem with the version, Windows version, and
   exact steps. Do not delete the database while investigating.

## A backup fails

Check the backup settings:

- The destination must be an absolute folder path, or blank to use the default backup folder.
- The destination must be writable and have free space.
- The destination cannot be inside the retired V1 application-data folder.
- Retention must be between 1 and 100 backups.

Then select **Save backup settings** and try **Create backup now** again.

## A restore is rejected or unavailable

Use **Backup → Preview restore** and review the reported schema, exam count, and integrity result.

- A backup from an unsupported schema cannot be restored by this build.
- A failed integrity or foreign-key check means the backup is not safe to restore.
- Keep the rejected backup file; do not edit it.
- Restore a different verified backup if one is available.

A safety copy is created automatically before every restore. Do not replace the live database
manually.

## A managed PDF is missing

Managed PDFs are separate from the SQLite database and are not included in database backups.

- The resource entry remains even if its file is missing.
- Use **Relink PDF** on a missing resource, or **Attach PDF** / **Replace PDF** to choose a local
  copy.
- **Remove PDF** removes the app's file reference; it does not delete the original file from disk.
- To preserve managed PDFs, close the application and copy the `resources` folder shown above to the
  same secure location as your backups.

## Data appears to be missing after reinstalling or moving computers

1. Confirm you are signed in to the same Windows user profile that contains the Physics Tracker data.
2. Check the database path shown above.
3. If you have a verified backup, use **Backup → Preview restore** to inspect and restore it.
4. If you moved to another computer, also copy the `resources` folder; database backups do not
   contain managed PDF bytes.
5. Do not create a new database over an existing one or edit SQLite files manually.

## Report a bug

Open a GitHub issue:

<https://github.com/CrimsonVisa/Physics-exam-Tracker/issues>

Include:

- Physics Tracker version
- Windows version
- clear reproduction steps
- the exact error message, if any
- screenshots or logs only after removing personal study data, names, paths, credentials, databases,
  backups, and PDFs

Do not attach real databases, backups, personal notes, exam PDFs, credentials, or private keys.

For security vulnerabilities, follow [SECURITY.md](../SECURITY.md) instead of opening a public issue.

## Related documentation

- [README](../README.md)
- [Privacy and data handling](PRIVACY.md)
- [Feature guide](FEATURES.md)
- [v2.0.0 release notes](releases/RELEASE_NOTES_2.0.0.md)
