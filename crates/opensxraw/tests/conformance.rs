//! Conformance test against a real corpus fixture.
//!
//! Fixture: PXD022088/Rcor2KOESC1 - a TripleTOF 5600 DDA run (596 KB .wiff,
//! 1.9 MB .wiff.scan). The smallest complete legacy pair in the corpus.

use openmassspec_core::conformance::assert_source_invariants;
use openmassspec_core::SpectrumSource;
use opensxraw::reader::Reader;
use std::path::PathBuf;

fn fixture_wiff() -> PathBuf {
    PathBuf::from("/workspaces/Projects/Data/SRaw/PXD022088/Rcor2KOESC1.wiff")
}

/// Open the corpus fixture, or return `None` (with a skip message) when it
/// is absent - the corpus lives out of tree, so these tests skip cleanly on
/// CI runners instead of failing the build.
fn open_fixture_or_skip() -> Option<Reader> {
    let path = fixture_wiff();
    if !path.exists() {
        eprintln!("skip: corpus not present at {}", path.display());
        return None;
    }
    Some(Reader::open(&path).expect("Reader::open failed"))
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
