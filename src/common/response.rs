// src/common/response.rs

use super::address::Sdi12Addr;
use super::crc; // For potential CRC validation during parsing
use super::error::Sdi12Error;
use super::types::{BinaryDataType, Sdi12Value, Sdi12ParsingError};
// Need alloc for String/Vec based responses if the 'alloc' feature is enabled
#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

// --- Response Structures ---

/// Represents any valid, parsed response received from an SDI-12 sensor.
/// Includes the address of the sensor that sent the response.
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// Simple Acknowledge (`a<CR><LF>`) from `a!` or `?!`.
    Acknowledge { address: Sdi12Addr },

    /// Service Request (`a<CR><LF>`) sent autonomously by sensor.
    ServiceRequest { address: Sdi12Addr },

    /// Identification Information (`aII...<CR><LF>`) from `aI!`.
    /// Using Vec/String, requires 'alloc'. Fixed-size arrays could be an alternative for no-alloc.
    #[cfg(feature = "alloc")]
    Identification(IdentificationInfo),

    /// Address Confirmation (`b<CR><LF>`) from `aAb!`. Contains the *new* address confirmed by sensor.
    Address { address: Sdi12Addr },

    /// Timing information (`atttn[nn]<CR><LF>`) from M/C/V/HA/HB/Identify commands.
    MeasurementTiming(MeasurementTiming),

    /// Data values (`a<values>[<CRC>]<CR><LF>`) from D/R commands. Includes CRC if requested.
    /// Using Vec, requires 'alloc'. Could use `heapless::Vec` for no-alloc.
    #[cfg(feature = "alloc")]
    Data(DataInfo),

    /// Binary Data Packet (`Address PacketSize DataType Payload CRC`) from DB commands.
    /// Payload stored in Vec, requires 'alloc'. Could use `heapless::Vec` for no-alloc.
    #[cfg(feature = "alloc")]
    BinaryData(BinaryDataInfo),

    /// Metadata Parameter Information (`a,field1,field2;[<CRC>]<CR><LF>`) from Identify Parameter commands.
    /// Using Vec<String>, requires 'alloc'.
    #[cfg(feature = "alloc")]
    Metadata(MetadataInfo),

    // Add other potential response types if needed (e.g., specific error responses if defined)
}


// --- Supporting Structs for Response Variants ---

/// Information returned by the Send Identification (`aI!`) command. (Sec 4.4.2)
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentificationInfo {
    /// The address of the responding sensor.
    pub address: Sdi12Addr,
    /// SDI-12 Compatibility Level (e.g., 14 for V1.4). Parsed from "ll".
    pub sdi_version: u8,
    /// Vendor Identification (8 chars). Parsed from "cccccccc".
    pub vendor: String, // Max 8 chars, could use heapless::String<8>
    /// Sensor Model (6 chars). Parsed from "mmmmmm".
    pub model: String, // Max 6 chars, could use heapless::String<6>
    /// Sensor firmware/hardware version (3 chars). Parsed from "vvv".
    pub version: String, // Max 3 chars, could use heapless::String<3>
    /// Optional sensor-specific info (e.g., serial number). Up to 13 chars. Parsed from "xxx...xx".
    pub optional: Option<String>, // Max 13 chars, could use heapless::String<13>
}

/// Timing and count information returned by Measurement/Concurrent/Identify commands. (Sec 4.4.5 etc.)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MeasurementTiming {
    /// The address of the responding sensor.
    pub address: Sdi12Addr,
    /// Time estimate in seconds until data is ready (ttt). 0-999.
    pub time_seconds: u16,
    /// Number of measurement values that will be returned (n, nn, or nnn). 0-999.
    pub values_count: u16,
}

/// Data values returned by Send Data (`aDn!`) or Read Continuous (`aRn!`) commands.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
pub struct DataInfo {
    /// The address of the responding sensor.
    pub address: Sdi12Addr,
    /// The parsed data values.
    pub values: Vec<Sdi12Value>, // Could use heapless::Vec for no-alloc
    /// CRC value included in the response, if one was requested and sent.
    pub crc: Option<u16>,
}

/// Binary data packet returned by Send Binary Data (`aDBn!`) command. (Sec 5.2)
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryDataInfo {
    /// The address of the responding sensor.
    pub address: Sdi12Addr,
    /// The total size in bytes of the `payload` (from packet header).
    pub packet_size: u16,
    /// The type of data contained in the `payload`.
    pub data_type: BinaryDataType,
    /// The raw binary payload. Interpretation depends on `data_type`. Max 1000 bytes.
    pub payload: Vec<u8>, // Could use heapless::Vec for no-alloc
    /// The 16-bit binary CRC value received at the end of the packet.
    pub crc: u16,
}

/// Metadata information returned by Identify Measurement Parameter commands. (Sec 6.2)
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataInfo {
    /// The address of the responding sensor.
    pub address: Sdi12Addr,
    /// The parsed fields (comma-separated values). Field 0 is address. Field 1=param ID, Field 2=units.
    pub fields: Vec<String>, // Could use heapless::Vec<heapless::String<_>>
    /// CRC value included in the response, if one was requested and sent.
    pub crc: Option<u16>,
}


// --- Parsing Logic (Placeholder - Complex Task) ---

/// Error type specific to response parsing.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ResponseParseError {
    /// Input buffer was empty.
    EmptyInput,
    /// Input buffer doesn't end with <CR><LF>.
    MissingCrLf,
    /// Response string is too short for the expected format.
    TooShort,
    /// Invalid address character at the start.
    InvalidAddressChar,
    /// Expected specific character not found (e.g., ',' or ';').
    UnexpectedCharacter,
    /// Failed to parse the <values> part.
    ValueError(Sdi12ParsingError),
    /// Failed to parse numeric parts (e.g., ttt, nnn).
    NumericError,
    /// CRC validation failed.
    CrcMismatch,
    /// Version 'll' in identification response is invalid.
    InvalidVersionFormat,
    /// Binary packet size field is inconsistent with actual payload length.
    InconsistentBinaryPacketSize,
    /// Invalid binary data type code received.
    InvalidBinaryDataType,
    /// Feature like 'alloc' needed but not enabled.
    FeatureNotEnabled,
    /// Generic "invalid format" for cases not covered above.
    InvalidFormat,
}

impl From<Sdi12ParsingError> for ResponseParseError {
    fn from(e: Sdi12ParsingError) -> Self { ResponseParseError::ValueError(e) }
}

/// Parses a complete response byte slice (including address, data, potential CRC, and trailing <CR><LF>)
/// into a `Response` enum variant.
///
/// This is a complex function that needs careful implementation using manual slicing
/// or a parsing library like `nom`.
pub fn parse_response(buffer: &[u8]) -> Result<Response, ResponseParseError> {
    // TODO: Implement robust parsing logic here.
    // Steps:
    // 1. Check minimum length (at least 3: a<CR><LF>).
    // 2. Check for <CR><LF> at the end. Trim them.
    // 3. Extract address character, validate it using Sdi12Addr::new.
    // 4. Check for potential CRC (3 ASCII chars or 2 binary bytes - but binary is handled separately).
    // 5. Based on length and characters after address, determine response type:
    //    - `a<CR><LF>` only? -> Acknowledge or ServiceRequest (distinction might need context?)
    //    - `b<CR><LF>` only? -> Address confirmation (b must be valid address)
    //    - `aI...` -> Identification
    //    - `atttn[nn]<CR><LF>` -> MeasurementTiming
    //    - `a<values>...` -> Data
    //    - `a,...;` -> Metadata
    // 6. Parse specific fields based on type (vendor, model, ttt, nnn, values, fields).
    // 7. Handle potential ASCII CRC if present.
    // 8. Construct and return the appropriate Response variant.

    Err(ResponseParseError::InvalidFormat) // Placeholder
}

/// Parses a high-volume binary packet (which does NOT end in <CR><LF>).
pub fn parse_binary_packet(buffer: &[u8]) -> Result<Response, ResponseParseError> {
     // TODO: Implement binary packet parsing logic.
     // Steps:
     // 1. Check minimum length (address + size(2) + type(1) + crc(2) = 6 bytes).
     // 2. Extract address, size, type, CRC bytes.
     // 3. Validate address char.
     // 4. Validate binary CRC using `crc::verify_packet_crc_binary`.
     // 5. Extract payload slice based on packet size. Check consistency.
     // 6. Convert type byte to BinaryDataType enum.
     // 7. Construct BinaryDataInfo.
     // 8. Return Response::BinaryData.

     Err(ResponseParseError::InvalidFormat) // Placeholder
}


// --- Unit Tests (Basic Structure Checks) ---
#[cfg(test)]
mod tests {
    // TODO: Add extensive tests for parsing logic once implemented.
    // For now, just basic checks if using alloc feature.

    #[cfg(feature = "alloc")]
    #[test]
    fn test_response_struct_compilation() {
        use super::*;
        let addr = Sdi12Addr::new('1').unwrap();

        let _ack = Response::Acknowledge { address: addr };
        let _sr = Response::ServiceRequest { address: addr };
        let _id = Response::Identification(IdentificationInfo {
            address: addr, sdi_version: 14, vendor: "V".into(), model: "M".into(), version: "1".into(), optional: None,
        });
        let _ad = Response::Address { address: addr };
         let _mt = Response::MeasurementTiming(MeasurementTiming { address: addr, time_seconds: 10, values_count: 5 });
         let _d = Response::Data(DataInfo { address: addr, values: vec![Sdi12Value::new(1.0)], crc: None });
         let _bd = Response::BinaryData(BinaryDataInfo {
            address: addr, packet_size: 1, data_type: BinaryDataType::UnsignedU8, payload: vec![1], crc: 0x1234
         });
         let _md = Response::Metadata(MetadataInfo { address: addr, fields: vec!["f1".into()], crc: None });
    }
}