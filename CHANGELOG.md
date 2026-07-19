# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Reader::iter_chromatograms` (Sigilweaver/OpenSXRaw#21): emits a single
  total ion current chromatogram (`MS:1000235`) built from the already-decoded
  `idx_records` - one point per record, `time_sec` from `retention_time_min`
  and `intensity` from the record's Idx `tic` (cps). No new raw-format decode
  work is involved, only wiring existing fields into `ChromatogramRecord`, so
  TIC chromatograms now appear in the mzML `<chromatogramList>` OpenSXRaw
  produces. The per-spectrum `SpectrumRecord.total_ion_current` field stays
  `None` as before (that value must match `sum(raw intensities)` for the
  conformance suite's `rel_close` check, which does not apply to a separate
  chromatogram trace). Basepeak (BPC) and SRM/MRM chromatograms are
  intentionally left out - both require net-new decode work and should be
  tracked as separate follow-up issues. (contributed by @Nabejo)

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
