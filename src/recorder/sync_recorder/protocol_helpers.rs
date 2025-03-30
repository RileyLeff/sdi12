// src/recorder/sync_recorder/protocol_helpers.rs

use super::SyncRecorder;
use crate::common::{
    address::Sdi12Addr,
    command::{Command, IdentifyMeasurementParameterCommand}, // Import Command and sub-enums
    error::Sdi12Error,
    hal_traits::{Sdi12Instant, Sdi12Serial, Sdi12Timer},
    response::PayloadSlice, // Only needed for test helper function now
};
use core::fmt::Debug;

impl<IF> SyncRecorder<IF>
where
    IF: Sdi12Serial + Sdi12Timer,
    IF::Error: Debug,
    IF::Instant: Sdi12Instant,
{
    /// Parses the raw response line, checking address, CRC (if needed),
    /// and returns the start/end indices of the payload within the original line buffer.
    pub(super) fn process_response_payload(
        &mut self,
        response_line: &[u8],
        original_cmd: &Command,
    ) -> Result<(usize, usize), Sdi12Error<IF::Error>> { // Return (start, end) indices

        if response_line.len() < 3 { // Must have at least address + CR + LF
            return Err(Sdi12Error::InvalidFormat);
        }

        // 1. Check and strip <CR><LF>
        let crlf_len = 2;
        let data_end_idx = response_line.len() - crlf_len;
        if response_line[data_end_idx..] != [b'\r', b'\n'] {
            return Err(Sdi12Error::InvalidFormat); // Missing or incorrect terminator
        }
        let response_without_crlf = &response_line[..data_end_idx];

        if response_without_crlf.is_empty() { // Needs at least address
             return Err(Sdi12Error::InvalidFormat);
        }

        // 2. Check address
        let received_addr_char = response_without_crlf[0] as char;
        let expected_addr = match original_cmd {
             Command::AddressQuery => None, // Special case, accept any valid address
             _ => Some(original_cmd.address()),
        };

        let received_addr = Sdi12Addr::new(received_addr_char)
            .map_err(|_| Sdi12Error::InvalidAddress(received_addr_char))?; // Map error

        if let Some(expected) = expected_addr {
             if received_addr != expected {
                return Err(Sdi12Error::UnexpectedResponse);
             }
        }

        // 3. Determine payload boundaries and process CRC if needed
        let payload_start_index = 1; // Payload starts after the address byte
        let mut payload_end_index = response_without_crlf.len(); // End is before CRLF initially

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

        if crc_expected {
             // TODO: Handle binary CRC case differently if needed
            let crc_len = 3; // Assuming ASCII CRC
            if response_without_crlf.len() < payload_start_index + crc_len { // Need address + CRC
                return Err(Sdi12Error::InvalidFormat);
            }
            // CRC verification uses the slice *including* address but *excluding* CRLF
            crate::common::crc::verify_response_crc_ascii(response_without_crlf)
                 .map_err(|e| match e {
                     Sdi12Error::CrcMismatch{..} => e, // Pass through CRC error
                     _ => Sdi12Error::InvalidFormat,    // Other verification errors become InvalidFormat
                 })?;
             // Adjust payload end index to be before the CRC
             payload_end_index = response_without_crlf.len() - crc_len;
        }

        // Return the calculated indices relative to the start of the original response_line buffer
        Ok((payload_start_index, payload_end_index))
    }
}

// --- Unit Tests for Protocol Helpers ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{
        address::Sdi12Addr,
        command::{Command, DataIndex, MeasurementIndex},
        hal_traits::{Sdi12Serial, Sdi12Timer}, // Removed Sdi12Instant
        FrameFormat, Sdi12Error,
        response::PayloadSlice, // Keep for test helper
    };
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
    fn addr(c: char) -> Sdi12Addr { Sdi12Addr::new(c).unwrap() }

    // Helper to create PayloadSlice from indices and buffer for tests
    fn slice_from_indices<'a>(buffer: &'a [u8], start: usize, end: usize) -> PayloadSlice<'a> {
         PayloadSlice(&buffer[start..end])
    }

    #[test]
    fn test_process_response_payload_simple_ack() {
        let mock_if = MockInterface;
        let mut recorder = SyncRecorder::new(mock_if);
        let line = b"0\r\n";
        let cmd = Command::AcknowledgeActive{ address: addr('0') };
        let result = recorder.process_response_payload(line, &cmd);
        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!((start, end), (1, 1)); // Indices for empty payload
        assert_eq!(slice_from_indices(line, start, end).as_bytes(), b"");
    }
     #[test]
    fn test_process_response_payload_data_no_crc() {
        let mock_if = MockInterface;
        let mut recorder = SyncRecorder::new(mock_if);
        let line = b"1+12.3-45\r\n"; // 11 bytes total
        let cmd = Command::SendData{ address: addr('1'), index: DataIndex::new(0).unwrap() };
        let result = recorder.process_response_payload(line, &cmd);
         assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!(start, 1);
        assert_eq!(end, 9); // line length 11, minus 2 for CRLF = 9
        assert_eq!(slice_from_indices(line, start, end).as_bytes(), b"+12.3-45");
    }
    #[test]
    fn test_process_response_payload_data_with_crc_ok() {
         let mock_if = MockInterface;
        let mut recorder = SyncRecorder::new(mock_if);
        let line = b"0+3.14OqZ\r\n"; // 10 bytes total
        let cmd = Command::StartMeasurementCRC{ address: addr('0'), index: MeasurementIndex::Base };
        let result = recorder.process_response_payload(line, &cmd);
        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!(start, 1);
        assert_eq!(end, 6); // line len 10, minus 2 CRLF, minus 3 CRC = 5 -> end index is 1+5 = 6
        assert_eq!(slice_from_indices(line, start, end).as_bytes(), b"+3.14");
    }
     #[test]
    fn test_process_response_payload_data_with_crc_bad() {
        let mock_if = MockInterface;
        let mut recorder = SyncRecorder::new(mock_if);
        let line = b"0+3.14OqX\r\n"; // Bad CRC
        let cmd = Command::StartMeasurementCRC{ address: addr('0'), index: MeasurementIndex::Base };
        let result = recorder.process_response_payload(line, &cmd);
        assert!(matches!(result, Err(Sdi12Error::CrcMismatch { .. })));
    }
     #[test]
    fn test_process_response_payload_wrong_address() {
        let mock_if = MockInterface;
        let mut recorder = SyncRecorder::new(mock_if);
        let line = b"1+12.3\r\n";
        let cmd = Command::SendData{ address: addr('0'), index: DataIndex::new(0).unwrap() }; // Sent to 0
        let result = recorder.process_response_payload(line, &cmd);
        assert!(matches!(result, Err(Sdi12Error::UnexpectedResponse)));
    }
    #[test]
    fn test_process_response_payload_address_query() {
         let mock_if = MockInterface;
        let mut recorder = SyncRecorder::new(mock_if);
        let line = b"5\r\n"; // Response from sensor 5
        let cmd = Command::AddressQuery; // Query command
        let result = recorder.process_response_payload(line, &cmd);
        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!((start, end), (1, 1)); // Empty payload
        assert_eq!(slice_from_indices(line, start, end).as_bytes(), b"");
    }
     #[test]
    fn test_process_response_payload_invalid_format() {
        let mock_if = MockInterface;
        let mut recorder = SyncRecorder::new(mock_if);
        let cmd = Command::AcknowledgeActive{ address: addr('0') };

        assert!(matches!(recorder.process_response_payload(b"0", &cmd), Err(Sdi12Error::InvalidFormat))); // Too short
        assert!(matches!(recorder.process_response_payload(b"0\r", &cmd), Err(Sdi12Error::InvalidFormat))); // Too short
        assert!(matches!(recorder.process_response_payload(b"\r\n", &cmd), Err(Sdi12Error::InvalidFormat))); // No address
        // Check valid empty payload case
        let (start, end) = recorder.process_response_payload(b"0\r\n", &cmd).unwrap();
        assert_eq!((start, end), (1, 1));
    }
}