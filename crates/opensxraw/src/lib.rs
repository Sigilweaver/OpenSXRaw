#![cfg_attr(not(test), warn(clippy::unwrap_used, clippy::expect_used))]
//! Reader for SCIEX legacy `.wiff` / `.wiff.scan` file pairs.
//!
//! [`Reader`] opens a file pair and implements
//! [`openmassspec_core::SpectrumSource`]. The decoded per-file values it
//! exposes ([`IdxRecord`], [`Calibration`], [`DdeRecord`],
//! [`InstrumentInfo`], [`SourceParameter`]) are read through accessor
//! methods.

pub(crate) mod raw;
pub mod reader;

pub use raw::calibration::Calibration;
pub use raw::dde::DdeRecord;
pub use raw::idx::IdxRecord;
pub use raw::instrument_log::InstrumentInfo;
pub use raw::ion_source::SourceParameter;
pub use reader::Reader;

/// Errors returned by this crate.
///
/// The enum is `#[non_exhaustive]`; match it with a wildcard arm.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// I/O failure opening or reading the `.wiff` or `.wiff.scan` file.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// The `.wiff` container is missing a required stream or a stream does
    /// not have the expected layout.
    #[error("parse error: {0}")]
    Parse(String),
    /// A scan block could not be read from the `.wiff.scan` file: an I/O
    /// error, or a block that starts past the end of the file.
    #[error("cannot read scan block for scan={scan_number} at offset {offset}: {source}")]
    #[non_exhaustive]
    ScanRead {
        /// 1-based scan number (the `N` of the `scan=N` native ID).
        scan_number: u32,
        /// Byte offset of the block in the `.wiff.scan` file.
        offset: u64,
        /// The underlying failure.
        #[source]
        source: std::io::Error,
    },
}

impl From<Error> for openmassspec_core::Error {
    fn from(e: Error) -> Self {
        openmassspec_core::Error::vendor(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
