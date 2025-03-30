// src/recorder/sync_recorder/io_helpers.rs

use super::SyncRecorder; // Access SyncRecorder definition
use crate::common::{
    error::Sdi12Error,
    hal_traits::{Sdi12Instant, Sdi12Serial, Sdi12Timer},
    timing, FrameFormat,
};
use core::fmt::Debug;
use core::ops::Sub;
use core::time::Duration;
use nb::Result as NbResult;

// Implementation block for I/O related helpers
impl<IF> SyncRecorder<IF>
where
    IF: Sdi12Serial + Sdi12Timer,
    IF::Error: Debug,
    IF::Instant: Sdi12Instant,
{
    /// Executes a non-blocking I/O operation (`f`) repeatedly until it
    /// stops returning `WouldBlock`, returning the final result or a timeout error.
    pub(super) fn execute_blocking_io_with_timeout<FN, T>( // Make pub(super)
        &mut self,
        timeout: Duration,
        mut f: FN,
    ) -> Result<T, Sdi12Error<IF::Error>>
    where
        FN: FnMut(&mut IF) -> NbResult<T, IF::Error>,
    {
        let start_time = self.interface.now();
        let deadline = start_time + timeout;

        loop {
            match f(&mut self.interface) {
                Ok(result) => return Ok(result),
                Err(nb::Error::WouldBlock) => {
                    if self.interface.now() >= deadline {
                        return Err(Sdi12Error::Timeout);
                    }
                    // Optional delay - small delay might prevent busy-spinning 100% CPU
                    self.interface.delay_us(100); // e.g., 100us delay
                }
                Err(nb::Error::Other(e)) => return Err(Sdi12Error::Io(e)),
            }
        }
    }

     /// Checks timing state and sends a break if necessary.
     pub(super) fn check_and_send_break(&mut self) -> Result<(), Sdi12Error<IF::Error>> { // Make pub(super)
        let now = self.interface.now();
        let mut break_needed = true;

        if let Some(last_time) = self.last_activity_time {
            let elapsed = now.sub(last_time);
            if elapsed <= timing::PRE_COMMAND_BREAK_MARKING_THRESHOLD {
                break_needed = false;
            }
        }

        if break_needed {
            let break_timeout = timing::BREAK_DURATION_MIN + Duration::from_millis(5);
            self.execute_blocking_io_with_timeout(break_timeout, |iface| iface.send_break())?;
            self.interface.delay_us(timing::POST_BREAK_MARKING_MIN.as_micros() as u32);
            // Update time *after* break sequence completes successfully
            self.last_activity_time = Some(self.interface.now());
        }

        Ok(())
    }

    /// Sends the already formatted command bytes over the serial interface.
    pub(super) fn send_command_bytes(&mut self, cmd_bytes: &[u8]) -> Result<(), Sdi12Error<IF::Error>> { // Make pub(super)
        self.interface
            .set_config(FrameFormat::Sdi12_7e1)
            .map_err(Sdi12Error::Io)?;

        let write_duration = timing::BYTE_DURATION * cmd_bytes.len() as u32;
        let write_timeout = write_duration + Duration::from_millis(20); // 20ms buffer

        for byte in cmd_bytes {
            self.execute_blocking_io_with_timeout(write_timeout, |iface| {
                iface.write_byte(*byte)
            })?;
        }

        let flush_timeout = Duration::from_millis(10);
        self.execute_blocking_io_with_timeout(flush_timeout, |iface| iface.flush())?;

        // NOTE: Do not update last_activity_time here. Update only after successful response.
        Ok(())
    }

     /// Reads a complete response line (up to <CR><LF>) into the buffer.
     pub(super) fn read_response_line<'buf>( // Make pub(super)
        &mut self,
        buffer: &'buf mut [u8],
    ) -> Result<&'buf [u8], Sdi12Error<IF::Error>> {
        // Calculate timeout: Response start time + time for max standard response length
        let max_resp_len = 96; // Generous buffer
        let read_allowance = timing::BYTE_DURATION * max_resp_len;
        let read_timeout = timing::RESPONSE_START_TIME_MAX + read_allowance + Duration::from_millis(50);

        let mut bytes_read = 0;
        loop {
            if bytes_read >= buffer.len() {
                return Err(Sdi12Error::BufferOverflow {
                    needed: bytes_read + 1,
                    got: buffer.len(),
                });
            }

            // Define a shorter timeout for subsequent bytes once the first byte arrived
            let current_timeout = if bytes_read == 0 {
                read_timeout
            } else {
                 // Timeout based on inter-character spacing + buffer
                timing::INTER_CHARACTER_MARKING_MAX + Duration::from_millis(5)
            };

            match self.execute_blocking_io_with_timeout(current_timeout, |iface| iface.read_byte()) {
                Ok(byte) => {
                    buffer[bytes_read] = byte;
                    bytes_read += 1;

                    // Check for <CR><LF>
                    if bytes_read >= 2
                        && buffer[bytes_read - 2] == b'\r'
                        && buffer[bytes_read - 1] == b'\n'
                    {
                        return Ok(&buffer[..bytes_read]);
                    }
                }
                Err(Sdi12Error::Timeout) => {
                    if bytes_read > 0 {
                        // Received some bytes but didn't get CRLF in time
                        return Err(Sdi12Error::InvalidFormat);
                    } else {
                        // Timed out waiting for the first byte
                        return Err(Sdi12Error::Timeout);
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
}
// src/recorder/sync_recorder/io_helpers.rs
// ... (main code) ...

// --- Unit Tests for IO Helpers ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{
        address::Sdi12Addr, // Keep needed imports for tests
        command::Command,
        hal_traits::{Sdi12Serial, Sdi12Timer}, // Remove Sdi12Instant from here
        FrameFormat, Sdi12Error,
    };
    use core::time::Duration;
    use nb::Result as NbResult; // FIX: Added import for tests
    use nb; // Keep nb for errors like nb::Error::WouldBlock

    // --- Mock Instant ---
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct MockInstant(u64);
    impl core::ops::Add<Duration> for MockInstant { /* ... */
        type Output = Self;
        fn add(self, rhs: Duration) -> Self {
            MockInstant(self.0.saturating_add(rhs.as_micros() as u64))
        }
     }
    impl core::ops::Sub<MockInstant> for MockInstant { /* ... */
        type Output = Duration;
        fn sub(self, rhs: MockInstant) -> Duration {
            Duration::from_micros(self.0.saturating_sub(rhs.0))
        }
    }
    // --- Mock Comm Error ---
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    struct MockCommError;
    // --- Mock Interface ---
    #[derive(Clone)]
    struct MockInterface { /* ... */
        break_sent: bool,
        pub config: FrameFormat,
        current_time_us: u64,
        read_queue: [Option<u8>; 96],
        read_pos: usize,
        write_log: [Option<u8>; 96],
        write_pos: usize,
        #[cfg(feature = "std")]
        io_call_counts: std::collections::HashMap<&'static str, u32>,
        #[cfg(not(feature = "std"))]
        _marker: core::marker::PhantomData<&'static str>,
    }
     impl MockInterface { /* ... */
        fn new() -> Self {
             MockInterface {
                break_sent: false,
                config: FrameFormat::Sdi12_7e1,
                current_time_us: 0,
                read_queue: [None; 96],
                read_pos: 0,
                write_log: [None; 96],
                write_pos: 0,
                 #[cfg(feature = "std")]
                 io_call_counts: std::collections::HashMap::new(),
                 #[cfg(not(feature = "std"))]
                 _marker: core::marker::PhantomData,
            }
        }
        fn advance_time(&mut self, us: u64) {
             self.current_time_us = self.current_time_us.saturating_add(us);
        }
         #[cfg(feature = "std")]
        fn increment_call_count(&mut self, name: &'static str) {
            *self.io_call_counts.entry(name).or_insert(0) += 1;
        }
        #[cfg(not(feature = "std"))]
        fn increment_call_count(&mut self, _name: &'static str) {}
         #[cfg(feature = "std")]
        fn get_call_count(&self, name: &'static str) -> u32 {
             *self.io_call_counts.get(name).unwrap_or(&0)
        }
         #[cfg(not(feature = "std"))]
        fn get_call_count(&self, _name: &'static str) -> u32 { 0 }
        fn stage_read_data(&mut self, data: &[u8]) {
             self.read_pos = 0;
             self.read_queue = [None; 96];
             assert!(data.len() <= self.read_queue.len());
             for (i, byte) in data.iter().enumerate() {
                 self.read_queue[i] = Some(*byte);
             }
        }
     }
     impl Sdi12Timer for MockInterface { /* ... Use MockInstant ... */
        type Instant = MockInstant;
        fn delay_us(&mut self, us: u32) { self.advance_time(us as u64); }
        fn delay_ms(&mut self, ms: u32) { self.advance_time((ms as u64) * 1000); }
        fn now(&self) -> Self::Instant { MockInstant(self.current_time_us) }
     }
     impl Sdi12Serial for MockInterface { /* ... Use NbResult ... */
         type Error = MockCommError;
         fn read_byte(&mut self) -> NbResult<u8, Self::Error> { /* ... */
            self.increment_call_count("read_byte");
            if self.read_pos < self.read_queue.len() {
                if let Some(byte) = self.read_queue[self.read_pos] {
                    self.read_pos += 1;
                    Ok(byte)
                } else {
                     Err(nb::Error::WouldBlock)
                }
            } else {
                 Err(nb::Error::WouldBlock)
            }
         }
         fn write_byte(&mut self, byte: u8) -> NbResult<(), Self::Error> { /* ... */
            self.increment_call_count("write_byte");
             if self.write_pos < self.write_log.len() {
                 self.write_log[self.write_pos] = Some(byte);
                 self.write_pos += 1;
                 Ok(())
             } else {
                 Err(nb::Error::Other(MockCommError))
             }
         }
         fn flush(&mut self) -> NbResult<(), Self::Error> { self.increment_call_count("flush"); Ok(()) } // Uses NbResult
         fn send_break(&mut self) -> NbResult<(), Self::Error> { self.increment_call_count("send_break"); self.break_sent = true; Ok(()) } // Uses NbResult
         fn set_config(&mut self, config: FrameFormat) -> Result<(), Self::Error> { self.increment_call_count("set_config"); self.config = config; Ok(()) }
     }
     // Helper
     fn addr(c: char) -> Sdi12Addr { Sdi12Addr::new(c).unwrap() }

    // ... (All tests copied from previous mod.rs tests block) ...
    #[test]
    #[cfg(feature = "std")]
    fn test_execute_blocking_io_with_timeout() { /* ... as before ... */
        let mut mock_interface = MockInterface::new();
       let mut recorder = SyncRecorder::new(mock_interface);
       // Test Ok path
        let result_ok: Result<i32, _> = recorder.execute_blocking_io_with_timeout(
            Duration::from_millis(10),
            |iface| {
                let count = iface.get_call_count("timeout_ok");
                iface.increment_call_count("timeout_ok");
                iface.advance_time(1_000);
                if count < 3 { Err(nb::Error::WouldBlock) } else { Ok(123) }
            }
        );
        assert_eq!(result_ok, Ok(123));
        assert_eq!(recorder.interface.get_call_count("timeout_ok"), 4);
        assert_eq!(recorder.interface.current_time_us, 4_000);

        // Reset
        recorder.interface.current_time_us = 0;
        recorder.interface.io_call_counts.clear();

       // Test Timeout path
        let result_timeout: Result<(), _> = recorder.execute_blocking_io_with_timeout(
            Duration::from_millis(5),
            |iface| {
                iface.increment_call_count("timeout_err");
                 iface.advance_time(2_000);
                 Err(nb::Error::WouldBlock)
            }
        );
         assert!(matches!(result_timeout, Err(Sdi12Error::Timeout)));
         assert_eq!(recorder.interface.get_call_count("timeout_err"), 3);
         assert_eq!(recorder.interface.current_time_us, 6_000);

         // Reset
         recorder.interface.current_time_us = 0;
         recorder.interface.io_call_counts.clear();

        // Test IO Error path
         let result_io_err: Result<(), _> = recorder.execute_blocking_io_with_timeout(
             Duration::from_millis(10),
             |iface| {
                 let count = iface.get_call_count("timeout_io_err");
                 iface.increment_call_count("timeout_io_err");
                 iface.advance_time(1_000);
                 if count < 2 {
                     Err(nb::Error::WouldBlock)
                 } else {
                     Err(nb::Error::Other(MockCommError))
                 }
             }
         );
         assert!(matches!(result_io_err, Err(Sdi12Error::Io(MockCommError))));
         assert_eq!(recorder.interface.get_call_count("timeout_io_err"), 3);
         assert_eq!(recorder.interface.current_time_us, 3_000);
    }
    #[test]
    fn test_read_response_line_success() { /* ... as before ... */
         let mut mock_if = MockInterface::new();
        let data_to_read = b"1+12.3\r\n";
        mock_if.stage_read_data(data_to_read);
        let mut recorder = SyncRecorder::new(mock_if);
        let mut buffer = [0u8; 32];

        let result = recorder.read_response_line(&mut buffer);
        assert!(result.is_ok());
        let line_slice = result.unwrap();
        let len = line_slice.len();
        assert_eq!(line_slice, data_to_read);
        assert_eq!(len, data_to_read.len());
        assert_eq!(&buffer[..len], data_to_read);
    }
    #[test]
    fn test_read_response_line_timeout_no_data() { /* ... as before ... */
         let mock_if = MockInterface::new();
         let mut recorder = SyncRecorder::new(mock_if);
         let mut buffer = [0u8; 32];
         let result = recorder.read_response_line(&mut buffer);
         assert!(matches!(result, Err(Sdi12Error::Timeout)));
    }
    #[test]
    fn test_read_response_line_timeout_partial_data() { /* ... as before ... */
         let mut mock_if = MockInterface::new();
         mock_if.stage_read_data(b"1+12.3");
         let mut recorder = SyncRecorder::new(mock_if);
         let mut buffer = [0u8; 32];
         let result = recorder.read_response_line(&mut buffer);
         assert!(matches!(result, Err(Sdi12Error::InvalidFormat)));
    }
     #[test]
    fn test_read_response_line_buffer_overflow() { /* ... as before ... */
         let mut mock_if = MockInterface::new();
         mock_if.stage_read_data(b"1+12.345\r\n"); // 10 bytes
         let mut recorder = SyncRecorder::new(mock_if);
         let mut buffer = [0u8; 8]; // Buffer too small
         let result = recorder.read_response_line(&mut buffer);
         assert!(matches!(result, Err(Sdi12Error::BufferOverflow{needed: 9, got: 8})));
    }
    #[test]
    fn test_send_command_bytes_success() { /* ... as before ... */
        let mock_if = MockInterface::new();
        let mut recorder = SyncRecorder::new(mock_if.clone());
        let cmd_bytes = b"1M!";
        let result = recorder.send_command_bytes(cmd_bytes);

        assert!(result.is_ok());
        assert_eq!(recorder.interface.write_log[0], Some(b'1'));
        assert_eq!(recorder.interface.write_log[1], Some(b'M'));
        assert_eq!(recorder.interface.write_log[2], Some(b'!'));
        assert_eq!(recorder.interface.write_pos, 3);
        assert_eq!(recorder.interface.config, FrameFormat::Sdi12_7e1);
        #[cfg(feature="std")]
        assert_eq!(recorder.interface.get_call_count("flush"), 1);
    }
    #[test]
    fn test_check_and_send_break_needed() { /* ... as before ... */
         let mut mock_if = MockInterface::new();
        mock_if.current_time_us = 200_000;
        let mut recorder = SyncRecorder::new(mock_if);
        recorder.last_activity_time = Some(MockInstant(10_000));

        let result = recorder.check_and_send_break();
        assert!(result.is_ok());
        assert!(recorder.interface.break_sent);
        assert!(recorder.interface.current_time_us as u128 >= 200_000 + crate::common::timing::POST_BREAK_MARKING_MIN.as_micros());
        assert!(recorder.last_activity_time.is_some());
    }
     #[test]
    fn test_check_and_send_break_not_needed() { /* ... as before ... */
        let mut mock_if = MockInterface::new();
        mock_if.current_time_us = 50_000;
        let mut recorder = SyncRecorder::new(mock_if);
         recorder.last_activity_time = Some(MockInstant(10_000));

        let result = recorder.check_and_send_break();
        assert!(result.is_ok());
        assert!(!recorder.interface.break_sent);
        assert_eq!(recorder.interface.current_time_us, 50_000);
        assert_eq!(recorder.last_activity_time, Some(MockInstant(10_000)));
    }
}