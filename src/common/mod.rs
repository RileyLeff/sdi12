// src/common/mod.rs

// --- Declare all public modules within common ---
pub mod address;
pub mod command;
pub mod crc;
pub mod error;
pub mod frame;
pub mod hal_traits;
pub mod response; // The new sub-module
pub mod timing;
pub mod types;

// --- Re-export key types/traits/functions for easier access ---

// From address.rs
pub use address::Sdi12Addr;

// From command.rs
pub use command::{
    Command, CommandIndexError, MeasurementIndex, ContinuousIndex, DataIndex, IdentifyParameterIndex,
    IdentifyMeasurementCommand, IdentifyMeasurementParameterCommand,
};

// From crc.rs
pub use crc::{
    calculate_crc16, encode_crc_ascii, decode_crc_ascii, verify_response_crc_ascii,
    encode_crc_binary, decode_crc_binary, verify_packet_crc_binary,
};

// From error.rs
pub use error::Sdi12Error;

// From frame.rs
pub use frame::FrameFormat;

// From hal_traits.rs
pub use hal_traits::{Sdi12Serial, Sdi12Timer}; // Core sync traits

// From response.rs (Simplified re-exports)
pub use response::{
    ResponseParseError, // The error enum for frame/crc/address issues
    MeasurementTiming,  // The struct for specifically parsed timing responses
    PayloadSlice,       // The wrapper for returned raw payloads
};


// From timing.rs (constants - users can access via common::timing::*)
// No re-exports by default unless specifically desired, e.g.:
// pub use timing::BREAK_DURATION_MIN;

// From types.rs
pub use types::{BinaryDataType, Sdi12ParsingError, Sdi12Value};


// --- Feature-gated re-exports ---

// Async traits (from hal_traits.rs)
#[cfg(feature = "async")]
pub use hal_traits::Sdi12SerialAsync;

// Native HAL integration traits (from hal_traits.rs)
#[cfg(feature = "impl-native")]
pub use hal_traits::NativeSdi12Uart;
#[cfg(all(feature = "async", feature = "impl-native"))]
pub use hal_traits::NativeSdi12UartAsync;