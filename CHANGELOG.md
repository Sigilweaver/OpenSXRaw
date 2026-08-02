# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- CI now downloads the `PXD022088/Rcor2KOESC1` `.wiff` + `.wiff.scan` pair
  from PRIDE ahead of `cargo test`, so `crates/opensxraw/tests/conformance.rs`
  exercises a real decode path in CI instead of always skipping.
  (Sigilweaver/OpenSXRaw#33)

## [0.2.4] - 2026-07-29

### Fixed

- Adapted `RunMetadata` construction to `openmassspec-core` 1.4.0's new
  `acquisition_software_name`/`acquisition_software_version` fields
  (defaulted to `None`: no stream this reader currently decodes carries a
  parsed Analyst software version - `raw::summary_info`'s module doc notes
  `DocumentSummaryInformation` has a candidate free-text string, but that
  stream isn't parsed here yet). Bumped the declared `openmassspec-core`
  minimum to `"1.4.0"` to match. (Sigilweaver/OpenSXRaw#31)
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

- `Reader::list_samples` and `Reader::open_sample` (#25): a `.wiff`
  container can hold more than one sample (`SampleSubtree/Sample1`,
  `Sample2`, ...), but every stream path in the reader was hardcoded to
  `Sample1`, so multi-sample files had every other sample silently
  dropped with no error or warning. `Reader::list_samples` enumerates the
  sample subtrees actually present in a file (found by walking the CFBF
  directory, a public Microsoft container format, not SCIEX-specific), and
  `Reader::open_sample` opens a specific one by name. `Reader::open` keeps
  its existing signature for the common single-sample case, but now
  returns a clear error instead of quietly reading only the first sample
  when a file turns out to hold more than one - callers that need a
  specific sample from a multi-sample file should call `open_sample`
  directly. No corpus file with more than one sample was available to
  verify against, so the multi-sample paths are covered by synthetic CFBF
  fixtures built with the `cfb` crate's own write API rather than guessed
  at. (contributed by @Nabejo)

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

### Documentation

- Documented the `SpectrumRecord.polarity` investigation for
  Sigilweaver/OpenSXRaw#26 directly on the field in `reader.rs` (plus a
  cross-reference note on the `Idx` record's "Unknown" bytes in
  `raw/idx.rs`). None of the streams this reader currently decodes
  (`SummaryInformation`, `TOFCalibrationData`, `DDERealTimeDataEx`, `Idx`,
  the `.wiff.scan` token stream) carry a per-scan or per-run polarity
  signal - the `Idx` record's two "Unknown" bytes were already checked by
  issue #7 (for an unrelated bug) and found uniformly zero, ruling them
  out too. Polarity most likely lives in the undecoded per-Experiment
  `ExperimentHeader`/`ExperimentHeaderEx` method structures. No behavior
  change: `polarity` stays `None`, now as a documented, investigated
  conclusion rather than a silent placeholder, per this project's
  clean-room "don't guess" policy. (Sigilweaver/OpenSXRaw#26, contributed
  by @Nabejo)

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
