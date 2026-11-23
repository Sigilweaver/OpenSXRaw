---
sidebar_position: 3
---

# mzML export

OpenSXRaw doesn't ship a dedicated export binary yet, but since `Reader`
implements `openmassspec_core::SpectrumSource`, it can be written to mzML
using the same writer every reader in the OpenMassSpec stack uses, so
output is consistent across vendors:

```rust
use opensxraw::reader::Reader;
use openmassspec_core::write_mzml;

let mut reader = Reader::open("sample.wiff")?;
let mut out = std::fs::File::create("output.mzML")?;
let summary = write_mzml(&mut reader, &mut out)?; // fails: unknown_scan_mode
println!("{} spectra written", summary.spectra_written);
```

The reader cannot tell centroid from profile data, so every spectrum has
`scan_mode` unknown and the default `write_mzml` stops at the first
spectrum with the `unknown_scan_mode` check. To write the file, allow that
check, which marks each spectrum with an `unknown_scan_mode` warning and
leaves the profile/centroid term out of the mzML:

```rust
use openmassspec_core::{write_mzml_with_options, Check, Leniency, WriteOptions};

let options = WriteOptions::new()
    .with_leniency(Leniency::strict().with(Check::UnknownScanMode));
let summary = write_mzml_with_options(&mut reader, &mut out, &options)?;
```

`write_mzml` stops at the first spectrum that fails to decode and returns
that error. To write the readable spectra and skip the rest, use
`write_mzml_with_options` with `RecordErrorPolicy::Skip` and report the
skipped count from the returned `WriteSummary`.

`write_mzml` iterates the reader's spectra via `SpectrumSource` (the same
stream described in [Reader](./reader)) and emits PSI-MS CV-annotated
mzML - `MS:1000562` (ABI WIFF format) as the source-file format term,
with per-spectrum retention time, MS level, and precursor information
carried through where the reader populated it. Given the
[current reader limitations](./reader#what-the-reader-does-not-do)
(TOF m/z not accurate, QTRAP m/z in raw bins, every TripleTOF/ZenoTOF
scan labelled MS2 with no precursor m/z), exported mzML will
carry those same gaps until the underlying fields are decoded.
