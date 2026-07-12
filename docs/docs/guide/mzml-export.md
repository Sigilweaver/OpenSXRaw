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
write_mzml(&mut reader, &mut out)?;
```

`write_mzml` iterates the reader's spectra via `SpectrumSource` (the same
stream described in [Reader](./reader)) and emits PSI-MS CV-annotated
mzML - `MS:1000562` (ABI WIFF format) as the source-file format term,
with per-spectrum retention time, MS level, and precursor information
carried through where the reader populated it. Given the
[current reader limitations](./reader#what-the-reader-does-not-yet-do)
(no m/z calibration, no real MS2 precursor m/z yet), exported mzML will
carry those same gaps until the underlying fields are decoded.
