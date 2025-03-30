// src/recorder/sync_recorder/mod.rs

// Necessary imports from the common module and std/core
use crate::common::{
    address::Sdi12Addr,
    // Import Command AND the sub-enums needed for matching
    command::{Command, IdentifyMeasurementParameterCommand}, // Added IdentifyMeasurementParameterCommand
    error::Sdi12Error,
    // Sdi12Instant trait itself isn't needed directly here, but IF::Instant is
    hal_traits::{Sdi12Instant, Sdi12Serial, Sdi12Timer},
    response::{PayloadSlice, ResponseParseError}, // ResponseParseError warning is ok for now
    timing, FrameFormat,
};
use core::fmt::Debug;
use core::ops::Sub; // FIX: Added import for .sub()
use core::time::Duration;
use nb::Result as NbResult;

/// Represents an SDI-12 Recorder (Datalogger) instance for SYNCHRONOUS operations.
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
        let payload = self.execute_transaction(&cmd, &mut read_buffer)?;

        if payload.as_bytes().is_empty() {
            Ok(())
        } else {
            Err(Sdi12Error::InvalidFormat)
        }
    }

    // --- Core Transaction Logic (Private Helper) ---
    fn execute_transaction<'buf>(
        &mut self,
        command: &Command,
        read_buffer: &'buf mut [u8],
    ) -> Result<PayloadSlice<'buf>, Sdi12Error<IF::Error>> {
        self.check_and_send_break()?;
        // FIX: Map the CommandFormatError using map_err explicit conversion
        let command_buffer = command.format_into()
            .map_err(Sdi12Error::CommandFormatFailed)?;

        self.send_command_bytes(command_buffer.as_bytes())?;
        let line_slice = self.read_response_line(read_buffer)?;
        let payload = self.process_response_payload(line_slice, command)?;

        self.last_activity_time = Some(self.interface.now());

        Ok(payload)
    }

    // --- Low-Level I/O & Protocol Helpers (Private) ---
    fn check_and_send_break(&mut self) -> Result<(), Sdi12Error<IF::Error>> {
        let now = self.interface.now();
        let mut break_needed = true;

        if let Some(last_time) = self.last_activity_time {
            let elapsed = now.sub(last_time); // Now works with `use core::ops::Sub;`
            if elapsed <= timing::PRE_COMMAND_BREAK_MARKING_THRESHOLD {
                break_needed = false;
            }
        }

        if break_needed {
            let break_timeout = timing::BREAK_DURATION_MIN + Duration::from_millis(5);
            self.execute_blocking_io_with_timeout(break_timeout, |iface| iface.send_break())?;
            self.interface.delay_us(timing::POST_BREAK_MARKING_MIN.as_micros() as u32);
            self.last_activity_time = Some(self.interface.now());
        }

        Ok(())
    }

    fn send_command_bytes(&mut self, cmd_bytes: &[u8]) -> Result<(), Sdi12Error<IF::Error>> {
        self.interface
            .set_config(FrameFormat::Sdi12_7e1)
            .map_err(Sdi12Error::Io)?;

        let write_duration = timing::BYTE_DURATION * cmd_bytes.len() as u32;
        let write_timeout = write_duration + Duration::from_millis(20);

        for byte in cmd_bytes {
            self.execute_blocking_io_with_timeout(write_timeout, |iface| {
                iface.write_byte(*byte)
            })?;
        }

        let flush_timeout = Duration::from_millis(10);
        self.execute_blocking_io_with_timeout(flush_timeout, |iface| iface.flush())?;

        Ok(())
    }

    fn read_response_line<'buf>(
        &mut self,
        buffer: &'buf mut [u8],
    ) -> Result<&'buf [u8], Sdi12Error<IF::Error>> {
        let max_resp_len = 96;
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

            match self.execute_blocking_io_with_timeout(read_timeout, |iface| iface.read_byte()) {
                Ok(byte) => {
                    buffer[bytes_read] = byte;
                    bytes_read += 1;

                    if bytes_read >= 2
                        && buffer[bytes_read - 2] == b'\r'
                        && buffer[bytes_read - 1] == b'\n'
                    {
                        return Ok(&buffer[..bytes_read]);
                    }
                }
                Err(Sdi12Error::Timeout) => {
                    if bytes_read > 0 {
                        return Err(Sdi12Error::InvalidFormat);
                    } else {
                        return Err(Sdi12Error::Timeout);
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn process_response_payload<'buf>(
        &mut self,
        response_line: &'buf [u8],
        original_cmd: &Command,
    ) -> Result<PayloadSlice<'buf>, Sdi12Error<IF::Error>> {
        if response_line.len() < 3 {
            return Err(Sdi12Error::InvalidFormat);
        }

        let crlf_len = 2;
        let data_end_idx = response_line.len() - crlf_len;
        if response_line[data_end_idx..] != [b'\r', b'\n'] {
            return Err(Sdi12Error::InvalidFormat);
        }
        let response_without_crlf = &response_line[..data_end_idx];

        if response_without_crlf.is_empty() {
            return Err(Sdi12Error::InvalidFormat);
        }

        let received_addr_char = response_without_crlf[0] as char;
        let expected_addr = match original_cmd {
            Command::AddressQuery => None,
            _ => Some(original_cmd.address()), // Uses method re-added to Command
        };

        let received_addr = Sdi12Addr::new(received_addr_char)
            .map_err(|_| Sdi12Error::InvalidAddress(received_addr_char))?;

        if let Some(expected) = expected_addr {
            if received_addr != expected {
                return Err(Sdi12Error::UnexpectedResponse);
            }
        }

        let crc_expected = matches!(
            original_cmd,
            Command::StartMeasurementCRC { .. }
                | Command::StartConcurrentMeasurementCRC { .. }
                | Command::ReadContinuousCRC { .. }
                | Command::StartHighVolumeASCII { .. }
                | Command::StartHighVolumeBinary { .. }
                | Command::IdentifyMeasurementParameter(
                    IdentifyMeasurementParameterCommand::MeasurementCRC { .. }
                        | IdentifyMeasurementParameterCommand::ConcurrentMeasurementCRC { .. }
                        | IdentifyMeasurementParameterCommand::ReadContinuousCRC { .. }
                        | IdentifyMeasurementParameterCommand::HighVolumeASCII { .. }
                        | IdentifyMeasurementParameterCommand::HighVolumeBinary { .. }
                )
        );

        let payload_part = if crc_expected {
            let crc_len = 3;
            if response_without_crlf.len() < 1 + crc_len {
                return Err(Sdi12Error::InvalidFormat);
            }
            let data_part_end = response_without_crlf.len() - crc_len;
            let data_part = &response_without_crlf[..data_part_end];

            crate::common::crc::verify_response_crc_ascii(response_without_crlf)
                .map_err(|e| match e {
                    Sdi12Error::CrcMismatch { .. } => e,
                    _ => Sdi12Error::InvalidFormat,
                })?;

            &data_part[1..]

        } else {
            &response_without_crlf[1..]
        };

        Ok(PayloadSlice(payload_part))
    }

    // --- Timeout Helper ---
    fn execute_blocking_io_with_timeout<FN, T>(
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
                    self.interface.delay_us(100);
                }
                Err(nb::Error::Other(e)) => return Err(Sdi12Error::Io(e)),
            }
        }
    }
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    // FIX: Add imports for index types
    use crate::common::command::{DataIndex, MeasurementIndex};
    use crate::common::{
        address::Sdi12Addr,
        // FIX: Removed unused Sdi12Instant import
        hal_traits::{Sdi12Serial, Sdi12Timer},
        FrameFormat, Sdi12Error, Command,
    };
    use nb;
    use core::time::Duration;

    #[cfg(feature = "std")]
    use std::collections::HashMap;
    #[cfg(not(feature = "std"))]
    use core::marker::PhantomData;

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
    // FIX: Add derive(Clone)
    #[derive(Clone)]
    struct MockInterface {
        break_sent: bool,
        pub config: FrameFormat, // Make public for test access
        current_time_us: u64,
        read_queue: [Option<u8>; 96],
        read_pos: usize,
        write_log: [Option<u8>; 96],
        write_pos: usize,
        #[cfg(feature = "std")]
        io_call_counts: HashMap<&'static str, u32>,
        #[cfg(not(feature = "std"))]
        _marker: PhantomData<&'static str>,
    }
    impl MockInterface {
        fn new() -> Self { /* ... */
             MockInterface {
                break_sent: false,
                config: FrameFormat::Sdi12_7e1,
                current_time_us: 0,
                read_queue: [None; 96],
                read_pos: 0,
                write_log: [None; 96],
                write_pos: 0,
                 #[cfg(feature = "std")]
                 io_call_counts: HashMap::new(),
                 #[cfg(not(feature = "std"))]
                 _marker: PhantomData,
            }
        }
        fn advance_time(&mut self, us: u64) { /* ... */
             self.current_time_us = self.current_time_us.saturating_add(us);
        }
        #[cfg(feature = "std")]
        fn increment_call_count(&mut self, name: &'static str) { /* ... */
            *self.io_call_counts.entry(name).or_insert(0) += 1;
        }
        #[cfg(not(feature = "std"))]
        fn increment_call_count(&mut self, _name: &'static str) {}
        #[cfg(feature = "std")]
        fn get_call_count(&self, name: &'static str) -> u32 { /* ... */
             *self.io_call_counts.get(name).unwrap_or(&0)
        }
        #[cfg(not(feature = "std"))]
        fn get_call_count(&self, _name: &'static str) -> u32 { 0 }
        fn stage_read_data(&mut self, data: &[u8]) { /* ... */
             self.read_pos = 0;
             self.read_queue = [None; 96];
             assert!(data.len() <= self.read_queue.len());
             for (i, byte) in data.iter().enumerate() {
                 self.read_queue[i] = Some(*byte);
             }
        }
    }
    impl Sdi12Timer for MockInterface { /* ... */
        type Instant = MockInstant;
        fn delay_us(&mut self, us: u32) { self.advance_time(us as u64); }
        fn delay_ms(&mut self, ms: u32) { self.advance_time((ms as u64) * 1000); }
        fn now(&self) -> Self::Instant { MockInstant(self.current_time_us) }
    }
    impl Sdi12Serial for MockInterface { /* ... */
        type Error = MockCommError;
         fn read_byte(&mut self) -> NbResult<u8, Self::Error> {
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
        fn write_byte(&mut self, byte: u8) -> NbResult<(), Self::Error> {
            self.increment_call_count("write_byte");
             if self.write_pos < self.write_log.len() {
                 self.write_log[self.write_pos] = Some(byte);
                 self.write_pos += 1;
                 Ok(())
             } else {
                 Err(nb::Error::Other(MockCommError))
             }
        }
        fn flush(&mut self) -> NbResult<(), Self::Error> { self.increment_call_count("flush"); Ok(()) }
        fn send_break(&mut self) -> NbResult<(), Self::Error> { self.increment_call_count("send_break"); self.break_sent = true; Ok(()) }
        fn set_config(&mut self, config: FrameFormat) -> Result<(), Self::Error> { self.increment_call_count("set_config"); self.config = config; Ok(()) }
    }
    fn addr(c: char) -> Sdi12Addr { Sdi12Addr::new(c).unwrap() }

    #[test]
    fn test_recorder_construction() { /* ... */
        let mock_interface = MockInterface::new();
        let _recorder = SyncRecorder::new(mock_interface);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_execute_blocking_io_with_timeout() { /* ... */
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
        assert_eq!(recorder.interface.get_call_count("timeout_ok"), 4); // Use recorder.interface
        assert_eq!(recorder.interface.current_time_us, 4_000);

        // Reset (access mock via recorder.interface)
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
    fn test_read_response_line_success() {
        let mut mock_if = MockInterface::new();
        let data_to_read = b"1+12.3\r\n";
        mock_if.stage_read_data(data_to_read);
        let mut recorder = SyncRecorder::new(mock_if);
        let mut buffer = [0u8; 32];

        let result = recorder.read_response_line(&mut buffer);
        assert!(result.is_ok());
        let line_slice = result.unwrap();
        let len = line_slice.len(); // FIX E0502: Copy length
        assert_eq!(line_slice, data_to_read);
        assert_eq!(len, data_to_read.len());
        assert_eq!(&buffer[..len], data_to_read); // FIX E0502: Use copied length
    }

    #[test]
    fn test_read_response_line_timeout_no_data() {
        let mock_if = MockInterface::new();
        let mut recorder = SyncRecorder::new(mock_if);
        let mut buffer = [0u8; 32];
        let result = recorder.read_response_line(&mut buffer);
        assert!(matches!(result, Err(Sdi12Error::Timeout)));
    }

    #[test]
    fn test_read_response_line_timeout_partial_data() {
        let mut mock_if = MockInterface::new();
        mock_if.stage_read_data(b"1+12.3");
        let mut recorder = SyncRecorder::new(mock_if);
        let mut buffer = [0u8; 32];
        let result = recorder.read_response_line(&mut buffer);
        assert!(matches!(result, Err(Sdi12Error::InvalidFormat)));
    }

    #[test]
    fn test_read_response_line_buffer_overflow() {
        let mut mock_if = MockInterface::new();
        mock_if.stage_read_data(b"1+12.345\r\n");
        let mut recorder = SyncRecorder::new(mock_if);
        let mut buffer = [0u8; 8];
        let result = recorder.read_response_line(&mut buffer);
        assert!(matches!(result, Err(Sdi12Error::BufferOverflow { needed: 9, got: 8 })));
    }

    #[test]
    fn test_send_command_bytes_success() {
        let mock_if = MockInterface::new(); // FIX Warning: remove mut
        // FIX E0382: Clone the mock interface
        let mut recorder = SyncRecorder::new(mock_if.clone());
        let cmd_bytes = b"1M!";
        let result = recorder.send_command_bytes(cmd_bytes);

        assert!(result.is_ok());
        // Access interface via recorder
        assert_eq!(recorder.interface.write_log[0], Some(b'1'));
        assert_eq!(recorder.interface.write_log[1], Some(b'M'));
        assert_eq!(recorder.interface.write_log[2], Some(b'!'));
        assert_eq!(recorder.interface.write_pos, 3);
        // Check config was set via recorder
        assert_eq!(recorder.interface.config, FrameFormat::Sdi12_7e1); // FIX E0382: access via recorder.interface
        #[cfg(feature = "std")]
        assert_eq!(recorder.interface.get_call_count("flush"), 1);
    }

    #[test]
    fn test_check_and_send_break_needed() {
        let mut mock_if = MockInterface::new();
        mock_if.current_time_us = 200_000;
        let mut recorder = SyncRecorder::new(mock_if);
        recorder.last_activity_time = Some(MockInstant(10_000));

        let result = recorder.check_and_send_break();
        assert!(result.is_ok());
        assert!(recorder.interface.break_sent);
        // FIX E0308: Cast lhs to u128
        assert!(recorder.interface.current_time_us as u128 > 200_000 + timing::POST_BREAK_MARKING_MIN.as_micros());
        assert!(recorder.last_activity_time.is_some());
    }

    #[test]
    fn test_check_and_send_break_not_needed() {
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

    #[test]
    fn test_process_response_payload_simple_ack() {
        let mock_if = MockInterface::new(); // FIX Warning: remove mut
        let mut recorder = SyncRecorder::new(mock_if);
        let line = b"0\r\n";
        let cmd = Command::AcknowledgeActive { address: addr('0') };
        let result = recorder.process_response_payload(line, &cmd);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_bytes(), b"");
    }

    #[test]
    fn test_process_response_payload_data_no_crc() {
        let mock_if = MockInterface::new(); // FIX Warning: remove mut
        let mut recorder = SyncRecorder::new(mock_if);
        let line = b"1+12.3-45\r\n";
        let cmd = Command::SendData { address: addr('1'), index: DataIndex::new(0).unwrap() };
        let result = recorder.process_response_payload(line, &cmd);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_bytes(), b"+12.3-45");
    }

    #[test]
    fn test_process_response_payload_data_with_crc_ok() {
        let mock_if = MockInterface::new(); // FIX Warning: remove mut
        let mut recorder = SyncRecorder::new(mock_if);
        let line = b"0+3.14OqZ\r\n";
        let cmd = Command::StartMeasurementCRC { address: addr('0'), index: MeasurementIndex::Base };
        let result = recorder.process_response_payload(line, &cmd);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_bytes(), b"+3.14");
    }

    #[test]
    fn test_process_response_payload_data_with_crc_bad() {
        let mock_if = MockInterface::new(); // FIX Warning: remove mut
        let mut recorder = SyncRecorder::new(mock_if);
        let line = b"0+3.14OqX\r\n";
        let cmd = Command::StartMeasurementCRC { address: addr('0'), index: MeasurementIndex::Base };
        let result = recorder.process_response_payload(line, &cmd);
        assert!(matches!(result, Err(Sdi12Error::CrcMismatch { .. })));
    }

    #[test]
    fn test_process_response_payload_wrong_address() {
        let mock_if = MockInterface::new(); // FIX Warning: remove mut
        let mut recorder = SyncRecorder::new(mock_if);
        let line = b"1+12.3\r\n";
        let cmd = Command::SendData { address: addr('0'), index: DataIndex::new(0).unwrap() };
        let result = recorder.process_response_payload(line, &cmd);
        assert!(matches!(result, Err(Sdi12Error::UnexpectedResponse)));
    }

    #[test]
    fn test_process_response_payload_address_query() {
        let mock_if = MockInterface::new(); // FIX Warning: remove mut
        let mut recorder = SyncRecorder::new(mock_if);
        let line = b"5\r\n";
        let cmd = Command::AddressQuery;
        let result = recorder.process_response_payload(line, &cmd);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_bytes(), b"");
    }

    #[test]
    fn test_process_response_payload_invalid_format() {
        let mock_if = MockInterface::new(); // FIX Warning: remove mut
        let mut recorder = SyncRecorder::new(mock_if);
        let cmd = Command::AcknowledgeActive { address: addr('0') };

        assert!(matches!(recorder.process_response_payload(b"0", &cmd), Err(Sdi12Error::InvalidFormat)));
        assert!(matches!(recorder.process_response_payload(b"0\r", &cmd), Err(Sdi12Error::InvalidFormat)));
        assert!(matches!(recorder.process_response_payload(b"\r\n", &cmd), Err(Sdi12Error::InvalidFormat)));
        assert_eq!(recorder.process_response_payload(b"0\r\n", &cmd).unwrap().as_bytes(), b"");
    }

    #[test]
    #[ignore]
    fn test_acknowledge_end_to_end_mock() {
        let mut mock_if = MockInterface::new();
        mock_if.stage_read_data(b"0\r\n");
        let mut recorder = SyncRecorder::new(mock_if);
        let result = recorder.acknowledge(addr('0'));
        assert!(result.is_ok());
        assert!(recorder.interface.break_sent);
        assert_eq!(recorder.interface.write_log[0], Some(b'0'));
        assert_eq!(recorder.interface.write_log[1], Some(b'!'));
        assert_eq!(recorder.interface.write_pos, 2);
        assert!(recorder.last_activity_time.is_some());
    }
}