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
//!   2. Decode tokens until a `ff ff ff ff` terminator is seen at a token
//!      boundary (it falls somewhere inside the next block's window region).
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
//! time-bin index; the reader converts it to m/z (Da) using the
//! `TOFCalibrationData` constants when present (see `raw::calibration`).
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
/// Returns the decoded scan points, an empty vec if the block lies outside
/// the file, or an error if the file cannot be opened or read.
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
    //     extent (the Idx `scan_size` field), an
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

    let f = std::fs::File::open(scan_path)?;
    let mut f = std::io::BufReader::new(f);
    f.seek(SeekFrom::Start(payload_start))?;
    // A single `read()` may legally return fewer bytes than requested, which
    // would silently truncate the payload. `take().read_to_end()` loops until
    // `read_len` bytes are read or EOF is reached.
    let mut buf = Vec::with_capacity(read_len);
    f.take(read_len as u64).read_to_end(&mut buf)?;

    // `decode_payload` stops at the `ff ff ff ff` terminator only when it
    // starts on a token boundary. A byte-wise search would also match the
    // terminator pattern inside a multi-byte intensity value (for example
    // `fd ff ff` followed by an `ff` prefix byte) and cut the payload short.
    Ok(decode_payload(&buf))
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

    #[test]
    fn terminator_pattern_inside_value_does_not_stop_decode() {
        // `fd ff ff` is a 3-byte token with intensity 0xffff; the following
        // `ff` starts a 5-byte token whose value begins with `ff`. The bytes
        // `ff ff ff ff` appear at index 2, which is not a token boundary, so
        // decoding must continue to the real terminator at index 9.
        let data: Vec<u8> = vec![
            0x05, 0xfd, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff,
        ];
        let pts = decode_payload(&data);
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0].raw_mz_bin, 5);
        assert_eq!(pts[0].raw_intensity, 0xffff);
        assert_eq!(pts[1].raw_intensity, 0xff);
    }

    /// Writes `data` to a uniquely-named file under the OS temp dir and
    /// returns its path. Used instead of a `tempfile` dependency since these
    /// are the only tests in the crate that need a real file on disk.
    fn write_temp_scan_file(name: &str, data: &[u8]) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "opensxraw_test_{}_{}.wiff.scan",
            std::process::id(),
            name
        ));
        std::fs::write(&path, data).unwrap();
        path
    }

    #[test]
    fn read_scan_block_decodes_within_bounds() {
        // 56 bytes of header junk, then a 2-byte payload (gap 41, 1-byte
        // intensity 1), then the ff ff ff ff terminator.
        let mut data = vec![0u8; 56];
        data.extend_from_slice(&[0x29, 0x81, 0xff, 0xff, 0xff, 0xff]);
        let file_size = data.len() as u64; // 62

        let path = write_temp_scan_file("decodes_within_bounds", &data);
        let points = read_scan_block(
            &path, 0, /* scan_size */ 1000, /* next_scan_offset */ 6, file_size,
        )
        .unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].raw_mz_bin, 41);
        assert_eq!(points[0].raw_intensity, 1);
    }

    #[test]
    fn read_scan_block_keeps_value_that_contains_terminator_bytes() {
        let mut data = vec![0u8; 56];
        data.extend_from_slice(&[
            0x05, 0xfd, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff,
        ]);
        let file_size = data.len() as u64;
        let path = write_temp_scan_file("value_contains_terminator", &data);
        let points = read_scan_block(&path, 0, 1000, 13, file_size).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].raw_intensity, 0xffff);
        assert_eq!(points[1].raw_intensity, 0xff);
    }

    #[test]
    fn read_scan_block_caps_crafted_offset_to_file_size() {
        // Regression test for the memory-DoS: an Idx claiming a ~4 GiB
        // scan_size / next_scan_offset against a tiny real file must not
        // attempt a multi-gigabyte allocation. If this hangs or OOMs, the
        // bound was reintroduced as a no-op.
        let data = vec![0u8; 64];
        let file_size = data.len() as u64;
        let path = write_temp_scan_file("caps_crafted_offset", &data);

        let points =
            read_scan_block(&path, 0, u32::MAX as u64, u32::MAX as u64, file_size).unwrap();
        std::fs::remove_file(&path).ok();

        // No terminator in an all-zero buffer, so decode runs to the end of
        // the (correctly bounded) slice - the assertion that matters is that
        // this returned at all rather than allocating ~4 GiB.
        assert!(points.is_empty());
    }

    #[test]
    fn read_scan_block_returns_empty_when_offset_past_file_end() {
        let data = vec![0u8; 64];
        let path = write_temp_scan_file("offset_past_file_end", &data);

        let points = read_scan_block(&path, 10_000, 1000, 11_000, data.len() as u64).unwrap();
        std::fs::remove_file(&path).ok();

        assert!(points.is_empty());
    }

    #[test]
    fn read_scan_block_returns_empty_on_truncated_file() {
        // File shorter than the 56-byte block header itself.
        let data = vec![0u8; 10];
        let path = write_temp_scan_file("truncated_file", &data);

        let points = read_scan_block(&path, 0, 1000, 6, data.len() as u64).unwrap();
        std::fs::remove_file(&path).ok();

        assert!(points.is_empty());
    }
}
