//! Parsing of `MethodSubtree/MethodN/DeviceMethod0/Period0/ExperimentM/
//! IonSourceParamsTable/ParameterK/ParameterData` streams: named ion-source
//! parameters (curtain gas, nebulizer gas flows, source/interface
//! temperature, collision gas, ion spray voltage, ...) recorded once per
//! acquisition method/experiment in Analyst's method editor.
//!
//! # Record layout (confirmed against corpus)
//!
//! Each `ParameterData` stream is a small fixed-preamble record followed by
//! a UTF-16LE parameter name and a little-endian `f32` value (the value is
//! then repeated once more, presumably a "current value" / "display value"
//! pair - only the first copy is read here):
//!
//! | Offset        | Type      | Description                                  |
//! |---------------|-----------|-----------------------------------------------|
//! | `0x00..0x22`  | ...       | Opaque preamble, not decoded                  |
//! | `0x22..0x24`  | u16       | Name length, in bytes (`char_count * 2`)      |
//! | `0x24..`      | UTF-16LE  | Parameter name (`name_len` bytes)              |
//! | (after name)  | f32       | Parameter value, little-endian                |
//! | (+4 more)     | f32       | Same value repeated                            |
//!
//! Verified against the full local corpus (200 `.wiff` files, 2026-08-03):
//! this layout decodes 9 distinct parameter names (`GS1`, `GS2`, `CUR`,
//! `TEM`, `CAD`, `IHT`, `COLUMN TEM`, `ISVF`, `IS`) to values that are all
//! physically sane for their name - curtain/nebulizer gas flows in the
//! 0-90 range, source/interface temperatures in the 0-350 range, and (see
//! below) ion spray voltage in the low thousands of volts - which is strong
//! self-consistency evidence for the offset arithmetic, independent of any
//! single field.
//!
//! # Ion spray voltage as a polarity signal (issue #26)
//!
//! The parameter named `ISVF` ("Ion Spray Voltage Floating", present on the
//! 169/200 TripleTOF/ZenoTOF-family corpus files - the same split as
//! `raw::calibration`'s analyzer-family signal) or `IS` (present on the
//! other 31/200, QTRAP-family files) is the electrospray needle voltage.
//! This is not a SCIEX-specific fact: it is standard, textbook electrospray
//! ionization physics that spray voltage polarity directly determines which
//! ion polarity gets accelerated toward the orifice - a positive voltage
//! produces positive ions, a negative voltage produces negative ions.
//!
//! Every one of the 200 local corpus files (23 PRIDE projects, both
//! instrument families) has a positive `ISVF`/`IS` value (2200-5500 V),
//! consistent with the corpus being entirely positive-mode tryptic-peptide
//! proteomics (PRIDE's scope - see `CORPUS.md`) and with every project's
//! public PRIDE metadata checked (none mention negative-mode acquisition).
//! No corpus file contradicts a positive-voltage-means-positive-mode
//! reading. The corpus does not, however, contain a single negative-mode
//! fixture, so the negative branch is argued from public ESI physics and
//! self-consistency (the field's own embedded name) rather than confirmed
//! against a real file - flagged explicitly at the call site in
//! `reader.rs`.

use byteorder::{ByteOrder, LittleEndian};

/// Byte offset of the `u16` name-length field within a `ParameterData`
/// stream.
const NAME_LEN_OFFSET: usize = 0x22;
/// Byte offset where the UTF-16LE parameter name begins.
const NAME_OFFSET: usize = 0x24;

/// One decoded named parameter from an `IonSourceParamsTable/ParameterN/
/// ParameterData` stream.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceParameter {
    pub name: String,
    pub value: f32,
}

impl SourceParameter {
    /// Parse a `ParameterData` stream's name and value.
    ///
    /// Returns `None` on any structural mismatch (too short, a name length
    /// that doesn't fit, or a name that isn't valid UTF-16) rather than
    /// erroring, since this stream is optional context, not required to
    /// read a spectrum.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < NAME_OFFSET {
            return None;
        }
        let name_len = LittleEndian::read_u16(&data[NAME_LEN_OFFSET..NAME_LEN_OFFSET + 2]) as usize;
        let name_end = NAME_OFFSET.checked_add(name_len)?;
        let value_end = name_end.checked_add(4)?;
        if data.len() < value_end || name_len % 2 != 0 {
            return None;
        }

        let name_units: Vec<u16> = data[NAME_OFFSET..name_end]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let name = String::from_utf16(&name_units).ok()?;

        let value = LittleEndian::read_f32(&data[name_end..value_end]);
        Some(SourceParameter { name, value })
    }
}

/// Parameter names observed in the corpus for the ion spray (electrospray
/// needle) voltage: `ISVF` on TripleTOF/ZenoTOF-family files, `IS` on
/// QTRAP-family files - see the module doc.
pub const ION_SPRAY_VOLTAGE_NAMES: [&str; 2] = ["ISVF", "IS"];

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic `ParameterData` stream for a given name/value,
    /// matching the corpus-confirmed layout (opaque preamble padded to
    /// `NAME_LEN_OFFSET`, then length-prefixed UTF-16LE name, then the f32
    /// value repeated twice).
    fn build_stream(name: &str, value: f32) -> Vec<u8> {
        let mut data = vec![0u8; NAME_LEN_OFFSET];
        let name_utf16: Vec<u8> = name.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        data.extend_from_slice(&(name_utf16.len() as u16).to_le_bytes());
        data.extend_from_slice(&name_utf16);
        data.extend_from_slice(&value.to_le_bytes());
        data.extend_from_slice(&value.to_le_bytes()); // repeated value
        data
    }

    #[test]
    fn parses_isvf_positive_voltage() {
        let data = build_stream("ISVF", 5500.0);
        let param = SourceParameter::from_bytes(&data).unwrap();
        assert_eq!(param.name, "ISVF");
        assert!((param.value - 5500.0).abs() < 1e-6);
    }

    #[test]
    fn parses_negative_voltage() {
        // No real corpus fixture has this, but the parser itself must not
        // special-case sign - see the module doc's polarity caveat.
        let data = build_stream("ISVF", -4500.0);
        let param = SourceParameter::from_bytes(&data).unwrap();
        assert!((param.value - (-4500.0)).abs() < 1e-6);
    }

    #[test]
    fn parses_short_two_char_name() {
        let data = build_stream("IS", 2200.0);
        let param = SourceParameter::from_bytes(&data).unwrap();
        assert_eq!(param.name, "IS");
        assert!((param.value - 2200.0).abs() < 1e-6);
    }

    #[test]
    fn parses_gas_flow_parameter() {
        let data = build_stream("GS1", 12.0);
        let param = SourceParameter::from_bytes(&data).unwrap();
        assert_eq!(param.name, "GS1");
        assert!((param.value - 12.0).abs() < 1e-6);
    }

    #[test]
    fn parses_multi_word_name() {
        let data = build_stream("COLUMN TEM", 30.0);
        let param = SourceParameter::from_bytes(&data).unwrap();
        assert_eq!(param.name, "COLUMN TEM");
    }

    #[test]
    fn too_short_returns_none() {
        assert!(SourceParameter::from_bytes(&[0u8; 10]).is_none());
    }

    #[test]
    fn truncated_value_returns_none() {
        let mut data = build_stream("ISVF", 5500.0);
        data.truncate(data.len() - 6); // cut into the value
        assert!(SourceParameter::from_bytes(&data).is_none());
    }

    #[test]
    fn odd_name_length_returns_none() {
        let mut data = vec![0u8; NAME_LEN_OFFSET];
        data.extend_from_slice(&3u16.to_le_bytes()); // odd byte length, invalid for UTF-16
        data.extend_from_slice(&[0u8; 3]);
        data.extend_from_slice(&5500.0f32.to_le_bytes());
        assert!(SourceParameter::from_bytes(&data).is_none());
    }
}
