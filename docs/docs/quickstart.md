---
sidebar_position: 3
---

# Quickstart

```rust
use opensxraw::reader::Reader;
use openmassspec_core::SpectrumSource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = Reader::open("sample.wiff")?;
    for spectrum in reader.iter_spectra() {
        println!("{}: {} peaks", spectrum.native_id, spectrum.mz.len());
    }
    Ok(())
}
```

`Reader::open` expects the paired `.wiff.scan` file to sit alongside the
`.wiff` file, with `.scan` appended to the `.wiff` filename (SCIEX's own
on-disk convention - Analyst always writes the pair this way).

## Next

- [Reader API](./guide/reader)
- [Instrument families](./guide/instrument-families)
- [Format specification](./format/overview)
