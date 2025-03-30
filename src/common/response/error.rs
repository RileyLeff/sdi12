// src/common/response/error.rs

use crate::common::types::Sdi12ParsingError; // Use crate path
use core::fmt;

/// Error type specific to response parsing.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ResponseParseError {
    /// Input buffer was empty.
    EmptyInput,
    /// Input buffer doesn't end with <CR><LF> (for ASCII responses).
    MissingCrLf,
    /// Response string is too short for the expected format.
    TooShort,
    /// Invalid address character at the start.
    InvalidAddressChar,
    /// Expected specific character not found (e.g., ',', ';', '+', '-').
    UnexpectedCharacter,
    /// Failed to parse the <values> part.
    ValueError(Sdi12ParsingError),
    /// Failed to parse numeric parts (e.g., ttt, nnn, version).
    NumericError,
    /// CRC validation failed or structure mismatch.
    CrcMismatch,
    /// Version 'll' in identification response is invalid.
    InvalidVersionFormat,
    /// Identification response parts (vendor, model, version) have wrong length.
    InvalidIdentificationLength,
    /// Binary packet size field is inconsistent with actual payload length.
    InconsistentBinaryPacketSize,
    /// Invalid binary data type code received.
    InvalidBinaryDataType,
    /// Feature like 'alloc' needed but not enabled (e.g., trying to parse Identification).
    FeatureNotEnabled,
    /// Generic "invalid format" for cases not covered above.
    InvalidFormat,
    /// Could not decode response content as UTF-8.
    InvalidUtf8,
}

// --- Error Conversions ---

impl From<Sdi12ParsingError> for ResponseParseError {
    fn from(e: Sdi12ParsingError) -> Self { ResponseParseError::ValueError(e) }
}

impl From<core::str::Utf8Error> for ResponseParseError {
    fn from(_: core::str::Utf8Error) -> Self { ResponseParseError::InvalidUtf8 }
}

impl From<core::num::ParseIntError> for ResponseParseError {
    fn from(_: core::num::ParseIntError) -> Self { ResponseParseError::NumericError }
}

// Note: f32::from_str error type (ParseFloatError) is in std, not core.
// If we stick to core::num::*, we might need a different float parsing approach
// or a dedicated no-std float parsing crate if floats are needed without std.
// For now, mapping the potential ParseIntError covers integer parts.
// Let's add a specific variant if needed.
// Update: Sdi12Value parser uses f32::from_str, so it implicitly requires std for that path.
// If we need truly no-std float parsing, Sdi12Value needs rework.
// Let's assume std is available for f32::from_str for now, or the parser guards it.

impl fmt::Display for ResponseParseError {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
         // Simple display for now
         write!(f, "{:?}", self)
     }
}

// If std feature is enabled, implement the Error trait
#[cfg(feature = "std")]
impl std::error::Error for ResponseParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ResponseParseError::ValueError(ref e) => Some(e), // Assuming Sdi12ParsingError impls Error
            // Note: str::Utf8Error, num::ParseIntError etc. also implement Error
            _ => None,
        }
    }
}

// Need to implement Error trait for Sdi12ParsingError as well if chaining is desired
#[cfg(feature = "std")]
impl std::error::Error for crate::common::types::Sdi12ParsingError {} // Basic impl