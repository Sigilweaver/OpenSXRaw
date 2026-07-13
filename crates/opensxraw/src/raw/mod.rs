//! Low-level parsing modules for .wiff and .wiff.scan files.

pub mod calibration;
pub mod dde;
pub mod idx;
pub mod scan;
pub mod summary_info;

/// Read exactly `length` bytes from `path` starting at `offset`.
pub fn read_bytes(path: &std::path::Path, offset: u64, length: usize) -> crate::Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path)?;
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; length];
    f.read_exact(&mut buf)?;
    Ok(buf)
}
