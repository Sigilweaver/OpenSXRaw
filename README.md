# OpenSXRaw

[![CI](https://github.com/Sigilweaver/OpenSXRaw/actions/workflows/ci.yml/badge.svg)](https://github.com/Sigilweaver/OpenSXRaw/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/opensxraw.svg)](https://crates.io/crates/opensxraw)
[![PyPI](https://img.shields.io/pypi/v/opensxraw.svg)](https://pypi.org/project/opensxraw/)
[![docs.rs](https://img.shields.io/docsrs/opensxraw)](https://docs.rs/opensxraw)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/Sigilweaver/OpenSXRaw/blob/main/LICENSE)
[![Rust MSRV](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)

> Part of the [OpenMassSpec](https://github.com/Sigilweaver/OpenMassSpec)
> stack for mass spectrometry raw-file access.

Rust and Python reader for SCIEX `.wiff`/`.wiff.scan` legacy mass
spectrometry data files, with no SCIEX SDK or software dependency.
Covers the TripleTOF and QTRAP instrument families.

Documentation: [sigilweaver.app/opensxraw/docs](https://sigilweaver.app/opensxraw/docs)

## Known issues and gaps

These apply to opensxraw 0.4.0 and all earlier releases. Figures come from
checks run on the public validation corpus (see [CORPUS.md](CORPUS.md)).

- **m/z values are not reliable on any instrument family.** No part of the
  m/z axis has been validated against a physical reference. Do not use
  OpenSXRaw m/z values for identification, accurate-mass work, or
  m/z-based quantification.
- **TripleTOF and ZenoTOF m/z values are wrong.** The reader applies one
  linear `(slope, intercept)` calibration per file. In a sample of 48 TOF
  files, 33 contain negative m/z values (down to about -16) and 8
  proteomics runs never exceed m/z 700. In 6 files checked in detail,
  peaks do not line up with known peptide fragment ions, common background
  ions, or 1.003 Da isotope spacing; offsets scatter across +-2000 ppm
  with no cluster. This affects all 170 TripleTOF 5600/5600+/6600 and
  ZenoTOF 7600 `.wiff` files in the corpus. The shape of the error is not
  yet known, so any recalibration you apply must be checked against known
  ions in your own data.
- **QTRAP m/z values are raw detector bins, not m/z.** They are integers
  from 0 to about 179,000 and can repeat within a spectrum. This affects
  all 27 QTRAP 5500/6500+ files in the corpus.
- **MS level is wrong on TripleTOF and ZenoTOF.** Every scan is labelled
  MS2, including MS1 survey scans, so TOF MS2 spectra also have no
  precursor m/z and no link to a parent MS1 scan. Treat `ms_level` as
  unreliable on these files. QTRAP MS levels are correct.
- **MRM transitions are not reported.** QTRAP MRM spectra do not expose
  their Q1/Q3 transitions, and no SRM chromatograms are produced. Only a
  TIC chromatogram is available.
- **`.wiff2` is not supported.** The format is encrypted. The reader
  currently rejects these files with "Invalid CFB file (wrong magic
  number)" rather than a message naming `.wiff2`.
- **Multi-sample `.wiff` files** must be opened with `Reader::open_sample`
  (Python: `opensxraw.open_sample`); `Reader::open` returns an error.
- **Not reported or untested:** centroid/profile mode (`scan_mode` is
  always `None`); negative polarity (handled in code, but no negative-mode
  file is in the corpus).

What has been checked and holds: the `.wiff` container and `Idx` scan
index, retention times (no backwards steps or read failures in a 54-file,
1.16 million-spectrum sample), instrument model, polarity, acquisition
timestamp, and QTRAP MS levels. On QTRAP DDA files, MS2 precursor m/z is
the value the instrument recorded at acquisition (present on 3,084 of
3,087 sampled MS2 scans); it has not been cross-checked against MS1,
because QTRAP MS1 m/z is in raw bins.

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
    let spectrum = spectrum?;
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

Apache-2.0. See [LICENSE](https://github.com/Sigilweaver/OpenSXRaw/blob/main/LICENSE).

The format specification was developed by binary analysis of public
mass-spectrometry datasets (PRIDE accessions). See
[CORPUS.md](https://github.com/Sigilweaver/OpenSXRaw/blob/main/CORPUS.md) and [ATTRIBUTION.md](https://github.com/Sigilweaver/OpenSXRaw/blob/main/ATTRIBUTION.md).
