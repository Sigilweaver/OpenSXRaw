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

    for s in &ms2[..ms2.len().min(5)] {
        assert!(
            s.precursor.is_some(),
            "MS2 spectrum {} has no precursor",
            s.native_id
        );
    }

    println!("First MS2: native_id={}", ms2[0].native_id);
}
