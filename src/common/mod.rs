// src/common/mod.rs

pub mod address;
pub mod command;
pub mod crc;
pub mod error;
pub mod frame; // Add this
pub mod hal_traits; // Add this
pub mod timing; // Add this line
pub mod types; // Add this
// Add other mods later: command, response, types, hal_traits

// Re-export common types for easier access
pub use address::Sdi12Addr;
pub use command::Command; // Re-export main Command enum
// Optionally re-export sub-enums if needed directly by users
pub use command::{IdentifyMeasurementCommand, IdentifyMeasurementParameterCommand};
pub use crc::{
    calculate_crc16, encode_crc_ascii, decode_crc_ascii, verify_response_crc_ascii,
    encode_crc_binary, decode_crc_binary, verify_packet_crc_binary,
};
pub use error::Sdi12Error;
pub use frame::FrameFormat;
pub use hal_traits::{Sdi12Serial, Sdi12Timer};
#[cfg(feature = "async")]
pub use hal_traits::Sdi12SerialAsync;
#[cfg(feature = "impl-native")]
pub use hal_traits::NativeSdi12Uart;
#[cfg(all(feature = "async", feature = "impl-native"))]
pub use hal_traits::NativeSdi12UartAsync;
// Re-export timing constants if desired, or access via common::timing::*
// Example: pub use timing::BREAK_DURATION_MIN;
pub use types::{BinaryDataType, Sdi12ParsingError, Sdi12Value}; // Add types re-exports
#[cfg(feature = "async")]
pub use hal_traits::Sdi12SerialAsync;
#[cfg(feature = "impl-native")]
pub use hal_traits::NativeSdi12Uart;
#[cfg(all(feature = "async", feature = "impl-native"))]
pub use hal_traits::NativeSdi12UartAsync;