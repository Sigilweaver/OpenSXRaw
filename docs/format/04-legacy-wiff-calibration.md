# Legacy .wiff m/z calibration and MS2 precursor m/z

Status: CONFIRMED (TripleTOF/`ExperimentTOF` files), OUT OF SCOPE (QTRAP-only files)

This documents the two CFBF streams scoped by issue #3: `SampleSubtree/Sample1/TOFCalibrationData`
(turns `raw_mz_bin` into physical m/z) and `SampleSubtree/Sample1/DDERealTimeDataEx`
(precursor m/z for MS2 scans). Both were decoded clean-room, from public
PRIDE corpus files and internal physics/chemistry consistency checks only -
no vendor software or msconvert comparison was used at any point (see the
project's clean-room rule). Confidence levels are called out explicitly
below rather than implied, since that comparison ceiling is the practical
limit on how far this could be validated.

## `ExperimentTOF` - ruled out as the calibration source

`MethodSubtree/Method1/DeviceMethod0/Period0/ExperimentN/ExperimentTOF` is
only 38 bytes: a handful of small `u32` fields (values like 5 and 14 seen
in testing), no plausible calibration-shaped floats. It does not appear to
hold the slope/offset constants and was not pursued further.

## `TOFCalibrationData` - the real calibration source

Present only on files that also have `ExperimentTOF` (TripleTOF-family
acquisitions using TOF detection, including MRM-HR/EPI methods on QTRAP
hardware). Absent entirely on plain QTRAP files (see "QTRAP scope" below).

Layout:

- 32-byte stream header (opaque, `04 00 00 00` observed at offset 4;
  meaning not resolved, skipped like the `Idx` header).
- Body is a table of `(f64 slope, f64 intercept)` pairs. The first pair
  starts at body offset 0.
- A `u32` count field at body offset 0x14 (i.e. stream offset 0x34) was
  confirmed across three different corpus files to exactly equal that
  file's total `Idx` record count (including placeholder records) -
  40448, 40690, and 189011 respectively. This is a strong structural tie
  between the calibration table and the scan index, though the table's
  own record framing beyond the first entry was not fully resolved (see
  "Known gaps" below).
- Scanning the whole body for `(f64, f64)` pairs where the first value
  falls in a plausible slope range shows the **slope is effectively
  constant per file** (varies only in the 9th-10th significant digit
  across thousands of positions in the same file, e.g.
  `0.0007027928`-`0.0007027934` in one corpus file), while the
  **intercept drifts slightly over the run** (e.g. observed range
  `0.3127`-`0.3637` within one file, alternating between a small set of
  nearby values rather than monotonically trending) - consistent with a
  live lock-mass recalibration feed correcting for small time-zero drift
  during acquisition, with a fixed hardware-level time-bin width.
- Across different files/runs, the slope itself varies only in the same
  narrow band (`0.00070276`-`0.00070280` seen across three files), while
  the intercept varies more (`0.31`-`1.07` seen across three files) -
  consistent with slope being close to a fixed digitizer sampling
  constant and intercept being a per-run time-zero offset.

### Formula

```
m/z = slope * raw_mz_bin + intercept
```

This is a **linear** relationship, not the quadratic `time ~ sqrt(m/z)`
form expected from first-principles TOF physics. The working theory is
that the vendor firmware already linearizes the digitized time bins onto
an m/z-like grid before this stream's constants are applied, so what's
stored here is a fine linear correction, not a full time-to-mass
reconstruction. This wasn't verified beyond output plausibility (see
"Validation" below) - if a future investigation finds evidence of a
quadratic term, revisit this.

### Validation

No isotope-level ground truth was established - per the project's
clean-room rule, no vendor software or msconvert comparison was used, and
that is the honest ceiling on validation without a reference reader.
What was confirmed:

- Decoding a genuine wide-range scan from a real SWATH corpus file
  (`PXD054774/DDA2.wiff`, not committed - PRIDE corpus) and applying the
  formula maps the observed bin range (24 to ~2,489,529) to m/z 1 to
  ~1750 - exactly the physically sane range for a peptide LC-MS survey
  scan, at a resolution of ~1400 bins/Da (sub-mDa steps, consistent with
  a high-resolution reflectron TOF).
- The largest, most reproducible peaks in that scan (isolated, no
  isotope satellites, huge and stable intensity) are almost certainly
  lock-mass/calibrant reference ions rather than analyte peaks - which
  is itself consistent with `TOFCalibrationData` being a live
  recalibration feed rather than a static one-time constant.
- A dedicated search for a cleanly resolved analyte isotope envelope
  (charge 1-3 spacing at ~1422.9/711.4/474.3 bins per isotope) did not
  turn up an unambiguous example in the scans checked. This may need
  denser/deeper LC-gradient data or dedicated deconvolution to find, and
  is flagged as an open gap rather than treated as a formula failure.

### Known gaps

- The calibration table's full record framing (beyond "first pair at
  body offset 0, count field at body offset 0x14") isn't resolved -
  scanning for repeats of the slope/intercept pattern shows irregular
  byte gaps (440, 40, 200, 240, 160... bytes) rather than one clean
  fixed-size record, suggesting either a sparse/deduplicated live-update
  log or that the naive 8-byte-aligned float scan is picking up false
  positives from adjacent unrelated fields.
- Given the above, and that the actual per-scan intercept drift is small
  (well under 0.1 Da across a run in the files checked), the Rust
  implementation (`raw/experiment_tof.rs`) uses **only the first
  `(slope, intercept)` pair** as a per-file constant rather than
  resolving the full live-recalibration table. This is a known,
  documented approximation - within roughly the observed drift band of
  the fully time-resolved value - not a silent inaccuracy.

## `TDCStatistics` - ruled out

Also present alongside `TOFCalibrationData` on TOF-family files. Body is
dense repeating `u32` integer patterns (a recurring `0x00000dc8` = 3528
sentinel/bucket-boundary value, surrounded by small counts) - this reads
as a per-bin hit-count histogram (time-to-digital-converter statistics,
matching the name), not calibration constants. Not used.

## `DDERealTimeDataEx` - MS2 precursor m/z

Present on files with data-dependent (IDA/DDA) precursor selection
(tested against a QTRAP-family file with MRM-survey-triggered EPI
scans, `PXD022088/Rcor2KOESC1.wiff`). Layout:

- 32-byte stream header (same opaque pattern as other streams here).
- Body is a flat array of fixed **76-byte records** (confirmed: body
  size divides evenly by 76 with no remainder, vs. no clean division at
  the previously-assumed 80 bytes).
- Record offset 0x00 (`u32`): a 1-based sequential record number,
  matching the record's position in the stream exactly in every record
  checked. Not an external link to anything - just an internal
  ordinal.
- Record offset 0x04 (`f64`): **precursor m/z**, already physically
  calibrated (values observed in the normal peptide range, e.g.
  486.7-1058.5 Da, not raw time bins). No further calibration needed for
  this field.
- Record offset 0x2C (`u32`, byte 44): increments across records but not
  strictly 1:1 with the record ordinal (gap between them grows over the
  file) and does not cleanly index into the `Idx` stream by absolute
  position or by MS1-only position in the files checked. Meaning not
  resolved; not used.
- No charge state or isolation width field was confidently identified in
  the remaining bytes.

### MS2 linkage (heuristic, not fully validated)

`DDERealTimeDataEx`'s record count (1170 in the test file) matches that
file's **MS1/survey scan count** exactly, not its MS2 count (1058) - so
this looks like one entry per DDA cycle (the precursor selected by that
cycle's survey step), not one entry per MS2 scan. The implementation
uses this model: walk `Idx` records in order, and for each MS2 scan use
the `DDERealTimeDataEx` record at the count of MS1 scans seen so far
minus one (i.e. "the most recently completed cycle's selected
precursor"). This is physically motivated (matches how DDA/IDA
triggering works) but the exact linkage field wasn't independently
confirmed against ground truth - flagged as an approximation, not a
proven mapping. If a future investigation resolves the byte-44 field's
meaning, prefer it over this heuristic.

## QTRAP scope

Files without `ExperimentTOF`/`TOFCalibrationData` (tested against
`Rcor2KOESC1.wiff`, a QTRAP-family file with `sMRM`/`sMRMEX` method
streams and no TOF calibration streams at all) have **no clean-room-
derivable calibration source** under this investigation. `raw_mz_bin`
stays uncalibrated for these files - the Phase B implementation guards
on `TOFCalibrationData`'s presence and leaves QTRAP-only files' `mz`
arrays as raw bin values, unchanged from today's behavior. This is a
scope boundary, not a bug: no calibration stream was found to decode for
this instrument family in this investigation.
