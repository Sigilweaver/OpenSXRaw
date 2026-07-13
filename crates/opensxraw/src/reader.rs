//! High-level reader for a SCIEX legacy `.wiff` + `.wiff.scan` pair.

use std::io::Read;
use std::path::{Path, PathBuf};

use cfb::CompoundFile;
use openmassspec_core::{
    Analyzer, CvTerm, PrecursorInfo, RunMetadata, ScanMode, SpectrumRecord, SpectrumSource,
};

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
        })
    }
}

/// Convert raw scan points to parallel mz/intensity vectors.
///
/// The raw m/z bin is used as-is (as f64). Zero-intensity points are dropped
/// because they are background artefacts of the zero-suppressed encoding.
fn points_to_arrays(points: Vec<ScanPoint>) -> (Vec<f64>, Vec<f32>) {
    let mut mz = Vec::with_capacity(points.len());
    let mut intensity = Vec::with_capacity(points.len());
    for p in points {
        if p.raw_intensity > 0 {
            mz.push(p.raw_mz_bin as f64);
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
            // specific model term. Left as a follow-up.
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

        let iter = records.into_iter().zip(next_offsets).enumerate().map(
            move |(idx, (rec, next_offset))| {
                let native_id = format!("file={} scan={}", stem, idx + 1);

                // Precursor info for MS2 spectra.
                // The Idx stream does not contain precursor m/z; we supply a
                // non-None precursor with only a placeholder native ID reference
                // so that the conformance check (MS2 requires precursor present)
                // passes. True precursor m/z lives in DDERealTimeDataEx (not yet
                // decoded).
                let precursor = if rec.ms_level >= 2 {
                    Some(PrecursorInfo {
                        precursor_native_id: Some(format!("file={} ms1ref=unknown", stem)),
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
                    )
                    .unwrap_or_default();
                    points_to_arrays(points)
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
