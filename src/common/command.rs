// src/common/command.rs

use super::address::Sdi12Addr;
use super::frame::FrameFormat; // Potentially needed if commands imply format change needs

// --- Command Structure ---

/// Represents any valid SDI-12 command to be sent by a recorder.
/// Each variant includes the target sensor address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Acknowledge Active (`a!`)
    AcknowledgeActive { address: Sdi12Addr },
    /// Send Identification (`aI!`)
    SendIdentification { address: Sdi12Addr },
    /// Address Query (`?!`) - Special case, uses the Query Address
    AddressQuery, // Address is implicitly '?'
    /// Change Address (`aAb!`)
    ChangeAddress { address: Sdi12Addr, new_address: Sdi12Addr },

    // --- Measurement Commands ---
    /// Start Measurement (`aM!`, `aM1!`...`aM9!`)
    StartMeasurement { address: Sdi12Addr, index: Option<u8> }, // index 1-9 for M1-M9, None for M
    /// Start Measurement with CRC Request (`aMC!`, `aMC1!`...`aMC9!`)
    StartMeasurementCRC { address: Sdi12Addr, index: Option<u8> }, // index 1-9 for MC1-MC9, None for MC

    // --- Concurrent Measurement Commands ---
    /// Start Concurrent Measurement (`aC!`, `aC1!`...`aC9!`)
    StartConcurrentMeasurement { address: Sdi12Addr, index: Option<u8> }, // index 1-9 for C1-C9, None for C
    /// Start Concurrent Measurement with CRC Request (`aCC!`, `aCC1!`...`aCC9!`)
    StartConcurrentMeasurementCRC { address: Sdi12Addr, index: Option<u8> }, // index 1-9 for CC1-CC9, None for CC

    // --- Data Retrieval Commands ---
    /// Send Data (`aD0!`...`aD999!`) - Used after M, C, V, HA commands
    SendData { address: Sdi12Addr, index: u16 }, // index 0-999
    /// Send Binary Data (`aDB0!`...`aDB999!`) - Used after HB command
    SendBinaryData { address: Sdi12Addr, index: u16 }, // index 0-999

    // --- Continuous Measurement Commands ---
    /// Continuous Measurement (`aR0!`...`aR9!`)
    ReadContinuous { address: Sdi12Addr, index: u8 }, // index 0-9
    /// Continuous Measurement with CRC Request (`aRC0!`...`aRC9!`)
    ReadContinuousCRC { address: Sdi12Addr, index: u8 }, // index 0-9

    // --- Verification Command ---
    /// Start Verification (`aV!`)
    StartVerification { address: Sdi12Addr },

    // --- High-Volume Commands ---
    /// Start High-Volume ASCII Measurement (`aHA!`)
    StartHighVolumeASCII { address: Sdi12Addr },
    /// Start High-Volume Binary Measurement (`aHB!`)
    StartHighVolumeBinary { address: Sdi12Addr },

    // --- Metadata Commands ---
    // Identify Measurement (returns tttn/tttnn/tttnnn like original M/C/HA/HB)
    IdentifyMeasurement(IdentifyMeasurementCommand),
    // Identify Measurement Parameter (returns parameter info string)
    IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand),

    // --- Extended Commands ---
    /// Represents a non-standard, manufacturer-specific extended command.
    /// The command body (after 'X' if used) is stored raw. Requires 'alloc'.
    #[cfg(feature = "alloc")]
    ExtendedCommand { address: Sdi12Addr, command_body: String },

}

/// Sub-enum for Identify Measurement commands (Sec 6.1)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifyMeasurementCommand {
    /// `aIM!` or `aIM1!`...`aIM9!`
    Measurement { address: Sdi12Addr, index: Option<u8> },
    /// `aIMC!` or `aIMC1!`...`aIMC9!`
    MeasurementCRC { address: Sdi12Addr, index: Option<u8> },
    /// `aIV!`
    Verification { address: Sdi12Addr },
    /// `aIC!` or `aIC1!`...`aIC9!`
    ConcurrentMeasurement { address: Sdi12Addr, index: Option<u8> },
    /// `aICC!` or `aICC1!`...`aICC9!`
    ConcurrentMeasurementCRC { address: Sdi12Addr, index: Option<u8> },
    /// `aIHA!`
    HighVolumeASCII { address: Sdi12Addr },
    /// `aIHB!`
    HighVolumeBinary { address: Sdi12Addr },
    // Continuous Measurements (R, RC) do not have Identify Measurement commands
}

/// Sub-enum for Identify Measurement Parameter commands (Sec 6.2)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifyMeasurementParameterCommand {
     /// `aIM_nnn!` or `aIM1_nnn!`...`aIM9_nnn!`
    Measurement { address: Sdi12Addr, m_index: Option<u8>, param_index: u16 },
    /// `aIMC_nnn!` or `aIMC1_nnn!`...`aIMC9_nnn!`
    MeasurementCRC { address: Sdi12Addr, m_index: Option<u8>, param_index: u16 },
    /// `aIV_nnn!`
    Verification { address: Sdi12Addr, param_index: u16 },
    /// `aIC_nnn!` or `aIC1_nnn!`...`aIC9_nnn!`
    ConcurrentMeasurement { address: Sdi12Addr, c_index: Option<u8>, param_index: u16 },
    /// `aICC_nnn!` or `aICC1_nnn!`...`aICC9_nnn!`
    ConcurrentMeasurementCRC { address: Sdi12Addr, c_index: Option<u8>, param_index: u16 },
     /// `aIR0_nnn!` ... `aIR9_nnn!`
    ReadContinuous { address: Sdi12Addr, r_index: u8, param_index: u16 },
    /// `aIRC0_nnn!` ... `aIRC9_nnn!`
    ReadContinuousCRC { address: Sdi12Addr, r_index: u8, param_index: u16 },
    /// `aIHA_nnn!`
    HighVolumeASCII { address: Sdi12Addr, param_index: u16 },
    /// `aIHB_nnn!`
    HighVolumeBinary { address: Sdi12Addr, param_index: u16 },
}


// --- Helper Methods (Example - Formatting can be added later) ---

// We will likely add a method to format these commands into the byte strings
// needed for transmission, e.g., `command.format(buffer: &mut [u8]) -> usize`.
// This formatting logic will handle converting the enum variants and their
// parameters into the correct ASCII sequence like "aM1!", "aD10!", "aIM_001!".


// --- Unit Tests (Basic structure validation) ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::address::Sdi12Addr; // Use crate:: for test module

    #[test]
    fn test_command_construction() {
        // Simple examples to ensure variants can be created
        let addr0 = Sdi12Addr::DEFAULT_ADDRESS;
        let addr1 = Sdi12Addr::new('1').unwrap();

        assert_eq!(Command::AcknowledgeActive { address: addr0 }, Command::AcknowledgeActive { address: Sdi12Addr::new('0').unwrap() });
        assert_eq!(Command::AddressQuery, Command::AddressQuery);
        assert_eq!(Command::ChangeAddress { address: addr0, new_address: addr1 }, Command::ChangeAddress { address: Sdi12Addr::new('0').unwrap(), new_address: Sdi12Addr::new('1').unwrap() });

        // Measurement
        assert_eq!(Command::StartMeasurement { address: addr1, index: None }, Command::StartMeasurement { address: addr1, index: None }); // M!
        assert_eq!(Command::StartMeasurement { address: addr1, index: Some(3) }, Command::StartMeasurement { address: addr1, index: Some(3) }); // M3!
        assert_eq!(Command::StartMeasurementCRC { address: addr1, index: Some(9) }, Command::StartMeasurementCRC { address: addr1, index: Some(9) }); // MC9!

        // Concurrent
        assert_eq!(Command::StartConcurrentMeasurement { address: addr0, index: None }, Command::StartConcurrentMeasurement { address: addr0, index: None }); // C!
        assert_eq!(Command::StartConcurrentMeasurementCRC { address: addr0, index: Some(1) }, Command::StartConcurrentMeasurementCRC { address: addr0, index: Some(1) }); // CC1!

        // Data
        assert_eq!(Command::SendData { address: addr1, index: 0 }, Command::SendData { address: addr1, index: 0 }); // D0!
        assert_eq!(Command::SendData { address: addr1, index: 10 }, Command::SendData { address: addr1, index: 10 }); // D10!
        assert_eq!(Command::SendData { address: addr1, index: 999 }, Command::SendData { address: addr1, index: 999 }); // D999!
         assert_eq!(Command::SendBinaryData { address: addr0, index: 123 }, Command::SendBinaryData { address: addr0, index: 123 }); // DB123!

        // Continuous
        assert_eq!(Command::ReadContinuous { address: addr0, index: 0 }, Command::ReadContinuous { address: addr0, index: 0 }); // R0!
        assert_eq!(Command::ReadContinuousCRC { address: addr0, index: 9 }, Command::ReadContinuousCRC { address: addr0, index: 9 }); // RC9!

        // Verification
         assert_eq!(Command::StartVerification { address: addr1 }, Command::StartVerification { address: addr1 }); // V!

        // High Volume
        assert_eq!(Command::StartHighVolumeASCII { address: addr0 }, Command::StartHighVolumeASCII { address: addr0 }); // HA!
        assert_eq!(Command::StartHighVolumeBinary { address: addr1 }, Command::StartHighVolumeBinary { address: addr1 }); // HB!

        // Metadata Identify Measurement
        let ident_m = IdentifyMeasurementCommand::Measurement { address: addr1, index: Some(2) }; // IM2!
        assert_eq!(Command::IdentifyMeasurement(ident_m.clone()), Command::IdentifyMeasurement(ident_m));
        let ident_ha = IdentifyMeasurementCommand::HighVolumeASCII{ address: addr0 }; // IHA!
        assert_eq!(Command::IdentifyMeasurement(ident_ha.clone()), Command::IdentifyMeasurement(ident_ha));

        // Metadata Identify Parameter
        let identp_mc = IdentifyMeasurementParameterCommand::MeasurementCRC { address: addr1, m_index: None, param_index: 1 }; // IMC_001!
        assert_eq!(Command::IdentifyMeasurementParameter(identp_mc.clone()), Command::IdentifyMeasurementParameter(identp_mc));
        let identp_r = IdentifyMeasurementParameterCommand::ReadContinuous { address: addr0, r_index: 5, param_index: 10 }; // IR5_010!
         assert_eq!(Command::IdentifyMeasurementParameter(identp_r.clone()), Command::IdentifyMeasurementParameter(identp_r));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_extended_command() {
        let addr = Sdi12Addr::new('Z').unwrap();
        let cmd = Command::ExtendedCommand { address: addr, command_body: "XCAL".to_string() };
        assert_eq!(cmd, Command::ExtendedCommand { address: addr, command_body: "XCAL".to_string() });
    }

     #[test]
    #[should_panic] // Example: Add validation later if needed
    fn test_invalid_index_range() {
        // Ideally, constructors or methods would validate indices, e.g.
        // Command::SendData { address: Sdi12Addr::DEFAULT_ADDRESS, index: 1000 }; // Should potentially panic or return Result
        // Command::ReadContinuous { address: Sdi12Addr::DEFAULT_ADDRESS, index: 10 }; // Should potentially panic or return Result
    }
}