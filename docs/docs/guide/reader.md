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
`ms_level`, `retention_time_sec`, and `mz`/`intensity` arrays. For MS2+
scans, `precursor` is populated with a placeholder native-ID reference
rather than a real precursor m/z (SCIEX's `Idx` stream does not store
precursor m/z; it lives in the not-yet-decoded `DDERealTimeDataEx`
stream) - see [Format specification](../format/legacy-wiff-cfbf) for
where that data actually lives on disk.

## What the reader does not yet do

- **m/z calibration**: `mz` values are the raw time-bin integer read
  directly from the token stream, not a calibrated Da value. Real
  calibration requires the `ExperimentTOF` method-stream constants, not
  yet decoded.
- **Precursor m/z**: as above, MS2 spectra don't carry a real
  `target_mz` yet.
- **Instrument-aware scan mode**: every spectrum is currently reported as
  profile-mode with a TOFMS analyzer, regardless of whether the source
  file is actually a QTRAP acquisition (which is nominal-mass, not true
  time-of-flight). This is a simplification in the current reader, not a
  property of the format itself.

## Error handling

Public functions return `opensxraw::Result<T>`. The error type is
`opensxraw::Error`, which wraps the failure category (`Io`, `Parse`) and
a message.
