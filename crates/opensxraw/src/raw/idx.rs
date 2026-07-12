//! Parsing of the `SampleSubtree/Sample1/Idx` CFBF stream.
//!
//! The Idx stream maps scan indices to byte ranges in the paired `.wiff.scan` file.
//!
//! # Stream layout (confirmed against corpus)
//!
//! - 32-byte stream header (opaque, skipped)
//! - Followed by contiguous 54-byte index records
//!
//! # Record layout (54 bytes, little-endian)
//!
//! | Offset | Type    | Description                                 |
//! |--------|---------|---------------------------------------------|
//! | 0x00   | u32     | Byte offset of block in `.wiff.scan`        |
//! | 0x04   | u32     | Byte size of block in `.wiff.scan`          |
//! | 0x08   | u32     | Unknown (0 except first record)             |
//! | 0x0C   | f32     | Retention time (minutes)                    |
//! | 0x10   | u8      | MS level flag (1 = MS1, 0 = MS2)           |
//! | 0x11   | u8      | Unknown                                     |
//! | 0x12   | f64     | Total Ion Current (cps, not raw counts)     |
//! | 0x1A   | f64     | Secondary field (grid-spacing related)      |
//! | 0x22   | [u8;20] | Zero padding                                |

use byteorder::{ByteOrder, LittleEndian};

pub const IDX_STREAM_HEADER: usize = 32;
pub const IDX_RECORD_SIZE: usize = 54;

/// One decoded record from the Idx stream.
#[derive(Debug, Clone)]
pub struct IdxRecord {
    /// Byte offset of the scan block in the `.wiff.scan` file.
    pub scan_offset: u32,
    /// Byte size of the scan block in the `.wiff.scan` file.
    pub scan_size: u32,
    /// Retention time in minutes.
    pub retention_time_min: f32,
    /// MS level: 1 for MS1, 2 for MS2 (derived from flag byte: 1 -> 1, 0 -> 2).
    pub ms_level: u32,
    /// Total Ion Current from the Idx record (in cps, not directly comparable
    /// to sum of raw intensity tokens).
    pub tic: f64,
    /// Secondary float64 field at 0x1A. Physical meaning is not fully resolved;
    /// it is related to the scan's time-bin grid spacing but is not a simple
    /// 1:1 mapping. Not exposed in SpectrumRecord.
    pub _field_1a: f64,
}

impl IdxRecord {
    /// Parse one 54-byte Idx record from a byte slice.
    ///
    /// Returns `None` if the record is a placeholder (scan_offset == 0 and
    /// scan_size == 0 - these exist in the stream but point to no real data).
    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        debug_assert_eq!(buf.len(), IDX_RECORD_SIZE);
        let scan_offset = LittleEndian::read_u32(&buf[0x00..0x04]);
        let scan_size = LittleEndian::read_u32(&buf[0x04..0x08]);

        // Placeholder records have scan_size == 0 or scan_size too small
        // for a real block (must be > 56 to contain at least the block header).
        if scan_size <= 56 {
            return None;
        }

        let retention_time_min = LittleEndian::read_f32(&buf[0x0C..0x10]);
        let ms_level_flag = buf[0x10];
        let ms_level = if ms_level_flag == 1 { 1 } else { 2 };
        let tic = LittleEndian::read_f64(&buf[0x12..0x1A]);
        let field_1a = LittleEndian::read_f64(&buf[0x1A..0x22]);

        Some(IdxRecord {
            scan_offset,
            scan_size,
            retention_time_min,
            ms_level,
            tic,
            _field_1a: field_1a,
        })
    }

    /// Parse the entire Idx stream bytes into a list of valid scan records.
    pub fn parse_stream(data: &[u8]) -> crate::Result<Vec<IdxRecord>> {
        if data.len() < IDX_STREAM_HEADER {
            return Err(crate::Error::Parse(
                "Idx stream too short for header".into(),
            ));
        }

        let payload = &data[IDX_STREAM_HEADER..];
        let n_records = payload.len() / IDX_RECORD_SIZE;
        let mut records = Vec::with_capacity(n_records / 4);

        for i in 0..n_records {
            let start = i * IDX_RECORD_SIZE;
            let end = start + IDX_RECORD_SIZE;
            if end > payload.len() {
                break;
            }
            if let Some(rec) = IdxRecord::from_bytes(&payload[start..end]) {
                records.push(rec);
            }
        }

        if records.is_empty() {
            return Err(crate::Error::Parse(
                "Idx stream contained no valid scan records".into(),
            ));
        }

        Ok(records)
    }
}
