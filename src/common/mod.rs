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

// From response/mod.rs (and its sub-modules via its own `pub use`)
pub use response::{
    Response, // Main enum from response/mod.rs
    ResponseParseError, // From response/error.rs
    MeasurementTiming,  // From response/timing.rs
    parse_response,     // From response/parse.rs
    parse_binary_packet // From response/parse.rs
};

// From timing.rs (constants - users can access via common::timing::*)
// No re-exports by default unless specifically desired, e.g.:
// pub use timing::BREAK_DURATION_MIN;

// From types.rs
pub use types::{BinaryDataType, Sdi12ParsingError, Sdi12Value};


// --- Feature-gated re-exports ---

// Alloc-dependent response types (from response sub-modules)
#[cfg(feature = "alloc")]
pub use response::{
    IdentificationInfo, // From response/identification.rs
    DataInfo,           // From response/data.rs
    BinaryDataInfo,     // From response/data.rs
    MetadataInfo,       // From response/metadata.rs
};

// Async traits (from hal_traits.rs)
#[cfg(feature = "async")]
pub use hal_traits::Sdi12SerialAsync;

// Native HAL integration traits (from hal_traits.rs)
#[cfg(feature = "impl-native")]
pub use hal_traits::NativeSdi12Uart;
#[cfg(all(feature = "async", feature = "impl-native"))]
pub use hal_traits::NativeSdi12UartAsync;