//! Parsing of the `SampleSubtree/Sample1/DDERealTimeDataEx` CFBF stream.
//!
//! Present on files with data-dependent (IDA/DDA) precursor selection.
//! Absent on files that don't do DDA-style triggering (e.g. plain SWATH,
//! plain MRM). See `docs/format/04-legacy-wiff-calibration.md` for the full
//! investigation.
//!
//! # Stream layout (confirmed against corpus)
//!
//! - 32-byte stream header (opaque, skipped, same convention as `Idx`).
//! - Body is a flat array of fixed 76-byte records (confirmed: body size
//!   divides evenly by 76 with no remainder).
//!
//! # Record layout (76 bytes, little-endian)
//!
//! | Offset | Type  | Description                                          |
//! |--------|-------|-------------------------------------------------------|
//! | 0x00   | u32   | 1-based sequential record ordinal (matches position)  |
//! | 0x04   | f64   | Precursor m/z, already physically calibrated          |
//! | 0x08.. | ...   | Remaining 64 bytes: no field confidently identified   |
//!
//! No charge state or isolation width field was confidently identified.
//!
//! # MS2 linkage (heuristic, not fully validated)
//!
//! This stream's record count matches the file's MS1/survey scan count, not
//! its MS2 count - consistent with one entry per DDA cycle (the precursor
//! selected by that cycle's survey step), not one entry per MS2 scan.
//! Callers should track "how many MS1 scans have been seen so far" while
//! iterating `Idx` records in order, and use the DDE record at
//! `(that count) - 1` for each MS2 scan encountered. This is physically
//! motivated but not independently confirmed against ground truth.

use byteorder::{ByteOrder, LittleEndian};

pub const DDE_STREAM_HEADER: usize = 32;
pub const DDE_RECORD_SIZE: usize = 76;

/// One decoded record from the `DDERealTimeDataEx` stream.
#[derive(Debug, Clone, Copy)]
pub struct DdeRecord {
    /// Precursor m/z, already physically calibrated.
    pub precursor_mz: f64,
}

impl DdeRecord {
    fn from_bytes(buf: &[u8]) -> Self {
        debug_assert_eq!(buf.len(), DDE_RECORD_SIZE);
        let precursor_mz = LittleEndian::read_f64(&buf[0x04..0x0c]);
        DdeRecord { precursor_mz }
    }

    /// Parse the entire `DDERealTimeDataEx` stream bytes into a list of
    /// records, in stream order.
    pub fn parse_stream(data: &[u8]) -> Vec<DdeRecord> {
        let body = match data.get(DDE_STREAM_HEADER..) {
            Some(b) => b,
            None => return Vec::new(),
        };
        let n_records = body.len() / DDE_RECORD_SIZE;
        let mut records = Vec::with_capacity(n_records);
        for i in 0..n_records {
            let start = i * DDE_RECORD_SIZE;
            let end = start + DDE_RECORD_SIZE;
            records.push(DdeRecord::from_bytes(&body[start..end]));
        }
        records
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_body() {
        assert_eq!(DdeRecord::parse_stream(&[0u8; 32]).len(), 0);
    }

    #[test]
    fn too_short_returns_empty() {
        assert_eq!(DdeRecord::parse_stream(&[0u8; 10]).len(), 0);
    }

    #[test]
    fn parses_one_record() {
        let mut data = vec![0u8; 32];
        let mut rec = vec![0u8; DDE_RECORD_SIZE];
        LittleEndian::write_u32(&mut rec[0x00..0x04], 1);
        LittleEndian::write_f64(&mut rec[0x04..0x0c], 493.2749);
        data.extend_from_slice(&rec);

        let records = DdeRecord::parse_stream(&data);
        assert_eq!(records.len(), 1);
        assert!((records[0].precursor_mz - 493.2749).abs() < 1e-6);
    }

    #[test]
    fn parses_multiple_records_in_order() {
        let mut data = vec![0u8; 32];
        for (i, mz) in [493.2749, 547.2980, 486.7264].iter().enumerate() {
            let mut rec = vec![0u8; DDE_RECORD_SIZE];
            LittleEndian::write_u32(&mut rec[0x00..0x04], (i + 1) as u32);
            LittleEndian::write_f64(&mut rec[0x04..0x0c], *mz);
            data.extend_from_slice(&rec);
        }

        let records = DdeRecord::parse_stream(&data);
        assert_eq!(records.len(), 3);
        assert!((records[1].precursor_mz - 547.2980).abs() < 1e-6);
    }
}
