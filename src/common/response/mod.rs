// src/common/response/mod.rs

// Define the main Response enum in the module's root
mod error;
mod identification;
mod timing;
mod data;
mod metadata;
pub mod parse; // Make parse functions public

// Re-export items for external use
pub use error::ResponseParseError;
pub use timing::MeasurementTiming;
// Re-export parse functions
pub use parse::{parse_response, parse_binary_packet};

// Conditionally re-export alloc-dependent structs
#[cfg(feature = "alloc")]
pub use identification::IdentificationInfo;
#[cfg(feature = "alloc")]
pub use data::{DataInfo, BinaryDataInfo};
#[cfg(feature = "alloc")]
pub use metadata::MetadataInfo;

// --- Response Enum Definition ---
use crate::common::address::Sdi12Addr;

/// Represents any valid, parsed response received from an SDI-12 sensor.
/// Includes the address of the sensor that sent the response.
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// Simple Acknowledge (`a<CR><LF>`) from `a!` or `?!`.
    Acknowledge { address: Sdi12Addr },
    /// Service Request (`a<CR><LF>`) sent autonomously by sensor.
    ServiceRequest { address: Sdi12Addr },
    /// Identification Information (`aII...<CR><LF>`) from `aI!`. Needs `alloc`.
    #[cfg(feature = "alloc")]
    Identification(IdentificationInfo),
    /// Address Confirmation (`b<CR><LF>`) from `aAb!`. Address is the *new* confirmed address.
    Address { address: Sdi12Addr },
    /// Timing information (`atttn[nn]<CR><LF>`) from M/C/V/HA/HB/Identify commands.
    MeasurementTiming(MeasurementTiming),
    /// Data values (`a<values>[<CRC>]<CR><LF>`) from D/R commands. Needs `alloc`.
    #[cfg(feature = "alloc")]
    Data(DataInfo),
    /// Binary Data Packet (`Address PacketSize DataType Payload CRC`) from DB commands. Needs `alloc`.
    #[cfg(feature = "alloc")]
    BinaryData(BinaryDataInfo),
    /// Metadata Parameter Information (`a,field1,field2;[<CRC>]<CR><LF>`). Needs `alloc`.
    #[cfg(feature = "alloc")]
    Metadata(MetadataInfo),
    /// Sensor indicates aborted measurement (`a<CR><LF>` or `a<CRC><CR><LF>`).
    Aborted { address: Sdi12Addr, crc: Option<u16> },
}