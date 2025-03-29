// src/common/mod.rs

pub mod address;
pub mod crc;
pub mod error;
pub mod frame; // Add this
pub mod hal_traits; // Add this
pub mod timing; // Add this line
// Add other mods later: command, response, types, hal_traits

// Re-export common types for easier access
pub use address::Sdi12Addr;
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