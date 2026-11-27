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
let summary = write_mzml(&mut reader, &mut out)?;
println!("{} spectra written", summary.spectra_written);
```

No `.wiff` stream says whether a scan is centroid or profile, so every
spectrum has `scan_mode` unknown. The reader declares this once in its run
metadata (`RunMetadata::unstated` holds `Check::UnknownScanMode`), so the
default `write_mzml` converts the file. Each spectrum carries an
`unknown_scan_mode` warning, the mzML has no profile or centroid term on its
spectra, and the run gets one `unstated_by_source` warning. The summary
reports them:

```rust
assert!(summary.has_warnings());
println!("{:?}", summary.warning_counts);
// {"unknown_scan_mode": 2228, "unstated_by_source": 1}
```

A spectrum whose scan block decodes to no peaks carries the `empty_spectrum`
warning and declares no base peak or m/z range.

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
