# Third-Party Notices

Physics Tracker application code is licensed under the MIT license; see `legal/LICENSE`.
The installer also includes the MIT license page and the legal resources under the installed `legal` directory.

The complete package-by-package inventory, exact versions, license identifiers, sources, notices, and bundled destinations is in `THIRD_PARTY_COVERAGE.md` in the installed legal directory (repository path: `legal/THIRD_PARTY_COVERAGE.md`).

## Installed legal directory

```text
legal/LICENSE
legal/THIRD_PARTY_NOTICES.md
legal/THIRD_PARTY_COVERAGE.md
legal/licenses/
legal/notices/
legal/fonts/
```

License texts are deduplicated by exact SHA-256 content. Package attribution is retained in the coverage table, including package name, version, license identifier, authors/source, and destination file. Duplicate standard texts are shared rather than copied once per dependency.

## Coverage summary

- `(MIT OR Apache-2.0) AND Unicode-3.0`: 1 package records
- `0BSD OR MIT OR Apache-2.0`: 1 package records
- `Apache-2.0`: 3 package records
- `Apache-2.0 / MIT`: 1 package records
- `Apache-2.0 AND MIT`: 1 package records
- `Apache-2.0 OR ISC OR MIT`: 2 package records
- `Apache-2.0 OR MIT`: 28 package records
- `Apache-2.0/MIT`: 1 package records
- `BSD-3-Clause`: 3 package records
- `BSD-3-Clause AND MIT`: 1 package records
- `BSD-3-Clause/MIT`: 1 package records
- `CC0-1.0 OR MIT-0 OR Apache-2.0`: 1 package records
- `ISC`: 4 package records
- `ISC AND (Apache-2.0 OR ISC)`: 1 package records
- `ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0)`: 1 package records
- `MIT`: 55 package records
- `MIT / Apache-2.0`: 1 package records
- `MIT OR Apache-2.0`: 134 package records
- `MIT OR Apache-2.0 OR Zlib`: 1 package records
- `MIT OR Zlib OR Apache-2.0`: 2 package records
- `MIT/Apache-2.0`: 6 package records
- `MPL-2.0`: 8 package records
- `OFL-1.1`: 6 package records
- `Unicode-3.0`: 15 package records
- `Unlicense OR MIT`: 4 package records
- `Unlicense/MIT`: 2 package records
- `Zlib`: 1 package records

## Runtime and bundled assets

- The frontend runtime closure includes `@tauri-apps/api`, Tauri dialog/notification/opener plugins, KaTeX, React, React DOM, and scheduler. `commander` is a KaTeX CLI dependency and is documented but not observed in the bundled browser output.
- The normal Rust dependency closure is listed in the coverage report. It includes MIT, Apache-2.0, MPL-2.0, Unicode-3.0, BSD, ISC, Unlicense, Zlib, and other licenses.
- Bundled fonts: Computer Modern Unicode, Inter, Manrope, IBM Plex Sans, Plus Jakarta Sans, and JetBrains Mono; all full OFL texts are installed under `legal/fonts`.
- The NSIS template is derived from `@tauri-apps/cli` 2.12.4; its MIT and Apache-2.0 texts are bundled under `legal/licenses`.

## Build-time exclusions

Development-only npm packages, Cargo build dependencies, and Cargo dev dependencies are not part of the installed application and are not bundled. The basis for exclusions is that they are not present in the frontend bundle or the normal Rust dependency closure; `@tauri-apps/cli` is included specially because its NSIS template is redistributed. The full exclusion rationale is recorded in the coverage report.

## MPL-2.0 source availability

MPL-2.0 applies to the identified covered dependencies `cssparser` 0.35.0 and 0.37.0,
`cssparser-macros` 0.6.1 and 0.7.1, `dtoa-short` 0.3.5, `option-ext` 0.2.0, and `selectors` 0.31.0
and 0.38.0. The full MPL-2.0 text is bundled under `legal/licenses/MPL-2.0-*.txt`.

The corresponding source code is available from the upstream repositories at these revision-pinned
locations:

- `cssparser` 0.35.0: <https://github.com/servo/rust-cssparser/tree/3dd4f63639925438d9932dafc0d81395847f9be4>
- `cssparser` 0.37.0: <https://github.com/servo/rust-cssparser/tree/4c49486494fb24dc01390e3baca9698ef1744c71>
- `cssparser-macros` 0.6.1 (`macros/`): <https://github.com/servo/rust-cssparser/tree/0ebd17bcd13808f087b083f34d2f816e380b5155/macros>
- `cssparser-macros` 0.7.1 (`macros/`): <https://github.com/servo/rust-cssparser/tree/d128a90e1203fc4afb959420d6dc5e24f77a3191/macros>
- `dtoa-short` 0.3.5: <https://github.com/upsuper/dtoa-short/tree/2d905cdb8b2e08163dc0d015f529877fe657b4ef>
- `option-ext` 0.2.0: <https://github.com/soc/option-ext/tree/272f22fc9ea1ac6b08f01704af52c4ac338df4e2>
- `selectors` 0.31.0 (`selectors/`): <https://github.com/servo/stylo/tree/7f8df16fddb15b6ac9097f8ab25caffff1a7095e/selectors>
- `selectors` 0.38.0 (`selectors/`): <https://github.com/servo/stylo/tree/572ecba2d1600e7c3d490586692a209faf703baa/selectors>

`cssparser-macros` and `selectors` are workspace members; their package manifests and source files
are in the `macros/` and `selectors/` subdirectories, not at the repository root. No source archive
is included with this installer. Release verification must confirm that the recorded source links
remain accessible and correspond to the versions being distributed.

## Remaining legal review items

- The AWS-LC upstream `NOTICE` from `aws/aws-lc-rs` commit
  `5d496f88b40f327e28315deecc1a5a1fd1e81e95` is included at
  `legal/notices/aws-lc-rs-NOTICE-5d496f88.txt`.
- The `regex-syntax` 0.8.11 Unicode license is included at
  `legal/licenses/Unicode-3.0-regex-syntax-14016799.txt`.
- No other standalone Apache NOTICE files were found among the normal Cargo package archives at
  audit time; any embedded attribution material in the AWS-LC root license is included.
- Re-run the audit if dependency versions or the lockfiles change. No dependency versions were
  changed for this bundle.
