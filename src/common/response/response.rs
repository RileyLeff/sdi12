// src/common/response.rs

use super::address::Sdi12Addr;
use super::crc; // For potential CRC access/storage
use super::error::Sdi12Error; // Potentially needed by users of this module
use super::types::{BinaryDataType, Sdi12Value, Sdi12ParsingError};
use core::fmt;
use core::str::FromStr; // Needed for parsing numeric parts

// Need alloc for String/Vec based responses if the 'alloc' feature is enabled
#[cfg(feature = "alloc")]
use alloc::{string::{String, ToString}, vec::Vec}; // Added ToString

// --- Response Structures ---

/// Represents any valid, parsed response received from an SDI-12 sensor.
/// Includes the address of the sensor that sent the response.
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// Simple Acknowledge (`a<CR><LF>`) from `a!` or `?!`.
    Acknowledge { address: Sdi12Addr },

    /// Service Request (`a<CR><LF>`) sent autonomously by sensor.
    /// Note: Distinguishing this from Acknowledge often requires recorder state context.
    /// The parser might initially parse both as Acknowledge.
    ServiceRequest { address: Sdi12Addr },

    /// Identification Information (`aII...<CR><LF>`) from `aI!`.
    #[cfg(feature = "alloc")]
    Identification(IdentificationInfo),

    /// Address Confirmation (`b<CR><LF>`) from `aAb!`. Contains the *new* address confirmed by sensor.
    Address { address: Sdi12Addr }, // The address is the *new* address 'b'

    /// Timing information (`atttn[nn]<CR><LF>`) from M/C/V/HA/HB/Identify commands.
    MeasurementTiming(MeasurementTiming),

    /// Data values (`a<values>[<CRC>]<CR><LF>`) from D/R commands. Includes CRC if requested/present.
    #[cfg(feature = "alloc")]
    Data(DataInfo),

    /// Binary Data Packet (`Address PacketSize DataType Payload CRC`) from DB commands.
    #[cfg(feature = "alloc")]
    BinaryData(BinaryDataInfo),

    /// Metadata Parameter Information (`a,field1,field2;[<CRC>]<CR><LF>`) from Identify Parameter commands.
    #[cfg(feature = "alloc")]
    Metadata(MetadataInfo),

    /// Sensor indicates aborted measurement (`a<CR><LF>` or `a<CRC><CR><LF>`) in response to `aDn!`.
    /// This helps distinguish an empty ack from an explicit abort signal.
    Aborted {
        address: Sdi12Addr,
        crc: Option<u16>, // CRC is present if requested by MC/CC command
    },
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
    pub vendor: String, // Consider heapless::String<8> if heapless feature added
    /// Sensor Model (6 chars). Parsed from "mmmmmm".
    pub model: String, // Consider heapless::String<6>
    /// Sensor firmware/hardware version (3 chars). Parsed from "vvv".
    pub version: String, // Consider heapless::String<3>
    /// Optional sensor-specific info (e.g., serial number). Up to 13 chars. Parsed from "xxx...xx".
    pub optional: Option<String>, // Consider heapless::String<13>
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
    pub values: Vec<Sdi12Value>, // Consider heapless::Vec if heapless feature added
    /// CRC value included in the response, if one was requested and present.
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
    pub payload: Vec<u8>, // Consider heapless::Vec if heapless feature added
    /// The 16-bit binary CRC value received at the end of the packet.
    pub crc: u16,
}

/// Metadata information returned by Identify Measurement Parameter commands. (Sec 6.2)
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataInfo {
    /// The address of the responding sensor.
    pub address: Sdi12Addr,
    /// The parsed fields (comma-separated values). Field 0=address(redundant), 1=param ID, 2=units...
    pub fields: Vec<String>, // Consider heapless::Vec<heapless::String<_>>
    /// CRC value included in the response, if one was requested and present.
    pub crc: Option<u16>,
}


// --- Parsing Logic ---

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
    CrcMismatch, // Could also be used if CRC present when not expected or vice versa
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
    /// Could not decode ASCII CRC characters.
    InvalidAsciiCrcEncoding,
     /// Could not decode binary CRC bytes.
    InvalidBinaryCrcEncoding,
}

impl From<Sdi12ParsingError> for ResponseParseError {
    fn from(e: Sdi12ParsingError) -> Self { ResponseParseError::ValueError(e) }
}

// Helper to check and trim <CR><LF>
fn trim_cr_lf(buffer: &[u8]) -> Option<&[u8]> {
    if buffer.len() >= 2 && buffer[buffer.len() - 2..] == [b'\r', b'\n'] {
        Some(&buffer[..buffer.len() - 2])
    } else {
        None
    }
}

/// Parses a standard ASCII response byte slice (including address, data, potential CRC,
/// and required trailing <CR><LF>) into a `Response` enum variant.
/// Does *not* handle binary packets.
pub fn parse_response(buffer: &[u8]) -> Result<Response, ResponseParseError> {
    let data = trim_cr_lf(buffer).ok_or(ResponseParseError::MissingCrLf)?;

    if data.is_empty() {
        return Err(ResponseParseError::TooShort); // Need at least address char
    }

    // Extract address
    let addr_char = data[0] as char;
    let address = Sdi12Addr::new(addr_char).map_err(|_| ResponseParseError::InvalidAddressChar)?;
    let remaining = &data[1..]; // Data after address

    // Handle simple cases first
    if remaining.is_empty() {
        // Could be Acknowledge, ServiceRequest, or Aborted (without CRC)
        // For now, let's tentatively parse as Acknowledge. The recorder state machine
        // needs to interpret it based on context (e.g., was a Dn command just sent?).
        // A dedicated Aborted variant helps parser distinguish if needed later.
         if buffer == &[addr_char as u8, b'\r', b'\n'] { // Check original buffer length
            return Ok(Response::Acknowledge { address });
            // Or contextually: Ok(Response::ServiceRequest { address });
            // Or contextually: Ok(Response::Aborted { address, crc: None });
         } else {
             return Err(ResponseParseError::InvalidFormat); // Should not happen if trim_cr_lf worked
         }
    }

    // Try to detect and handle potential ASCII CRC (3 bytes at the end)
    let (payload, crc_val) = if data.len() >= 4 && (data[data.len()-3] & 0xC0 == 0x40) { // Heuristic: check if 3rd last byte looks like ASCII CRC start
        let crc_bytes = &data[data.len()-3..];
        let potential_payload = &data[..data.len()-3];
        // Need to be careful: Data itself could end in something resembling a CRC.
        // Robust parsing might require knowing *if* CRC was *expected*.
        // Let's assume for now if it looks like CRC, we parse it.
        match crc::verify_response_crc_ascii::<()>(data) { // Verify entire `data` part
            Ok(()) => {
                // CRC is valid and matches the payload part
                let crc = crc::decode_crc_ascii(crc_bytes); // Decode again to store
                (potential_payload, Some(crc)) // Payload is data before CRC
            },
            Err(Sdi12Error::CrcMismatch{..}) => return Err(ResponseParseError::CrcMismatch),
            Err(_) => (data, None), // If verify failed for other reason (format), assume no CRC
        }
    } else {
        (data, None) // Assume no CRC
    };

    // Re-extract address and remaining payload after CRC check
    let address = Sdi12Addr::new(payload[0] as char).map_err(|_| ResponseParseError::InvalidAddressChar)?; // Should be same address
    let remaining = &payload[1..]; // Data after address, before potential CRC

     // Now parse based on remaining content
     if remaining.is_empty() && crc_val.is_some() {
         // This is `a<CRC><CR><LF>`, likely an Aborted response
         return Ok(Response::Aborted { address, crc: crc_val });
     }

    match remaining.get(0) {
        // Identification: aI... -> remaining starts with 'I' (or version digits '1'?)
        // Spec V1.4 examples all show `a{ll}{vendor}{model}{version}[opt]`
        // ll = version (e.g. "14"), vendor=8, model=6, version=3, opt=0-13
        Some(&c) if c.is_ascii_digit() && remaining.len() >= (2 + 8 + 6 + 3) => {
            #[cfg(feature = "alloc")]
            {
                let version_str = core::str::from_utf8(&remaining[0..2]).map_err(|_| ResponseParseError::InvalidVersionFormat)?;
                let sdi_version = u8::from_str(version_str).map_err(|_| ResponseParseError::InvalidVersionFormat)?;

                let vendor_end = 2 + 8;
                let model_end = vendor_end + 6;
                let sens_ver_end = model_end + 3;

                if remaining.len() < sens_ver_end { return Err(ResponseParseError::InvalidIdentificationLength); }

                let vendor = String::from_utf8(remaining[2..vendor_end].to_vec()).map_err(|_| ResponseParseError::InvalidFormat)?;
                let model = String::from_utf8(remaining[vendor_end..model_end].to_vec()).map_err(|_| ResponseParseError::InvalidFormat)?;
                let version = String::from_utf8(remaining[model_end..sens_ver_end].to_vec()).map_err(|_| ResponseParseError::InvalidFormat)?;

                let optional = if remaining.len() > sens_ver_end {
                    Some(String::from_utf8(remaining[sens_ver_end..].to_vec()).map_err(|_| ResponseParseError::InvalidFormat)?)
                } else {
                    None
                };

                 Ok(Response::Identification(IdentificationInfo { address, sdi_version, vendor, model, version, optional }))
            }
            #[cfg(not(feature = "alloc"))]
            { Err(ResponseParseError::FeatureNotEnabled) }
        }

        // Measurement Timing: atttn[nn] -> remaining is 4, 5, or 6 digits
        Some(&c) if c.is_ascii_digit() && (remaining.len() == 4 || remaining.len() == 5 || remaining.len() == 6) => {
            let time_str = core::str::from_utf8(&remaining[0..3]).map_err(|_| ResponseParseError::NumericError)?;
            let count_str = core::str::from_utf8(&remaining[3..]).map_err(|_| ResponseParseError::NumericError)?;
            let time_seconds = u16::from_str(time_str).map_err(|_| ResponseParseError::NumericError)?;
            let values_count = u16::from_str(count_str).map_err(|_| ResponseParseError::NumericError)?;
             Ok(Response::MeasurementTiming(MeasurementTiming { address, time_seconds, values_count }))
        }

        // Metadata: a,field1,field2; -> starts with ',' and ends with ';'
        Some(b',') if remaining.ends_with(b";") => {
             #[cfg(feature = "alloc")]
             {
                let fields_str = core::str::from_utf8(&remaining[1..remaining.len()-1]).map_err(|_| ResponseParseError::InvalidFormat)?;
                let fields = fields_str.split(',').map(|s| s.to_string()).collect();
                 Ok(Response::Metadata(MetadataInfo { address, fields, crc: crc_val }))
             }
             #[cfg(not(feature = "alloc"))]
             { Err(ResponseParseError::FeatureNotEnabled) }
        }

        // Data: a+... or a-... -> starts with '+' or '-'
        Some(b'+') | Some(b'-') => {
             #[cfg(feature = "alloc")]
             {
                 // Need to split the `remaining` string by sign characters, respecting that
                 // a sign is *part* of the value.
                 let mut values = Vec::new();
                 let mut current_start = 0;
                 for (i, c) in remaining.iter().enumerate().skip(1) { // Skip first sign
                     if (*c == b'+' || *c == b'-') && i > current_start {
                         // Found start of next value, parse previous one
                         let value_str = core::str::from_utf8(&remaining[current_start..i]).map_err(|_| ResponseParseError::InvalidFormat)?;
                         values.push(Sdi12Value::parse_single(value_str)?);
                         current_start = i;
                     }
                 }
                 // Parse the last value
                 let value_str = core::str::from_utf8(&remaining[current_start..]).map_err(|_| ResponseParseError::InvalidFormat)?;
                 values.push(Sdi12Value::parse_single(value_str)?);

                 Ok(Response::Data(DataInfo { address, values, crc: crc_val }))
             }
             #[cfg(not(feature = "alloc"))]
             { Err(ResponseParseError::FeatureNotEnabled) }
        }

        // Fallback/Unknown
        _ => Err(ResponseParseError::InvalidFormat),
    }

    // Note: Address response ('b<CR><LF>') isn't handled here because 'b' is a valid address.
    // The recorder logic would likely check if the received address matches the expected *new* address.
    // The parser returns `Response::Acknowledge` with address 'b'.
}

/// Parses a high-volume binary packet (which does NOT end in <CR><LF>).
pub fn parse_binary_packet(buffer: &[u8]) -> Result<Response, ResponseParseError> {
    #[cfg(feature = "alloc")]
    {
        if buffer.len() < 6 { // Min len: Addr(1) + Size(2) + Type(1) + CRC(2)
            return Err(ResponseParseError::TooShort);
        }

        // Verify CRC first
        crc::verify_packet_crc_binary::<()>(buffer) // Use () for generic E in verify
            .map_err(|e| match e {
                Sdi12Error::CrcMismatch{..} => ResponseParseError::CrcMismatch,
                _ => ResponseParseError::InvalidFormat // Should only be CRC or Format error here
            })?;

        // Extract fields (assuming CRC is okay)
        let addr_char = buffer[0] as char;
        let address = Sdi12Addr::new(addr_char).map_err(|_| ResponseParseError::InvalidAddressChar)?;

        let packet_size = u16::from_le_bytes([buffer[1], buffer[2]]);
        let type_byte = buffer[3];
        let data_type = BinaryDataType::from_u8(type_byte).ok_or(ResponseParseError::InvalidBinaryDataType)?;

        let payload_end_index = 1 + 2 + 1 + (packet_size as usize); // Index after payload
        let crc_index = buffer.len() - 2;

        // Validate lengths
        // The payload should end exactly where the CRC begins
        if payload_end_index != crc_index || packet_size > 1000 {
            return Err(ResponseParseError::InconsistentBinaryPacketSize);
        }

        let payload = buffer[4..payload_end_index].to_vec();
        let crc = u16::from_le_bytes([buffer[crc_index], buffer[crc_index + 1]]);

        Ok(Response::BinaryData(BinaryDataInfo {
            address,
            packet_size,
            data_type,
            payload,
            crc,
        }))
    }
    #[cfg(not(feature = "alloc"))]
    {
        let _ = buffer; // Avoid unused warning
        Err(ResponseParseError::FeatureNotEnabled)
    }
}


// --- Unit Tests (Basic Structure Checks and Parsing Placeholders) ---
#[cfg(test)]
mod tests {
    // TODO: Add extensive tests for parsing logic once implemented.
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
        let _ad = Response::Address { address: addr }; // Address is the NEW address 'b'
         let _mt = Response::MeasurementTiming(MeasurementTiming { address: addr, time_seconds: 10, values_count: 5 });
         let _d = Response::Data(DataInfo { address: addr, values: vec![Sdi12Value::new(1.0)], crc: None });
         let _bd = Response::BinaryData(BinaryDataInfo {
            address: addr, packet_size: 1, data_type: BinaryDataType::UnsignedU8, payload: vec![1], crc: 0x1234
         });
         let _md = Response::Metadata(MetadataInfo { address: addr, fields: vec!["f1".into()], crc: None });
         let _ab = Response::Aborted{ address: addr, crc: None };
    }

    #[test]
    fn test_parsing_placeholder() {
        // Replace with real parsing tests later
        use super::*;
        assert!(matches!(parse_response(b"1\r\n"), Ok(Response::Acknowledge { address }) if address.as_char() == '1'));
        assert!(matches!(parse_response(b"1+12.3\r\n"), Err(ResponseParseError::FeatureNotEnabled))); // Requires alloc
        assert!(matches!(parse_binary_packet(b""), Err(ResponseParseError::FeatureNotEnabled))); // Requires alloc
    }

     #[test]
    fn test_trim_cr_lf() {
        use super::trim_cr_lf;
        assert_eq!(trim_cr_lf(b"123\r\n"), Some(&b"123"[..]));
        assert_eq!(trim_cr_lf(b"\r\n"), Some(&b""[..]));
        assert_eq!(trim_cr_lf(b"123"), None);
        assert_eq!(trim_cr_lf(b"123\n"), None);
        assert_eq!(trim_cr_lf(b"123\r"), None);
         assert_eq!(trim_cr_lf(b""), None);
         assert_eq!(trim_cr_lf(b"\r"), None);
    }

}