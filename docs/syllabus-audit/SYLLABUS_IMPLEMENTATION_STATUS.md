# Syllabus Corrections — Implementation Status

> **Historical implementation snapshot (2026-11-11):** Test totals, Clippy/Prettier results, and Git state below are point-in-time implementation evidence. They are not current release metrics. See this folder's [`README.md`](README.md) for archive status and current release links.

Date: **2026-11-11**

Status: **Approved corrections implemented in source and seeds. No live database migration or application launch was performed.**

## Decision summary

| Area         | Approved decision                                                                                                                      | Implementation result                                                                                                                                                |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| GATE         | Keep GATE 2027 as the sole active GATE syllabus. GATE 2025 is a historical comparison reference only.                                  | No GATE node, ID, code, source attribution, or concept mapping was changed. No GATE 2025 syllabus version was created.                                               |
| CSIR-UGC NET | Apply the six approved wording corrections while preserving Part A/Part B and all IDs.                                                 | Six topic titles corrected in the seed and refreshed in place by stable code.                                                                                        |
| JEST         | Preserve all six headings, 49 nodes and 42 topics. Do not claim the supplied undated PDF independently verifies 2026.                  | No JEST node/title/ID/mapping changed. Active “JEST 2026” version label retained; provenance notes now distinguish the undated supplied reference.                   |
| TIFR         | Retain all eight broad areas; do not invent detailed official topics. Handle the redundant “Broad area” child nodes non-destructively. | All eight units and all eight child nodes remain. The children were relabelled “Coverage note: …”; one referenced child and its historical allocation remain intact. |

## Implementation mechanism

No schema migration was required.

The V2 syllabus seed now supports two explicit, opt-in refresh fields:

- `refreshTitle`: updates an existing node’s title only, matched by its stable `(exam_id, code)` and only when the node is active. The node ID, code, parent, category and display order are unchanged.
- `refreshNotes`: refreshes approved provenance notes on an existing content version and syllabus source. It preserves their IDs, labels, URLs and node relationships.

`prepare` applies these refreshes deterministically. The refresh runs through the normal database preparation path on the next authorized application startup. It was **not** executed against the live database during this implementation, and no live migration was run.

## CSIR-UGC NET corrections applied

All six changes retain the existing topic code/ID, parent unit, Part A/Part B placement and concept mappings.

| Correction | Stable code                                                                                                  | New title                                                                                                                                             |
| ---------- | ------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| CSIR-C01   | `csir-part-a-core-electronics-and-experimental-meth-t001-semiconductor-devices-diodes-junctions-transistors` | Semiconductor devices: diodes, junctions, transistors, field-effect devices, device structure, characteristics, frequency dependence and applications |
| CSIR-C02   | `csir-part-b-advanced-electronics-and-experimental--t002-transducers-for-temperature-pressure-magnetic-fiel` | Transducers for temperature, pressure/vacuum, magnetic fields, vibration, optical measurements and particle detectors                                 |
| CSIR-C03   | `csir-part-b-advanced-electronics-and-experimental--t004-impedance-matching-amplification-filtering-and-noi` | Impedance matching, amplification (op-amp based, instrumentation amplifiers, feedback), filtering and noise reduction                                 |
| CSIR-C04   | `csir-part-b-advanced-nuclear-and-particle-physics-t006-evidence-of-shell-structure-and-single-particle-sh`  | Evidence of shell structure, single-particle shell model, its validity and limitations                                                                |
| CSIR-C05   | `csir-part-b-advanced-nuclear-and-particle-physics-t012-quantum-numbers-gell-mann-nishijima-formula-and-qu`  | Quantum numbers (charge, spin, parity, isospin, strangeness), Gell-Mann-Nishijima formula and quark model                                             |
| CSIR-C06   | `csir-part-b-advanced-condensed-matter-physics-t014-ordered-phases-of-matter-and-liquid-crystalline-or`      | Ordered phases of matter: translational and orientational order, and kinds of liquid crystalline order                                                |

Verified invariants: 158 CSIR nodes, 141 topics, 2 sections (Part A and Part B), 42 CSIR concept mappings, and unchanged node IDs.

## GATE 2027 preservation

GATE 2025 was **not** adopted as an active content version. No `gate-ph-2025-*` nodes or 2025 content version exist.

Verified invariants:

- Active source and content label: **GATE 2027 Physics (PH) Syllabus**.
- Source URL remains the GATE 2027 source.
- 104 nodes, 92 topics, 11 units.
- All existing GATE node IDs and codes are preserved.
- All **36** GATE concept mappings remain present.
- A historical study-session allocation to a GATE 2027 node survives preparation.
- No GATE 2025 nodes, version, or mappings were created.

### Classification of the 19 audit “tracker-only” entries

These entries remain active because GATE 2027 is the chosen baseline. Their relationship to the supplied GATE 2025 comparison is separate from their status in the active 2027 syllabus. The primary classification is therefore **Existing GATE 2027 content** for all 19; five are granular academic enrichment within broader 2027 nodes. All 19 still require verification against the official GATE 2027 PDF.

|   # | Entry                                                                                                  | Primary classification     | Nature / GATE 2025 relationship                               | Verification                            |
| --: | ------------------------------------------------------------------------------------------------------ | -------------------------- | ------------------------------------------------------------- | --------------------------------------- |
|   1 | `gate-ph-2027-measurements-and-error-analysis-t001-units-and-dimensions-dimensional-analysis`          | Existing GATE 2027 content | Academic enrichment; absent from 2025                         | Requires official 2027 PDF confirmation |
|   2 | `gate-ph-2027-measurements-and-error-analysis-t002-least-count-and-significant-figures`                | Existing GATE 2027 content | Academic enrichment; absent from 2025                         | Requires official 2027 PDF confirmation |
|   3 | `gate-ph-2027-measurements-and-error-analysis-t003-methods-of-measurement-and-error-analysis`          | Existing GATE 2027 content | Academic enrichment; absent from 2025                         | Requires official 2027 PDF confirmation |
|   4 | `gate-ph-2027-measurements-and-error-analysis-t004-2-probe-and-4-probe-methods-for-resistance-measure` | Existing GATE 2027 content | Academic enrichment; absent from 2025                         | Requires official 2027 PDF confirmation |
|   5 | `gate-ph-2027-measurements-and-error-analysis-t005-grounding-for-electrical-circuits-and-ground-loops` | Existing GATE 2027 content | Academic enrichment; absent from 2025                         | Requires official 2027 PDF confirmation |
|   6 | `gate-ph-2027-measurements-and-error-analysis-t006-design-of-dc-power-supply`                          | Existing GATE 2027 content | Academic enrichment; absent from 2025                         | Requires official 2027 PDF confirmation |
|   7 | `gate-ph-2027-measurements-and-error-analysis-t007-signal-processing-through-lock-in-amplifiers`       | Existing GATE 2027 content | Academic enrichment; absent from 2025                         | Requires official 2027 PDF confirmation |
|   8 | `gate-ph-2027-optical-physics-t001-wave-equation-plane-and-spherical-waves-superposit`                 | Existing GATE 2027 content | Academic enrichment; absent from 2025                         | Requires official 2027 PDF confirmation |
|   9 | `gate-ph-2027-optical-physics-t002-interference-spatial-and-temporal-coherence-dielec`                 | Existing GATE 2027 content | Academic enrichment; absent from 2025                         | Requires official 2027 PDF confirmation |
|  10 | `gate-ph-2027-optical-physics-t003-michelson-interferometer-fabry-perot-interferomete`                 | Existing GATE 2027 content | Academic enrichment; absent from 2025                         | Requires official 2027 PDF confirmation |
|  11 | `gate-ph-2027-optical-physics-t004-diffraction-fresnel-and-fraunhofer-diffraction-rec`                 | Existing GATE 2027 content | Academic enrichment; absent from 2025                         | Requires official 2027 PDF confirmation |
|  12 | `gate-ph-2027-optical-physics-t005-diffraction-from-double-slit-and-many-slits`                        | Existing GATE 2027 content | Academic enrichment; absent from 2025                         | Requires official 2027 PDF confirmation |
|  13 | `gate-ph-2027-optical-physics-t006-dispersion-by-a-grating`                                            | Existing GATE 2027 content | Academic enrichment; absent from 2025                         | Requires official 2027 PDF confirmation |
|  14 | `gate-ph-2027-optical-physics-t007-polarization-jones-vectors-and-matrices-birefringe`                 | Existing GATE 2027 content | Academic enrichment; absent from 2025                         | Requires official 2027 PDF confirmation |
|  15 | `gate-ph-2027-mathematical-physics-t003-linear-differential-equations-first-and-second-ord`            | Existing GATE 2027 content | Academic enrichment; extra first-order scope relative to 2025 | Requires official 2027 PDF confirmation |
|  16 | `gate-ph-2027-quantum-mechanics-t005-particle-in-single-and-double-delta-function-poten`               | Existing GATE 2027 content | Academic enrichment; not explicit in 2025                     | Requires official 2027 PDF confirmation |
|  17 | `gate-ph-2027-atomic-and-molecular-physics-t004-zeeman-paschen-back-and-stark-effects`                 | Existing GATE 2027 content | Academic enrichment; Paschen-Back not explicit in 2025        | Requires official 2027 PDF confirmation |
|  18 | `gate-ph-2027-atomic-and-molecular-physics-t009-nmr-esr-x-ray-and-mossbauer-spectroscopies`            | Existing GATE 2027 content | Academic enrichment; Mössbauer not explicit in 2025           | Requires official 2027 PDF confirmation |
|  19 | `gate-ph-2027-electronics-t004-waveform-generators-sine-square-and-triangular-wav`                     | Existing GATE 2027 content | Academic enrichment; waveform generators not explicit in 2025 | Requires official 2027 PDF confirmation |

No entry in this table was deleted or relabelled as a GATE 2025 requirement.

## JEST provenance handling

The active JEST syllabus remains unchanged:

- Six subject headings.
- 49 syllabus nodes.
- 42 topics.
- 28 concept mappings.
- All node IDs and titles unchanged.
- Active content label remains **JEST 2026 Physics Syllabus**, because the tracker’s active source is separate from the undated audit PDF.

The JEST source notes and content-version notes now state:

> “Tracker active syllabus retains the JEST 2026 label from the official JEST source. The separately supplied audit reference PDF is undated and does not independently verify the 2026 cycle.”

This is a provenance correction only. No publication year was invented and no JEST topic was added or removed.

## TIFR handling

The supplied TIFR document gives only eight broad subject areas. No detailed official topics were added.

All eight parent units remain. All eight child placeholders remain, but are now labelled `Coverage note: …` instead of `Broad area: …`. This is a non-destructive clarification; it does not claim topic-level official coverage.

Reference inspection before the change:

- TIFR topic progress rows: **0**.
- TIFR concept mappings: **2**, both attached to parent units, not child topics.
- PYQ, timer, checklist, planner, resource, flashcard, sticky-note and quick-start syllabus references to the child topics: **0**.
- One historical allocation exists: session 1 has 4 seconds on `tifr-gs-2024-classical-mechanics-t001-broad-area-classical-mechanics`. That child node was preserved and relabelled; the allocation was verified to survive preparation.

No TIFR child node was deleted. The application has no supported syllabus-node deletion workflow, and one child has historical study data. Relabelling was therefore the safe approved correction.

## ID, mapping and progress preservation

- Total syllabus nodes remain **328**.
- Node counts remain: CSIR 158, GATE 104, TIFR 17, JEST 49.
- Topic counts remain: CSIR 141, GATE 92, TIFR 8, JEST 42.
- Concept mappings remain **108** total: CSIR 42, GATE 36, TIFR 2, JEST 28.
- No progress rows, study sessions, topic allocations, PYQ records, or user data were deleted.
- A CSIR progress row attached to a corrected topic was retained through preparation in the regression test.
- Historical GATE and TIFR session-topic allocations were retained through preparation in the regression tests.
- No duplicate syllabus node was introduced.
- No GATE 2025 node or content version was created.

## Verification

| Check                                       | Result                                                                                                                                                                                                                                                                                                      |
| ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust tests                                  | **PASS** — 479 passed, 3 ignored                                                                                                                                                                                                                                                                            |
| Frontend tests                              | **PASS** — 87 files, 762 tests                                                                                                                                                                                                                                                                              |
| Frontend typecheck                          | **PASS**                                                                                                                                                                                                                                                                                                    |
| Frontend lint                               | **PASS**                                                                                                                                                                                                                                                                                                    |
| `cargo fmt --check`                         | **PASS**                                                                                                                                                                                                                                                                                                    |
| Production `tauri build`                    | **PASS**                                                                                                                                                                                                                                                                                                    |
| Prettier on changed frontend files          | **PASS**                                                                                                                                                                                                                                                                                                    |
| `git diff --check`                          | **PASS**                                                                                                                                                                                                                                                                                                    |
| `cargo clippy --all-targets -- -D warnings` | Strict run still fails on pre-existing `type_complexity` and `needless_option_as_deref` findings in `builtin_constants.rs`, `analytics_charts.rs`, `reference.rs`, and `sticky_notes.rs`. With those two unrelated lint classes allowed, clippy passes; no new warning came from the changed syllabus code. |

The existing full-repository Prettier check still reports the same four pre-existing files (`Physics_Tracker_V2.0_Specification_V1.3.2.md`, `syllabus_seed.json`, `mapping_seed.json`, and the font README). The changed frontend files pass Prettier; the seed file retains its existing formatting convention.

## Remaining source-provenance questions

1. The official GATE 2027 PDF was not part of the supplied audit set. The GATE 2027 node structure is preserved by decision, but the 19 classified entries still require confirmation against that official 2027 document.
2. The supplied JEST PDF is undated. The active JEST 2026 label remains but is not independently verified by that PDF.
3. The supplied TIFR PDF is an undated sample-question paper, not a detailed GS 2024 syllabus. Only its eight broad areas are treated as official.
4. The seed refreshes will be applied to an existing database only when the application next opens it through the normal authorized startup path. That was not performed here.

## Git status

Before implementation: branch `main`, HEAD `80d93b0297608b8ae8d2d74be5a3aea6d13cb406`, staged `0`, tracked modified `33`, untracked `66`.

After implementation: unchanged branch/HEAD, staged `0`, tracked modified `39`, untracked `67`. Seven tracked files were changed by this implementation; the tracked-modified count increased by six because `syllabus.rs` was already modified in the pre-existing working tree. One new implementation-status document was added. Nothing was staged or committed.

## Live-data safety

- The live SQLite database was not modified.
- No live migration, checkpoint, or application launch occurred.
- A read-only copy of DB + WAL + SHM was used only to inspect TIFR references; the copy was removed after inspection.
- The pre-v5 and post-v5 backups were not modified.
- V1 was not accessed.
- No files were staged or committed.
