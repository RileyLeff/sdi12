//! SDI-12 command definitions.
//!
//! See SDI-12 Specification v1.4, Section 5.7 "Command Set".

use core::fmt;

use super::{address::Sdi12Addr, Sdi12Error};

/// Represents an SDI-12 command.
///
/// Note: The `Display` implementation for this enum generates the standard SDI-12 command string
/// format (e.g., `aM!`, `aD0!`, `aAn!`). Extended commands require separate handling for formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Acknowledge Active Command (`a!`) - Causes the addressed sensor to send an acknowledgment.
    AcknowledgeActive { address: Sdi12Addr },

    /// Change Address Command (`aAn!`) - Changes the sensor's address from `a` to `n`.
    ChangeAddress { address: Sdi12Addr, new_address: Sdi12Addr },

    /// Start Measurement Command (`aM!` or `aM1!`..`aM9!`) - Initiates a measurement sequence.
    /// `None` corresponds to `aM!`. `Some(i)` corresponds to `aMi!` (1 <= i <= 9).
    StartMeasurement { address: Sdi12Addr, measurement_index: Option<u8> },

    /// Start Concurrent Measurement Command (`aMC!` or `aMC1!`..`aMC9!`) - Initiates a concurrent measurement.
    /// `None` corresponds to `aMC!`. `Some(i)` corresponds to `aMCi!` (1 <= i <= 9).
    StartConcurrentMeasurement { address: Sdi12Addr, measurement_index: Option<u8> },

    /// Start Verification Command (`aV!`) - Initiates a verification sequence.
    StartVerification { address: Sdi12Addr },

    /// Send Data Command (`aD0!`..`aD9!`) - Requests data from a completed measurement.
    SendData { address: Sdi12Addr, data_index: u8 }, // Index 0-9

    /// Continuous Measurement Command (`aR0!`..`aR9!`) - Initiates continuous measurements.
    ContinuousMeasurement { address: Sdi12Addr, measurement_index: u8 }, // Index 0-9

    /// Identify Sensor Command (`aI!`) - Requests sensor identification information.
    IdentifySensor { address: Sdi12Addr },

    /// Represents an Extended Command (`aX...`).
    /// The specific format depends on the manufacturer and command.
    /// Formatting these requires custom logic.
    Extended(ExtendedCommand),
    // TODO: Potentially define common extended commands? (e.g., HUM?)
}

/// Represents the payload of an extended SDI-12 command.
/// SDI-12 spec does not define a universal format for these.
/// For now, it just holds the raw command bytes/string after the address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedCommand {
    /// The sensor address the command is directed to.
    pub address: Sdi12Addr,
    /// The command string *excluding* the initial address but *including* the final '!'.
    /// Example: For `0XHUM!`, `payload` would be `"XHUM!"`.
    // Using a simple array; adjust size if longer extended commands are common/needed.
    // Consider `heapless::String` if `alloc` feature is enabled and variable length is desired.
    pub payload: [u8; 16], // Max length TBD - adjust as needed
    pub len: usize,        // Actual length of payload used
                           // Consider adding specific known extended commands as variants later
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::AcknowledgeActive { address } => write!(f, "{}!", address),
            Command::ChangeAddress { address, new_address } => {
                // Spec Sec 5.7.5: aAn!
                write!(f, "{}A{}!", address, new_address)
            }
            Command::StartMeasurement { address, measurement_index } => {
                // Spec Sec 5.7.1: aM! or aM1!..aM9!
                match measurement_index {
                    None => write!(f, "{}M!", address),
                    Some(idx) => {
                        if *idx > 0 && *idx <= 9 {
                            write!(f, "{}M{}!", address, idx)
                        } else {
                            // Invalid index according to spec for aMx!
                            Err(fmt::Error)
                        }
                    }
                }
            }
            Command::StartConcurrentMeasurement { address, measurement_index } => {
                // Spec Sec 5.7.1: aMC! or aMC1!..aMC9!
                 match measurement_index {
                    None => write!(f, "{}MC!", address),
                    Some(idx) => {
                        if *idx > 0 && *idx <= 9 {
                            write!(f, "{}MC{}!", address, idx)
                        } else {
                            // Invalid index according to spec for aMCx!
                            Err(fmt::Error)
                        }
                    }
                }
            }
            Command::StartVerification { address } => {
                // Spec Sec 5.7.2: aV!
                write!(f, "{}V!", address)
            }
            Command::SendData { address, data_index } => {
                // Spec Sec 5.7.3: aD0!..aD9!
                // Ensure index is 0-9
                if *data_index <= 9 {
                    write!(f, "{}D{}!", address, data_index)
                } else {
                     Err(fmt::Error)
                }
            }
            Command::ContinuousMeasurement { address, measurement_index } => {
                // Spec Sec 5.7.4: aR0!..aR9!
                // Ensure index is 0-9
                if *measurement_index <= 9 {
                    write!(f, "{}R{}!", address, measurement_index)
                } else {
                     Err(fmt::Error)
                }
            }
            Command::IdentifySensor { address } => {
                // Spec Sec 5.7.6: aI!
                write!(f, "{}I!", address)
            }
            // Extended commands need careful formatting based on their specific structure
            Command::Extended(ext_cmd) => {
                 // Format as aX...! where X... is the payload
                write!(f, "{}", ext_cmd.address)?;
                // Write the payload bytes directly
                // Using write_str assumes valid UTF-8, which might not be true for all extended commands.
                // If payload can contain non-UTF8, write bytes individually or use a different approach.
                if let Ok(payload_str) = core::str::from_utf8(&ext_cmd.payload[..ext_cmd.len]) {
                    f.write_str(payload_str)
                } else {
                    // Handle non-UTF8 payload - maybe error? Or try lossy conversion?
                    Err(fmt::Error) // Simplest safe option for now
                }
            }
        }
    }
}


impl Command {
    /// Returns the address the command is directed to.
    pub fn address(&self) -> Sdi12Addr {
        match self {
            Command::AcknowledgeActive { address } => *address,
            Command::ChangeAddress { address, .. } => *address,
            Command::StartMeasurement { address, .. } => *address,
            Command::StartConcurrentMeasurement { address, .. } => *address,
            Command::StartVerification { address } => *address,
            Command::SendData { address, .. } => *address,
            Command::ContinuousMeasurement { address, .. } => *address,
            Command::IdentifySensor { address } => *address,
            Command::Extended(ext_cmd) => ext_cmd.address,
        }
    }

    /// Checks if a response is expected for this command according to the SDI-12 spec.
    /// Note: This doesn't account for service requests.
    pub fn requires_response(&self) -> bool {
        match self {
            // Commands that primarily expect a response (acknowledgement, data, info)
            Command::AcknowledgeActive { .. } => true,
            Command::StartMeasurement { .. } => true,      // Expects service request or ack+timing
            Command::StartConcurrentMeasurement { .. } => true, // Expects ack+timing
            Command::StartVerification { .. } => true,      // Expects ack+timing
            Command::SendData { .. } => true,               // Expects data or ack
            Command::IdentifySensor { .. } => true,         // Expects identification string
            // Commands that change state but might have a response
            Command::ChangeAddress { .. } => true, // Expects the new address as acknowledgment
            // Commands related to continuous measurements might or might not have immediate actionable responses
            // other than the data stream itself, which isn't a typical command-response pair.
            // R commands usually just get an ack+timing, then data streams.
            Command::ContinuousMeasurement { .. } => true, // Expects ack+timing
            // Extended commands are manufacturer-specific; assume they might require a response.
            Command::Extended(_) => true, // Safer to assume yes
        }
    }

    // Helper to create extended commands (example)
    // You might want more specific constructors for known extended commands.
    pub fn new_extended(
        address: Sdi12Addr,
        payload_str: &str,
    ) -> Result<Self, Sdi12Error> {
        if !payload_str.ends_with('!') {
            return Err(Sdi12Error::CommandFormat("Extended command payload must end with '!'"));
        }
        if payload_str.len() > 16 { // Matches array size
             return Err(Sdi12Error::CommandFormat("Extended command payload too long"));
        }
        let mut payload = [0u8; 16];
        payload[..payload_str.len()].copy_from_slice(payload_str.as_bytes());
        Ok(Command::Extended(ExtendedCommand {
            address,
            payload,
            len: payload_str.len(),
        }))
    }
}

// Placeholder for potential ExtendedCommand specific methods if needed later
impl ExtendedCommand {
    // pub fn command_code(&self) -> Option<&str> { ... }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::address::Sdi12Addr; // Make sure Addr is imported
    // Need a Write implementation for testing formatting errors
    use heapless::String as HeaplessString;
    use core::fmt::Write; // Import the Write trait

    // Helper for creating commands in tests
    fn addr(c: char) -> Sdi12Addr {
        Sdi12Addr::new(c).unwrap()
    }

    #[test]
    fn test_command_formatting() {
        assert_eq!(
            Command::AcknowledgeActive { address: addr('1') }.to_string(),
            "1!"
        );
        assert_eq!(
            Command::ChangeAddress { address: addr('1'), new_address: addr('2') }.to_string(),
            "1A2!"
        );
        assert_eq!(
            Command::StartMeasurement { address: addr('1'), measurement_index: None }.to_string(),
            "1M!"
        );
        assert_eq!(
            Command::StartMeasurement { address: addr('1'), measurement_index: Some(3) }.to_string(),
            "1M3!"
        );
        assert_eq!(
            Command::StartConcurrentMeasurement { address: addr('1'), measurement_index: None }.to_string(),
            "1MC!"
        );
         assert_eq!(
            Command::StartConcurrentMeasurement { address: addr('1'), measurement_index: Some(9) }.to_string(),
            "1MC9!"
        );
        assert_eq!(
            Command::StartVerification { address: addr('1') }.to_string(),
            "1V!"
        );
        assert_eq!(
            Command::SendData { address: addr('1'), data_index: 0 }.to_string(),
            "1D0!"
        );
        assert_eq!(
            Command::SendData { address: addr('1'), data_index: 9 }.to_string(),
            "1D9!"
        );
        assert_eq!(
            Command::ContinuousMeasurement { address: addr('1'), measurement_index: 0 }.to_string(),
             "1R0!"
        );
         assert_eq!(
            Command::ContinuousMeasurement { address: addr('1'), measurement_index: 5 }.to_string(),
             "1R5!"
        );
        assert_eq!(
            Command::IdentifySensor { address: addr('1') }.to_string(),
            "1I!"
        );

        // Test extended command formatting
        let ext_cmd = Command::new_extended(addr('0'), "XTEST!").unwrap();
         assert_eq!(ext_cmd.to_string(), "0XTEST!");

        let ext_cmd_long = Command::new_extended(addr('Z'), "LONGPAYLOAD1234!").unwrap();
        assert_eq!(ext_cmd_long.to_string(), "ZLONGPAYLOAD1234!");
    }

    #[test]
    fn test_invalid_measurement_index_format() {
         // Index 0 is invalid for aMx! / aMCx!
         let cmd_m0 = Command::StartMeasurement { address: addr('1'), measurement_index: Some(0) };
         // fmt::Display returns Err(fmt::Error), .to_string() yields empty
         // Using write! macro directly to check the Result
         let mut output = HeaplessString::<8>::new(); // Need a Write impl
         assert!(write!(output, "{}", cmd_m0).is_err());

         let cmd_mc0 = Command::StartConcurrentMeasurement { address: addr('1'), measurement_index: Some(0) };
         output.clear();
         assert!(write!(output, "{}", cmd_mc0).is_err());


         // Index > 9 is invalid
         let cmd_m10 = Command::StartMeasurement { address: addr('1'), measurement_index: Some(10) };
         output.clear();
          assert!(write!(output, "{}", cmd_m10).is_err());

         let cmd_mc10 = Command::StartConcurrentMeasurement { address: addr('1'), measurement_index: Some(10) };
         output.clear();
          assert!(write!(output, "{}", cmd_mc10).is_err());
    }

     #[test]
    fn test_invalid_data_index_format() {
         // Index > 9 is invalid for aDx!
         let cmd_d10 = Command::SendData { address: addr('1'), data_index: 10 };
         let mut output = HeaplessString::<8>::new();
         assert!(write!(output, "{}", cmd_d10).is_err());
    }

      #[test]
    fn test_invalid_continuous_index_format() {
         // Index > 9 is invalid for aRx!
         let cmd_r10 = Command::ContinuousMeasurement { address: addr('1'), measurement_index: 10 };
         let mut output = HeaplessString::<8>::new();
         assert!(write!(output, "{}", cmd_r10).is_err());
    }

     #[test]
     fn test_extended_command_creation_errors() {
         // Use a simple MockError for the Result type in new_extended if necessary
         // Or adjust Sdi12Error if it has a specific CommandFormat variant
         #[derive(Debug, PartialEq)]
         enum MockError { CommandFormat(&'static str) }
         impl From<MockError> for Sdi12Error<MockError> { // Assuming Sdi12Error<E>
             fn from(e: MockError) -> Self {
                 match e {
                     MockError::CommandFormat(s) => Sdi12Error::CommandFormat(s) // Adjust variant if name differs
                 }
             }
         }

         // Assuming new_extended returns Result<Self, Sdi12Error<some_error_type>>
         assert!(Command::new_extended(addr('0'), "XTEST").is_err()); // Missing !
         assert!(Command::new_extended(addr('0'), "THISPAYLOADISWAYTOOLONGFORARRAY!").is_err()); // Too long
     }

    #[test]
    fn test_address_retrieval() {
        assert_eq!(Command::AcknowledgeActive { address: addr('0') }.address(), addr('0'));
        assert_eq!(Command::ChangeAddress { address: addr('1'), new_address: addr('2') }.address(), addr('1'));
        assert_eq!(Command::StartMeasurement { address: addr('3'), measurement_index: None }.address(), addr('3'));
        assert_eq!(Command::StartConcurrentMeasurement { address: addr('4'), measurement_index: Some(1) }.address(), addr('4'));
        assert_eq!(Command::StartVerification { address: addr('5') }.address(), addr('5'));
        assert_eq!(Command::SendData { address: addr('6'), data_index: 0 }.address(), addr('6'));
        assert_eq!(Command::ContinuousMeasurement { address: addr('7'), measurement_index: 9 }.address(), addr('7'));
        assert_eq!(Command::IdentifySensor { address: addr('8') }.address(), addr('8'));
        let ext_cmd = Command::new_extended(addr('9'), "XCMD!").unwrap();
        assert_eq!(ext_cmd.address(), addr('9'));
    }

    #[test]
    fn test_requires_response() {
        assert!(Command::AcknowledgeActive { address: addr('0') }.requires_response());
        assert!(Command::ChangeAddress { address: addr('1'), new_address: addr('2') }.requires_response());
        assert!(Command::StartMeasurement { address: addr('3'), measurement_index: None }.requires_response());
        assert!(Command::StartConcurrentMeasurement { address: addr('4'), measurement_index: Some(1) }.requires_response());
        assert!(Command::StartVerification { address: addr('5') }.requires_response());
        assert!(Command::SendData { address: addr('6'), data_index: 0 }.requires_response());
        // Continuous measurements expect an ack+timing initially
        assert!(Command::ContinuousMeasurement { address: addr('7'), measurement_index: 9 }.requires_response());
        assert!(Command::IdentifySensor { address: addr('8') }.requires_response());
        let ext_cmd = Command::new_extended(addr('9'), "XCMD!").unwrap();
        assert!(ext_cmd.requires_response()); // Assume true for extended
    }
}