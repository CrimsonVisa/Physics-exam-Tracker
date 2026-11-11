# Syllabus Audit — Executive Summary

Audit date: **2026-11-11**  
Repository: `C:\Users\visa1\Documents\Application\csir-net-tracker`  
Branch: `main`  
HEAD: `80d93b0297608b8ae8d2d74be5a3aea6d13cb406`

> **Implementation status (2026-11-11):** The approved corrections have been implemented. GATE 2027 remains the sole active GATE syllabus; the proposed GATE 2025 re-baseline was **not** adopted. CSIR wording, JEST provenance notes and TIFR placeholder relabelling are complete. See `SYLLABUS_IMPLEMENTATION_STATUS.md` for applied changes, preservation results and remaining source-verification questions.

## Scope and result

This was an audit-only phase. Four supplied PDFs were extracted page-by-page with rendered-page spot checks, and compared against the actual V2 syllabus seed, node hierarchy, exam mappings and live logical state. No database, seed, mapping, progress, PYQ or production file was modified; no application was launched.

### Overall counts

Status counts below are over the 309 extracted reference entries. Cross-cutting findings (baseline, mapping, data risk) and duplicate-node findings are counted separately.

| Measure                                       | Result |
| --------------------------------------------- | -----: |
| Reference entries extracted and audited       |    309 |
| Tracker nodes audited                         |    328 |
| Tracker topic nodes audited                   |    283 |
| MATCH                                         |    281 |
| PARTIAL                                       |     10 |
| MISSING                                       |     13 |
| MISCLASSIFIED                                 |      4 |
| TERMINOLOGY DIFFERENCE                        |      1 |
| DUPLICATE findings                            |      9 |
| Tracker-only NOT EXPLICITLY SPECIFIED entries |     19 |

### Exam-by-exam

| Exam                   | Reference entries | Tracker nodes/topics | MATCH | PARTIAL | MISSING | MISCLASSIFIED | Other                        |
| ---------------------- | ----------------: | -------------------: | ----: | ------: | ------: | ------------: | ---------------------------- |
| CSIR-UGC NET           |               141 |            158 / 141 |   135 |       5 |       0 |             0 | 1 terminology difference     |
| GATE Physics 2025      |               118 |             104 / 92 |    96 |       5 |      13 |             4 | 1 duplicate; 19 tracker-only |
| JEST Physics           |                42 |              49 / 42 |    42 |       0 |       0 |             0 | provenance caveat            |
| TIFR Physical Sciences |                 8 |               17 / 8 |     8 |       0 |       0 |             0 | 8 redundant child topics     |

## Key conclusions

### CSIR-UGC NET — strong and near-complete

The CSIR track is the strongest match. All 141 audited topics are present, with Part A Core and Part B Advanced correctly separated. There are **no missing, misclassified or duplicate error findings**. Five topics lose explicit qualifiers and one uses terminology that obscures the source: frequency dependence in semiconductor devices, pressure/vacuum and particle detectors in transducers, amplification qualifiers, shell-model validity/limitations, the particle quantum-number list, and liquid-crystal order distinctions.

### GATE — critical baseline mismatch

The supplied official reference is **GATE 2025**, which has nine sections and no separate Measurements or Optical Physics unit. The tracker identifies itself as **GATE 2027**, cites a GATE 2027 URL, and contains eleven units. This is a **CRITICAL** baseline mismatch: the tracker cannot claim complete official GATE 2025 coverage, and the 2027 content cannot be verified because only the 2025 document was supplied.

After the baseline issue, the largest explicit GATE 2025 gaps are:

- Special functions in mathematical physics
- Laplace transform
- Liouville’s theorem, action-angle variables and Hamilton-Jacobi theory
- Radiation from a moving charge
- WKB approximation and symmetries in quantum mechanics
- Tight binding, optical properties of solids, Kramers-Kronig and inter/intra-band transitions
- Nuclear radii and charge distributions
- Lasers classified under Optical Physics rather than Atomic/Molecular Physics
- Semiconductor equilibrium and metal-semiconductor junctions placed under Solid State rather than Electronics

A low-severity unit-title difference also exists: the tracker says “Thermodynamics and Statistical Mechanics”, while GATE 2025 says “Thermodynamics and Statistical Physics”. The eight topic clauses themselves match.

### JEST — complete wording, unverified cycle

All 42 JEST topics match the supplied six-heading syllabus on pages 2-3. The tracker correctly uses six headings, not the informal five-area description. However, the supplied PDF is undated and cannot confirm the tracker’s JEST 2026 label.

### TIFR — broad areas only

All eight TIFR broad areas match. The supplied document gives no detailed topic list and explicitly describes the standard as undergraduate/master’s-level Physics. Therefore no topic-by-topic TIFR gap can be asserted. The eight child topics titled “Broad area: …” are semantic duplicates of their parent units and should be treated as coverage notes, not official topics.

## Critical and high-severity items

| ID                                   | Severity | Finding                                                          | Consequence                                                                                                     |
| ------------------------------------ | -------- | ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| GATE-BASELINE-01                     | CRITICAL | GATE 2027 tracker vs GATE 2025 reference                         | All GATE official-coverage claims and the existing GATE mappings use the wrong baseline for the supplied audit  |
| GATE-MAPPING-01                      | HIGH     | All 36 GATE mappings point to 2027 node codes                    | A 2025 re-baseline requires a mapping migration                                                                 |
| GATE-DATA-01                         | HIGH     | Historical session allocation points to a 2027 Measurement topic | The node cannot be deleted; historical study time must be preserved                                             |
| GATE-004/006/016/019/034/046/078/099 | HIGH     | Missing explicit GATE 2025 topics                                | Special functions, Laplace, Liouville, Hamilton-Jacobi, radiation, WKB, solid optical properties, nuclear radii |
| GATE-068                             | HIGH     | Lasers misclassified under Optical Physics                       | GATE 2025 places lasers in Atomic/Molecular Physics                                                             |

## Source documents accessed

| Exam             | File                                           | PDF pages | SHA-256                                                            | Version observation                                       |
| ---------------- | ---------------------------------------------- | --------: | ------------------------------------------------------------------ | --------------------------------------------------------- |
| csir-net-physics | `20201221140202824physicalscience_syllbus.pdf` |         3 | `A535B91E52DED1F85F4EC08FE101450D23130C6DF916F760B560AF511E33A468` | 2020 PDF (creation/modification metadata 2020-12-21)      |
| gate-physics     | `GATE _PH_2025_Syllabus.pdf`                   |         3 | `0F4C42A3DA21EBDBE31E949430D8128DEB110B943426C137B91698049E7FDBB3` | GATE 2025 (logo on page 1; metadata created 2024-06-28)   |
| jest-physics     | `JEST Syllabus.pdf`                            |         3 | `0179B991EF259352977649C1E555D5C2272186F36F30693F941D5A7CBD7A6287` | Undated JEST syllabus; PDF metadata modified 2021-07-30   |
| tifr-gs-physics  | `Paper_Physics.pdf`                            |         2 | `4278CFF8F1A94E0CCFBDEF5D4EB72AA056F55A52B844A5B50714B9F13DEA508A` | Undated sample-question document; PDF metadata 2005-09-15 |

All four PDFs were accessible. No document was unavailable. The TIFR file is a sample-question paper with a broad syllabus statement, not a detailed GS 2024 syllabus; the JEST file is undated.

## TIFR limitation

The TIFR audit is intentionally limited to eight broad areas. The document says the syllabus is “typically a course of studies in Physics as the main subject in the Undergraduate and Masters levels” and lists eight areas. It does not specify sub-topics. Any TIFR subtopic in the overlap matrix is marked **inferred**, never official.

## Evidence and deliverables

- `SYLLABUS_DETAILED_COMPARISON.md` — full section/topic matrix and non-match explanations.
- `SYLLABUS_CROSS_EXAM_OVERLAP.md` — shared-topic matrix and depth distinctions.
- `SYLLABUS_CORRECTION_PROPOSAL.md` — historical proposed changes, preserved IDs, migration risks and the non-adopted GATE 2025 hierarchy.
- `SYLLABUS_IMPLEMENTATION_STATUS.md` — approved implementation record, GATE 2027 preservation, JEST provenance handling, TIFR relabelling and test results.
- `syllabus_audit.json` — machine-readable reference inventory, statuses, source pages, explicit subtopic fragments, the complete extracted page text for all four PDFs, live-state snapshot, overlap, correction proposal and implementation status.

## Mapping observations

- `mapping_seed.json` contains 108 curated/proposed mappings and **zero duplicate (concept, exam, node) triples**.
- The live database has one approved mapping (CSIR Part A Classical Mechanics) despite the seed being entirely proposed. That review state is user data and must be preserved.
- All 36 GATE mappings target GATE 2027 node codes, so a 2025 re-baseline requires remapping.
- TIFR has mappings for only two of eight broad areas; this is a curation gap, not an official syllabus gap.
- No mapping duplicate was found. The nine duplicate findings are syllabus-node duplicates: eight redundant TIFR “Broad area” child topics and one GATE nuclear-models overlap.

## Findings requiring manual review

1. **GATE baseline:** confirm whether GATE 2025 or GATE 2027 is the authorised target. No GATE content migration should begin before this decision.
2. **JEST and TIFR document cycle:** the supplied files are undated. Confirm the official 2026 JEST and GS 2024 TIFR documents, or adjust the tracker provenance labels.
3. **GATE enrichment:** decide whether the 19 tracker-only 2027 items remain visible as enrichment or are archived with the 2027 content version. They should not be deleted solely because the 2025 PDF does not name them.
4. **TIFR child topics:** confirm that no external workflow references the eight “Broad area: …” child nodes before archiving or relabelling them.
5. **GATE migration design:** decide whether to accept new `gate-ph-2025-*` codes (clean provenance) or retain 2027-prefixed codes under a 2025 source (fewer IDs, misleading codes). The proposal recommends new 2025 codes and archived legacy IDs.
6. **CSIR optional wording:** approve which low-risk wording completions are worth applying. None affects IDs, progress or mappings.

## Data and safety confirmation

- The live database was **not modified**. A read-only copy of DB + WAL + SHM was used only to quantify reference risk.
- No schema migration, seed change, mapping change, progress change or PYQ change was made.
- No production application was launched.
- V1 was not accessed.
- No backup was modified.
- Git work was not staged, committed, reset, cleaned, restored or stashed.

## Git status

Before this audit’s five new deliverables were added, the working tree had branch `main`, HEAD `80d93b0297608b8ae8d2d74be5a3aea6d13cb406`, staged `0`, tracked modified `33`, and untracked `61` (including the four supplied PDFs and all pre-existing uncommitted work).

After adding the five deliverables, the status is branch `main`, unchanged HEAD, staged `0`, tracked modified `33`, and untracked `66`. The five audit files account for the entire increase. `git status --short -- docs/syllabus-audit` lists the folder as untracked. Nothing was staged or committed.

## Stop condition

The audit and correction proposal are complete. No correction has been implemented. Implementation must wait for explicit approval of the GATE baseline and the proposed changes.
