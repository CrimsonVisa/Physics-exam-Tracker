# Legal Notices and License Texts

This directory is installed with Physics Tracker and contains the project MIT license, third-party notices, the complete package coverage report, and the full license texts used by the shipped application.

- `LICENSE` - project MIT license.
- `THIRD_PARTY_NOTICES.md` - human-readable third-party summary and installed layout.
- `THIRD_PARTY_COVERAGE.md` - package-by-package audit covering package name, version, classification, license, source, notices, destination, and status.
- `licenses/` - deduplicated full license texts copied from exact package/upstream sources.
- `notices/` - package-specific SPDX/NOTICE material.
- `fonts/` - full OFL texts and notices for the bundled fonts.

License texts are deduplicated by exact SHA-256 content. Package attribution remains in the coverage report. No license text was fabricated; shared text is identified with its source package in the coverage report.

Build-time-only npm and Cargo dependencies are excluded from the installed legal bundle where they are not shipped. The special exception is the Tauri NSIS template tooling, whose MIT/Apache-2.0 notices are included because the template is redistributed.
