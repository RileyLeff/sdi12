// src/recorder/sync_recorder/mod.rs

// Declare the implementation detail modules
mod io_helpers;
mod protocol_helpers;
mod transaction;

// Necessary imports for struct definition and public methods
use crate::common::{
    address::Sdi12Addr,
    command::Command,
    error::Sdi12Error,
    hal_traits::{Sdi12Instant, Sdi12Serial, Sdi12Timer},
    // response::PayloadSlice, // Not needed directly in this file anymore
};
use core::fmt::Debug;
// use core::time::Duration;

#[derive(Debug)]
pub struct SyncRecorder<IF>
where
    IF: Sdi12Serial + Sdi12Timer,
    IF::Error: Debug,
    IF::Instant: Sdi12Instant,
{
    interface: IF,
    last_activity_time: Option<IF::Instant>,
}

// Implementation block for constructor and public methods
impl<IF> SyncRecorder<IF>
where
    IF: Sdi12Serial + Sdi12Timer,
    IF::Error: Debug,
    IF::Instant: Sdi12Instant,
{
    pub fn new(interface: IF) -> Self {
        SyncRecorder {
            interface,
            last_activity_time: None,
        }
    }

    // --- Public Blocking Methods ---

    pub fn acknowledge(&mut self, address: Sdi12Addr) -> Result<(), Sdi12Error<IF::Error>> {
        let cmd = Command::AcknowledgeActive { address };
        const ACK_BUF_SIZE: usize = 96;
        let mut read_buffer = [0u8; ACK_BUF_SIZE];
        let (start, end) = self.execute_transaction(&cmd, &mut read_buffer)?;

        if start == end { Ok(()) } else { Err(Sdi12Error::InvalidFormat) }
    }

    /// Sends a pre-constructed SDI-12 command and returns the raw payload indices.
    ///
    /// This method allows sending any command supported by the `Command` enum,
    /// including basic, concurrent, high-volume, metadata, and (with 'alloc') extended commands.
    /// It handles break generation, command formatting, retries, response reading,
    /// address/CRC validation, and returns the start/end indices of the validated payload
    /// within the provided read buffer on success.
    ///
    /// # Arguments
    /// * `command`: The `sdi12::common::Command` to send.
    /// * `read_buffer`: A mutable byte slice to store the sensor's response line.
    ///                  A size of ~96 bytes is recommended for standard commands.
    ///
    /// # Returns
    /// * `Ok((usize, usize))` containing the start and end indices of the payload within `read_buffer`.
    /// * `Err(Sdi12Error)` on communication error, timeout, or invalid response framing/CRC.
    pub fn send_command<'buf>( // Add lifetime marker for read_buffer
        &mut self,
        command: &Command,
        read_buffer: &'buf mut [u8],
    ) -> Result<(usize, usize), Sdi12Error<IF::Error>> {
        // Directly use the core transaction logic defined in transaction.rs
        self.execute_transaction(command, read_buffer)
    }

    // TODO: Implement other specific public methods like send_identification etc.

} // End impl SyncRecorder

// --- Unit Tests ---
#[cfg(test)]
mod tests {
   // Minimal tests for construction remain. Tests for acknowledge/send_command
   // behavior are better placed with execute_transaction tests as they rely heavily on it.
    use super::*;
    use crate::common::address::Sdi12Addr; // Keep for potential future tests
    use crate::common::hal_traits::{Sdi12Instant, Sdi12Serial, Sdi12Timer};
    use crate::common::{FrameFormat, Sdi12Error};
    use core::time::Duration;
    use nb::Result as NbResult;
    use nb;

    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct MockInstant(u64);
    impl core::ops::Add<Duration> for MockInstant { type Output = Self; fn add(self, rhs: Duration) -> Self { MockInstant(self.0.saturating_add(rhs.as_micros() as u64)) } }
    impl core::ops::Sub<MockInstant> for MockInstant { type Output = Duration; fn sub(self, rhs: MockInstant) -> Duration { Duration::from_micros(self.0.saturating_sub(rhs.0)) } }
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    struct MockCommError;
    #[derive(Clone)]
    struct MockInterface;
    impl Sdi12Timer for MockInterface { type Instant = MockInstant; fn delay_us(&mut self, _us: u32) {} fn delay_ms(&mut self, _ms: u32) {} fn now(&self) -> Self::Instant { MockInstant(0) } }
    impl Sdi12Serial for MockInterface { type Error = MockCommError; fn read_byte(&mut self) -> NbResult<u8, Self::Error> { Err(nb::Error::WouldBlock) } fn write_byte(&mut self, _byte: u8) -> NbResult<(), Self::Error> { Ok(()) } fn flush(&mut self) -> NbResult<(), Self::Error> { Ok(()) } fn send_break(&mut self) -> NbResult<(), Self::Error> { Ok(()) } fn set_config(&mut self, _config: FrameFormat) -> Result<(), Self::Error> { Ok(()) } }

    #[test]
    fn test_recorder_construction_in_mod() {
        let mock_interface = MockInterface;
        let recorder = SyncRecorder::new(mock_interface);
        assert!(recorder.last_activity_time.is_none());
    }
}