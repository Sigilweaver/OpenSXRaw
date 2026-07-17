//! Decoding of `.wiff.scan` spectrum blocks.
//!
//! # Block structure (confirmed against corpus)
//!
//! Each block is located by an `IdxRecord`. The block is a contiguous region
//! in the `.wiff.scan` file. Its structure is:
//!
//! - Bytes [0, scan_offset+56): Not part of this block's payload. The 56
//!   bytes starting at `scan_offset` are a window that contains the TAIL of
//!   the *previous* block's token stream, followed by a `ff ff ff ff`
//!   terminator that marks the end of the previous block, followed by a small
//!   sync region.
//! - Bytes [scan_offset+56, next_scan_offset): The variable-length token-stream
//!   payload for this block. The payload ends at the `ff ff ff ff` terminator
//!   found inside the *next* block's 56-byte window.
//!
//! In practice, reading a block means:
//!   1. Seek to `scan_offset + 56` for the payload start.
//!   2. Scan forward for `ff ff ff ff` (which will fall somewhere inside the
//!      next block's window region) to find where to stop.
//!   3. Decode the token stream in between.
//!
//! # Token encoding
//!
//! The payload is a self-synchronizing zero-suppressed command stream. Each
//! byte determines the command:
//!
//! - `b < 0x80` (0..127): **GAP** - adds `b` to the running m/z accumulator.
//!   No point emitted.
//! - `0x80..=0xfb` (128..251): **1-byte intensity** - emits a point at the
//!   current m/z accumulator with intensity `b & 0x7f` (0..123).
//! - `0xfc`: **2-byte intensity** - reads 1 following byte, emits point with
//!   that byte as intensity.
//! - `0xfd`: **3-byte intensity** - reads 2 following bytes (little-endian u16),
//!   emits point.
//! - `0xfe`: **4-byte intensity** - reads 3 following bytes (little-endian u24),
//!   emits point.
//! - `0xff`: **5-byte intensity** - reads 4 following bytes (little-endian u32),
//!   emits point.
//!
//! The m/z accumulator starts at 0. The raw accumulated value is an integer
//! time-bin index; conversion to physical m/z (Da) requires calibration
//! constants from the method streams, which are not yet decoded.
//!
//! # Terminator
//!
//! A run of four consecutive `0xff` bytes (`ff ff ff ff`) terminates the
//! payload. Because `0xff` as a command prefix means "read 4 following bytes",
//! four `0xff` bytes in a row would require reading 4 more `0xff` bytes, which
//! then recurse - this pattern cannot arise from valid data and is used as a
//! reliable sentinel.

use byteorder::{ByteOrder, LittleEndian};

/// A single decoded spectrum point: (raw_mz_bin, raw_intensity).
///
/// `raw_mz_bin` is the accumulated integer time-bin index. `raw_intensity`
/// is the raw ADC/TDC count value.
#[derive(Debug, Clone, Copy)]
pub struct ScanPoint {
    pub raw_mz_bin: u32,
    pub raw_intensity: u32,
}

/// Decode the token-stream payload for one scan block.
///
/// `payload` is the raw bytes starting at `scan_offset + 56` up to (but not
/// including) the `ff ff ff ff` terminator. The terminator should have been
/// found before calling this function; if not, decoding stops at end-of-slice.
pub fn decode_payload(payload: &[u8]) -> Vec<ScanPoint> {
    let mut points = Vec::new();
    let mut mz_bin: u32 = 0;
    let mut i = 0usize;

    while i < payload.len() {
        let b = payload[i];

        // Terminator check: four consecutive 0xff marks end of payload.
        // In practice callers strip this before passing payload, but guard
        // here defensively.
        if b == 0xff
            && i + 3 < payload.len()
            && payload[i + 1] == 0xff
            && payload[i + 2] == 0xff
            && payload[i + 3] == 0xff
        {
            break;
        }

        match b {
            // GAP: advance m/z accumulator
            0x00..=0x7f => {
                mz_bin = mz_bin.wrapping_add(b as u32);
                i += 1;
            }
            // 1-byte intensity (low 7 bits)
            0x80..=0xfb => {
                let intensity = (b & 0x7f) as u32;
                points.push(ScanPoint {
                    raw_mz_bin: mz_bin,
                    raw_intensity: intensity,
                });
                i += 1;
            }
            // 0xfc: 1 following byte as intensity
            0xfc => {
                if i + 1 >= payload.len() {
                    break;
                }
                let intensity = payload[i + 1] as u32;
                points.push(ScanPoint {
                    raw_mz_bin: mz_bin,
                    raw_intensity: intensity,
                });
                i += 2;
            }
            // 0xfd: 2 following bytes, little-endian u16
            0xfd => {
                if i + 2 >= payload.len() {
                    break;
                }
                let intensity = LittleEndian::read_u16(&payload[i + 1..i + 3]) as u32;
                points.push(ScanPoint {
                    raw_mz_bin: mz_bin,
                    raw_intensity: intensity,
                });
                i += 3;
            }
            // 0xfe: 3 following bytes, little-endian u24
            0xfe => {
                if i + 3 >= payload.len() {
                    break;
                }
                let b0 = payload[i + 1] as u32;
                let b1 = payload[i + 2] as u32;
                let b2 = payload[i + 3] as u32;
                let intensity = b0 | (b1 << 8) | (b2 << 16);
                points.push(ScanPoint {
                    raw_mz_bin: mz_bin,
                    raw_intensity: intensity,
                });
                i += 4;
            }
            // 0xff: 4 following bytes, little-endian u32. Should not appear
            // singly (it would hit the terminator check above for four in a
            // row), but handle a lone 0xff gracefully.
            0xff => {
                if i + 4 >= payload.len() {
                    break;
                }
                let intensity = LittleEndian::read_u32(&payload[i + 1..i + 5]);
                points.push(ScanPoint {
                    raw_mz_bin: mz_bin,
                    raw_intensity: intensity,
                });
                i += 5;
            }
        }
    }

    points
}

/// Absolute ceiling on a single scan block read, independent of the Idx or
/// the file size. No known TripleTOF/QTRAP block comes remotely close to
/// this; it exists purely to bound worst-case allocation from a malformed
/// or adversarial Idx stream.
const MAX_BLOCK_READ_LEN: u64 = 64 * 1024 * 1024; // 64 MiB

/// Read and decode one scan block from the `.wiff.scan` file.
///
/// `scan_offset` and `scan_size` come from the IdxRecord.
/// `scan_path` is the path to the `.wiff.scan` file.
/// `next_scan_offset` is the `scan_offset` of the *following* scan, or
/// `file_size` for the last scan. It is used to bound the terminator search.
/// `scan_file_size` is the actual on-disk size of `scan_path`.
///
/// Returns the decoded scan points, or an empty vec if the block cannot be read.
pub fn read_scan_block(
    scan_path: &std::path::Path,
    scan_offset: u64,
    scan_size: u64,
    next_scan_offset: u64,
    scan_file_size: u64,
) -> crate::Result<Vec<ScanPoint>> {
    use std::io::{Read, Seek, SeekFrom};

    let payload_start = scan_offset + 56;

    // Bound the read length from several independent sources, since a
    // crafted Idx can lie about any single one of them:
    //   - `next_scan_offset + 64`: where the following block (and thus this
    //     block's terminator) is expected to start, per the module doc,
    //     plus slack for the next block's 56-byte header window.
    //   - `scan_offset + scan_size + 64`: this record's own declared block
    //     extent (the Idx `scan_size` field, previously unused here), an
    //     estimate of the same boundary that doesn't depend on any other
    //     record.
    //   - `scan_file_size`: the actual size of the file on disk - the only
    //     bound here that isn't attacker-controlled, and the one that
    //     actually stops the unbounded-allocation case.
    //   - `MAX_BLOCK_READ_LEN`: a sane absolute ceiling regardless of how
    //     large the file legitimately is.
    let read_end = (next_scan_offset + 64)
        .min(scan_offset + scan_size + 64)
        .min(scan_file_size)
        .min(payload_start.saturating_add(MAX_BLOCK_READ_LEN));
    if read_end <= payload_start {
        return Ok(vec![]);
    }
    let read_len = (read_end - payload_start) as usize;

    let mut f = std::fs::File::open(scan_path)?;
    f.seek(SeekFrom::Start(payload_start))?;
    let mut buf = vec![0u8; read_len];
    let n = f.read(&mut buf)?;
    buf.truncate(n);

    // Find the ff ff ff ff terminator in the buffer.
    let term_pos = find_terminator(&buf);
    let payload = &buf[..term_pos];

    Ok(decode_payload(payload))
}

/// Find the position of the first `ff ff ff ff` sequence in `buf`.
/// Returns `buf.len()` if not found (decode everything).
fn find_terminator(buf: &[u8]) -> usize {
    for i in 0..buf.len().saturating_sub(3) {
        if buf[i] == 0xff && buf[i + 1] == 0xff && buf[i + 2] == 0xff && buf[i + 3] == 0xff {
            return i;
        }
    }
    buf.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_empty() {
        assert_eq!(decode_payload(&[]).len(), 0);
    }

    #[test]
    fn decode_gap_only() {
        // pure gaps, no points
        let pts = decode_payload(&[0x10, 0x20, 0x30]);
        assert_eq!(pts.len(), 0);
    }

    #[test]
    fn decode_single_1byte_intensity() {
        // gap 41 (0x29), 1-byte intensity 0x81 -> intensity 1, mz_bin 41
        let pts = decode_payload(&[0x29, 0x81]);
        assert_eq!(pts.len(), 1);
        assert_eq!(pts[0].raw_mz_bin, 41);
        assert_eq!(pts[0].raw_intensity, 1);
    }

    #[test]
    fn decode_fd_prefix() {
        // gap 0, fd prefix, value 0x0155 = 341
        let pts = decode_payload(&[0x00, 0xfd, 0x55, 0x01]);
        assert_eq!(pts.len(), 1);
        assert_eq!(pts[0].raw_mz_bin, 0);
        assert_eq!(pts[0].raw_intensity, 341);
    }

    #[test]
    fn terminator_stops_decode() {
        // 2 valid tokens, then ff ff ff ff
        let data: Vec<u8> = vec![0x29, 0x81, 0xff, 0xff, 0xff, 0xff, 0x29, 0x81];
        let pts = decode_payload(&data);
        assert_eq!(pts.len(), 1);
    }
}
