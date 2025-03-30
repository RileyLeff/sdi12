// src/common/response.rs

use crate::common::address::Sdi12Addr;
use crate::common::types::Sdi12ParsingError; // Keep for error composition
use core::fmt;

/// Error type specific to parsing the framing/address/CRC of an SDI-12 response.
/// Does not cover errors from parsing the actual payload content (data values, ID fields etc.).
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ResponseParseError {
    /// Input buffer was empty.
    EmptyInput,
    /// Input buffer doesn't end with <CR><LF> (for ASCII responses).
    MissingCrLf,
    /// Response string is too short for basic structure (e.g., address or CRC).
    TooShort,
    /// Invalid or unexpected address character at the start (e.g., '?').
    InvalidAddressChar,
    /// CRC validation failed.
    CrcMismatch,
    /// Binary packet size/structure inconsistent (if library handles binary framing).
    InconsistentBinaryPacketSize,
    /// Feature needed for a specific check/parse is not enabled.
    FeatureNotEnabled,
    /// Generic framing or structural format error.
    InvalidFormat,
    // NOTE: Errors like ValueError, NumericError, InvalidIdentificationLength etc.
    // are removed as they relate to parsing the *payload*, which is now the user's responsibility
    // or handled by optional helpers. ResponseParseError focuses on the layer the library handles.
}

impl fmt::Display for ResponseParseError {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
         // Simple display for now
         write!(f, "{:?}", self)
     }
}

// If std feature is enabled, implement the Error trait
#[cfg(feature = "std")]
impl std::error::Error for ResponseParseError {}


/// Timing and count information returned directly by Measurement/Concurrent/Identify commands.
/// (Example: `aTTTN<CR><LF>`)
/// This is one structure the library *might* still parse directly, as it's not payload data.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MeasurementTiming {
    /// The address of the responding sensor.
    pub address: Sdi12Addr,
    /// Time estimate in seconds until data is ready (ttt). 0-999.
    pub time_seconds: u16,
    /// Number of measurement values that will be returned (n, nn, or nnn). 0-999.
    pub values_count: u16,
}


// --- Placeholder for the Payload Slice Wrapper ---
// This struct would be returned by recorder methods after validating
// address, CRC, CRLF and stripping them.

/// Represents the validated payload of an SDI-12 response, borrowed from a read buffer.
/// Excludes the leading address, trailing CRC (if any), and trailing <CR><LF>.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PayloadSlice<'a>(pub &'a [u8]);

impl<'a> PayloadSlice<'a> {
    /// Returns the payload as a byte slice.
    pub fn as_bytes(&self) -> &'a [u8] {
        self.0
    }

    /// Attempts to interpret the payload as a UTF-8 string slice.
    pub fn as_str(&self) -> Result<&'a str, core::str::Utf8Error> {
        core::str::from_utf8(self.0)
    }

    // Optional: Add helper methods here later under features?
    // #[cfg(feature = "alloc")]
    // pub fn parse_data_values(&self) -> Result<Vec<Sdi12Value>, ResponseParseError> { ... }
}

impl<'a> AsRef<[u8]> for PayloadSlice<'a> {
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}

// No parsing functions like parse_response defined here anymore.
// That logic moves into internal recorder helpers or optional user-facing helpers.

// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;

     fn addr(c: char) -> Sdi12Addr { Sdi12Addr::new(c).unwrap() }

    #[test]
    fn test_measurement_timing_struct() {
        let mt = MeasurementTiming {
            address: addr('1'),
            time_seconds: 15,
            values_count: 4,
        };
        assert_eq!(mt.time_seconds, 15);
    }

     #[test]
    fn test_payload_slice_wrapper() {
        let data: &[u8] = b"+1.23-45";
        let payload = PayloadSlice(data);
        assert_eq!(payload.as_bytes(), b"+1.23-45");
        assert_eq!(payload.as_ref(), b"+1.23-45");
        assert_eq!(payload.as_str().unwrap(), "+1.23-45");

        let non_utf8: &[u8] = &[0x80, 0x81]; // Invalid UTF-8
        let payload_bad = PayloadSlice(non_utf8);
        assert!(payload_bad.as_str().is_err());
    }
}