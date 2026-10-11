---
sidebar_position: 1
---

# Reader

The entry point is `Reader`. `Reader::open` takes a path to the `.wiff`
file, opens it as a CFBF container, reads the
`SampleSubtree/Sample1/Idx` stream up front to build the full scan index,
and expects the paired `.wiff.scan` file (same name, `.scan` appended) to
sit alongside it. Individual scan payloads are decoded on demand from
`.wiff.scan` as you iterate.

```rust
use opensxraw::reader::Reader;

let reader = Reader::open("sample.wiff")?;
```

`Reader` implements `openmassspec_core::SpectrumSource`, the shared trait
every vendor reader in the OpenMassSpec stack implements:

```rust
use openmassspec_core::SpectrumSource;

let metadata = reader.run_metadata();
let scan_count = reader.spectrum_count_hint().unwrap_or(0);
println!("{} ({} scans)", metadata.source_file_name, scan_count);

let mut reader = reader;
for spectrum in reader.iter_spectra() {
    let spectrum = spectrum?;
    println!("{}\t{}\t{} peaks", spectrum.native_id, spectrum.ms_level, spectrum.mz.len());
}
```

Each yielded `SpectrumRecord` carries `index`, `scan_number`, `native_id`,
`ms_level`, `retention_time_sec`, `analyzer`, `polarity`, and
`mz`/`intensity` arrays. Native IDs use the PSI-MS "scan number only
nativeID format" (`MS:1000776`): `scan=N`, where `N` is the 1-based
position in the `Idx` stream.

- **m/z**: on TripleTOF/ZenoTOF files the raw time-bin values are converted
  to m/z (Da) with the linear `(slope, intercept)` pair read from the
  `TOFCalibrationData` stream. QTRAP files carry no calibration body, so
  their `mz` values stay as raw bin values. Neither is accurate m/z: the
  TOF values fail physical checks (known ions, isotope spacing) and some
  are negative. See Known issues in the README.
- **Precursors**: MS2+ spectra carry a `precursor` whose
  `precursor_native_id` points at the preceding MS1 scan (`None` if no MS1
  has been seen yet). `selected_mz`/`target_mz` come from the
  `DDERealTimeDataEx` stream when present, linked by DDA cycle (a
  heuristic, see `raw::dde`); they stay `None` on files without that
  stream (for example plain SWATH or MRM).
- **Analyzer**: `TOFMS` when a calibration body is present (TripleTOF and
  ZenoTOF), `TQMS` otherwise (QTRAP).
- **Polarity**: from the sign of the ion spray voltage in the method's
  `IonSourceParamsTable`.

## What the reader does not do

- **QTRAP m/z**: no clean-room calibration source is known for QTRAP
  files, so their `mz` values are raw bins.
- **Correct TOF m/z**: only the first `(slope, intercept)` pair of
  `TOFCalibrationData` is used as a per-file constant, and the result does
  not match known ions in the corpus.
- **TripleTOF/ZenoTOF MS level**: every scan is labelled MS2, including
  MS1 survey scans, so TOF MS2 spectra have no precursor m/z.
- **MRM transitions**: Q1/Q3 are not exposed and no SRM chromatograms are
  produced.
- **Charge state and isolation window**: not decoded.
- **Centroid/profile scan mode**: `scan_mode` is left unset (`None`) on
  every spectrum. No decoded stream (`Idx`, `DDERealTimeDataEx`,
  `SummaryInformation`) carries a reliable per-scan or per-experiment
  centroid/profile indicator.

`iter_spectra` yields `openmassspec_core::Result<SpectrumRecord>` items.
If a scan block cannot be read from the `.wiff.scan` file (an I/O error,
or a block that starts past the end of the file), that item is an `Err`
wrapping `opensxraw::Error::ScanRead`, and `Reader::scan_read_failures()`
counts it. Iteration continues with the next scan, so a caller can stop
at the first error (`?`) or skip and count.

## Decoded per-file values

`Reader` exposes what it decoded at open time through accessor methods:
`idx_records()` (scan index: offset, size, retention time, MS level,
instrument TIC), `calibration()`, `dde_records()`, `instrument_info()`,
`ion_source_parameters()`, `polarity()`, `start_timestamp()`, `sample()`
and `stem()`. `Reader::list_samples` and `Reader::open_sample` select one
sample of a multi-sample `.wiff` file.

## Error handling

Public functions return `opensxraw::Result<T>`. The error type is
`opensxraw::Error`, with variants `Io`, `Parse` and `ScanRead`. It is
`#[non_exhaustive]`, so match it with a wildcard arm.
`opensxraw::Error` converts into `openmassspec_core::Error` with `?` or
`.into()`; inside the core error it is the boxed `Vendor` payload and can
be recovered with `downcast_ref::<opensxraw::Error>()`.
