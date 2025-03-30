// src/common/error.rs

#[cfg(feature = "alloc")]
use alloc::string::String;

// Import the specific command error types
use crate::common::command::{CommandFormatError, CommandIndexError}; // Added this line

// No more cfg_attr needed here, thiserror is always available
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Sdi12Error<E = ()>
where
    E: core::fmt::Debug, // Still need Debug for the generic Io error
{
    /// Underlying I/O error from the HAL implementation.
    #[error("I/O error: {0:?}")] // Format string requires Debug on E
    Io(E),

    /// Operation timed out.
    #[error("Operation timed out")]
    Timeout,

    /// Invalid character received where it's not allowed (e.g., non-printable ASCII).
    #[error("Invalid character received: {0:#04x}")]
    InvalidCharacter(u8),

    /// Provided address character is not a valid SDI-12 address.
    #[error("Invalid SDI-12 address character: '{0}'")]
    InvalidAddress(char),

    /// Received response format is invalid or unexpected.
    #[error("Invalid response format")]
    InvalidFormat,

    /// Buffer provided was too small.
    #[error("Buffer overflow: needed {needed}, got {got}")]
    BufferOverflow { needed: usize, got: usize },

    /// UART framing error detected by HAL.
    #[error("UART framing error")]
    Framing,

    /// UART parity error detected by HAL.
    #[error("UART parity error")]
    Parity,

    /// Received CRC does not match calculated CRC.
    #[error("CRC mismatch: expected {expected:#06x}, calculated {calculated:#06x}")]
    CrcMismatch { expected: u16, calculated: u16 },

    /// Got a validly formatted response, but not the one expected in the current state.
    #[error("Unexpected response received")]
    UnexpectedResponse, // Consider adding details later

    /// Bus contention detected (multiple devices responding simultaneously).
    #[error("Bus contention detected")]
    BusContention,

    /// Error related to command index validation.
    #[error("Invalid command index: {0}")] // Changed from CommandFormat
    InvalidCommandIndex(CommandIndexError), // Wrap CommandIndexError

    /// Error during command formatting.
    #[error("Command formatting failed: {0}")] // Changed from CommandFormat
    CommandFormatFailed(CommandFormatError), // Wrap CommandFormatError

    /// An error specific to the sensor's implementation/handler.
    /// Only available when the "alloc" feature is enabled.
    #[cfg(feature = "alloc")]
    #[error("Sensor specific error: {0}")] // String implements Display
    SensorSpecific(String),

    // Add other variants as needed...
}

// No manual Display impl needed - thiserror handles it.
// No manual std::error::Error impl needed - thiserror handles it when its 'std' feature is enabled.

// Allow mapping from underlying HAL error if From is implemented
impl<E: core::fmt::Debug> From<E> for Sdi12Error<E> {
    fn from(e: E) -> Self {
        Sdi12Error::Io(e)
    }
}

// Map command index errors into the main error type
impl<E: core::fmt::Debug> From<CommandIndexError> for Sdi12Error<E> {
    fn from(e: CommandIndexError) -> Self {
        Sdi12Error::InvalidCommandIndex(e)
    }
}

// Map command format errors into the main error type
impl<E: core::fmt::Debug> From<CommandFormatError> for Sdi12Error<E> {
    fn from(e: CommandFormatError) -> Self {
        Sdi12Error::CommandFormatFailed(e)
    }
}