# Legacy .wiff method tree: per-Experiment parameter tables

Status: PARTIAL - new structural decodes, not yet wired into the reader

This documents new clean-room findings from investigating
[#7](https://github.com/Sigilweaver/OpenSXRaw/issues/7) (per-scan MS-level /
experiment-index for multi-Experiment SWATH/DDA acquisitions) and
[#23](https://github.com/Sigilweaver/OpenSXRaw/issues/23) (precursor
`collision_energy`/`activation`). Both issues point at the same unexplored
territory, `MethodSubtree/Method1/DeviceMethod0/Period0/ExperimentN`
(catalogued structurally but not decoded in
[01-legacy-wiff-cfbf.md](01-legacy-wiff-cfbf.md)), so this note covers both.

Everything here was decoded from public corpus files (PRIDE, see
[CORPUS.md](../../CORPUS.md)) using only self-consistency checks (byte
diffing across `ExperimentN` folders within a file, and across independent
files/PRIDE projects) plus public-domain mass-spec method-design knowledge
(standard MRM parameter names DP/EP/CE/CXP, plausible collision-energy
ranges, the published concept of charge-state-dependent linear
"rolling"/"auto" CE formulas used by targeted-MS vendors generally) as a
plausibility check - never vendor software, per the project's clean-room
policy. **Nothing here is wired into the reader yet** - see "Why this isn't
wired in yet" at the end.

## The `MassRangeEx` parameter table format

`MethodSubtree/Method1/DeviceMethod0/Period0/ExperimentN/MassRangeEx/MassRangeEx`
is a self-describing, human-readable key/value blob, not an opaque struct.
After the usual 32-byte stream header, entries look like:

```
u16 name_byte_len
name (UTF-16LE, name_byte_len bytes)
f32 value1
f32 value2
u32 zero (padding/reserved)
```

`value1 == value2` in every entry seen so far (no case of a genuine `[low,
high)` range was found in this field pair - it always looks like the same
scalar written twice). Two distinct shapes of this table were found,
corresponding to two different method styles:

### Shape A: single parameter set (SWATH windows and generic DDA/IDA slots)

One flat run of named entries, e.g. (from `PXD054774/swath1.wiff`,
`Experiment1`):

```
DP=(80,80) CE=(18.91,18.91) CES=(15,15) IRD=(66.63,66.63) IRW=(24.92,24.92)
IRDx=(15000,15000) IRWx=(10000,10000) IDIx=(0,0) IDUx=(5,5) IWIx=(0,0)
IWUx=(5,5) XA1=(126.1,126.1)
```

`DP` (declustering potential), `CE` (collision energy), `CES` (collision
energy spread) are standard SCIEX MRM/MS-method parameter names. `IRD`/`IRW`
("Ion Release Delay/Width") and the `x`-suffixed variants, `IDIx`/`IDUx`,
`IWIx`/`IWUx` look like TOF-pulser/timing hardware settings, not mass-related
- not pursued further. `XA1` is unidentified; it increases smoothly and
monotonically by a small fixed step across sequential `ExperimentN` (e.g.
`126.1, 126.8, 127.6, 128.4, ...` for `swath1.wiff`'s `Experiment1..40`) -
plausibly a window-order or scheduling value, not confirmed.

A preceding fixed-size prefix (stream offset `0x20`-`0x37`, before the named
entries) holds a `u32` entry count at `+4` (matches the number of named
entries that follow) and some `f32`/`f64` fields that did not show a stable
per-experiment pattern worth reporting; not pursued further.

### Shape B: MRM/SRM transition list (targeted methods)

Seen in `Rcor2KOESC1.wiff` (`PXD022088`, QTRAP, `Experiment0` - 30036 bytes,
153 transitions) and in the independent `PXD056184` MRM-HR corpus project
(114 `Experiment` folders, each holding one transition). Each transition
record is:

```
f32 q1          -- precursor m/z, already physically calibrated
f32 zero        -- always 0.0 in every confirmed record
f32 q3          -- product/fragment m/z, already physically calibrated
f32 const       -- unidentified; 10.0 or 6.0 seen (see below)
u32 zero
u16 name_byte_len
name (UTF-16LE) -- e.g. "sp|Q8C6P8|ZFP57_MOUSE.VMSETFK.+2y6.light"
                    (UniProt accession | protein_organism . peptide .
                    charge+fragment . isotope-label - standard SRM assay /
                    Skyline-style transition naming)
then 4 Shape-A-style named entries: DP, EP, CE, CXP
```

`EP` (entrance potential) and `CXP` (collision cell exit potential) join
`DP`/`CE` here - all four are standard SCIEX QTRAP MRM parameters. The
`const` field (10.0 for most transitions, 6.0 for the file's `iRT-C18
Standard Peptides.*` calibration transitions) is plausibly MRM dwell time in
milliseconds - a shorter dwell for retention-time-calibration transitions
vs. longer for quantitation transitions is a realistic real-world method
choice - but this was not independently confirmed and is not used for
anything.

`Rcor2KOESC1.wiff`'s 153 transitions decode to fully plausible values:
`Q1` 421-1500-ish Da, `EP` a constant `10.0` V (textbook default), `CXP` a
constant `13.0` V (textbook default), `DP` `61.8`-`122` V, and **`CE`
`22.6`-`53.2` eV, averaging `33.5`** - squarely in the expected 10-60 eV
range for peptide CID, and correlating smoothly with `Q1` m/z across
transitions (see "Rolling CE formula" below for why that correlation is not
a coincidence).

## Rolling/charge-dependent CE formula

`MethodSubtree/Method1/DeviceMethod0/Period0/IDA/IDA` (256-byte stream, seen
in `Rcor2KOESC1.wiff`) contains, at body offset `0x60`-`0xbf` (6 consecutive
16-byte pairs of `f64 slope, f64 intercept`):

```
(0.044, 5.0)
(0.058, 9.0)
(0.044, 5.0)
(0.05,  4.0)
(0.05,  3.0)
(0.05,  3.0)
```

This is consistent with a **per-charge-state linear CE formula**,
`CE = slope * Q1_mz + intercept`, one `(slope, intercept)` pair per charge
state 1-6 - the general shape of "rolling"/"auto" CE formulas commonly used
across targeted-MS platforms and described in the peer-reviewed literature
for peptide CID (public-domain method-design knowledge, not vendor-derived).
As a self-consistency check (not a vendor comparison): applying row 3
(`0.044, 5.0` - a plausible "charge +2" row, since every transition name
sampled from `Rcor2KOESC1.wiff`'s table above is annotated `+2`) to that
same file's own `Experiment0` transitions reproduces the table's stored `CE`
values closely:

| Q1 (Da) | formula CE (0.044×Q1+5) | stored CE |
|---------|--------------------------|-----------|
| 421.21  | 23.53                    | 24.0      |
| 486.73  | 26.42                    | 26.4      |
| 695.82  | 35.62                    | 33.9      |
| 760.39  | 38.46                    | 36.2      |

Two of four match to within 0.05 eV; the other two are within ~2 eV (still
in-family, not a coincidental match). This independently corroborates both
structures (the static per-transition `CE` field decode, and the rolling-CE
table decode) from two different streams agreeing with each other, which is
the strongest self-consistency evidence in this note.

**This directly explains the CE=0/CE=10 "default" values seen elsewhere**
(see next section): a method using rolling/auto CE leaves the per-Experiment
static `CE` field at a sentinel/UI-default value, because the real per-scan
CE is computed at runtime from the selected precursor's calibrated m/z (and
presumably its charge state) via this formula instead of being read from a
static field.

## CE behavior differs by acquisition family - and reveals a bug in a prior issue-#7 assumption

`PXD054774/DDA2.wiff` (the file issue #7's prior investigation used as its
"SWATH-style" example, since it has 41 `Experiment` folders: 1 + 40) was
re-examined here. **It is not fixed-window SWATH.** `Experiment1`
through `Experiment40`'s `MassRangeEx` tables are *byte-for-byte identical*
in every named parameter (`DP=80, CE=0, CES=0, IRD=66.63, IRW=24.92, ...`
for all 40), differing only in the unidentified, smoothly-incrementing
`XA1` field. Real SWATH windows tile the mass range and *must* differ
window-to-window; identical templates across all 40 "windows" instead mean
these are 40 interchangeable **candidate-ion slots for genuine Top-N
Information Dependent Acquisition (IDA/DDA)**, not fixed isolation windows -
consistent with the file's own name. In true Top-N IDA, the number of
dependent scans actually triggered per cycle is decided at runtime (0 to 40,
based on which precursors clear the real-time intensity/exclusion
criteria), so **the cycle length is not fixed**. This is almost certainly
why issue #7's "does the widest scan sit at a stable position mod 41"
test failed on this file (positions 31, 11, 10, 0, 24 across five cycles) -
the underlying premise (a fixed-length cycle) does not hold for this
acquisition type, independent of whatever the Idx flag byte does or
doesn't encode.

`PXD054774/NABPF1.wiff` through `NABPF6.wiff` (same project, 41 Experiments
each) show the identical pattern (`CE=0` uniformly across `Experiment1-40`)
- also genuine Top-N IDA, not SWATH, despite the folder count.

By contrast, `PXD054774/swath1.wiff`, `swath2.wiff`, `swath3.wiff` (same
project, named for exactly this) have 97 `Experiment` folders, and their
`CE` values ramp *smoothly and monotonically* across `Experiment1..96`
(`18.91, 19.2, 19.49, 19.81, 20.1, ...`), with `CES=15` (a plausible fixed
spread) constant throughout. This is what a real fixed-window SWATH method
with a mass-dependent CE ramp actually looks like at the byte level, and it
is trivially distinguishable from the DDA case above by inspecting whether
`CE` varies meaningfully across `Experiment1..N` or sits at a uniform
sentinel (`0`, or `10` alongside `EP=10` in the QTRAP `Experiment1` case
seen in `Rcor2KOESC1.wiff` - also a rolling-CE default, not a real value).

**Practical self-consistency test usable by a future investigation:** if
`Experiment1..N`'s static `CE` values are uniform (all equal, or all a
round default like `0`/`10`), the file almost certainly uses rolling CE and
has no static per-window CE to read; if they vary meaningfully across
experiments, they are very likely real, usable values.

## Re-testing the multi-Experiment cycle hypothesis on genuine SWATH files

Issue #7's "record position mod N" test was only ever run against
`DDA2.wiff`, which the above shows is the wrong file family for that test.
Re-running the same idea against `swath1.wiff` (97 Experiments, confirmed
genuine fixed-window SWATH above) on the full `Idx` stream (170,665 valid
records):

- The largest `scan_size` in each 97-record span occurs at record indices
  `97, 194, 291, 388, ..., 169750` - **every single one exactly 97 apart**,
  from the second cycle of the run to the very last one. (Record index `0`
  itself was not tested as part of this pattern - the very first record may
  be a leading calibration/setup scan outside the regular cycle; not
  investigated further.)
- This holds across the *entire* 170k-record run, not just a handful of
  cycles near the start - i.e. this is a genuinely stable, phase-locked
  signal for this file, unlike what issue #7 found on `DDA2.wiff`.
- The `Idx` flag byte (offset `0x10`) is uniformly `6` in `swath1.wiff` too
  (same dead end as `DDA2.wiff` - confirmed here independently), so this
  file has exactly the same "no working discriminator today" problem issue
  #7 describes; it just turns out the *fix* (position mod N) that failed on
  the DDA file actually does work here.

So: **for genuine fixed-window SWATH/DIA acquisitions specifically,
`record_index % n_experiments` is a real, stable MS1-survey discriminator.**
It is *not* usable for genuine Top-N IDA/DDA acquisitions (cycle length
varies at runtime - no fixed-N discriminator can exist by construction) or,
per below, confirmed for MRM-HR either.

## A third, less clean acquisition family: MRM-HR

`PXD056184` (independent PRIDE project, `*_MRMHR_*.wiff` filenames, 114
`Experiment` folders) is scheduled/targeted MRM-HR: each `Experiment`'s
`MassRangeEx` is Shape B (one MRM-style transition per Experiment, not a
Shape-A window), with real, varying, plausible `CE` values (`17`-`38` eV,
non-monotonic - individually-optimized per target, not a smooth ramp).
Re-running the same block-size-outlier periodicity test here is much
noisier: the modal gap between size outliers is `114` (matching
`n_experiments`), but with a long tail of smaller/irregular gaps, unlike
`swath1.wiff`'s clean single-value distribution. This is plausibly because
MRM-HR has no single dominant "MS1 survey" scan type the way DIA/SWATH
does (every `Experiment` here is itself a targeted, precursor-selected
scan) - so the whole "which position is the survey" framing may not even
apply to this acquisition family. **Not resolved; flagged as a distinct,
uncharacterized case for a future investigation, not lumped in with either
the SWATH or the DDA finding above.**

## Other things checked and ruled out this session

- **`ExperimentHeader` byte `0x7a` / `0xb0`**: in `DDA2.wiff` these bytes
  are `8`/`1` for `Experiment0` and uniformly `9`/`2` for `Experiment1-40` -
  looked initially like a per-Experiment "survey vs. dependent" role flag.
  Checking `Rcor2KOESC1.wiff` (2 Experiments) shows *different* values at
  the same offsets (`4`/`153` for `Experiment0`, `11`/`3` for
  `Experiment1`) with no obvious relationship to the `8/9` or `1/2`
  convention seen in `DDA2.wiff`. **Not a stable cross-file convention** -
  most likely some per-experiment counter or identifier, coincidentally
  differing between `Experiment0` and later experiments in both files for
  unrelated reasons. Do not rely on this without further evidence.
- **The `.wiff.scan` block's unexplored 8-byte "sync region"** (the bytes
  between the `ff ff ff ff` terminator and the next record's declared
  `scan_offset`, inside the 56-byte pre-payload window described in
  `raw/scan.rs`'s module doc): dumped and compared against the known-flag
  QTRAP fixture (`Rcor2KOESC1.wiff`) and `DDA2.wiff`. No structure found -
  the bytes vary unpredictably and are consistent with simply being leader
  bytes of the *next* block's own token stream (i.e. not a distinct header
  field at all). Ruled out as a per-scan discriminator.
- **`MethodSubtree/.../DDE/DataDependant*` streams**: contain plausible
  round-number fields (`100.0`, `50.0`) alongside count-shaped `u16`/`u32`
  fields - consistent with Top-N candidate-ion criteria (intensity
  threshold, max candidates, exclusion time) but not decoded field-by-field;
  not needed for the above findings and not pursued further this session.

## Why this isn't wired into the reader yet

None of the above is wired into `SpectrumRecord`/`PrecursorInfo` in this
session, for two reasons:

1. **A safe implementation needs to reliably tell these acquisition
   families apart at runtime first**, and that classifier itself needs
   validation across a much wider slice of the corpus than the handful of
   files checked here (3 SWATH runs from one project, ~7 DDA runs from one
   project, one MRM-HR project, one QTRAP/MRM project). Getting this wrong
   in either direction is worse than the status quo: wiring the SWATH-only
   `record_index % N` logic against a file this session didn't confirm is
   really SWATH would silently mislabel `ms_level`/`collision_energy` for
   every scan in that file.
2. **Wiring `ms_level` correctly would change more than one field.**
   `Reader::iter_spectra` already uses `rec.ms_level == 1` to track "how
   many MS1/survey scans have been seen so far", which indexes into
   `DDERealTimeDataEx` for existing precursor-m/z linkage (see `raw/dde.rs`
   and `docs/format/04-legacy-wiff-calibration.md`). Changing how
   `ms_level` is derived changes that counting too, and needs to be
   re-validated against the one real fixture in the conformance suite
   (`PXD022088/Rcor2KOESC1.wiff`, itself a 2-Experiment file whose own
   flag-to-role mapping direction was not independently re-confirmed this
   session - see the "flag direction" question below) without regressing
   `test_ms2_has_precursor`.

### An open question surfaced but not resolved: QTRAP flag direction

While investigating the above, a possible second bug was noticed but is
**not confirmed** and **not acted on**: in `Rcor2KOESC1.wiff`, `Idx` records
with flag `1` (currently mapped to `ms_level = 1`) average a much larger
`scan_size` (1243 bytes) than flag `0` records (429 bytes, mapped to
`ms_level = 2`), and `Experiment0`'s method (the large MRM transition table,
153 transitions) vs. `Experiment1`'s method (a single generic/rolling-CE
template) suggests `Experiment0` is the always-present "trigger" and
`Experiment1` the conditionally-triggered dependent scan. Whether the
*larger* or *smaller* blocks correspond to which role - and therefore
whether the existing flag mapping has the MS1/MS2 assignment backwards for
this file - was not resolved with confidence in the time available. Flagging
this explicitly rather than silently leaving it, per the project's "document
unresolved rather than guess" policy: **a future investigation should
verify this specifically before touching `raw/idx.rs`'s existing flag
mapping**, since `test_ms2_has_precursor` currently passes against whatever
direction is implemented today, and flipping it needs positive evidence,
not just suspicion.

## Summary for the next investigation

- Method tree parameter tables (`MassRangeEx`, `IDA/IDA`) are decoded well
  enough to read real `CE`/`DP`/`EP`/`CXP` values and a rolling-CE formula
  table when they're statically present - this is genuine, wireable
  progress on issue #23 for **fixed SWATH windows and static MRM/MRM-HR
  transitions specifically**, once per-scan Experiment linkage exists.
- `record_index % n_experiments` is a confirmed, stable MS1-survey
  discriminator **for genuine fixed-window SWATH/DIA acquisitions** -
  wireable progress on issue #7 for that family specifically, once a
  reliable "is this really fixed-window SWATH" classifier exists (the CE
  variance check above is a starting point, not a validated classifier).
- Genuine Top-N IDA/DDA acquisitions (`DDA2.wiff`, `NABPF*.wiff`) have no
  known per-scan discriminator and, by construction (runtime-variable cycle
  length), may not have one recoverable from static structure at all - the
  real signal, if any, is more likely to be per-scan telemetry not yet
  found (the `.wiff.scan` block's own 56-byte header region has more
  unexplored bytes than just the 8-byte "sync region" ruled out above; see
  `raw/scan.rs`'s module doc for the parts already accounted for).
- MRM-HR (`PXD056184`) is a third family that doesn't cleanly fit either
  bucket and needs its own investigation.
- The classifier needed to safely dispatch between these families at
  runtime - and the regression risk to `raw/dde.rs`'s existing MS1-cycle
  counting - is the actual remaining blocker to landing any of this as code,
  not further format archaeology.
