---
sidebar_position: 2
---

# Scan data

Each `SpectrumRecord` yielded by `iter_spectra` is assembled from one
`Idx` record in the `.wiff` file plus the matching data block in the
paired `.wiff.scan` file. The `Idx` record locates the block (byte offset
and size) and carries the per-scan metadata; the block itself holds the
peak data. See [Legacy .wiff.scan blocks](../format/legacy-wiff-scan) for
the on-disk layout.

## m/z and intensity

The `.wiff.scan` block payload is a custom byte-level prefix encoding of
`(delta_mz, intensity)` integer pairs (not a standard LZ/entropy codec).
The reader decodes the pairs, accumulates the m/z deltas into an axis, and
emits parallel `mz` (`f64`) / `intensity` (`f32`) arrays.

:::note m/z calibration
On TripleTOF/ZenoTOF files the raw time-bin values are converted to m/z
(Da) using the first `(slope, intercept)` pair of the `TOFCalibrationData`
stream. QTRAP files have no calibration body, so their `mz` values are
raw, uncalibrated bin values.
:::

## MS level and precursor

`ms_level` comes from the `Idx` MS-level flag. MS2+ spectra carry a
`precursor` whose `precursor_native_id` is the native ID of the preceding
MS1 scan (`None` when no MS1 precedes it). `selected_mz`/`target_mz` come
from the `DDERealTimeDataEx` stream on DDA files, linked by DDA cycle; on
files without that stream (MRM/targeted, plain SWATH) they stay `None`.

## Retention time

`retention_time_sec` is the `Idx` per-record retention time (stored on
disk in minutes) converted to seconds.

## Fields not yet populated

The analyzer is `TOFMS` for TripleTOF/ZenoTOF files and `TQMS` for QTRAP
files (see [Instrument families](./instrument-families)). The reader leaves
`total_ion_current` unset - the `Idx` TIC is in physically
calibrated cps and does not match the sum of the raw intensity counts, so
the mzML writer recomputes TIC from the intensity array instead. `scan_mode`
is also left unset (`None`): no currently-decoded stream carries a
per-scan or per-experiment centroid/profile indicator, and unconditionally
reporting `Profile` (the prior behavior) mislabeled centroided
acquisitions - see [Reader: what the reader does not yet
do](./reader#what-the-reader-does-not-yet-do). See the [format
overview](../format/overview) for the full list of known limitations.
