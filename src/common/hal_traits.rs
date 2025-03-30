// src/common/hal_traits.rs

use super::frame::FrameFormat;
use core::fmt::Debug;

// We need these traits potentially for the NativeSdi12Uart bounds
#[cfg(feature = "impl-native")]
use embedded_hal; // Use version 1.0
#[cfg(all(feature = "async", feature = "impl-native"))]
use embedded_hal_async; // Use version 1.0

/// Abstraction for timer/delay operations required by SDI-12.
///
/// Note: This could potentially be replaced by directly requiring
/// `embedded_hal::delay::DelayNs` if embedded-hal v1 is mandated.
pub trait Sdi12Timer {
    /// Delay for at least the specified number of microseconds.
    fn delay_us(&mut self, us: u32);

    /// Delay for at least the specified number of milliseconds.
    fn delay_ms(&mut self, ms: u32);
}

/// Abstraction for synchronous (non-blocking) SDI-12 serial communication.
pub trait Sdi12Serial {
    /// Associated error type for communication errors.
    /// Must implement Debug for error reporting.
    type Error: Debug;

    /// Attempts to read a single byte from the serial interface.
    ///
    /// Returns `Ok(byte)` if a byte was read, or `Err(nb::Error::WouldBlock)`
    /// if no byte is available yet. Other errors are returned as `Err(nb::Error::Other(Self::Error))`.
    fn read_byte(&mut self) -> nb::Result<u8, Self::Error>;

    /// Attempts to write a single byte to the serial interface.
    ///
    /// Returns `Ok(())` if the byte was accepted for transmission, or `Err(nb::Error::WouldBlock)`
    /// if the write buffer is full. Other errors are returned as `Err(nb::Error::Other(Self::Error))`.
    fn write_byte(&mut self, byte: u8) -> nb::Result<(), Self::Error>;

    /// Attempts to flush the transmit buffer, ensuring all written bytes have been sent.
    ///
    /// Returns `Ok(())` if the flush completed, or `Err(nb::Error::WouldBlock)` if
    /// transmission is still in progress. Other errors are returned as `Err(nb::Error::Other(Self::Error))`.
    fn flush(&mut self) -> nb::Result<(), Self::Error>;

    /// Sends the SDI-12 break condition (>= 12ms of spacing).
    ///
    /// Implementations must ensure the line is held low for the required duration.
    /// This might block or return `WouldBlock` depending on the implementation strategy.
    fn send_break(&mut self) -> nb::Result<(), Self::Error>;

    /// Changes the serial configuration (e.g., between 7E1 and 8N1).
    ///
    /// This operation might be blocking or complex, hence `Result` instead of `nb::Result`.
    /// Errors could occur if the hardware doesn't support the format or reconfiguration fails.
    fn set_config(&mut self, config: FrameFormat) -> Result<(), Self::Error>;
}

/// Abstraction for asynchronous SDI-12 serial communication (requires 'async' feature).
#[cfg(feature = "async")]
pub trait Sdi12SerialAsync {
    /// Associated error type for communication errors.
    /// Must implement Debug for error reporting.
    type Error: Debug;

    /// Asynchronously reads a single byte from the serial interface.
    async fn read_byte(&mut self) -> Result<u8, Self::Error>;

    /// Asynchronously writes a single byte to the serial interface.
    async fn write_byte(&mut self, byte: u8) -> Result<(), Self::Error>;

    /// Asynchronously flushes the transmit buffer.
    async fn flush(&mut self) -> Result<(), Self::Error>;

    /// Asynchronously sends the SDI-12 break condition.
    async fn send_break(&mut self) -> Result<(), Self::Error>;

    /// Asynchronously changes the serial configuration.
    async fn set_config(&mut self, config: FrameFormat) -> Result<(), Self::Error>;
}


/// Bundles standard embedded-hal serial traits with native SDI-12 specific operations.
///
/// Implement this trait for a HAL's UART peripheral if it provides native support
/// for sending break signals and changing configuration efficiently. Then, use the
/// `NativeAdapter` to make it compatible with `sdi12-rs`.
///
/// Requires `embedded-hal` v1.0 traits and is enabled by the `impl-native` feature.
#[cfg(feature = "impl-native")]
pub trait NativeSdi12Uart:
    embedded_hal::serial::ErrorType // Use fully qualified path
    // Specify Error association for dependent traits using qualified syntax
    + embedded_hal::serial::Read<u8, Error = <Self as embedded_hal::serial::ErrorType>::Error>
    + embedded_hal::serial::Write<u8, Error = <Self as embedded_hal::serial::ErrorType>::Error>
    + embedded_hal::serial::Flush<u8, Error = <Self as embedded_hal::serial::ErrorType>::Error>
    // Add Debug bound on the associated Error type for our own trait requirements
    + where <Self as embedded_hal::serial::ErrorType>::Error: Debug
{
    // Note: The associated Error type comes from embedded_hal::serial::ErrorType

    /// Sends the SDI-12 break condition using native hardware capabilities.
    fn native_send_break(&mut self) -> Result<(), Self::Error>;

    /// Changes the serial configuration using native hardware capabilities.
    fn native_set_config(&mut self, config: FrameFormat) -> Result<(), Self::Error>;
}

/// Async version of `NativeSdi12Uart`.
/// Requires `embedded-hal-async` traits and is enabled by the `async` and `impl-native` features.
#[cfg(all(feature = "async", feature = "impl-native"))]
pub trait NativeSdi12UartAsync:
    embedded_hal_async::serial::ErrorType // Use fully qualified path
    // Specify Error association for dependent traits using qualified syntax
    + embedded_hal_async::serial::Read<u8, Error = <Self as embedded_hal_async::serial::ErrorType>::Error>
    + embedded_hal_async::serial::Write<u8, Error = <Self as embedded_hal_async::serial::ErrorType>::Error>
    + embedded_hal_async::serial::Flush<u8, Error = <Self as embedded_hal_async::serial::ErrorType>::Error>
    // Add Debug bound on the associated Error type for our own trait requirements
    + where <Self as embedded_hal_async::serial::ErrorType>::Error: Debug
{
    // Note: The associated Error type comes from embedded_hal_async::serial::ErrorType

    /// Asynchronously sends the SDI-12 break condition using native hardware capabilities.
    async fn native_send_break(&mut self) -> Result<(), Self::Error>;

    /// Asynchronously changes the serial configuration using native hardware capabilities.
    async fn native_set_config(&mut self, config: FrameFormat) -> Result<(), Self::Error>;
}