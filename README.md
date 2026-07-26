# OpenSXRaw

[![CI](https://github.com/Sigilweaver/OpenSXRaw/actions/workflows/ci.yml/badge.svg)](https://github.com/Sigilweaver/OpenSXRaw/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/opensxraw.svg)](https://crates.io/crates/opensxraw)
[![PyPI](https://img.shields.io/pypi/v/opensxraw.svg)](https://pypi.org/project/opensxraw/)
[![docs.rs](https://img.shields.io/docsrs/opensxraw)](https://docs.rs/opensxraw)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust MSRV](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

> Part of the [OpenMassSpec](https://github.com/Sigilweaver/OpenMassSpec)
> stack for mass spectrometry raw-file access.

Rust and Python reader for SCIEX `.wiff`/`.wiff.scan` legacy mass
spectrometry data files, with no SCIEX SDK or software dependency.
Covers the TripleTOF and QTRAP instrument families.

Documentation: [sigilweaver.app/opensxraw/docs](https://sigilweaver.app/opensxraw/docs)

## Install

**Prefer [`openmassspec-io`](https://github.com/Sigilweaver/OpenMassSpec)
with the `sciex` feature** unless you need this parser standalone
(minimal dependencies, or building your own abstraction) - the umbrella
gives you format auto-detection, mzML conversion, and Arrow streaming
across all wired-in vendors for free:

```sh
cargo add openmassspec-io --features sciex
```

```sh
pip install openmassspec[sciex]
```

Standalone:

Rust:

```sh
cargo add opensxraw
```

Python:

```sh
pip install opensxraw
```

## Quickstart

Rust:

```rust
use opensxraw::reader::Reader;
use openmassspec_core::SpectrumSource;

let mut reader = Reader::open("sample.wiff")?;
for spectrum in reader.iter_spectra() {
    println!("{}: {} peaks", spectrum.native_id, spectrum.mz.len());
}
```

Python:

```python
import opensxraw

reader = opensxraw.RawReader("sample.wiff")
spectrum = reader.read_spectrum(0)
print(spectrum.ms_level, spectrum.retention_time_sec, len(spectrum.mz))
```

`Reader::open` (and `RawReader`) expects the paired `.wiff.scan` file to
sit alongside the `.wiff` file, with `.scan` appended to the `.wiff`
filename.

See the [docs site](https://sigilweaver.app/opensxraw/docs) for the full
guide, format specification, and API reference.

## License

Apache-2.0. See [LICENSE](LICENSE).

The format specification was developed by binary analysis of public
mass-spectrometry datasets (PRIDE accessions). See
[CORPUS.md](CORPUS.md) and [ATTRIBUTION.md](ATTRIBUTION.md).
