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
  their `mz` values stay as raw bin values.
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
- **Full TOF recalibration table**: only the first `(slope, intercept)`
  pair of `TOFCalibrationData` is used as a per-file constant.
- **Charge state and isolation window**: not decoded.
- **Centroid/profile scan mode**: `scan_mode` is left unset (`None`) on
  every spectrum. No decoded stream (`Idx`, `DDERealTimeDataEx`,
  `SummaryInformation`) carries a reliable per-scan or per-experiment
  centroid/profile indicator.

If a scan block cannot be read from the `.wiff.scan` file (an I/O error,
or a block that starts past the end of the file),
the spectrum is still yielded with empty arrays, a warning is logged via
the `log` crate, the error text is stored under the
`opensxraw.scan_read_error` key in `extra`, and
`Reader::scan_read_failures()` counts it.

## Error handling

Public functions return `opensxraw::Result<T>`. The error type is
`opensxraw::Error`, which wraps the failure category (`Io`, `Parse`) and
a message.
