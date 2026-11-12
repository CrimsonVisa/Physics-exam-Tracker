# Release Process

## Versioning

Physics Tracker uses semantic versioning for the application release line. The current application
version is `2.0.0`.

- Application version: `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`
- Git tag: `v2.0.0`
- Installer: `Physics Tracker_2.0.0_x64-setup.exe`

Update all three version declarations together for a release. Do not add a version suffix to the
display product name; the product is always **Physics Tracker**.

## Release prerequisites

- Clean `main` working tree
- CI green on the release commit
- Prettier, TypeScript, ESLint, Rust formatting, and Clippy green
- Frontend and Rust tests green
- README, feature guide, changelog, and release notes current
- Installer metadata verified
- MIT license text present in `LICENSE` and included with the installer
- Third-party notices current, including Apache-2.0, MPL-2.0, OFL-1.1, and attribution obligations
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
$installer = 'src-tauri\target\release\bundle\nsis\Physics Tracker_2.0.0_x64-setup.exe'

Get-Item $installer | Select-Object FullName,Length
(Get-Item $installer).VersionInfo |
  Select-Object ProductName,FileDescription,ProductVersion,FileVersion
Get-FileHash $installer -Algorithm SHA256
```

The installer is ignored by Git and must not be committed.

## GitHub Actions

- `.github/workflows/ci.yml` runs the Windows quality gate for pushes to `main` and pull requests.
- `.github/workflows/release.yml` builds and uploads the installer when a `v*` tag is pushed, or
  when the workflow is dispatched manually with an existing tag.
- `.github/dependabot.yml` proposes npm, Cargo, and GitHub Actions updates weekly.

Pin third-party Actions to immutable commit SHAs before treating the workflows as hardened supply
chain controls. The initial workflows use major-version tags for readability.

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
git tag -a v2.0.0 -m "Physics Tracker 2.0.0"
git push origin main
git push origin v2.0.0
```

The tag should point to the release commit, not a dirty working tree. Do not push experimental or
local-preservation branches.

Publish the NSIS installer, its SHA-256 checksum, and the release notes through GitHub Releases.
Do not upload databases, backups, syllabus PDFs, build directories, or source archives generated
from unrelated branches.

## Release checklist

- [ ] Version updated consistently
- [ ] Changelog and release notes updated
- [ ] README and docs links valid
- [ ] CI green
- [ ] Local full quality gate green
- [ ] Installer metadata verified
- [ ] MIT license text included in the installer/release assets
- [ ] Third-party notices and attribution obligations verified
- [ ] Installer SHA-256 recorded
- [ ] Unsigned status documented
- [ ] No secrets, databases, backups, PDFs, or generated artifacts included
- [ ] Tag created and pushed
- [ ] Release artifacts attached
- [ ] Post-release smoke test performed on a disposable Windows user profile
