# Release Process

## Versioning

Physics Tracker uses semantic versioning for the application release line. The current application
version is `2.1.0`.

- Application version: `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`
- Git tag: stable `vX.Y.Z`; for this release, `v2.1.0`
- Installer: `Physics Tracker_2.1.0_x64-setup.exe`

Update all version declarations together for a release. The release workflow fails before
building if the tag version does not exactly match `package.json`, `src-tauri/Cargo.toml`, and
`src-tauri/tauri.conf.json`. Do not add a version suffix to the display product name; the product is
always **Physics Tracker**.

## Release prerequisites

- Clean `main` working tree
- CI green on the release commit
- Prettier, TypeScript, ESLint, Rust formatting, and Clippy green
- Frontend and Rust tests green
- README, feature guide, changelog, and release notes current
- Installer metadata verified
- MIT license text present in `LICENSE` and displayed by the NSIS installer
- MIT license, `THIRD_PARTY_NOTICES.md`, the coverage report, full license texts, package notices,
  and font license texts bundled under `legal\` in the installed application
- Third-party coverage verified before release, including Apache-2.0, MPL-2.0, OFL-1.1, and
  dependency-specific notices; see `legal\THIRD_PARTY_COVERAGE.md`
- WebView2 download-bootstrapper behavior and its internet requirement documented; no offline
  installation claim
- 1.x/2.x separate identity and data-directory behavior documented
- No database, backup, PDF, build output, secret, or local configuration file staged

## Local verification

From the repository root:

```powershell
npm.cmd ci
npm.cmd run check
npm.cmd run tauri:build
```

Verify the build output:

```powershell
$installer = 'src-tauri\target\release\bundle\nsis\Physics Tracker_2.1.0_x64-setup.exe'

Get-Item $installer | Select-Object FullName,Length
(Get-Item $installer).VersionInfo |
  Select-Object ProductName,FileDescription,ProductVersion,FileVersion
Get-FileHash $installer -Algorithm SHA256
```

The installer is ignored by Git and must not be committed.

After installation, verify that `legal\LICENSE`, `legal\THIRD_PARTY_NOTICES.md`,
`legal\THIRD_PARTY_COVERAGE.md`, `legal\licenses\`, `legal\notices\`, and `legal\fonts\` are present
in the per-user installation folder and that the installer displays the MIT license page.

The NSIS installer uses the WebView2 download bootstrapper. If WebView2 is absent, installation
requires an internet connection; this build does not support offline WebView2 installation. Do not
claim offline installation support.

Physics Tracker 2.x uses `com.crimsonvisa.physics-tracker` and a separate data directory. Existing
1.x data remains under the legacy `com.crimsonvisa.csirnetphysics` identity and is not imported
automatically. Installing 2.x does not delete 1.x data.

## GitHub Actions

- `.github/workflows/ci.yml` runs the Windows quality gate for pushes to `main` and pull requests.
- `.github/workflows/release.yml` accepts only stable `vX.Y.Z` tags. It validates that the tag
  version matches `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, then
  builds and uploads the installer as a workflow artifact.
- Manual dispatch must be started from `main` with an existing stable `vX.Y.Z` tag.
- The release workflow creates a **draft** GitHub release with the installer and SHA-256 checksum.
  It does not publish automatically.
- `.github/dependabot.yml` proposes npm, Cargo, and GitHub Actions updates weekly.

All third-party Actions used by the CI and release workflows are pinned to full commit SHAs. Keep
them pinned to immutable commits rather than major-version tags.

After the draft is created, review it, replace the generated release notes with
`docs/releases/RELEASE_NOTES_2.1.0.md`, verify the attached installer and checksum, and publish the
draft manually.

## MPL-2.0 source availability

MPL-2.0 applies to `cssparser` 0.35.0 and 0.37.0, `cssparser-macros` 0.6.1 and 0.7.1, `dtoa-short`
0.3.5, `option-ext` 0.2.0, and `selectors` 0.31.0 and 0.38.0. The full MPL-2.0 text is bundled under
`legal\licenses\`; the corresponding source code can be obtained from the revision-pinned upstream
source URLs recorded in `legal\THIRD_PARTY_COVERAGE.md`. `cssparser-macros` and `selectors` are
workspace members, so their package manifests and source files are under `macros/` and `selectors/`,
not at the repository root.

No source archive is included with the installer. Before publishing, verify that the recorded source
links remain accessible and correspond to the versions being distributed.

## Signing and updates

The current configuration has **no code signing** and **no auto-updater**.

- Unsigned Windows installers may trigger SmartScreen warnings.
- Do not claim a signed release unless an Authenticode certificate has been configured and verified.
- Do not add updater signing keys or secrets to the repository.
- If an updater is added later, use a separate updater signing key stored in GitHub Secrets and
  publish signed updater artifacts.

## Tagging and publishing

After the release commit is approved and verified:

```powershell
git tag -a v2.1.0 -m "Physics Tracker 2.1.0"
git push origin main
git push origin v2.1.0
```

The tag should point to the release commit, not a dirty working tree. Do not push experimental or
local-preservation branches.

Pushing the tag starts the release workflow. It validates the stable `vX.Y.Z` tag and its version
against the three project manifests, runs the quality gate, builds the installer, and creates a
draft GitHub release with the installer and `.sha256` checksum. The workflow does not publish the
release. Alternatively, dispatch the workflow manually from `main` with an existing stable
`vX.Y.Z` tag.

Review the draft, replace the generated release notes with
`docs/releases/RELEASE_NOTES_2.1.0.md`, verify the attached installer and checksum, and publish the
draft manually. Do not upload databases, backups, syllabus PDFs, build directories, or source
archives generated from unrelated branches.

## Release checklist

- [ ] Version updated consistently
- [ ] Changelog and release notes updated
- [ ] README and docs links valid
- [ ] CI green
- [ ] Local full quality gate green
- [ ] Installer metadata verified
- [ ] MIT license page displayed and `legal\` coverage, licenses, notices, and fonts installed
- [ ] Third-party coverage report reviewed; license texts and attribution verified
- [ ] MPL-2.0 revision-pinned upstream source URLs verified as accessible
- [ ] MPL-2.0 workspace package paths verified (`macros/` for `cssparser-macros`, `selectors/` for `selectors`)
- [ ] MPL-2.0 package manifest versions verified against `Cargo.lock`
- [ ] Exact MPL-2.0 source revisions remain accessible
- [ ] WebView2 download-bootstrapper requirement documented; no offline-installation claim
- [ ] 1.x/2.x separate identity and data-directory behavior documented
- [ ] Installer SHA-256 recorded
- [ ] Unsigned status documented
- [ ] No secrets, databases, backups, PDFs, or generated artifacts included
- [ ] Tag created and pushed as a stable `vX.Y.Z` tag
- [ ] Tag version matches `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`
- [ ] Draft release notes replaced with `docs/releases/RELEASE_NOTES_2.1.0.md`
- [ ] Draft release manually published after verification
- [ ] Release artifacts attached
- [ ] Post-release smoke test performed on a disposable Windows user profile
