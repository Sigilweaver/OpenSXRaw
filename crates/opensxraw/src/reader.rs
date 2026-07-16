//! High-level reader for a SCIEX legacy `.wiff` + `.wiff.scan` pair.

use std::io::Read;
use std::path::{Path, PathBuf};

use cfb::CompoundFile;
use openmassspec_core::{
    Analyzer, CvTerm, PrecursorInfo, RunMetadata, ScanMode, SpectrumRecord, SpectrumSource,
};

use crate::raw::calibration::Calibration;
use crate::raw::dde::DdeRecord;
use crate::raw::idx::IdxRecord;
use crate::raw::scan::{read_scan_block, ScanPoint};
use crate::raw::summary_info::parse_create_timestamp;

/// The CFBF stream path for the scan index in a single-sample file.
const IDX_STREAM: &str = "SampleSubtree/Sample1/Idx";

/// The CFBF stream path for the standard OLE SummaryInformation property
/// set. The leading `\x05` is the OLE convention marking a stream name as
/// reserved/special rather than user data. See `raw::summary_info` for the
/// investigation behind using this as the acquisition start timestamp.
const SUMMARY_INFO_STREAM: &str = "\x05SummaryInformation";

/// The CFBF stream path for TOF m/z calibration constants. Only present on
/// TripleTOF-family acquisitions - see `raw::calibration` and
/// `docs/format/04-legacy-wiff-calibration.md`.
const CALIBRATION_STREAM: &str = "SampleSubtree/Sample1/TOFCalibrationData";

/// The CFBF stream path for data-dependent precursor selection records.
/// Only present on files with IDA/DDA-style precursor triggering - see
/// `raw::dde` and `docs/format/04-legacy-wiff-calibration.md`.
const DDE_STREAM: &str = "SampleSubtree/Sample1/DDERealTimeDataEx";

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
    /// Open a `.wiff` file and its paired `.wiff.scan` file.
    ///
    /// `wiff_path` is the path to the `.wiff` file. The `.wiff.scan` file is
    /// expected at the same path with `.scan` appended.
    pub fn open<P: AsRef<Path>>(wiff_path: P) -> crate::Result<Self> {
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
        let idx_data = {
            let mut stream = comp.open_stream(IDX_STREAM).map_err(|e| {
                crate::Error::Parse(format!("Stream '{}' not found: {}", IDX_STREAM, e))
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
        let calibration = comp
            .open_stream(CALIBRATION_STREAM)
            .ok()
            .and_then(|mut stream| {
                let mut buf = Vec::new();
                stream.read_to_end(&mut buf).ok()?;
                Calibration::from_bytes(&buf)
            });

        // Read DDA precursor-selection records, if present. Absent on files
        // without IDA/DDA-style precursor triggering - see `raw::dde`.
        let dde_records = comp
            .open_stream(DDE_STREAM)
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
            software_name: "opensxraw".to_string(),
            software_version: env!("CARGO_PKG_VERSION").to_string(),
            start_timestamp: self.start_timestamp.clone(),
            mobility_array_kind: None,
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
                        polarity: None,
                        scan_mode: Some(ScanMode::Profile),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::calibration::Calibration;

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
}
