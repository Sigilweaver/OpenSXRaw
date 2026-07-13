//! Parsing of the standard OLE `\x05SummaryInformation` property set stream.
//!
//! Unlike the other streams under `raw/`, this is not a SCIEX-specific
//! format: it is the generic Microsoft "SummaryInformation" property set
//! defined by `[MS-OLEPS]` (the same format Microsoft Office documents use
//! for their "Summary" tab), written into every CFBF-based file by the
//! compound file layer regardless of application. Its layout is public and
//! documented, so parsing it is not format reverse-engineering in the sense
//! the rest of this crate's `raw/` streams are.
//!
//! # Why this stream
//!
//! Investigating for issue #4 (instrument ID / acquisition timestamp are
//! hardcoded placeholders), a corpus survey of the local `.wiff` sample set
//! found:
//!
//! - `PIDSI_CREATE_DTM` (property ID 12, `VT_FILETIME`) is present in 196 of
//!   201 sampled `.wiff` files (the other 5 either lacked a
//!   `SummaryInformation` stream entirely or were not CFBF at all - almost
//!   certainly renamed `.wiff2` files or corrupt downloads).
//! - It agrees, to the second, with the human-readable "Checksum Time"
//!   string in `CFR/CFRFileHeader` once the local UTC offset implied by the
//!   sample's site is accounted for (verified against two independent
//!   fixtures: a Melbourne, Australia TripleTOF acquisition at UTC+10, and a
//!   US-Eastern QTRAP acquisition at UTC-4). That agreement is what gives
//!   confidence this is a true UTC instant (unlike some vendors' embedded
//!   timestamps, which are local wall-clock time with no recorded offset).
//! - Analyst creates the `.wiff` container at the start of an acquisition
//!   and streams data into it as the run proceeds, so the compound file's
//!   creation time is a reasonable proxy for the acquisition start time.
//!
//! No equivalent reliable field was found for the *instrument model*: the
//! only candidate text (the `FileRec_Str` / `SummaryInformation`
//! "author"/"comments" fields, and `SampleSubtree/Sample1/SampleDABE/CFR_INFO`)
//! carries the Analyst-configured instrument *name* and the acquisition
//! computer's hostname, both of which are free text set by the lab at
//! installation time. In one corpus fixture, this text happened to embed a
//! real model number (`"Monash_6500"`, alongside a `"6500-PC"` hostname); in
//! another, it was a personal username and a generic asset tag
//! (`"prot-user"` / `"D-CCB-04240"`) with no model information at all. This
//! is not a reliable, vendor-populated field and is deliberately not parsed
//! here - see the instrument CV term comment in `reader.rs`.

/// Property ID for `PIDSI_CREATE_DTM` ("time and date of creation") in the
/// SummaryInformation property set, per `[MS-OLEPS]` section 2.15.
const PIDSI_CREATE_DTM: u32 = 0x0000_000c;

/// `VARTYPE` code for `VT_FILETIME`, per `[MS-OLEPS]` / `[MS-OAUT]`.
const VT_FILETIME: u32 = 0x0040;

/// Byte-order marker that must open every `PropertySetStream`.
const PROPERTYSETSTREAM_BYTE_ORDER: u16 = 0xFFFE;

/// 100-nanosecond ticks between the FILETIME epoch (1601-01-01T00:00:00Z)
/// and the Unix epoch (1970-01-01T00:00:00Z).
const FILETIME_UNIX_EPOCH_DIFF_TICKS: i64 = 116_444_736_000_000_000;

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    data.get(offset..offset + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn read_u64(data: &[u8], offset: usize) -> Option<u64> {
    data.get(offset..offset + 8)
        .map(|b| u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
}

/// Convert a proleptic Gregorian civil date to days since 1970-01-01.
///
/// Howard Hinnant's `days_from_civil` algorithm
/// (<https://howardhinnant.github.io/date_algorithms.html#days_from_civil>,
/// public domain calendar arithmetic, independent of any vendor source).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Format a Windows `FILETIME` (100-ns ticks since 1601-01-01T00:00:00Z) as
/// an RFC 3339 UTC timestamp.
fn filetime_to_rfc3339(filetime: u64) -> String {
    let unix_ticks = filetime as i64 - FILETIME_UNIX_EPOCH_DIFF_TICKS;
    let total_secs = unix_ticks.div_euclid(10_000_000);
    let rem_ticks = unix_ticks.rem_euclid(10_000_000);
    let millis = rem_ticks / 10_000;

    let days = total_secs.div_euclid(86_400);
    let sec_of_day = total_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = sec_of_day / 3600;
    let minute = (sec_of_day % 3600) / 60;
    let second = sec_of_day % 60;

    if millis > 0 {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
    } else {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
    }
}

/// Parse the `PIDSI_CREATE_DTM` property out of a raw `\x05SummaryInformation`
/// stream and format it as an RFC 3339 UTC timestamp.
///
/// Returns `None` on any structural mismatch - missing byte-order marker,
/// out-of-range offsets, the property being absent, or an unexpected
/// `VARTYPE` - rather than erroring, since this is optional metadata that
/// not every `.wiff` file is guaranteed to carry (see the module doc
/// comment for corpus survey numbers).
///
/// # Stream layout (`[MS-OLEPS]` `PropertySetStream`)
///
/// - `[0..2]`: byte order marker, must be `0xFFFE`.
/// - `[2..4]`: version (ignored).
/// - `[4..8]`: `SystemIdentifier` (ignored).
/// - `[8..24]`: `CLSID` (ignored).
/// - `[24..28]`: `NumPropertySets` (`u32`).
/// - `[28..44]`: `FMTID0` (ignored - expected to be
///   `FMTID_SummaryInformation`, but not checked, since a mismatch would
///   just mean the property lookup below fails to find `PIDSI_CREATE_DTM`).
/// - `[44..48]`: `Offset0` (`u32`), byte offset from the start of the stream
///   to the property set's data section.
///
/// At `Offset0`:
/// - `[0..4]`: `Size` of the section (ignored).
/// - `[4..8]`: `NumProperties` (`u32`).
/// - `[8..]`: `NumProperties` pairs of `(PropertyID: u32, Offset: u32)`,
///   where `Offset` is relative to `Offset0`.
///
/// At each property's absolute offset:
/// - `[0..4]`: `VARTYPE` (`u32`).
/// - `[4..]`: the value, typed per `VARTYPE`. For `VT_FILETIME`, this is an
///   8-byte little-endian `FILETIME`.
pub fn parse_create_timestamp(data: &[u8]) -> Option<String> {
    if read_u16(data, 0)? != PROPERTYSETSTREAM_BYTE_ORDER {
        return None;
    }
    let num_property_sets = read_u32(data, 24)?;
    if num_property_sets == 0 {
        return None;
    }
    let offset0 = read_u32(data, 44)? as usize;
    let num_properties = read_u32(data, offset0 + 4)?;

    for i in 0..num_properties {
        let entry_off = offset0 + 8 + (i as usize) * 8;
        let property_id = read_u32(data, entry_off)?;
        if property_id != PIDSI_CREATE_DTM {
            continue;
        }
        let value_off = offset0 + read_u32(data, entry_off + 4)? as usize;
        let vartype = read_u32(data, value_off)?;
        if vartype != VT_FILETIME {
            return None;
        }
        let filetime = read_u64(data, value_off + 4)?;
        return Some(filetime_to_rfc3339(filetime));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal synthetic `SummaryInformation` stream containing only
    /// a `PIDSI_CREATE_DTM` property, mirroring the real layout closely
    /// enough to exercise the offset arithmetic.
    fn build_stream(filetime: u64) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFFFEu16.to_le_bytes()); // byte order
        data.extend_from_slice(&0u16.to_le_bytes()); // version
        data.extend_from_slice(&[0u8; 4]); // system identifier
        data.extend_from_slice(&[0u8; 16]); // CLSID
        data.extend_from_slice(&1u32.to_le_bytes()); // NumPropertySets
        data.extend_from_slice(&[0u8; 16]); // FMTID0 (unchecked)
        let offset0 = data.len() as u32 + 4;
        data.extend_from_slice(&offset0.to_le_bytes()); // Offset0

        let section_start = data.len();
        assert_eq!(section_start, offset0 as usize);
        data.extend_from_slice(&0u32.to_le_bytes()); // Size (unchecked)
        data.extend_from_slice(&1u32.to_le_bytes()); // NumProperties
        let value_rel_off = 8u32 + 8; // past the one (id, offset) pair
        data.extend_from_slice(&PIDSI_CREATE_DTM.to_le_bytes());
        data.extend_from_slice(&value_rel_off.to_le_bytes());
        data.extend_from_slice(&VT_FILETIME.to_le_bytes());
        data.extend_from_slice(&filetime.to_le_bytes());
        data
    }

    #[test]
    fn parses_known_filetime() {
        // 2019-06-25T04:31:23.912451Z from the Rcor2KOESC1.wiff corpus
        // fixture, truncated to millisecond precision (912 ms) since we
        // only format down to milliseconds.
        // FILETIME = unix_ticks + epoch_diff.
        let unix_seconds = 1_561_437_083i64; // 2019-06-25T04:31:23Z
        let millis = 912i64;
        let filetime =
            (unix_seconds * 10_000_000 + millis * 10_000 + FILETIME_UNIX_EPOCH_DIFF_TICKS) as u64;
        let stream = build_stream(filetime);
        assert_eq!(
            parse_create_timestamp(&stream).as_deref(),
            Some("2019-06-25T04:31:23.912Z")
        );
    }

    #[test]
    fn returns_none_on_bad_byte_order() {
        let mut stream = build_stream(FILETIME_UNIX_EPOCH_DIFF_TICKS as u64);
        stream[0] = 0x00;
        stream[1] = 0x00;
        assert_eq!(parse_create_timestamp(&stream), None);
    }

    #[test]
    fn returns_none_on_truncated_stream() {
        assert_eq!(parse_create_timestamp(&[]), None);
        assert_eq!(parse_create_timestamp(&[0xfe, 0xff]), None);
    }

    #[test]
    fn returns_none_when_property_absent() {
        // NumProperties = 0.
        let mut data = Vec::new();
        data.extend_from_slice(&0xFFFEu16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&[0u8; 4]);
        data.extend_from_slice(&[0u8; 16]);
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 16]);
        let offset0 = data.len() as u32 + 4;
        data.extend_from_slice(&offset0.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes()); // Size
        data.extend_from_slice(&0u32.to_le_bytes()); // NumProperties = 0
        assert_eq!(parse_create_timestamp(&data), None);
    }
}
