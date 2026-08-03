# Legacy .wiff ion source parameters and polarity

Status: CONFIRMED (parameter layout and all non-ISVF/IS values), PHYSICS-ARGUED
(negative polarity branch - no real fixture in the local corpus)

This documents `MethodSubtree/MethodN/DeviceMethod0/Period0/ExperimentM/
IonSourceParamsTable/ParameterK/ParameterData`, decoded while investigating
issue #26 ("polarity hardcoded to `None` for every spectrum"). Decoded
clean-room, from public PRIDE corpus files and public electrospray
ionization physics only - no vendor software or msconvert comparison was
used at any point (see the project's clean-room rule).

## Why this stream

Issue #26 had already been investigated once (PR #29) without corpus access,
which ruled out every stream this reader decoded at the time
(`SummaryInformation`, `TOFCalibrationData`, `DDERealTimeDataEx`, `Idx`, the
`.wiff.scan` token stream) and flagged the undecoded per-Experiment
`ExperimentHeader`/`ExperimentHeaderEx` method structures as the likely real
location, without being able to check further. With corpus access, this
pass:

1. Surveyed `ExperimentHeader`/`ExperimentHeaderEx` byte-for-byte across all
   200 local `.wiff` files, looking for a field with a small, project-
   homogeneous set of values. Every candidate found turned out to explain
   something *other* than polarity: offset `0x20` reproduces the same
   TOF-vs-quad/trap split as `TOFCalibrationData` (see
   `04-legacy-wiff-calibration.md`), and offsets `0x3c`/`0x3e`/`0x47`/`0x48`
   in `ExperimentHeaderEx` turned out to encode acquisition mode (SWATH vs.
   IDA/DDA vs. plain full-scan - confirmed by matching flag values to
   `SW_F*` vs. `Fish_IDA*` vs. `total*`/`swath*` filenames within the same
   PRIDE project, `PXD078909` and `PXD054774`). One remaining candidate
   (`ExperimentHeader` offset `0xb1`, a per-project-homogeneous binary flag)
   could not be explained even after cross-checking PRIDE project metadata
   for the two projects where it differs from the rest of the corpus - left
   unresolved, not polarity as far as could be determined.
2. Moved to `IonSourceParamsTable`, a stream never previously decoded by
   this reader, on the reasoning that ion source parameters (gas flows,
   temperatures, and critically ion spray voltage) are exactly where a
   polarity-determining physical setting would live in any ESI source
   configuration - this turned out to be right.

## `ParameterData` record layout

Each `IonSourceParamsTable/ParameterN/ParameterData` stream (N = 0, 1, 2,
...) holds one named parameter:

| Offset       | Type     | Description                                    |
|--------------|----------|-------------------------------------------------|
| `0x00..0x22` | ...      | Opaque preamble, not decoded                    |
| `0x22..0x24` | `u16`    | Name length in bytes (`char_count * 2`)          |
| `0x24..`     | UTF-16LE | Parameter name (`name_len` bytes)                |
| (after name) | `f32`    | Parameter value, little-endian                   |
| (+4 more)    | `f32`    | Same value repeated (a "current"/"display" pair) |

Confirmed against the full local corpus (200 files): 9 distinct parameter
names were found (`GS1`, `GS2`, `CUR`, `TEM`, `CAD`, `IHT`, `COLUMN TEM`,
`ISVF`, `IS`), and every one decodes via this layout to a value that is
physically sane for what its name suggests - curtain/nebulizer gas flows in
roughly the 0-90 range, source/interface temperatures roughly 0-350, and (see
below) ion spray voltage in the low thousands of volts. That multiple
independent parameter types all decode correctly at once, from the same
offset arithmetic, is the main evidence the byte layout above is right (not
just a coincidental fit for one field).

`Reader` reads this via `crate::raw::ion_source::SourceParameter::from_bytes`.

## Ion spray voltage as the polarity signal

The parameter named `ISVF` ("Ion Spray Voltage Floating", present on
169/200 corpus files - exactly the TripleTOF/ZenoTOF-family split, matching
`TOFCalibrationData`'s presence) or `IS` (present on the other 31/200,
QTRAP-family files) is the electrospray needle voltage. Spray voltage
polarity directly determines ion polarity: a positive voltage accelerates
and produces positive ions, a negative voltage produces negative ions. This
is standard electrospray ionization physics, documented in any mass
spectrometry text, independent of SCIEX or any other vendor.

Every one of the 200 local corpus files has a **positive** `ISVF`/`IS` value
(range 2200-5500 V), consistent with:

- The corpus being PRIDE-sourced tryptic-peptide proteomics (see
  `CORPUS.md`), which is almost universally run in positive-ion mode.
- Every project's public PRIDE metadata checked while investigating this
  (`PXD004362`, `PXD022088`, `PXD032192`, `PXD035159`, `PXD075858`) - none
  mention negative-mode acquisition.
- The value stays constant across every `ExperimentN` within a method for
  every multi-experiment file checked (e.g. SWATH cycles in
  `PXD054774/swath3.wiff`), so reading only `Experiment0` is sufficient.

`Reader` uses `voltage > 0.0 -> Positive`, `voltage < 0.0 -> Negative`,
`voltage == 0.0 -> None` (`find_ion_spray_voltage` in `reader.rs`).

### Known gap: the negative branch is unverified against a real file

No file in the local 200-file corpus has a negative `ISVF`/`IS` value, so
while the `Positive` branch is extensively corroborated (every corpus file,
cross-checked against PRIDE metadata and the parser's own multi-parameter
self-consistency), the `Negative` branch rests entirely on public ESI
physics and the field's self-declared name, not on decoding a real
negative-mode fixture. If a negative-mode SCIEX `.wiff` file is ever added
to the corpus, re-verify this branch against it.

### Multi-sample caveat

`MethodSubtree` is not nested per sample the way `SampleSubtree` is, so on
a multi-sample `.wiff` file (see Sigilweaver/OpenSXRaw#25) `Reader` cannot
cleanly determine which `MethodN` governs which sample. `find_ion_spray_voltage`
probes `Method1..MethodN` (bounded) and uses the first match, which is a
per-file, not per-sample, approximation - the same caveat that already
applies to the TOF-vs-quad/trap analyzer signal.

## `ExperimentHeader`/`ExperimentHeaderEx` fields ruled out for polarity

For completeness, the fields surveyed in `ExperimentHeader`/
`ExperimentHeaderEx` while chasing this (see "Why this stream" above) and
what they actually turned out to mean, so a future investigation doesn't
re-tread this ground:

- `ExperimentHeader`/`ExperimentHeaderEx` offset `0x20`: TOF-vs-quad/trap
  analyzer family (same signal as `TOFCalibrationData` presence).
- `ExperimentHeaderEx` offsets `0x3c`, `0x3e`, `0x47`, `0x48`: acquisition
  mode (SWATH/DIA vs. IDA/DDA vs. plain scan), confirmed by filename
  correlation within `PXD078909` and `PXD054774`.
- `ExperimentHeader` bytes `0x78-0x79`, `0x82-0x83`, `0x95-0xaf`: differ
  only on the ZenoTOF-family fixtures (`PXD074536`, `PXD071194`,
  `PXD045599`) vs. everything else - an instrument-generation difference in
  struct layout, not a per-run setting.
- `ExperimentHeader` offset `0xb1`: a project-homogeneous binary flag (0 in
  21/23 projects, 1 only in `PXD032192` and `PXD035159`). Checked against
  both projects' public PRIDE metadata and against two flag=0 QTRAP
  projects (`PXD004362`, `PXD022088`) as a counter-example - all four
  describe standard tryptic-peptide shotgun proteomics with SRM/MRM
  follow-up, no distinguishing detail found. Still unexplained; not used.
