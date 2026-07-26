# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- `Reader::iter_spectra` no longer hardcodes `scan_mode: Some(ScanMode::Profile)`
  for every spectrum (Sigilweaver/OpenSXRaw#27): that mislabeled centroided
  SCIEX acquisitions as profile data. `scan_mode` is now left `None` instead.
  No currently-decoded stream
  (`Idx`, `DDERealTimeDataEx`, `SummaryInformation`) carries a per-scan or
  per-experiment centroid/profile indicator to classify against, and per the
  project's clean-room policy this reader does not guess at one without a
  verified source - see the doc comment above the `scan_mode` field in
  `reader.rs` for the streams checked and ruled out. Finding the real
  indicator (if one exists in an undecoded stream) is tracked as follow-up
  work. (contributed by @Nabejo)

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

## [0.2.3] - 2026-07-25

### Fixed

- Adapted `RunMetadata` construction to `openmassspec-core` 1.3.0's new
  `analyzers`/`instrument_serial_number` fields (defaulted, as neither is
  decoded here - see `raw::summary_info`'s module doc for why no reliable
  serial number source exists).
- Declared `openmassspec-core` minimum was still `"1.0.0"`, undercounting
  every prior bump; now that the code needs 1.3.0's new `RunMetadata`
  fields to compile, bumped the declared minimum to `"1.3.0"` to match.

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
