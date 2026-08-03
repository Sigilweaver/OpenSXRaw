//! Conformance test against a real corpus fixture.
//!
//! Fixture: PXD022088/Rcor2KOESC1 - a QTRAP 6500+ DDA run (596 KB .wiff,
//! 1.9 MB .wiff.scan). The smallest complete legacy pair in the corpus.
//! Previously mislabeled "TripleTOF 5600" here - corrected per the
//! `Log` stream self-identification record found for issue #4 (see
//! `raw::instrument_log`), which agrees with this file's lack of a
//! `TOFCalibrationData` stream and `ExperimentTOF` method stream (issue #8's
//! TOF-vs-quad/trap analyzer family signal - see `raw::calibration`) and
//! with this file's free-text "Monash_6500" instrument-name hint.
//! `test_calibrated_mz_is_physically_plausible` below uses a different,
//! TripleTOF fixture for the calibrated-m/z path.

use openmassspec_core::conformance::assert_source_invariants;
use openmassspec_core::{Analyzer, Polarity, SpectrumSource};
use opensxraw::reader::Reader;
use std::path::PathBuf;

fn fixture_wiff_candidates() -> Vec<PathBuf> {
    vec![
        // CI / repo-root corpus dir (gitignored; populated by ci.yml's
        // "Download corpus fixture for conformance tests" step). This is
        // the one CI actually uses. The `.wiff.scan` sibling is downloaded
        // alongside it into the same directory.
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/PXD022088/Rcor2KOESC1.wiff"),
        // Local dev out-of-tree corpus checkout (see CORPUS.md).
        PathBuf::from("/workspaces/Projects/Data/SRaw/PXD022088/Rcor2KOESC1.wiff"),
    ]
}

/// Open the corpus fixture, or return `None` (with a skip message) when it
/// is absent from every candidate location - the corpus mostly lives out of
/// tree, so these tests skip cleanly rather than failing the build when
/// neither the CI-downloaded nor the local-dev copy is present.
fn open_fixture_or_skip() -> Option<Reader> {
    let candidates = fixture_wiff_candidates();
    let Some(path) = candidates.iter().find(|p| p.exists()) else {
        eprintln!(
            "skip: corpus not present at any of: {}",
            candidates
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        return None;
    };
    Some(Reader::open(path).expect("Reader::open failed"))
}

#[test]
fn test_start_timestamp_from_summary_info() {
    let Some(reader) = open_fixture_or_skip() else {
        return;
    };
    let metadata = reader.run_metadata();
    // The `.wiff` container's SummaryInformation PIDSI_CREATE_DTM property,
    // cross-checked against the human-readable "Checksum Time" string in
    // CFR/CFRFileHeader ("Tuesday, June 25, 2019 14:31:24", Melbourne
    // AEST = UTC+10) - the two agree to the second.
    assert_eq!(
        metadata.start_timestamp.as_deref(),
        Some("2019-06-25T04:31:23.912Z")
    );
}

#[test]
fn test_instrument_model_from_log_stream() {
    let Some(reader) = open_fixture_or_skip() else {
        return;
    };
    let metadata = reader.run_metadata();
    // The `.wiff` container's `Log` stream self-identification record
    // (`raw::instrument_log`) resolves to a specific psi-ms.obo term
    // instead of the generic "SCIEX instrument model" placeholder.
    assert_eq!(metadata.instrument.accession, "MS:1002582");
    assert_eq!(metadata.instrument.name, "QTRAP 6500+");
    assert_eq!(
        metadata.instrument_serial_number.as_deref(),
        Some("CG22641612")
    );
}

#[test]
fn test_opens_and_reads_idx() {
    let Some(reader) = open_fixture_or_skip() else {
        return;
    };
    assert!(
        !reader.idx_records.is_empty(),
        "expected at least one valid Idx record"
    );
    println!("Idx records: {}", reader.idx_records.len());

    // Verify both MS1 and MS2 scans are present.
    let has_ms1 = reader.idx_records.iter().any(|r| r.ms_level == 1);
    let has_ms2 = reader.idx_records.iter().any(|r| r.ms_level == 2);
    assert!(has_ms1, "no MS1 scans found in Idx");
    assert!(has_ms2, "no MS2 scans found in Idx");
    println!(
        "MS1: {}, MS2: {}",
        reader
            .idx_records
            .iter()
            .filter(|r| r.ms_level == 1)
            .count(),
        reader
            .idx_records
            .iter()
            .filter(|r| r.ms_level == 2)
            .count()
    );
}

#[test]
fn test_conformance_invariants() {
    let Some(mut reader) = open_fixture_or_skip() else {
        return;
    };
    let n = assert_source_invariants(&mut reader).expect("conformance invariants failed");
    assert!(n > 0, "expected at least one spectrum");
    println!("Conformance passed: {} spectra", n);
}

#[test]
fn test_iter_chromatograms_emits_tic() {
    let Some(mut reader) = open_fixture_or_skip() else {
        return;
    };
    let n_records = reader.idx_records.len();
    assert!(n_records > 0, "fixture should have Idx records");

    let chroms: Vec<_> = reader.iter_chromatograms().collect();

    // Exactly one TIC chromatogram: BPC/SRM need net-new decode work and are
    // intentionally out of scope (see Sigilweaver/OpenSXRaw#21).
    assert_eq!(chroms.len(), 1, "expected exactly one (TIC) chromatogram");
    let tic = &chroms[0];
    let cv = tic
        .chromatogram_type
        .as_ref()
        .expect("TIC chromatogram must carry a chromatogram_type CV term");
    assert_eq!(cv.accession, "MS:1000235");
    assert_eq!(cv.name, "total ion current chromatogram");
    assert!(tic.precursor_mz.is_none());
    assert!(tic.product_mz.is_none());

    // One point per Idx record, with parallel time/intensity arrays.
    assert_eq!(tic.time_sec.len(), n_records);
    assert_eq!(tic.intensity.len(), n_records);
    println!("TIC chromatogram: {} points", tic.time_sec.len());
}

#[test]
fn test_ms1_has_peaks() {
    let Some(mut reader) = open_fixture_or_skip() else {
        return;
    };
    let spectra: Vec<_> = reader.iter_spectra().collect();

    let ms1_with_peaks: Vec<_> = spectra
        .iter()
        .filter(|s| s.ms_level == 1 && !s.mz.is_empty())
        .collect();

    assert!(
        !ms1_with_peaks.is_empty(),
        "expected at least one MS1 spectrum with decoded peaks"
    );

    let first = ms1_with_peaks[0];
    println!(
        "First MS1 with peaks: scan={} rt={:.2}s peaks={}",
        first.scan_number,
        first.retention_time_sec,
        first.mz.len()
    );
    assert_eq!(
        first.mz.len(),
        first.intensity.len(),
        "mz/intensity length mismatch"
    );
}

#[test]
fn test_ms2_has_precursor() {
    let Some(mut reader) = open_fixture_or_skip() else {
        return;
    };
    let spectra: Vec<_> = reader.iter_spectra().collect();

    let ms2: Vec<_> = spectra.iter().filter(|s| s.ms_level == 2).collect();
    assert!(!ms2.is_empty(), "expected at least one MS2 spectrum");

    let mut with_selected_mz = 0;
    for s in &ms2 {
        let precursor = s
            .precursor
            .as_ref()
            .unwrap_or_else(|| panic!("MS2 spectrum {} has no precursor", s.native_id));
        assert!(
            precursor.precursor_native_id.is_some(),
            "MS2 spectrum {} has no precursor_native_id",
            s.native_id
        );
        if precursor.selected_mz.is_some() {
            with_selected_mz += 1;
        }
    }

    // DDERealTimeDataEx's cycle-based linkage (see `raw::dde`) resolves for
    // every MS2 scan except ones before the file's first MS1 survey scan -
    // a rare edge case, not the common case. Require the large majority to
    // have a real precursor m/z rather than the "ms1ref=unknown" fallback.
    let fraction_with_mz = with_selected_mz as f64 / ms2.len() as f64;
    assert!(
        fraction_with_mz > 0.9,
        "expected >90% of MS2 spectra to have precursor selected_mz, got {:.1}% ({}/{})",
        fraction_with_mz * 100.0,
        with_selected_mz,
        ms2.len()
    );

    println!(
        "First MS2: native_id={}, {}/{} MS2 spectra have precursor selected_mz",
        ms2[0].native_id,
        with_selected_mz,
        ms2.len()
    );
}

/// Fixture: PXD056391/TO14810HD - a small TripleTOF file with a
/// `TOFCalibrationData` stream, used to validate the calibrated m/z path
/// (the main `Rcor2KOESC1` fixture above is QTRAP-only and has no
/// calibration stream - see `docs/format/04-legacy-wiff-calibration.md`).
fn calibrated_fixture_wiff() -> PathBuf {
    PathBuf::from("/workspaces/Projects/Data/SRaw/PXD056391/TO14810HD.wiff")
}

#[test]
fn test_qtrap_fixture_gets_tqms_and_positive_polarity() {
    // Issues #8/#26: the main fixture is QTRAP 6500 (no TOFCalibrationData,
    // no ExperimentTOF stream - see the module doc on `raw::calibration`),
    // so every spectrum should get TQMS, and its ISVF/IS ion spray voltage
    // is positive (verified across the full local corpus - see
    // `raw::ion_source`), so polarity should resolve to Positive.
    let Some(mut reader) = open_fixture_or_skip() else {
        return;
    };
    let spectra: Vec<_> = reader.iter_spectra().collect();
    assert!(!spectra.is_empty());
    for s in &spectra {
        assert_eq!(s.analyzer, Some(Analyzer::TQMS));
        assert_eq!(s.polarity, Some(Polarity::Positive));
    }
}

#[test]
fn test_calibrated_mz_is_physically_plausible() {
    let path = calibrated_fixture_wiff();
    if !path.exists() {
        eprintln!("skip: corpus not present at {}", path.display());
        return;
    }
    let mut reader = Reader::open(&path).expect("Reader::open failed");
    let spectra: Vec<_> = reader.iter_spectra().collect();

    let with_peaks: Vec<_> = spectra.iter().filter(|s| !s.mz.is_empty()).collect();
    assert!(
        !with_peaks.is_empty(),
        "expected at least one spectrum with peaks"
    );

    // Raw (uncalibrated) time-bin values on this file run into the hundreds
    // of thousands; a real calibrated m/z spectrum for these runs stays
    // under ~2000 Da. If calibration silently stopped applying, this would
    // catch the regression back to raw bins.
    for s in &with_peaks {
        let max_mz = s.mz.iter().cloned().fold(f64::MIN, f64::max);
        assert!(
            max_mz < 5000.0,
            "spectrum {} has max mz {max_mz}, expected a calibrated value under 5000 Da",
            s.native_id
        );
    }
}

#[test]
fn test_tripletof_fixture_gets_tofms_and_positive_polarity() {
    // Issues #8/#26: this fixture has a real TOFCalibrationData stream
    // (TripleTOF-family), so every spectrum should get TOFMS, and its ISVF
    // ion spray voltage is positive, so polarity should resolve to Positive
    // - same corpus-wide invariant as the QTRAP fixture above.
    let path = calibrated_fixture_wiff();
    if !path.exists() {
        eprintln!("skip: corpus not present at {}", path.display());
        return;
    }
    let mut reader = Reader::open(&path).expect("Reader::open failed");
    let spectra: Vec<_> = reader.iter_spectra().collect();
    assert!(!spectra.is_empty());
    for s in &spectra {
        assert_eq!(s.analyzer, Some(Analyzer::TOFMS));
        assert_eq!(s.polarity, Some(Polarity::Positive));
    }
}
