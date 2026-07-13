//! Parsing of the `SampleSubtree/Sample1/TOFCalibrationData` CFBF stream.
//!
//! Only present on TripleTOF-family acquisitions (files that also carry a
//! `MethodSubtree/.../ExperimentTOF` stream). Absent entirely on QTRAP-only
//! files, which have no clean-room-derivable calibration source - see
//! `docs/format/04-legacy-wiff-calibration.md` for the full investigation.
//!
//! # Stream layout (confirmed against corpus)
//!
//! - 32-byte stream header (opaque, skipped, same convention as `Idx`).
//! - Body is a table of `(f64 slope, f64 intercept)` pairs; the first pair
//!   starts at body offset 0.
//! - A `u32` count field at body offset 0x14 was confirmed across multiple
//!   corpus files to exactly equal that file's total `Idx` record count,
//!   tying this table to the scan index. The table's full record framing
//!   beyond the first pair was not resolved (irregular byte gaps between
//!   repeats of the same slope/intercept pattern).
//!
//! Scanning many corpus files shows the slope is effectively constant per
//! file (varies only in the 9th-10th significant digit within a file) while
//! the intercept drifts slightly over a run (observed drift well under
//! 0.1 Da) - consistent with a live lock-mass recalibration feed on a fixed
//! digitizer time-bin width. This parser deliberately reads only the first
//! `(slope, intercept)` pair as a per-file constant rather than resolving
//! the full live-recalibration table; see the format doc for why that's an
//! acceptable approximation given the observed drift magnitude.
//!
//! # Formula
//!
//! `m/z = slope * raw_mz_bin + intercept`
//!
//! This is linear, not the quadratic `time ~ sqrt(m/z)` form expected from
//! first-principles TOF physics - the working theory is that the vendor
//! firmware already linearizes digitized time bins onto an m/z-like grid
//! before these constants apply. Validated only for physical plausibility
//! (mass range, resolution scale) against real corpus scans, not against
//! vendor software (clean-room rule) or isotope-level ground truth.

use byteorder::{ByteOrder, LittleEndian};

pub const CALIBRATION_STREAM_HEADER: usize = 32;

/// Per-file linear calibration constants read from `TOFCalibrationData`.
#[derive(Debug, Clone, Copy)]
pub struct Calibration {
    pub slope: f64,
    pub intercept: f64,
}

impl Calibration {
    /// Parse the first `(slope, intercept)` pair from a `TOFCalibrationData`
    /// stream. Returns `None` if the stream is too short to contain one.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let body = data.get(CALIBRATION_STREAM_HEADER..)?;
        if body.len() < 16 {
            return None;
        }
        let slope = LittleEndian::read_f64(&body[0..8]);
        let intercept = LittleEndian::read_f64(&body[8..16]);
        Some(Calibration { slope, intercept })
    }

    /// Convert a raw accumulated time-bin index to physical m/z.
    pub fn apply(&self, raw_mz_bin: u32) -> f64 {
        self.slope * raw_mz_bin as f64 + self.intercept
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_first_pair() {
        let mut data = vec![0u8; 32];
        let mut body = vec![0u8; 16];
        LittleEndian::write_f64(&mut body[0..8], 0.0007027934);
        LittleEndian::write_f64(&mut body[8..16], 0.3636924);
        data.extend_from_slice(&body);

        let cal = Calibration::from_bytes(&data).unwrap();
        assert!((cal.slope - 0.0007027934).abs() < 1e-12);
        assert!((cal.intercept - 0.3636924).abs() < 1e-9);
    }

    #[test]
    fn too_short_returns_none() {
        assert!(Calibration::from_bytes(&[0u8; 40]).is_none());
    }

    #[test]
    fn apply_formula() {
        let cal = Calibration {
            slope: 0.001,
            intercept: 0.5,
        };
        assert!((cal.apply(1000) - 1.5).abs() < 1e-12);
    }
}
