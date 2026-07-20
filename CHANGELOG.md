# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2] - 2026-07-20

### Fixed

- `read_scan_block` no longer allocates an unbounded read buffer from a
  crafted or corrupted Idx offset (a memory-DoS on malformed `.wiff`
  input). The read length is now bounded by the Idx's own `scan_size`
  field (previously computed but unused), the actual `.wiff.scan` file
  size, and a sane absolute ceiling, replacing a `min()` cap that was
  always a no-op. (#1, contributed by @Nabejo)

### Testing

- Added synthetic byte-slice unit tests for `IdxRecord` parsing, the
  `scan.rs` terminator scan, and `read_scan_block`'s offset bounds
  (including a regression test for the crafted-offset DoS fixed in #1),
  plus `points_to_arrays`. None of these need the out-of-tree corpus.
  (#2, contributed by @Nabejo)

## [0.2.1] - 2026-07-15

### Fixed

- Bumped `openmassspec-core` to 1.2.0 and added the `SpectrumRecord.faims_cv`
  field it requires, fixing a build break: 1.2.0 added that field as
  required, and `Reader::iter_spectra` constructed the struct literal
  without it. Always `None` - SCIEX instruments have no FAIMS interface.

## [0.2.0] - 2026-07-11

### Added

- Python bindings via a new `opensxraw-py` PyO3 crate, exposing
  `RawReader` and `Spectrum` to mirror the sibling readers' Python API.
  Packaged as `opensxraw` on PyPI; wheels (Linux/macOS/Windows) and an
  sdist build and publish from the release workflow.

### Testing

- The corpus conformance tests now skip cleanly (instead of failing the
  build) when the out-of-tree corpus is absent, e.g. on CI runners.

## [0.1.0] - 2026-07-11

### Added

- Initial Rust reader (`opensxraw`) for legacy SCIEX `.wiff`/`.wiff.scan`
  files, covering TripleTOF and QTRAP instrument families.
- Full CFBF stream catalog and `.wiff.scan` block/token-stream decoding,
  documented in `docs/format/`.
- `.wiff2` container investigation: confirmed proprietary AES page
  encryption (SQLCipher-style) and structural analysis of the
  plaintext/ciphertext boundary - see
  [docs/format/03-wiff2-container.md](docs/format/03-wiff2-container.md).
  `.wiff2` support remains deferred pending new information.
- Project renamed `OpenSRaw` -> `OpenSXRaw`.

### Known limitations

- m/z values are raw, uncalibrated time-bin integers - physical
  calibration requires `ExperimentTOF` method-stream constants, not yet
  decoded.
- MS2 precursor m/z is not yet populated (`DDERealTimeDataEx` not yet
  decoded); a placeholder native ID satisfies the shared conformance
  invariant in the meantime.
- The reader currently reports every spectrum as profile-mode / TOFMS
  analyzer regardless of actual instrument family (QTRAP records are
  nominal-mass, not true TOF) - this is a simplification, not yet
  instrument-aware.
