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

:::caution Uncalibrated m/z
The `mz` values are currently **raw, uncalibrated time-bin integers**, not
physical mass-to-charge values. Physical calibration requires the
`ExperimentTOF` method-stream constants, which are not yet decoded. The
arrays are internally consistent (peak ordering and relative spacing are
correct) but are not directly comparable to a calibrated vendor export.
:::

## MS level and precursor

`ms_level` comes from the `Idx` MS-level flag. MS2+ spectra carry a
`precursor`, but the `Idx` stream does not store precursor m/z, so the
current reader emits a placeholder `precursor_native_id` with no
`target_mz`. True precursor m/z lives in the not-yet-decoded
`DDERealTimeDataEx` stream (data-dependent scans) or the method subtree
(MRM/targeted scans).

## Retention time

`retention_time_sec` is the `Idx` per-record retention time (stored on
disk in minutes) converted to seconds.

## Fields not yet populated

The reader currently reports every spectrum as profile-mode / `TOFMS`
analyzer regardless of the actual instrument family (QTRAP records are not
yet distinguished), and leaves `total_ion_current` unset - the `Idx` TIC
is in physically calibrated cps and does not match the sum of the raw
intensity counts, so the mzML writer recomputes TIC from the intensity
array instead. See the [format overview](../format/overview) for the full
list of known limitations.
