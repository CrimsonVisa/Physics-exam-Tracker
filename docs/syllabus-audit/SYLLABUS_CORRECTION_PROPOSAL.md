# Syllabus Correction Proposal

Audit date: **2026-11-11**

> **Historical status: this was the original proposal.** The approved implementation is recorded in `SYLLABUS_IMPLEMENTATION_STATUS.md`. The GATE 2027 → 2025 re-baseline was **not** adopted; GATE 2027 remains the sole active GATE syllabus. CSIR wording corrections, JEST provenance notes and TIFR placeholder relabelling were implemented without changing node IDs or mappings.

## Decision gates

1. **GATE baseline:** decide whether the authorised baseline is GATE 2025 (the supplied document) or GATE 2027 (the current tracker source). Do not implement a mixed baseline. The proposed hierarchy below assumes GATE 2025 because that is the supplied reference.
2. **JEST/TIFR provenance:** confirm the exact cycle documents. The supplied files are undated and cannot prove the tracker’s 2026/2024 labels.
3. **TIFR granularity:** keep broad areas only. Do not turn inferred study topics into official TIFR requirements.
4. **Data migration:** any GATE re-baseline must preserve historical session allocations, mapping review state and future progress. No deletion of referenced nodes.

## CSIR-UGC NET — exact in-place corrections

These changes preserve every existing node code and ID. They are title/notes corrections only.

| ID       | Existing entry                                                                                               | Proposed entry                                                                                                                                        | Parent unit                                                 | Reason                                                                                                                                                         | Source / page                                     | Requirement                          | Risk                                              |
| -------- | ------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- | ------------------------------------ | ------------------------------------------------- |
| CSIR-C01 | `csir-part-a-core-electronics-and-experimental-meth-t001-semiconductor-devices-diodes-junctions-transistors` | Semiconductor devices: diodes, junctions, transistors, field-effect devices, device structure, characteristics, frequency dependence and applications | `csir-part-a-core-electronics-and-experimental-methods`     | The CSIR source explicitly includes frequency dependence, which is absent from the current topic and its sibling.                                              | 20201221140202824physicalscience_syllbus.pdf p. 2 | REQUIRED wording/coverage correction | LOW. Progress, mappings and node ID remain valid. |
| CSIR-C02 | `csir-part-b-advanced-electronics-and-experimental--t002-transducers-for-temperature-pressure-magnetic-fiel` | Transducers for temperature, pressure/vacuum, magnetic fields, vibration, optical measurements and particle detectors                                 | `csir-part-b-advanced-electronics-and-experimental-methods` | The source says pressure/vacuum and particle detectors; the tracker says pressure and particle measurements. The latter changes the intended instrument class. | 20201221140202824physicalscience_syllbus.pdf p. 2 | REQUIRED terminology correction      | LOW. No ID or mapping change.                     |
| CSIR-C03 | `csir-part-b-advanced-electronics-and-experimental--t004-impedance-matching-amplification-filtering-and-noi` | Impedance matching, amplification (op-amp based, instrumentation amplifier, feedback), filtering and noise reduction                                  | `csir-part-b-advanced-electronics-and-experimental-methods` | The source explicitly qualifies amplification; the tracker’s generic wording omits instrumentation amplifiers and feedback.                                    | 20201221140202824physicalscience_syllbus.pdf p. 3 | REQUIRED wording/coverage correction | LOW.                                              |
| CSIR-C04 | `csir-part-b-advanced-nuclear-and-particle-physics-t006-evidence-of-shell-structure-and-single-particle-sh`  | Evidence of shell structure, single-particle shell model, its validity and limitations                                                                | `csir-part-b-advanced-nuclear-and-particle-physics`         | The source explicitly asks for validity and limitations of the shell model.                                                                                    | 20201221140202824physicalscience_syllbus.pdf p. 3 | OPTIONAL wording completion          | LOW.                                              |
| CSIR-C05 | `csir-part-b-advanced-nuclear-and-particle-physics-t012-quantum-numbers-gell-mann-nishijima-formula-and-qu`  | Quantum numbers (charge, spin, parity, isospin, strangeness), Gell-Mann-Nishijima formula and quark model                                             | `csir-part-b-advanced-nuclear-and-particle-physics`         | The source enumerates the quantum numbers; the tracker’s generic phrase hides the required list.                                                               | 20201221140202824physicalscience_syllbus.pdf p. 3 | OPTIONAL wording completion          | LOW.                                              |
| CSIR-C06 | `csir-part-b-advanced-condensed-matter-physics-t014-ordered-phases-of-matter-and-liquid-crystalline-or`      | Ordered phases of matter: translational and orientational order, and kinds of liquid crystalline order                                                | `csir-part-b-advanced-condensed-matter-physics`             | The source distinguishes translational and orientational order and kinds of liquid-crystalline order; the tracker compresses them.                             | 20201221140202824physicalscience_syllbus.pdf p. 3 | OPTIONAL wording completion          | LOW.                                              |

### Required CSIR actions

- Add the explicit “frequency dependence” requirement to the Part A semiconductor topic.
- Restore “pressure/vacuum” and “particle detectors” in the Part B transducer topic.
- Restore the amplification qualifiers “op-amp based, instrumentation amplifier, feedback”.

### Optional CSIR wording actions

- Shell-model validity/limitations, the particle quantum-number list, and the translational/orientational liquid-crystal distinction improve fidelity but do not change the underlying topic coverage.

## JEST — provenance correction

| ID       | Existing entry                                               | Proposed entry                                                                                                 | Source / page            | Requirement                                           | Risk                                      |
| -------- | ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------- | ------------------------ | ----------------------------------------------------- | ----------------------------------------- |
| JEST-C01 | JEST 2026 Physics Syllabus content version and source record | Verify the exact JEST 2026 official PDF or relabel the content version/source to the supplied undated document | JEST Syllabus.pdf p. 1-3 | REQUIRED provenance correction before official claims | LOW for content; no user progress impact. |

No JEST topic wording change is proposed. All 42 audited JEST topics match the supplied pages 2-3. The only issue is that the supplied file cannot confirm the 2026 cycle label.

## TIFR — broad-area structure

| ID       | Existing entry                              | Proposed entry                                                                                                                                                                 | Source / page          | Requirement                 | Risk                                                                                                       |
| -------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------- | --------------------------- | ---------------------------------------------------------------------------------------------------------- |
| TIFR-C01 | Eight child topics titled “Broad area: …”   | Keep eight parent units; archive or relabel the eight redundant child topics as coverage notes, not official topics                                                            | Paper_Physics.pdf p. 1 | OPTIONAL structural cleanup | LOW. Live progress is empty and mappings target the parent units, but confirm references before archiving. |
| TIFR-C02 | Two of eight broad areas linked to concepts | Optionally add curated concept mappings for Mathematics, Electricity and Magnetism, Heat/Thermodynamics, General Physics, Modern Physics, and Electronics/Experimental Physics | Paper_Physics.pdf p. 1 | OPTIONAL mapping curation   | LOW. Mapping review state must be preserved.                                                               |

**Do not add detailed TIFR subtopics as official content.** If the learner wants a study checklist, keep it in a separate annotated layer marked “inferred”, not in the official syllabus nodes.

## GATE — baseline decision and proposed 2025 hierarchy

### Why a baseline decision is mandatory

The tracker currently identifies itself as **GATE 2027** and uses 11 units. The supplied official reference is **GATE 2025** and has nine sections. Node codes containing `gate-ph-2027` are immutable under the current schema trigger and cannot be renamed in place. Therefore a 2025 re-baseline must be treated as a new content version, not a cosmetic title edit.

### Proposed GATE 2025 root and units

| Proposed unit code                                    | Proposed title                         | GATE 2025 section | Topic action                                                                                                                                                             |
| ----------------------------------------------------- | -------------------------------------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `gate-ph-2025`                                        | GATE 2025 Physics (PH) Syllabus        | Root section      | New root for the 2025 content version                                                                                                                                    |
| `gate-ph-2025-mathematical-physics`                   | Mathematical Physics                   | Section 1         | Carry equivalent topics; add special functions and Laplace transform; add complex-analysis applications                                                                  |
| `gate-ph-2025-classical-mechanics`                    | Classical Mechanics                    | Section 2         | Carry equivalent topics; add Liouville, action-angle variables, Hamilton-Jacobi; add classical Rutherford scattering                                                     |
| `gate-ph-2025-electromagnetic-theory`                 | Electromagnetic Theory                 | Section 3         | Carry equivalent topics; add radiation from a moving charge                                                                                                              |
| `gate-ph-2025-quantum-mechanics`                      | Quantum Mechanics                      | Section 4         | State postulates and Dirac notation; add WKB and symmetries                                                                                                              |
| `gate-ph-2025-thermodynamics-and-statistical-physics` | Thermodynamics and Statistical Physics | Section 5         | Carry the eight equivalent topics                                                                                                                                        |
| `gate-ph-2025-atomic-and-molecular-physics`           | Atomic and Molecular Physics           | Section 6         | Carry t001–t009; add lasers from the legacy optical unit; keep Mössbauer as optional enrichment                                                                          |
| `gate-ph-2025-solid-state-physics`                    | Solid State Physics                    | Section 7         | Amend band theory for tight binding; add optical properties, Kramers-Kronig, inter/intra-band transitions, dielectric function, and detailed magnetic classes/anisotropy |
| `gate-ph-2025-electronics`                            | Electronics                            | Section 8         | Carry t001–t007; add semiconductor equilibrium and metal-semiconductor/ohmic/rectifying topics; retain waveform generators as optional enrichment                        |
| `gate-ph-2025-nuclear-and-particle-physics`           | Nuclear and Particle Physics           | Section 9         | Carry equivalent topics; add nuclear radii/charge distributions; narrow the t002/t003 “nuclear models” overlap                                                           |

### Legacy unit mapping

| GATE 2027 legacy unit                                | Proposed GATE 2025 treatment                                                                                   |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `gate-ph-2027-mathematical-physics`                  | Create `gate-ph-2025-mathematical-physics`; migrate equivalent topics                                          |
| `gate-ph-2027-classical-mechanics`                   | Create `gate-ph-2025-classical-mechanics`; migrate equivalent topics                                           |
| `gate-ph-2027-electromagnetic-theory`                | Create `gate-ph-2025-electromagnetic-theory`; migrate equivalent topics                                        |
| `gate-ph-2027-quantum-mechanics`                     | Create `gate-ph-2025-quantum-mechanics`; migrate equivalent topics                                             |
| `gate-ph-2027-thermodynamics-and-statistical-mechan` | Create `gate-ph-2025-thermodynamics-and-statistical-physics`; migrate equivalent topics and use the 2025 title |
| `gate-ph-2027-atomic-and-molecular-physics`          | Create `gate-ph-2025-atomic-and-molecular-physics`; migrate equivalent topics; add relocated lasers            |
| `gate-ph-2027-solid-state-physics`                   | Create `gate-ph-2025-solid-state-physics`; migrate equivalent topics; add optical/dielectric/magnetic detail   |
| `gate-ph-2027-electronics`                           | Create `gate-ph-2025-electronics`; migrate equivalent topics; add semiconductor/contact topics                 |
| `gate-ph-2027-nuclear-and-particle-physics`          | Create `gate-ph-2025-nuclear-and-particle-physics`; migrate equivalent topics; add nuclear radii               |
| `gate-ph-2027-measurements-and-error-analysis`       | No 2025 equivalent; archive as legacy/enrichment                                                               |
| `gate-ph-2027-optical-physics`                       | No 2025 equivalent; move lasers, retain EM polarization, archive other optical topics as enrichment            |

### Exact addition codes

| Proposed code                                                                               | Title                                                                         | Source page | Requirement          |
| ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ----------- | -------------------- |
| `gate-ph-2025-mathematical-physics-t007-special-functions`                                  | Special functions and solutions of second-order linear differential equations | p. 1        | REQUIRED             |
| `gate-ph-2025-mathematical-physics-t008-laplace-transform`                                  | Laplace transform                                                             | p. 1        | REQUIRED             |
| `gate-ph-2025-classical-mechanics-t009-liouville-theorem`                                   | Liouville’s theorem                                                           | p. 1        | REQUIRED             |
| `gate-ph-2025-classical-mechanics-t010-action-angle-variables`                              | Action-angle variables                                                        | p. 1        | REQUIRED             |
| `gate-ph-2025-classical-mechanics-t011-hamilton-jacobi-equation`                            | Hamilton-Jacobi equation                                                      | p. 1        | REQUIRED             |
| `gate-ph-2025-electromagnetic-theory-t009-radiation-from-a-moving-charge`                   | Radiation from a moving charge                                                | p. 1        | REQUIRED             |
| `gate-ph-2025-quantum-mechanics-t011-wkb-approximation`                                     | WKB approximation                                                             | p. 1        | REQUIRED             |
| `gate-ph-2025-quantum-mechanics-t012-symmetries-in-quantum-mechanics`                       | Symmetries in quantum mechanical systems                                      | p. 1        | REQUIRED             |
| `gate-ph-2025-solid-state-physics-t012-optical-properties-of-solids`                        | Optical properties of solids                                                  | p. 2        | REQUIRED             |
| `gate-ph-2025-solid-state-physics-t013-kramers-kronig-relation`                             | Kramers-Kronig relation                                                       | p. 2        | REQUIRED             |
| `gate-ph-2025-solid-state-physics-t014-intra-and-inter-band-transitions`                    | Intra- and inter-band transitions                                             | p. 2        | REQUIRED             |
| `gate-ph-2025-atomic-and-molecular-physics-t010-lasers`                                     | Lasers: Einstein coefficients, population inversion, two/three-level systems  | p. 2        | REQUIRED (relocated) |
| `gate-ph-2025-electronics-t008-semiconductors-in-equilibrium`                               | Electron and hole statistics in intrinsic and extrinsic semiconductors        | p. 2        | REQUIRED             |
| `gate-ph-2025-electronics-t009-metal-semiconductor-junctions-ohmic-and-rectifying-contacts` | Metal-semiconductor junctions; ohmic and rectifying contacts                  | p. 2        | REQUIRED             |

The exact topic numbering for carried topics must follow the 2025 unit and source order. Do not reuse a 2027 code for a new 2025 topic. The list above only fixes codes for genuinely new/relocated topics. All proposed `gate-ph-2025-*` codes were checked against all 328 existing tracker codes; there are no collisions.

### Amendments to carried GATE 2025 topics

| Carried topic code                                                        | Proposed amendment                                                                               | Source page |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ | ----------- |
| `gate-ph-2025-mathematical-physics-t004-complex-analysis`                 | Add “applications” to the existing complex-analysis topic                                        | p. 1        |
| `gate-ph-2025-quantum-mechanics-t001-basic-ideas-and-uncertainty`         | State the postulates of quantum mechanics explicitly                                             | p. 1        |
| `gate-ph-2025-quantum-mechanics-t002-hilbert-space`                       | Add Dirac bra-ket notation                                                                       | p. 1        |
| `gate-ph-2025-classical-mechanics-t003-central-force-motion`              | Add Rutherford scattering in the classical central-force context                                 | p. 1        |
| `gate-ph-2025-solid-state-physics-t005-band-theory`                       | Add the tight-binding model alongside the nearly-free-electron model                             | p. 2        |
| `gate-ph-2025-solid-state-physics-t009-dielectric-properties`             | Add the dielectric function                                                                      | p. 2        |
| `gate-ph-2025-solid-state-physics-t010-magnetic-properties`               | Enumerate dia/para/ferro/antiferro/ferri magnetism, domains and anisotropy                       | p. 2        |
| `gate-ph-2025-nuclear-and-particle-physics-t001-basic-nuclear-properties` | Add nuclear radii and charge distributions                                                       | p. 2        |
| `gate-ph-2025-nuclear-and-particle-physics-t002-nuclear-models`           | Narrow the general “nuclear models” wording or cross-reference the liquid-drop/shell-model topic | p. 2        |

### Legacy GATE 2027 content and references

| Legacy area                                               | Proposed treatment                                                                                                      | Reason                                                                                                |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `gate-ph-2027-measurements-and-error-analysis` (7 topics) | Archive; retain for historical/allocation references; optionally keep as enrichment outside the verified 2025 version   | No GATE 2025 measurement section                                                                      |
| `gate-ph-2027-optical-physics` (8 topics)                 | Move the laser concept to Atomic/Molecular; retain EM polarization; archive other optical topics as optional enrichment | No separate GATE 2025 optical section; interference/diffraction/grating/Jones content is not explicit |
| All equivalent 2027 topics                                | Preserve old node IDs/codes as archived legacy; create 2025 equivalents and record old→new migration rows               | Node codes are immutable; historical references must survive                                          |

### Data and mapping risk assessment

| Risk                          | Current live evidence                                                                                                                                             | Required safeguard                                                                                        |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| GATE concept mappings         | 36 curated/proposed mappings, all to 2027 nodes                                                                                                                   | Do not overwrite; create new 2025 mappings through review workflow; preserve old mapping rows for history |
| Historical session allocation | Session 1 has 4 s on `gate-ph-2027-measurements-and-error-analysis-t001-units-and-dimensions-dimensional-analysis`                                                | Archive the legacy node; preserve the allocation and historical title; do not delete or repoint silently  |
| Topic progress                | 0 `exam_node_progress` rows at audit time                                                                                                                         | Still design migration for future progress; never assume the live state remains empty                     |
| Other syllabus references     | 0 rows in sessions’ direct node field, checklist, planned blocks, timer state, resources, flashcards, sticky notes, quick-start allocations; 0 PYQ topic mappings | Recheck immediately before implementation; protect all FK references                                      |
| Analytics/readiness           | Joins use exam node/concept links                                                                                                                                 | Rebuild/verify after mapping migration; do not leave orphaned 2027 mappings active                        |
| Node codes                    | `trg_syllabus_code_immutable` prevents code edits                                                                                                                 | Use new 2025 codes for new nodes; use `exam_syllabus_node_migrations` for old→new relationships           |

### GATE classification moves regardless of baseline choice

- **Lasers:** GATE 2025 places them in Atomic and Molecular Physics, not a separate Optical Physics unit.
- **Semiconductor equilibrium and metal-semiconductor junctions:** GATE 2025 places them in Electronics, while the tracker has them in Solid State.
- **Rutherford scattering:** GATE 2025 names it in classical central-force motion as well as nuclear scattering. The tracker currently represents only the nuclear occurrence.
- **Nuclear models:** t002 “nuclear models” overlaps t003’s liquid-drop and shell-model detail; narrow the general title or add a cross-reference.

## Implementation preconditions

1. Approve the GATE baseline decision explicitly.
2. Re-verify the supplied document hashes and re-read the active live DB immediately before implementation (the audit used a read-only copy).
3. Create an online backup and a disposable rehearsal copy; do not test against the live DB.
4. Preserve all node IDs/codes for historical records; archive instead of delete; record migrations.
5. Re-run the syllabus, mapping, progress, analytics, PYQ and session-allocation regression tests.
6. Do not copy the audit JSON into seed files. It is an evidence document, not production data.

The exact source evidence for every proposed change is in `SYLLABUS_DETAILED_COMPARISON.md` and `syllabus_audit.json`.
