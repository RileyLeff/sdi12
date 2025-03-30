// src/recorder/sync_recorder/transaction.rs

use super::SyncRecorder;
use crate::common::{
    command::Command,
    error::Sdi12Error,
    hal_traits::{Sdi12Instant, Sdi12Serial, Sdi12Timer},
    timing, // Needed for retry timing
};
use core::fmt::Debug;
use core::time::Duration; // Needed for retry timing

// Define retry constant
const MAX_TRANSACTION_RETRIES: usize = 3;

impl<IF> SyncRecorder<IF>
where
    IF: Sdi12Serial + Sdi12Timer,
    IF::Error: Debug,
    IF::Instant: Sdi12Instant,
{
    /// Executes a full command-response transaction with retries.
    /// Returns payload start/end indices on success.
    pub(super) fn execute_transaction<'buf>(
        &mut self,
        command: &Command,
        read_buffer: &'buf mut [u8], // Still takes buffer for reading into
    ) -> Result<(usize, usize), Sdi12Error<IF::Error>> { // Return indices

        // 1. Ensure break if needed
        self.check_and_send_break()?;

        // 2. Format command
        let command_buffer = command.format_into()
            .map_err(Sdi12Error::CommandFormatFailed)?;

        let mut last_error: Sdi12Error<IF::Error> = Sdi12Error::Timeout; // Default error if all retries fail

        // 3. Retry Loop
        for attempt in 0..MAX_TRANSACTION_RETRIES {
            // 4. Send Command
            if let Err(e) = self.send_command_bytes(command_buffer.as_bytes()) {
                 // Treat send errors as fatal for now
                 return Err(e);
            }

            // 5. Read Response
            match self.read_response_line(read_buffer) {
                Ok(line_slice) => {
                    // 5a. Process Response Payload
                    // Pass the received slice (which is part of read_buffer)
                    match self.process_response_payload(line_slice, command) {
                        Ok(indices) => { // Successful processing returns indices
                            // Success! Update time and return indices.
                            self.last_activity_time = Some(self.interface.now());
                            return Ok(indices);
                        }
                        // Treat parsing errors as non-retryable for now
                        Err(e @ Sdi12Error::CrcMismatch { .. }) => return Err(e),
                        Err(e @ Sdi12Error::InvalidFormat) => return Err(e),
                        Err(e @ Sdi12Error::UnexpectedResponse) => return Err(e),
                        Err(e @ Sdi12Error::InvalidAddress( _)) => return Err(e),
                        Err(e) => return Err(e), // Propagate other errors
                    }
                }
                // 5b. Handle Read Errors - Timeout/InvalidFormat are retryable
                Err(Sdi12Error::Timeout) => {
                    last_error = Sdi12Error::Timeout;
                    // Continue to retry logic below
                }
                Err(Sdi12Error::InvalidFormat) => { // Treat incomplete read as retryable
                    last_error = Sdi12Error::InvalidFormat;
                     // Continue to retry logic below
                }
                 // Any other error (like Io) is fatal
                Err(e) => return Err(e),
            }

            // 6. Retry Logic (if we didn't return Ok or a fatal Err above)
            if attempt + 1 < MAX_TRANSACTION_RETRIES {
                // Wait slightly more than RETRY_WAIT_MIN (16.67ms)
                self.interface.delay_ms(20);
            } else {
                 // Retries exhausted
                 break;
            }
        } // End retry loop

        // 7. Post-Loop: If we finished the loop, all retries failed
        Err(last_error)
    }
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
     use super::*;
     use crate::common::{
        address::Sdi12Addr,
        command::{Command, MeasurementIndex},
        hal_traits::{Sdi12Serial, Sdi12Timer},
        FrameFormat, Sdi12Error, timing,
        response::PayloadSlice,
    };
    use core::time::Duration;
    use nb::Result as NbResult;
    use nb;

    // --- Mocks ---
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct MockInstant(u64);
    impl core::ops::Add<Duration> for MockInstant { /* ... */ type Output = Self; fn add(self, rhs: Duration) -> Self { MockInstant(self.0.saturating_add(rhs.as_micros() as u64)) } }
    impl core::ops::Sub<MockInstant> for MockInstant { /* ... */ type Output = Duration; fn sub(self, rhs: MockInstant) -> Duration { Duration::from_micros(self.0.saturating_sub(rhs.0)) } }
    // MockCommError still needs Clone if Sdi12Error::Io(e) might be used in set_read_error
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    struct MockCommError;

    // REMOVE derive(Clone) from MockInterface
    struct MockInterface {
        break_sent: bool,
        config: FrameFormat,
        current_time_us: u64,
        read_queue: [Option<u8>; 96],
        read_pos: usize,
        write_log: [Option<u8>; 96],
        write_pos: usize,
        #[cfg(feature = "std")]
        io_call_counts: std::collections::HashMap<&'static str, u32>,
        #[cfg(not(feature = "std"))]
        _marker: core::marker::PhantomData<&'static str>,
        fail_read_after: Option<usize>,
        fail_write_after: Option<usize>,
        // Field type is fine, Sdi12Error itself doesn't need to be Clone
        read_error_type: Option<Sdi12Error<MockCommError>>,
    }
     impl MockInterface { /* ... new(), advance_time(), increment_call_count(), stage_read_data() ... */
         fn new() -> Self {
             MockInterface {
                break_sent: false, config: FrameFormat::Sdi12_7e1,
                current_time_us: 0, read_queue: [None; 96], read_pos: 0,
                write_log: [None; 96], write_pos: 0,
                 #[cfg(feature = "std")]
                 io_call_counts: std::collections::HashMap::new(),
                 #[cfg(not(feature = "std"))]
                 _marker: core::marker::PhantomData,
                 fail_read_after: None, fail_write_after: None, read_error_type: None,
            }
          }
          fn advance_time(&mut self, us: u64) { self.current_time_us = self.current_time_us.saturating_add(us); }
          #[cfg(feature = "std")]
          fn increment_call_count(&mut self, name: &'static str) { *self.io_call_counts.entry(name).or_insert(0) += 1; }
          #[cfg(not(feature = "std"))]
          fn increment_call_count(&mut self, _name: &'static str) {}
          fn stage_read_data(&mut self, data: &[u8]) {
            self.read_pos = 0;
             self.read_queue = [None; 96];
             assert!(data.len() <= self.read_queue.len());
             for (i, byte) in data.iter().enumerate() {
                 self.read_queue[i] = Some(*byte);
             }
           }
          fn set_fail_read_after(&mut self, count: usize) { self.fail_read_after = Some(count); }
          // Accept error by value, store it. MockCommError needs to be Clone if Io variant is used.
          fn set_read_error(&mut self, error: Sdi12Error<MockCommError>) { self.read_error_type = Some(error); }
     }
     impl Sdi12Timer for MockInterface { /* ... */
        type Instant = MockInstant;
        fn delay_us(&mut self, us: u32) { self.advance_time(us as u64); }
        fn delay_ms(&mut self, ms: u32) { self.advance_time((ms as u64) * 1000); }
        fn now(&self) -> Self::Instant { MockInstant(self.current_time_us) }
      }
     impl Sdi12Serial for MockInterface {
         type Error = MockCommError;
        fn read_byte(&mut self) -> NbResult<u8, Self::Error> {
            self.increment_call_count("read_byte");
            #[cfg(feature = "std")]
            let calls = self.io_call_counts.get("read_byte").copied().unwrap_or(0);
            #[cfg(not(feature = "std"))]
            let calls = 0;

            if let Some(fail_count) = self.fail_read_after {
                if calls > fail_count {
                    // REMOVE .cloned() - match on reference, copy error if needed
                    match self.read_error_type.as_ref().unwrap_or(&Sdi12Error::Timeout) {
                        Sdi12Error::Timeout => return Err(nb::Error::WouldBlock),
                        // Copy the MockCommError (it derives Copy)
                        Sdi12Error::Io(e) => return Err(nb::Error::Other(*e)),
                        _ => return Err(nb::Error::WouldBlock),
                    }
                }
            }
             if self.read_pos < self.read_queue.len() { if let Some(byte) = self.read_queue[self.read_pos] { self.read_pos += 1; Ok(byte) } else { Err(nb::Error::WouldBlock) } } else { Err(nb::Error::WouldBlock) }
         }
        fn write_byte(&mut self, byte: u8) -> NbResult<(), Self::Error> { /* ... */
             self.increment_call_count("write_byte");
             if self.write_pos < self.write_log.len() { self.write_log[self.write_pos] = Some(byte); self.write_pos += 1; Ok(()) } else { Err(nb::Error::Other(MockCommError)) }
         }
        fn flush(&mut self) -> NbResult<(), Self::Error> { self.increment_call_count("flush"); Ok(()) }
        fn send_break(&mut self) -> NbResult<(), Self::Error> { self.increment_call_count("send_break"); self.break_sent = true; Ok(()) }
        fn set_config(&mut self, config: FrameFormat) -> Result<(), Self::Error> { self.increment_call_count("set_config"); self.config = config; Ok(()) }
    }
    fn addr(c: char) -> Sdi12Addr { Sdi12Addr::new(c).unwrap() }

    #[test]
    fn test_transaction_success_no_retry() { /* ... Test remains the same ... */
         let mut mock_if = MockInterface::new();
         let ack_response = b"0\r\n";
         mock_if.stage_read_data(ack_response);
         let mut recorder = SyncRecorder::new(mock_if);
         let cmd = Command::AcknowledgeActive { address: addr('0') };
         let mut buffer = [0u8; 32];

         let result = recorder.execute_transaction(&cmd, &mut buffer);
         assert!(result.is_ok());
         let (start, end) = result.unwrap();
         assert_eq!(PayloadSlice(&buffer[start..end]).as_bytes(), b"");
         assert!(recorder.interface.break_sent);
         assert_eq!(recorder.interface.write_log[..2], [Some(b'0'), Some(b'!')]);
         assert!(recorder.last_activity_time.is_some());
    }

     #[test]
     #[cfg(feature = "std")]
     //#[ignore] // Keep ignored until timing logic is verified
     fn test_transaction_timeout_with_retries() { /* ... Test remains the same ... */
         let mut mock_if = MockInterface::new();
         mock_if.set_fail_read_after(0);
         // Use Timeout variant which doesn't involve cloning E
         mock_if.set_read_error(Sdi12Error::Timeout);

         let mut recorder = SyncRecorder::new(mock_if);
         let cmd = Command::AcknowledgeActive { address: addr('1') };
         let mut buffer = [0u8; 32];

         let start_time = recorder.interface.now();
         let result = recorder.execute_transaction(&cmd, &mut buffer);
         let end_time = recorder.interface.now();

         assert!(matches!(result, Err(Sdi12Error::Timeout)));

         let cmd_len = cmd.format_into().unwrap().len();
         assert_eq!(recorder.interface.io_call_counts.get("write_byte").unwrap_or(&0), &(cmd_len * MAX_TRANSACTION_RETRIES) as &u32);
         assert!(recorder.interface.io_call_counts.get("read_byte").unwrap_or(&0) > &(MAX_TRANSACTION_RETRIES as u32));

         let expected_min_delay = Duration::from_millis(20) * (MAX_TRANSACTION_RETRIES - 1) as u32;
         assert!(end_time.sub(start_time) >= expected_min_delay);
    }

    #[test]
    fn test_transaction_crc_error_no_retry() {
         let mut mock_if = MockInterface::new();
         let crc_error_response = b"0+12.3XXX\r\n";
         mock_if.stage_read_data(crc_error_response);
         // No longer need to clone mock_if
         let mut recorder = SyncRecorder::new(mock_if);
         let cmd = Command::StartMeasurementCRC { address: addr('0'), index: MeasurementIndex::Base };
         let mut buffer = [0u8; 32];
         let result = recorder.execute_transaction(&cmd, &mut buffer);
         assert!(matches!(result, Err(Sdi12Error::CrcMismatch{..})));

         // Access counts via recorder.interface directly
         #[cfg(feature = "std")]
         {
            assert_eq!(recorder.interface.io_call_counts.get("write_byte").unwrap_or(&0), &(cmd.format_into().unwrap().len()) as &u32);
         }
    }
}