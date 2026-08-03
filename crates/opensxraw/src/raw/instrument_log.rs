//! Parsing of the `SampleSubtree/<Sample>/Log` stream's device
//! self-identification records.
//!
//! # Why this stream (issue #4, round 3)
//!
//! Issue #4's first two rounds (see `raw::summary_info`'s module doc,
//! "Round 2") enumerated every stream that looked like a plausible home for
//! a structured instrument model field - `SummaryInformation`,
//! `DocumentSummaryInformation`, `FileRec_Str`, `VendorAppMethod`,
//! `CFR/CFRFileHeader`, device/channel tables, `AcqMethodFileInfoStm`, and
//! the `MSConfigInfo` binary struct - and ruled all of them out, either as
//! free text (site-configured, unreliable) or as a binary field with no
//! vendor documentation to interpret it safely.
//!
//! Neither round inspected `SampleSubtree/<Sample>/Log`: an
//! append-only, mostly UTF-16LE text log of device status events written
//! during acquisition (LC pump/autosampler method dumps, mass spec vacuum
//! status, etc.) - not a name that suggested "instrument identity" up
//! front. It turns out to carry exactly that: near the start of every
//! sampled corpus file, before the LC method text dumps, the mass
//! spectrometer writes a one-time self-identification record shaped like:
//!
//! ```text
//! Mass Spectrometer:QTRAP 6500+ Low Mass:0:,Config Table Version: 00,
//! Firmware Version: ------- ------- PIL1602 PIB1100,
//! Component Name: LINEAR ION TRAP QUADRUPOLE LC/MS/MS MASS SPECTROMETER,
//! Component ID: QTRAP 6500+,Manufacturer: AB SCIEX INSTRUMENTS,
//! Model: 5038125-K, Serial Number: CG22641612
//! ```
//!
//! (line breaks added for readability; the real stream has none between
//! these comma-separated fields). The newer ZenoTOF generation drops the
//! `Config Table Version`/`:0:` framing and lowercases `Component Id`, but
//! keeps the same `Component Id:`/`Manufacturer:`/`Serial Number:` fields:
//!
//! ```text
//! Mass Spectrometer: ZenoTOF 7600 System,Firmware Version: ...,
//! Component Name: ZenoTOF 7600 System,Component Id: ZenoTOF 7600 System,
//! Manufacturer: AB Sciex Instruments,Serial Number: FB22892205,
//! Source Housing: OptiFlow(R) 1-50uL Micro/MicroCal
//! ```
//!
//! Unlike the `SummaryInformation` author/hostname strings (round 1's only
//! candidate, correctly ruled out as site-configured), this is a firmware
//! status report keyed by literal `Manufacturer:`/`Component ID:`/
//! `Serial Number:` labels, not something a lab technician typed in. Other
//! devices in the same log (LC pumps, autosamplers, valves) report their
//! own `Component Id:`/`Manufacturer:` in the same shape - see
//! `parse_instrument_info`'s doc for how the mass-spec-specific entry is
//! picked out from the rest of the log.
//!
//! # Corpus validation
//!
//! Checked across all 201 locally-available complete `.wiff` files
//! (`Data/SRaw`, all three represented families - TripleTOF, QTRAP,
//! ZenoTOF): every file's `Log` stream (when present under the default
//! sample) yielded exactly one mass-spec identification record, with six
//! distinct `Component ID`/`Component Id` values across the corpus -
//! `"QTRAP 5500"`, `"QTRAP 6500+"`, `"Triple TOF 5600"` (note the extra
//! space - a firmware-side quirk, not a typo introduced here),
//! `"TripleTOF 5600+"`, `"TripleTOF 6600"`, and `"ZenoTOF 7600 System"` -
//! all of which normalize (see `resolve_cv_term`) to a real
//! `psi-ms.obo` term under `MS:1000121` ("SCIEX instrument model").
//!
//! Cross-checked for self-consistency against the free-text hints round 1
//! already found unreliable on their own: e.g. `PXD022088/Rcor2KOESC1.wiff`
//! reports `Component ID: QTRAP 6500+` here, and separately carries the
//! `SummaryInformation` `PIDSI_AUTHOR` string `"Monash_6500"` and hostname
//! `"6500-PC"` (see `raw::summary_info`) - independent signals agreeing on
//! the same instrument family and generation. The same correlation held for
//! every other file spot-checked while developing this parser (`QTRAP5500`
//! hostname/author strings pairing with a `QTRAP 5500` log record,
//! `6600ONLINE`/`6600-PC` hostnames pairing with `TripleTOF 6600`,
//! `5600-PC` pairing with `Triple TOF 5600`).
//!
//! Note: `PXD022088/Rcor2KOESC1.wiff` (the small fixture used by
//! `tests/conformance.rs` and referenced in `docs/format/02-legacy-wiff-scan.md`)
//! was previously mislabeled "TripleTOF 5600" in both of those places - a
//! stale/incorrect label, contradicted by its own lack of a
//! `TOFCalibrationData` stream (`docs/format/04-legacy-wiff-calibration.md`
//! already correctly called it "QTRAP-family") and now also by this
//! stream's `Component ID: QTRAP 6500+`. Both mentions are corrected
//! alongside this module landing.
//!
//! # Byte layout: deliberately not fully reverse-engineered
//!
//! The `Log` stream has real record framing (small binary headers between
//! entries - entry index, a type/category code, and what looks like a
//! per-entry byte count), but decoding that framing exactly isn't necessary
//! to extract this field reliably, and doing so on a six-model, one-corpus
//! sample would risk overfitting a decoder to coincidence rather than
//! structure. Instead, `parse_instrument_info` decodes the whole stream as
//! UTF-16LE (each entry's own text is a contiguous run with no embedded
//! binary noise - only the small headers *between* entries decode as
//! garbage) and pattern-matches the known field labels directly. This is
//! textual pattern matching within a stream whose content was decoded from
//! the corpus's own bytes, not a guess at unseen structure - if a future
//! file's log doesn't match this shape, extraction just returns `None` and
//! the generic CV term stays in place (see `resolve_cv_term`), rather than
//! fabricating a result.

/// Parsed identification fields for the mass spectrometer device, read from
/// its self-reported entry in the `Log` stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentInfo {
    /// The instrument's self-reported `Component ID`/`Component Id` value,
    /// e.g. `"TripleTOF 6600"`, `"QTRAP 5500"`, `"ZenoTOF 7600 System"`.
    /// Kept verbatim (including whatever spacing/casing quirk the firmware
    /// used) - normalization only happens for CV term resolution, in
    /// `resolve_cv_term`.
    pub component_id: String,
    /// Self-reported manufacturer string, e.g. `"AB Sciex Instruments"`.
    pub manufacturer: Option<String>,
    /// Self-reported catalog/part number, e.g. `"5021500/T"`. This is not
    /// the instrument serial number - see `serial_number`.
    pub model_number: Option<String>,
    /// Self-reported instrument serial number, e.g. `"BR20271408PL"`.
    pub serial_number: Option<String>,
}

/// Decode a byte buffer as UTF-16LE, lossily. `.wiff` streams are written by
/// a Windows application and this stream's entries are UTF-16LE text with
/// small binary headers in between (see the module doc's "Byte layout"
/// section) - decoding the whole buffer this way keeps entry text intact
/// and turns the interleaved binary headers into a handful of replacement/
/// control characters that the field-extraction logic below never matches
/// against.
fn decode_utf16_lossy(data: &[u8]) -> String {
    let units: Vec<u16> = data
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

/// Extract the value of a `"<key><value>,"`-shaped field from `window`,
/// starting the search at `window`'s beginning. The value runs from just
/// after `key` to the next comma, carriage return, or line feed (whichever
/// comes first) - a comma separates fields within an entry, while CR/LF
/// marks the end of the last field in an entry, right before the next
/// entry's binary header. Returns `None` if `key` isn't found, or the value
/// is empty after trimming whitespace.
fn extract_field(window: &str, key: &str) -> Option<String> {
    let idx = window.find(key)?;
    let after = &window[idx + key.len()..];
    let value_end = after.find([',', '\r', '\n']).unwrap_or(after.len());
    let value = after[..value_end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// Parse the mass spectrometer's self-identification record out of a raw
/// `Log` stream.
///
/// The log holds one entry per device event (LC pumps, autosamplers,
/// valves, and the mass spec itself all report their own status), and more
/// than one entry can start with `"Mass Spectrometer:"` (a "Start of Run"/
/// "End of Run" vacuum-status entry uses the same prefix but carries no
/// `Manufacturer:`/`Component ID:` fields). This scans every
/// `"Mass Spectrometer:"` occurrence in order and returns the first one
/// whose nearby text (bounded by the next such occurrence, so fields from a
/// different entry can't bleed in) has a `Manufacturer:` value containing
/// `"sciex"` (case-insensitive) - i.e. the one genuine identification
/// record, not a status update. Other devices in the same log (e.g. a
/// Waters LC subsystem on a ZenoTOF 7600 fixture) have their own
/// `Component Id:`/`Manufacturer:` pairs with a different, non-SCIEX
/// manufacturer, so the `Manufacturer:` check is what keeps this pointed at
/// the mass spec specifically rather than the first device the log
/// mentions.
///
/// Returns `None` if the stream doesn't contain a matching record - callers
/// treat that as "not resolved" the same way as any other optional field
/// here, not as an error.
pub fn parse_instrument_info(data: &[u8]) -> Option<InstrumentInfo> {
    const TAG: &str = "Mass Spectrometer:";
    let text = decode_utf16_lossy(data);

    let mut search_from = 0usize;
    while let Some(rel) = text[search_from..].find(TAG) {
        let start = search_from + rel;
        let window_end = text[start + 1..]
            .find(TAG)
            .map(|next_rel| start + 1 + next_rel)
            .unwrap_or_else(|| text.len());
        let window = &text[start..window_end];

        if let Some(manufacturer) = extract_field(window, "Manufacturer:") {
            if manufacturer.to_ascii_lowercase().contains("sciex") {
                let component_id = extract_field(window, "Component ID:")
                    .or_else(|| extract_field(window, "Component Id:"));
                if let Some(component_id) = component_id {
                    return Some(InstrumentInfo {
                        component_id,
                        manufacturer: Some(manufacturer),
                        model_number: extract_field(window, "Model:"),
                        serial_number: extract_field(window, "Serial Number:"),
                    });
                }
            }
        }

        search_from = start + TAG.len();
    }
    None
}

/// `(psi-ms.obo accession, canonical name, normalized match key)` for every
/// `Component ID`/`Component Id` value directly observed across the local
/// corpus (`Data/SRaw`, 201 files, all three represented families), each
/// verified against a fresh copy of `psi-ms.obo`
/// (<https://github.com/HUPO-PSI/psi-ms-CV>) - every entry below is a real
/// `is_a: MS:1000121 ! SCIEX instrument model` term, not a guessed
/// accession. Deliberately not a larger list of every SCIEX model in the
/// ontology: this project only asserts models it has actually seen an
/// instrument self-report in a real corpus file.
const KNOWN_MODELS: &[(&str, &str, &str)] = &[
    ("MS:1000931", "QTRAP 5500", "QTRAP5500"),
    ("MS:1002581", "QTRAP 6500", "QTRAP6500"),
    ("MS:1002582", "QTRAP 6500+", "QTRAP6500+"),
    ("MS:1000932", "TripleTOF 5600", "TRIPLETOF5600"),
    ("MS:1002584", "TripleTOF 5600+", "TRIPLETOF5600+"),
    ("MS:1002533", "TripleTOF 6600", "TRIPLETOF6600"),
    ("MS:1003293", "ZenoTOF 7600", "ZENOTOF7600"),
];

/// Normalize a self-reported `Component ID`/`Component Id` string for
/// matching against `KNOWN_MODELS`: uppercase, and strip all whitespace (so
/// `"Triple TOF 5600"` and `"TripleTOF 5600"` compare equal - the firmware
/// itself is inconsistent about the space, observed directly in the
/// corpus - see the module doc), then strip a trailing `"SYSTEM"` word (the
/// ZenoTOF generation appends `" System"` to its own component ID).
fn normalize(component_id: &str) -> String {
    let mut s: String = component_id
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    if let Some(stripped) = s.strip_suffix("SYSTEM") {
        s = stripped.to_string();
    }
    s
}

/// Resolve a self-reported `Component ID`/`Component Id` string to a
/// `(accession, canonical name)` pair from `psi-ms.obo`, or `None` if it
/// doesn't match any model this project has directly observed in the
/// corpus (see `KNOWN_MODELS`). Callers should fall back to the generic
/// `MS:1000121` ("SCIEX instrument model") term on `None` rather than
/// fabricating a specific one.
pub fn resolve_cv_term(component_id: &str) -> Option<(&'static str, &'static str)> {
    let key = normalize(component_id);
    KNOWN_MODELS
        .iter()
        .find(|(_, _, match_key)| *match_key == key)
        .map(|(accession, name, _)| (*accession, *name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic `Log` stream byte buffer: encodes `text` as
    /// UTF-16LE, with a few junk bytes before and after to mimic the real
    /// stream's binary entry headers (see the module doc's "Byte layout"
    /// section) - `parse_instrument_info` must skip over these rather than
    /// choke on them.
    fn build_log_stream(entries: &[&str]) -> Vec<u8> {
        let mut buf = vec![0u8, 0, 0, 0, 4, 0, 0, 0]; // stream preamble junk
        for entry in entries {
            buf.extend_from_slice(&[0, 0, 0, 0, 1, 0, 0, 0, 8, 0, 0, 0]); // entry header junk
            for unit in entry.encode_utf16() {
                buf.extend_from_slice(&unit.to_le_bytes());
            }
            buf.extend_from_slice(&[0x0d, 0x00, 0x0a, 0x00]); // \r\n terminator
        }
        buf
    }

    #[test]
    fn parses_legacy_qtrap_identification_record() {
        // Verbatim shape from PXD022088/Rcor2KOESC1.wiff's Log stream.
        let stream = build_log_stream(&[
            "Software Application:Gradient 1:0:",
            "Mass Spectrometer:QTRAP 6500+ Low Mass:0:,Config Table Version: 00, \
             Firmware Version: ------- ------- PIL1602 PIB1100,Component Name: \
             LINEAR ION TRAP QUADRUPOLE LC/MS/MS MASS SPECTROMETER,Component ID: \
             QTRAP 6500+,Manufacturer: AB SCIEX INSTRUMENTS,Model: 5038125-K, \
             Serial Number: CG22641612",
            "Mass Spectrometer:QTRAP 6500+ Low Mass:0:,Start of Run - Detailed Status",
        ]);

        let info = parse_instrument_info(&stream).expect("expected a match");
        assert_eq!(info.component_id, "QTRAP 6500+");
        assert_eq!(info.manufacturer.as_deref(), Some("AB SCIEX INSTRUMENTS"));
        assert_eq!(info.model_number.as_deref(), Some("5038125-K"));
        assert_eq!(info.serial_number.as_deref(), Some("CG22641612"));
    }

    #[test]
    fn parses_zenotof_identification_record_with_lowercase_id_field() {
        // Verbatim shape from the ZenoTOF 7600 corpus fixture: no "Config
        // Table Version"/":0:" framing, "Component Id" (lowercase d), no
        // trailing comma before the next entry (Serial Number is last).
        let stream = build_log_stream(&[
            "Mass Spectrometer: ZenoTOF 7600 System,Firmware Version: AION_QTOF_ICX \
             Version: 0 05 (0 05),Component Name: ZenoTOF 7600 System,Component Id: \
             ZenoTOF 7600 System,Manufacturer: AB Sciex Instruments,Serial Number: \
             FB22892205,Source Housing: OptiFlow",
            "Valve: Valve Model,Component Name: Valve Model,Component Id: Valve \
             Model,Manufacturer: Valve,",
        ]);

        let info = parse_instrument_info(&stream).expect("expected a match");
        assert_eq!(info.component_id, "ZenoTOF 7600 System");
        assert_eq!(info.manufacturer.as_deref(), Some("AB Sciex Instruments"));
        assert_eq!(info.model_number, None);
        assert_eq!(info.serial_number.as_deref(), Some("FB22892205"));
    }

    #[test]
    fn skips_non_sciex_devices_that_also_say_mass_spectrometer_prefixed_entries() {
        // A "Start of Run" status entry (no Manufacturer:/Component ID:)
        // appears before the real identification entry - parsing must skip
        // past it rather than returning None or a bogus partial match.
        let stream = build_log_stream(&[
            "Mass Spectrometer:TripleTOF 6600:0:,Start of Run - Detailed Status,\
             Vacuum Status:. At Pressure",
            "Mass Spectrometer:TripleTOF 6600:0:, Config Table Version: 00, \
             Component ID: TripleTOF 6600, Manufacturer: AB Sciex Instruments, \
             Model: 5021500/T, Serial Number: BR20271408PL, Source Housing: Nanospray",
        ]);

        let info = parse_instrument_info(&stream).expect("expected a match");
        assert_eq!(info.component_id, "TripleTOF 6600");
        assert_eq!(info.serial_number.as_deref(), Some("BR20271408PL"));
    }

    #[test]
    fn returns_none_when_no_sciex_manufacturer_field_present() {
        let stream = build_log_stream(&[
            "Software Application:Gradient 1:0:",
            "IntegratedSystem: Waters Microscale LC System,Component Name: Waters \
             Microscale LC System,Component Id: Waters Microscale LC System,\
             Manufacturer: Waters,",
        ]);
        assert_eq!(parse_instrument_info(&stream), None);
    }

    #[test]
    fn returns_none_on_empty_stream() {
        assert_eq!(parse_instrument_info(&[]), None);
    }

    #[test]
    fn resolve_cv_term_matches_known_models_case_and_space_insensitively() {
        assert_eq!(
            resolve_cv_term("QTRAP 5500"),
            Some(("MS:1000931", "QTRAP 5500"))
        );
        assert_eq!(
            resolve_cv_term("QTRAP 6500+"),
            Some(("MS:1002582", "QTRAP 6500+"))
        );
        // Firmware-side spacing quirk observed directly in the corpus
        // (PXD078909/Fish_IDA-1.wiff): "Triple TOF 5600" with a space.
        assert_eq!(
            resolve_cv_term("Triple TOF 5600"),
            Some(("MS:1000932", "TripleTOF 5600"))
        );
        assert_eq!(
            resolve_cv_term("TripleTOF 6600"),
            Some(("MS:1002533", "TripleTOF 6600"))
        );
        // ZenoTOF's " System" suffix must be stripped.
        assert_eq!(
            resolve_cv_term("ZenoTOF 7600 System"),
            Some(("MS:1003293", "ZenoTOF 7600"))
        );
    }

    #[test]
    fn resolve_cv_term_returns_none_for_unknown_model() {
        assert_eq!(resolve_cv_term("QTRAP 4500"), None);
        assert_eq!(resolve_cv_term(""), None);
    }
}
