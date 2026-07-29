//! High-level reader for a SCIEX legacy `.wiff` + `.wiff.scan` pair.

use std::io::Read;
use std::path::{Path, PathBuf};

use cfb::CompoundFile;
use openmassspec_core::{
    Analyzer, ChromatogramRecord, CvTerm, PrecursorInfo, RunMetadata, SpectrumRecord,
    SpectrumSource,
};

use crate::raw::calibration::Calibration;
use crate::raw::dde::DdeRecord;
use crate::raw::idx::IdxRecord;
use crate::raw::scan::{read_scan_block, ScanPoint};
use crate::raw::summary_info::parse_create_timestamp;

/// The CFBF storage under which each sample's data lives, one child storage
/// per sample (`Sample1`, `Sample2`, ...). A `.wiff` file can hold more than
/// one sample (see Sigilweaver/OpenSXRaw#25); `Reader::list_samples`
/// enumerates the child storages actually present so callers aren't limited
/// to whatever `Reader::open` picks by default.
const SAMPLE_SUBTREE_STORAGE: &str = "SampleSubtree";

/// The CFBF stream path for the scan index of the given sample.
fn idx_stream(sample: &str) -> String {
    format!("{SAMPLE_SUBTREE_STORAGE}/{sample}/Idx")
}

/// The CFBF stream path for the standard OLE SummaryInformation property
/// set. The leading `\x05` is the OLE convention marking a stream name as
/// reserved/special rather than user data. See `raw::summary_info` for the
/// investigation behind using this as the acquisition start timestamp. This
/// stream lives at the container root rather than under a sample's storage.
const SUMMARY_INFO_STREAM: &str = "\x05SummaryInformation";

/// The CFBF stream path for TOF m/z calibration constants of the given
/// sample. Only present on TripleTOF-family acquisitions - see
/// `raw::calibration` and `docs/format/04-legacy-wiff-calibration.md`.
fn calibration_stream(sample: &str) -> String {
    format!("{SAMPLE_SUBTREE_STORAGE}/{sample}/TOFCalibrationData")
}

/// The CFBF stream path for data-dependent precursor selection records of
/// the given sample. Only present on files with IDA/DDA-style precursor
/// triggering - see `raw::dde` and `docs/format/04-legacy-wiff-calibration.md`.
fn dde_stream(sample: &str) -> String {
    format!("{SAMPLE_SUBTREE_STORAGE}/{sample}/DDERealTimeDataEx")
}

/// Order sample subtree names so the common `SampleN` convention sorts
/// numerically (`Sample2` before `Sample10`) rather than lexicographically,
/// which is how the CFBF directory itself orders them internally. Names
/// that don't fit the `Sample<digits>` pattern sort after ones that do, in
/// alphabetical order.
fn sort_sample_names(names: &mut [String]) {
    names.sort_by_key(|name| {
        match name
            .strip_prefix("Sample")
            .and_then(|n| n.parse::<u64>().ok())
        {
            Some(n) => (0u8, n, String::new()),
            None => (1u8, 0u64, name.clone()),
        }
    });
}

/// Open state for a `.wiff` / `.wiff.scan` pair.
pub struct Reader {
    /// Stem name of the file (e.g. "Rcor2KOESC1") used in native IDs.
    pub stem: String,
    /// Path to the `.wiff.scan` file.
    scan_path: PathBuf,
    /// Decoded index records, in order.
    pub idx_records: Vec<IdxRecord>,
    /// File size of the `.wiff.scan` file (used to bound the last block read).
    scan_file_size: u64,
    /// Acquisition start timestamp (RFC 3339, UTC), read from the `.wiff`
    /// container's standard OLE `SummaryInformation` property set. `None`
    /// when that stream is absent or unparseable - see `raw::summary_info`.
    pub start_timestamp: Option<String>,
    /// Linear m/z calibration constants read from `TOFCalibrationData`.
    /// `None` on files without that stream (e.g. QTRAP-only acquisitions),
    /// in which case `mz` arrays stay as raw uncalibrated bin values - see
    /// `raw::calibration`.
    calibration: Option<Calibration>,
    /// Decoded `DDERealTimeDataEx` records, in stream order. Empty on files
    /// without that stream (no DDA-style precursor triggering).
    dde_records: Vec<DdeRecord>,
}

impl Reader {
    /// List the sample subtree names present in a `.wiff` file's
    /// `SampleSubtree` storage, e.g. `["Sample1"]` for the common
    /// single-sample case, or `["Sample1", "Sample2"]` when the container
    /// holds more than one sample (see Sigilweaver/OpenSXRaw#25). The
    /// returned names are sorted numerically by their trailing digits (see
    /// `sort_sample_names`) and are suitable to pass straight to
    /// `Reader::open_sample`.
    ///
    /// This only opens the `.wiff` container and walks its directory
    /// structure - it does not read or decode any sample's data.
    pub fn list_samples<P: AsRef<Path>>(wiff_path: P) -> crate::Result<Vec<String>> {
        let wiff_path = wiff_path.as_ref();
        let wiff_file = std::fs::File::open(wiff_path)?;
        let comp = CompoundFile::open(wiff_file)?;

        let mut names: Vec<String> = comp
            .read_storage(SAMPLE_SUBTREE_STORAGE)
            .map_err(|e| {
                crate::Error::Parse(format!(
                    "storage '{SAMPLE_SUBTREE_STORAGE}' not found in {}: {}",
                    wiff_path.display(),
                    e
                ))
            })?
            .filter(|entry| entry.is_storage())
            .map(|entry| entry.name().to_string())
            .collect();

        sort_sample_names(&mut names);
        Ok(names)
    }

    /// Open a `.wiff` file and its paired `.wiff.scan` file.
    ///
    /// `wiff_path` is the path to the `.wiff` file. The `.wiff.scan` file is
    /// expected at the same path with `.scan` appended.
    ///
    /// A `.wiff` container can hold more than one sample (see
    /// Sigilweaver/OpenSXRaw#25). This constructor only supports the
    /// common single-sample case: if `list_samples` finds more than one
    /// sample subtree, this returns an error rather than silently reading
    /// just one of them - use `Reader::open_sample` to pick a specific
    /// sample explicitly.
    pub fn open<P: AsRef<Path>>(wiff_path: P) -> crate::Result<Self> {
        let wiff_path = wiff_path.as_ref();
        let samples = Self::list_samples(wiff_path)?;

        let sample = match samples.as_slice() {
            [] => {
                return Err(crate::Error::Parse(format!(
                    "no sample subtrees found under '{SAMPLE_SUBTREE_STORAGE}' in {}",
                    wiff_path.display()
                )));
            }
            [only] => only.clone(),
            multiple => {
                return Err(crate::Error::Parse(format!(
                    "{} contains {} samples ({}); Reader::open only supports a \
                     single-sample file - use Reader::open_sample to pick one",
                    wiff_path.display(),
                    multiple.len(),
                    multiple.join(", ")
                )));
            }
        };

        Self::open_sample(wiff_path, &sample)
    }

    /// Open a specific sample within a `.wiff` file and its paired
    /// `.wiff.scan` file.
    ///
    /// `wiff_path` is the path to the `.wiff` file. The `.wiff.scan` file is
    /// expected at the same path with `.scan` appended. `sample` is a
    /// sample subtree name as returned by `Reader::list_samples` (e.g.
    /// `"Sample1"`, `"Sample2"`).
    pub fn open_sample<P: AsRef<Path>>(wiff_path: P, sample: &str) -> crate::Result<Self> {
        let wiff_path = wiff_path.as_ref();

        // Build .wiff.scan path: append ".scan" to the .wiff extension.
        let scan_path = {
            let mut p = wiff_path.to_path_buf();
            let mut name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            name.push_str(".scan");
            p.set_file_name(name);
            p
        };

        if !scan_path.exists() {
            return Err(crate::Error::Parse(format!(
                ".wiff.scan file not found: {}",
                scan_path.display()
            )));
        }

        let scan_file_size = std::fs::metadata(&scan_path)?.len();

        // Open the CFBF container. CompoundFile::open takes any Read + Seek,
        // so open a std::fs::File first.
        let wiff_file = std::fs::File::open(wiff_path)?;
        let mut comp = CompoundFile::open(wiff_file)?;

        // Read the Idx stream.
        let idx_path = idx_stream(sample);
        let idx_data = {
            let mut stream = comp.open_stream(&idx_path).map_err(|e| {
                crate::Error::Parse(format!("Stream '{}' not found: {}", idx_path, e))
            })?;
            let mut buf = Vec::new();
            stream.read_to_end(&mut buf)?;
            buf
        };

        let idx_records = IdxRecord::parse_stream(&idx_data)?;

        // Read the acquisition start timestamp from the standard OLE
        // SummaryInformation property set, if present. This is optional
        // metadata: not every `.wiff` file carries this stream (see
        // `raw::summary_info`'s corpus survey), so any failure here just
        // leaves `start_timestamp` as `None` rather than failing `open`.
        let start_timestamp = comp
            .open_stream(SUMMARY_INFO_STREAM)
            .ok()
            .and_then(|mut stream| {
                let mut buf = Vec::new();
                stream.read_to_end(&mut buf).ok()?;
                parse_create_timestamp(&buf)
            });

        // Read TOF calibration constants, if present. Absent on QTRAP-only
        // files - see `raw::calibration`.
        let calibration =
            comp.open_stream(calibration_stream(sample))
                .ok()
                .and_then(|mut stream| {
                    let mut buf = Vec::new();
                    stream.read_to_end(&mut buf).ok()?;
                    Calibration::from_bytes(&buf)
                });

        // Read DDA precursor-selection records, if present. Absent on files
        // without IDA/DDA-style precursor triggering - see `raw::dde`.
        let dde_records = comp
            .open_stream(dde_stream(sample))
            .ok()
            .map(|mut stream| {
                let mut buf = Vec::new();
                let _ = stream.read_to_end(&mut buf);
                DdeRecord::parse_stream(&buf)
            })
            .unwrap_or_default();

        let stem = wiff_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".into());

        Ok(Reader {
            stem,
            scan_path,
            idx_records,
            scan_file_size,
            start_timestamp,
            calibration,
            dde_records,
        })
    }
}

/// Convert raw scan points to parallel mz/intensity vectors.
///
/// When `calibration` is available (TripleTOF-family files), the raw m/z
/// bin is converted to physical m/z via `Calibration::apply`. Otherwise
/// (QTRAP-only files - see `raw::calibration`) the raw bin is used as-is,
/// matching prior behavior. Zero-intensity points are dropped because they
/// are background artefacts of the zero-suppressed encoding.
fn points_to_arrays(
    points: Vec<ScanPoint>,
    calibration: Option<Calibration>,
) -> (Vec<f64>, Vec<f32>) {
    let mut mz = Vec::with_capacity(points.len());
    let mut intensity = Vec::with_capacity(points.len());
    for p in points {
        if p.raw_intensity > 0 {
            let point_mz = match calibration {
                Some(cal) => cal.apply(p.raw_mz_bin),
                None => p.raw_mz_bin as f64,
            };
            mz.push(point_mz);
            intensity.push(p.raw_intensity as f32);
        }
    }
    (mz, intensity)
}

impl SpectrumSource for Reader {
    fn run_metadata(&self) -> RunMetadata {
        RunMetadata {
            source_file_name: format!("{}.wiff", self.stem),
            source_file_format: CvTerm::new("MS:1000562", "ABI WIFF format"),
            native_id_format: CvTerm::new("MS:1000823", "SCIEX nativeID format"),
            // Still a generic placeholder, not resolved per-file: no CFBF
            // stream was found carrying a vendor-populated instrument model
            // string. The only candidate text (SummaryInformation's
            // author/comments fields, CFR_INFO) is Analyst's free-text
            // "instrument name" plus the acquisition PC's hostname, both
            // configured per-site rather than written by the instrument
            // firmware - see `raw::summary_info`'s module doc for the
            // corpus evidence this isn't reliable enough to promote to a
            // specific model term.
            //
            // Issue #4 round 2 went further and enumerated every other
            // stream in the container (DocumentSummaryInformation,
            // FileRec_Str, VendorAppMethod, CFR/CFRFileHeader, device/method
            // tables, and a corpus-wide model-substring scan), plus probed
            // the binary MSConfigInfo struct for a structured instrument
            // type field. None panned out - see `raw::summary_info`'s
            // module doc ("Round 2") for the full list and why each was
            // ruled out. This is confirmed investigated-and-not-resolvable
            // from the current file structure, not just unattempted.
            instrument: CvTerm::new("MS:1000121", "SCIEX instrument model"),
            // No serial number source either - same ruled-out CFBF streams
            // as the instrument model above.
            instrument_serial_number: None,
            software_name: "opensxraw".to_string(),
            software_version: env!("CARGO_PKG_VERSION").to_string(),
            // No Analyst acquisition software version is wired up here: the
            // only candidate source is the `DocumentSummaryInformation`
            // stream's "Analyst file type: Data file, Analyst <version>"
            // free-text string noted in `raw::summary_info`'s module doc
            // (Round 2 survey) - that stream isn't parsed anywhere in this
            // crate yet, so there is nothing to wire in without new parsing
            // work.
            acquisition_software_name: None,
            acquisition_software_version: None,
            start_timestamp: self.start_timestamp.clone(),
            mobility_array_kind: None,
            analyzers: Vec::new(),
        }
    }

    fn spectrum_count_hint(&self) -> Option<usize> {
        Some(self.idx_records.len())
    }

    fn iter_spectra<'a>(&'a mut self) -> Box<dyn Iterator<Item = SpectrumRecord> + 'a> {
        // Clone everything the iterator needs so it can be Send-compatible and
        // avoids borrow issues with the mutable self reference.
        let records = self.idx_records.clone();
        let scan_path = self.scan_path.clone();
        let scan_file_size = self.scan_file_size;
        let stem = self.stem.clone();
        let calibration = self.calibration;
        let dde_records = self.dde_records.clone();

        // Build an offset table for lookahead: next_offsets[i] is the byte
        // offset to use as the end bound when reading block i's payload.
        // For block i, we use records[i+1].scan_offset; for the last block,
        // use the file size.
        let next_offsets: Vec<u64> = {
            let mut v = Vec::with_capacity(records.len());
            for i in 0..records.len() {
                let next = if i + 1 < records.len() {
                    records[i + 1].scan_offset as u64
                } else {
                    scan_file_size
                };
                v.push(next);
            }
            v
        };

        // For each record, precompute the native ID of the most recent MS1
        // scan seen *before* it, and how many MS1 scans have completed
        // before it. The latter indexes into `dde_records`: DDERealTimeDataEx
        // carries one entry per DDA cycle (matching MS1 count, not MS2
        // count - see `raw::dde`), so an MS2 scan's precursor is the DDE
        // record at (MS1-scans-seen-so-far - 1).
        let (last_ms1_native_id, ms1_count_before): (Vec<Option<String>>, Vec<usize>) = {
            let mut last_ids = Vec::with_capacity(records.len());
            let mut counts = Vec::with_capacity(records.len());
            let mut cur_last_id: Option<String> = None;
            let mut cur_count = 0usize;
            for (i, rec) in records.iter().enumerate() {
                last_ids.push(cur_last_id.clone());
                counts.push(cur_count);
                if rec.ms_level == 1 {
                    cur_last_id = Some(format!("file={} scan={}", stem, i + 1));
                    cur_count += 1;
                }
            }
            (last_ids, counts)
        };

        let iter = records
            .into_iter()
            .zip(next_offsets)
            .zip(last_ms1_native_id)
            .zip(ms1_count_before)
            .enumerate()
            .map(
                move |(idx, (((rec, next_offset), last_ms1_id), ms1_count))| {
                    let native_id = format!("file={} scan={}", stem, idx + 1);

                    // Precursor info for MS2 spectra. `precursor_native_id`
                    // references the preceding MS1 survey scan actually seen in
                    // this file's Idx order. `selected_mz`/`target_mz` come from
                    // DDERealTimeDataEx when available (heuristic cycle-based
                    // linkage - see `raw::dde`); when that stream is absent or
                    // the linkage doesn't resolve, precursor m/z stays `None`
                    // rather than a guess. A small number of files have an MS2
                    // scan before any MS1 has been seen at all (no survey scan
                    // to reference yet); fall back to an explicit "unknown"
                    // placeholder id only in that edge case, so the record still
                    // carries the required precursor info without fabricating a
                    // scan reference.
                    let precursor = if rec.ms_level >= 2 {
                        let precursor_mz = ms1_count
                            .checked_sub(1)
                            .and_then(|dde_idx| dde_records.get(dde_idx))
                            .map(|dde| dde.precursor_mz);
                        let precursor_native_id =
                            last_ms1_id.or_else(|| Some(format!("file={} ms1ref=unknown", stem)));
                        Some(PrecursorInfo {
                            selected_mz: precursor_mz,
                            target_mz: precursor_mz,
                            precursor_native_id,
                            ..Default::default()
                        })
                    } else {
                        None
                    };

                    // Decode the scan payload.
                    let (mz, intensity) = {
                        let points = read_scan_block(
                            &scan_path,
                            rec.scan_offset as u64,
                            rec.scan_size as u64,
                            next_offset,
                            scan_file_size,
                        )
                        .unwrap_or_default();
                        points_to_arrays(points, calibration)
                    };

                    SpectrumRecord {
                        index: idx,
                        scan_number: (idx + 1) as u32,
                        native_id,
                        ms_level: rec.ms_level,
                        // Investigated for issue #26 ("polarity hardcoded to
                        // None for every spectrum") and still `None`: no
                        // stream this reader currently decodes carries a
                        // per-scan or per-run polarity signal.
                        //
                        // - `raw::summary_info` (`SummaryInformation`/
                        //   `DocumentSummaryInformation`): only the OLE
                        //   creation timestamp and free-text author/company
                        //   fields - no polarity property.
                        // - `raw::calibration` (`TOFCalibrationData`): only
                        //   the linear (slope, intercept) m/z pair.
                        // - `raw::dde` (`DDERealTimeDataEx`): only a
                        //   precursor m/z per DDA cycle.
                        // - `raw::idx` (`Idx` record): the two "Unknown"
                        //   bytes (0x08, 0x11) were checked by issue #7 while
                        //   chasing a different bug and found uniformly zero
                        //   across ~97k records of a multi-Experiment
                        //   (SWATH/DDA) corpus fixture - not a varying
                        //   per-scan flag of any kind, polarity included.
                        // - `raw::scan` (`.wiff.scan` block payload): a pure
                        //   m/z/intensity token stream, no header fields.
                        //
                        // On this instrument family, polarity is a method
                        // setting recorded per-Experiment (SCIEX
                        // `MethodSubtree/Method1/DeviceMethod0/PeriodN/
                        // ExperimentN/ExperimentHeader(Ex)`), which this
                        // reader does not decode at all yet - that binary
                        // struct's layout is an open question, not something
                        // ruled out. Populating this field would require
                        // decoding that struct and correlating each scan back
                        // to its owning Experiment, clean-room, against
                        // corpus fixtures; per CONTRIBUTING's "don't guess"
                        // policy, `None` (this reader's existing
                        // "not resolved" convention, matching
                        // `instrument_serial_number` and
                        // `mobility_array_kind` elsewhere in this file) is
                        // left in place rather than fabricating a decode.
                        // `openmassspec_core::Polarity` has no `Unknown`
                        // variant, so `Option::None` is the correct way to
                        // represent "not determined" here.
                        polarity: None,
                        // Left unset rather than asserting `Profile` for every
                        // scan (issue #27): that was wrong whenever the
                        // source acquisition is actually centroided, and none
                        // of the streams already decoded here carry a
                        // per-scan or per-experiment centroid/profile
                        // indicator to replace it with. Checked and ruled out
                        // for this: the Idx record's byte 0x11 ("Unknown",
                        // always observed as 0x00 - see `raw::idx`), the 64
                        // unidentified trailing bytes of each
                        // `DDERealTimeDataEx` record (see `raw::dde`), and the
                        // `SummaryInformation`/`DocumentSummaryInformation`
                        // property sets (see `raw::summary_info`) - none
                        // distinguish centroid from profile scans in the
                        // fixtures available in this pass. A structural
                        // heuristic based on the decoded token stream itself
                        // (e.g. regular fixed-grid deltas vs. sparse isolated
                        // peaks) was considered but rejected: with no known-
                        // centroid/known-profile ground truth to validate it
                        // against, promoting it to real output would be
                        // exactly the kind of unverified guess the project's
                        // clean-room policy rules out. Finding the actual
                        // indicator (if one exists in an undecoded stream) is
                        // open follow-up work.
                        //
                        // Note this does not fully resolve the mzML-level
                        // symptom: `openmassspec_core`'s mzML writer treats
                        // any `scan_mode` other than `Some(ScanMode::Centroid)`
                        // (including `None`) as profile spectrum
                        // (`MS:1000128`) for cvParam purposes. Leaving this
                        // `None` stops OpenSXRaw from asserting a specific
                        // mode it cannot actually verify, but does not by
                        // itself relabel centroided acquisitions as centroid
                        // in the mzML output - that requires the indicator
                        // above to be found first.
                        scan_mode: None,
                        analyzer: Some(Analyzer::TOFMS),
                        filter: None,
                        retention_time_sec: rec.retention_time_min as f64 * 60.0,
                        // Do NOT populate total_ion_current: the Idx TIC is in cps
                        // (physically calibrated) and does not match sum(raw intensities).
                        // The conformance suite checks this with rel_close; leaving None
                        // means the mzML writer will compute TIC from intensity arrays
                        // instead.
                        total_ion_current: None,
                        base_peak_mz: None,
                        base_peak_intensity: None,
                        low_mz: None,
                        high_mz: None,
                        ion_injection_time_ms: None,
                        inv_mobility: None,
                        faims_cv: None, // SCIEX instruments have no FAIMS interface.
                        precursor,
                        mz,
                        intensity,
                        inv_mobility_per_peak: None,
                    }
                },
            );

        Box::new(iter)
    }

    fn iter_chromatograms<'a>(&'a mut self) -> Box<dyn Iterator<Item = ChromatogramRecord> + 'a> {
        // Emit a single total-ion-current chromatogram built entirely from
        // data already decoded into `idx_records`: one point per Idx record,
        // `time_sec` from `retention_time_min * 60.0` and `intensity` from the
        // record's `tic` field. No new raw-format decode work is involved,
        // only wiring existing fields into the chromatogram shape.
        //
        // This is deliberately distinct from the per-spectrum
        // `SpectrumRecord.total_ion_current` field, which `iter_spectra`
        // leaves `None` on purpose: the conformance suite checks that field
        // with `rel_close` against `sum(raw intensities)`, and the Idx TIC is
        // in cps (physically calibrated) so it would not match. That check
        // applies only to the per-spectrum field; a TIC chromatogram is a
        // separate trace of (retention time, cps) points that the conformance
        // suite never inspects, so the mismatch reasoning does not apply here.
        //
        // Only TIC is emitted. A basepeak chromatogram (BPC) would need a
        // per-scan base peak, which nothing decodes today, and SRM/MRM
        // chromatograms would need transition-level data that this reader
        // does not decode at all - both are tracked as separate follow-ups
        // (see Sigilweaver/OpenSXRaw#21).
        let mut time_sec = Vec::with_capacity(self.idx_records.len());
        let mut intensity = Vec::with_capacity(self.idx_records.len());
        for rec in &self.idx_records {
            time_sec.push(rec.retention_time_min * 60.0);
            intensity.push(rec.tic as f32);
        }

        let record = ChromatogramRecord {
            index: 0,
            id: "TIC".to_string(),
            chromatogram_type: Some(CvTerm::new("MS:1000235", "total ion current chromatogram")),
            precursor_mz: None,
            product_mz: None,
            time_sec,
            intensity,
        };

        Box::new(std::iter::once(record))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::calibration::Calibration;
    use std::path::PathBuf;

    /// Build a `Reader` from synthetic `idx_records` only, bypassing file I/O,
    /// so the chromatogram wiring can be exercised without a corpus fixture.
    /// This mirrors what `Reader::open` populates: `iter_chromatograms` reads
    /// only `idx_records`, so the remaining fields are inert placeholders.
    fn reader_with_idx(idx_records: Vec<IdxRecord>) -> Reader {
        Reader {
            stem: "synthetic".to_string(),
            scan_path: PathBuf::from("synthetic.wiff.scan"),
            idx_records,
            scan_file_size: 0,
            start_timestamp: None,
            calibration: None,
            dde_records: Vec::new(),
        }
    }

    fn idx_record(retention_time_min: f32, ms_level: u32, tic: f64) -> IdxRecord {
        IdxRecord {
            scan_offset: 0,
            scan_size: 0,
            retention_time_min,
            ms_level,
            tic,
            _field_1a: 0.0,
        }
    }

    #[test]
    fn iter_chromatograms_emits_single_tic_from_idx_records() {
        // One MS1 and one MS2 record: the TIC chromatogram has one point per
        // Idx record regardless of MS level.
        let mut reader =
            reader_with_idx(vec![idx_record(0.0, 1, 100.0), idx_record(0.5, 2, 250.0)]);

        let chroms: Vec<ChromatogramRecord> = reader.iter_chromatograms().collect();

        assert_eq!(chroms.len(), 1, "expected exactly one TIC chromatogram");
        let tic = &chroms[0];
        assert_eq!(tic.index, 0);
        assert_eq!(tic.id, "TIC");
        let cv = tic
            .chromatogram_type
            .as_ref()
            .expect("TIC chromatogram must carry a chromatogram_type CV term");
        assert_eq!(cv.accession, "MS:1000235");
        assert_eq!(cv.name, "total ion current chromatogram");

        // TIC carries no precursor/product m/z (those are for SRM/MRM).
        assert!(tic.precursor_mz.is_none());
        assert!(tic.product_mz.is_none());

        // time_sec is retention_time_min * 60.0; intensity is the Idx tic.
        assert_eq!(tic.time_sec, vec![0.0, 30.0]);
        assert_eq!(tic.intensity, vec![100.0, 250.0]);
        assert_eq!(tic.time_sec.len(), tic.intensity.len());
    }

    #[test]
    fn iter_chromatograms_tic_intensity_uses_idx_tic_not_spectrum_field() {
        // Regression guard for the reasoning in the method's doc comment: the
        // TIC chromatogram intensity comes straight from the Idx record's
        // physically-calibrated `tic` (cps), independent of the per-spectrum
        // `SpectrumRecord.total_ion_current` field (which stays `None` so the
        // conformance suite's `rel_close` check against sum(intensities) is
        // not tripped). A large cps value that could never equal a raw
        // intensity sum must still pass through verbatim.
        let mut reader = reader_with_idx(vec![idx_record(1.0, 1, 1_234_567.0)]);
        let chroms: Vec<ChromatogramRecord> = reader.iter_chromatograms().collect();
        assert_eq!(chroms.len(), 1);
        assert_eq!(chroms[0].time_sec, vec![60.0]);
        assert_eq!(chroms[0].intensity, vec![1_234_567.0]);
    }

    #[test]
    fn iter_spectra_leaves_scan_mode_unset() {
        // Regression guard for issue #27: scan_mode must not assert a
        // specific mode (it used to be hardcoded to `Profile` for every
        // spectrum, which mislabeled centroided SCIEX acquisitions) since no
        // decoded stream currently carries a reliable per-scan or
        // per-experiment centroid/profile indicator - see the doc comment on
        // `iter_spectra`'s `SpectrumRecord` construction.
        let mut reader = reader_with_idx(vec![idx_record(0.0, 1, 100.0), idx_record(0.1, 2, 50.0)]);
        let spectra: Vec<_> = reader.iter_spectra().collect();
        assert_eq!(spectra.len(), 2);
        for spectrum in &spectra {
            assert!(
                spectrum.scan_mode.is_none(),
                "scan_mode must be None, not an unverified guess"
            );
        }
    }

    fn point(raw_mz_bin: u32, raw_intensity: u32) -> ScanPoint {
        ScanPoint {
            raw_mz_bin,
            raw_intensity,
        }
    }

    #[test]
    fn points_to_arrays_drops_zero_intensity_points() {
        let points = vec![point(10, 0), point(20, 5), point(30, 0)];
        let (mz, intensity) = points_to_arrays(points, None);
        assert_eq!(mz, vec![20.0]);
        assert_eq!(intensity, vec![5.0]);
    }

    #[test]
    fn points_to_arrays_uses_raw_bin_without_calibration() {
        let points = vec![point(100, 1), point(200, 2)];
        let (mz, intensity) = points_to_arrays(points, None);
        assert_eq!(mz, vec![100.0, 200.0]);
        assert_eq!(intensity, vec![1.0, 2.0]);
    }

    #[test]
    fn points_to_arrays_applies_calibration_when_present() {
        let cal = Calibration {
            slope: 0.001,
            intercept: 0.5,
        };
        let points = vec![point(1000, 1)];
        let (mz, _intensity) = points_to_arrays(points, Some(cal));
        assert!((mz[0] - 1.5).abs() < 1e-12);
    }

    #[test]
    fn points_to_arrays_on_empty_input() {
        let (mz, intensity) = points_to_arrays(vec![], None);
        assert!(mz.is_empty());
        assert!(intensity.is_empty());
    }

    // --- Multi-sample support (Sigilweaver/OpenSXRaw#25) ---
    //
    // These build minimal synthetic CFBF files with the `cfb` crate's own
    // write API (a public, non-SCIEX-specific container format) rather than
    // relying on the out-of-tree corpus, so they run everywhere. No SCIEX
    // format knowledge beyond what's already used elsewhere in this module
    // (the `SampleSubtree/<Sample>/Idx` stream layout) is needed.

    use crate::raw::idx::{IDX_RECORD_SIZE, IDX_STREAM_HEADER};
    use byteorder::{ByteOrder, LittleEndian};
    use std::io::Write;

    /// One 54-byte Idx record with a real (non-placeholder) block, encoding
    /// `scan_offset` so tests can tell which sample's Idx stream was read.
    fn idx_record_bytes(scan_offset: u32) -> Vec<u8> {
        let mut buf = vec![0u8; IDX_RECORD_SIZE];
        LittleEndian::write_u32(&mut buf[0x00..0x04], scan_offset);
        LittleEndian::write_u32(&mut buf[0x04..0x08], 200); // scan_size, > 56
        buf[0x10] = 1; // ms_level flag: MS1
        buf
    }

    /// Builds a synthetic `.wiff` CFBF file at a uniquely-named path under
    /// the OS temp dir, with one `SampleSubtree/<name>/Idx` stream per
    /// entry in `samples`, and returns the `.wiff` path. Each sample's Idx
    /// stream holds one valid record whose `scan_offset` is `1000 * (1 +
    /// position in `samples`)`, so callers can verify which sample got
    /// loaded. Also writes an empty paired `.wiff.scan` file, since
    /// `Reader::open`/`open_sample` require it to exist (its contents don't
    /// matter here - only `idx_records`, populated at `open` time, is
    /// exercised by these tests, not lazy scan-block decoding).
    fn write_synthetic_wiff(name: &str, samples: &[&str]) -> PathBuf {
        let mut wiff_path = std::env::temp_dir();
        wiff_path.push(format!(
            "opensxraw_test_{}_{}.wiff",
            std::process::id(),
            name
        ));
        let mut scan_path = wiff_path.clone();
        scan_path.set_file_name(format!(
            "{}.scan",
            wiff_path.file_name().unwrap().to_string_lossy()
        ));

        let file = std::fs::File::create(&wiff_path).unwrap();
        let mut comp = cfb::CompoundFile::create(file).unwrap();
        comp.create_storage(SAMPLE_SUBTREE_STORAGE).unwrap();
        for (i, sample) in samples.iter().enumerate() {
            let storage = format!("{SAMPLE_SUBTREE_STORAGE}/{sample}");
            comp.create_storage(&storage).unwrap();

            let mut idx_data = vec![0u8; IDX_STREAM_HEADER];
            idx_data.extend(idx_record_bytes(1000 * (i as u32 + 1)));

            let mut stream = comp.create_stream(idx_stream(sample)).unwrap();
            stream.write_all(&idx_data).unwrap();
        }
        comp.flush().unwrap();
        drop(comp);

        std::fs::write(&scan_path, b"placeholder").unwrap();
        wiff_path
    }

    fn remove_synthetic_wiff(wiff_path: &Path) {
        std::fs::remove_file(wiff_path).ok();
        let mut scan_path = wiff_path.to_path_buf();
        scan_path.set_file_name(format!(
            "{}.scan",
            wiff_path.file_name().unwrap().to_string_lossy()
        ));
        std::fs::remove_file(&scan_path).ok();
    }

    #[test]
    fn sort_sample_names_orders_numerically_not_lexicographically() {
        let mut names = vec![
            "Sample10".to_string(),
            "Sample2".to_string(),
            "Sample1".to_string(),
        ];
        sort_sample_names(&mut names);
        assert_eq!(names, vec!["Sample1", "Sample2", "Sample10"]);
    }

    #[test]
    fn sort_sample_names_puts_non_matching_names_last_alphabetically() {
        let mut names = vec![
            "Zzz".to_string(),
            "Sample2".to_string(),
            "Aaa".to_string(),
            "Sample1".to_string(),
        ];
        sort_sample_names(&mut names);
        assert_eq!(names, vec!["Sample1", "Sample2", "Aaa", "Zzz"]);
    }

    #[test]
    fn list_samples_finds_single_sample() {
        let path = write_synthetic_wiff("list_single", &["Sample1"]);
        let samples = Reader::list_samples(&path).unwrap();
        remove_synthetic_wiff(&path);
        assert_eq!(samples, vec!["Sample1"]);
    }

    #[test]
    fn list_samples_finds_multiple_samples_in_numeric_order() {
        let path = write_synthetic_wiff("list_multi", &["Sample2", "Sample1", "Sample3"]);
        let samples = Reader::list_samples(&path).unwrap();
        remove_synthetic_wiff(&path);
        assert_eq!(samples, vec!["Sample1", "Sample2", "Sample3"]);
    }

    #[test]
    fn open_succeeds_on_single_sample_file() {
        let path = write_synthetic_wiff("open_single", &["Sample1"]);
        let reader = Reader::open(&path).unwrap();
        remove_synthetic_wiff(&path);
        assert_eq!(reader.idx_records.len(), 1);
        assert_eq!(reader.idx_records[0].scan_offset, 1000);
    }

    #[test]
    fn open_errors_clearly_on_multi_sample_file_instead_of_silently_truncating() {
        // Regression test for Sigilweaver/OpenSXRaw#25: previously the
        // hardcoded Sample1 path meant a multi-sample file would silently
        // read only its first sample, with no error or warning that
        // Sample2's data was skipped entirely. `open` must now refuse this
        // rather than pick one on the caller's behalf.
        let path = write_synthetic_wiff("open_multi", &["Sample1", "Sample2"]);
        let err = Reader::open(&path).err().unwrap();
        remove_synthetic_wiff(&path);
        let message = err.to_string();
        assert!(
            message.contains("Sample1") && message.contains("Sample2"),
            "error should name the samples found, got: {message}"
        );
    }

    #[test]
    fn open_sample_reads_the_requested_sample_not_just_the_first() {
        let path = write_synthetic_wiff("open_sample_second", &["Sample1", "Sample2"]);
        let reader = Reader::open_sample(&path, "Sample2").unwrap();
        remove_synthetic_wiff(&path);
        assert_eq!(reader.idx_records.len(), 1);
        assert_eq!(reader.idx_records[0].scan_offset, 2000);
    }

    #[test]
    fn open_errors_when_sample_subtree_storage_is_missing() {
        let mut wiff_path = std::env::temp_dir();
        wiff_path.push(format!(
            "opensxraw_test_{}_no_sample_subtree.wiff",
            std::process::id()
        ));
        let file = std::fs::File::create(&wiff_path).unwrap();
        cfb::CompoundFile::create(file).unwrap().flush().unwrap();
        std::fs::write(format!("{}.scan", wiff_path.display()), b"x").unwrap();

        let err = Reader::open(&wiff_path).err().unwrap();
        remove_synthetic_wiff(&wiff_path);
        assert!(err.to_string().contains(SAMPLE_SUBTREE_STORAGE));
    }
}
