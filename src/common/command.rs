// src/common/command.rs

use super::address::Sdi12Addr;
use core::convert::TryFrom;
use core::fmt;

// Conditionally import alloc::string::String
#[cfg(feature = "alloc")]
use alloc::string::String;

// Import alloc types needed specifically for tests when 'alloc' is enabled
#[cfg(feature = "alloc")]
use alloc::string::ToString; // *** ADD THIS LINE ***

// --- Error Type for Index Validation ---

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CommandIndexError {
    MeasurementOutOfRange,    // For M/MC/C/CC (1-9)
    ContinuousOutOfRange,     // For R/RC (0-9)
    DataOutOfRange,           // For D/DB (0-999)
    IdentifyParamOutOfRange, // For _nnn (1-999, maybe 000 is allowed? Spec implies 001)
}

impl fmt::Display for CommandIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandIndexError::MeasurementOutOfRange => write!(f, "Measurement index must be 1-9"),
            CommandIndexError::ContinuousOutOfRange => write!(f, "Continuous index must be 0-9"),
            CommandIndexError::DataOutOfRange => write!(f, "Data index must be 0-999"),
            CommandIndexError::IdentifyParamOutOfRange => write!(f, "Identify Parameter index must be 1-999"), // Assuming 001-999
        }
    }
}
// Consider adding #[cfg(feature = "std")] impl std::error::Error for CommandIndexError {} if needed


// --- Validated Index Types ---

/// Represents the index `n` for M[n], MC[n], C[n], CC[n] commands.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MeasurementIndex {
    /// Represents the base command (M!, MC!, C!, CC!).
    Base,
    /// Represents an indexed command M/MC/C/CC[1-9].
    Indexed(u8), // Value is guaranteed to be 1-9
}

impl MeasurementIndex {
    /// Creates a MeasurementIndex from Option<u8>, validating the range.
    pub fn new(index_opt: Option<u8>) -> Result<Self, CommandIndexError> {
        match index_opt {
            None => Ok(Self::Base),
            Some(i) if (1..=9).contains(&i) => Ok(Self::Indexed(i)),
            Some(_) => Err(CommandIndexError::MeasurementOutOfRange),
        }
    }
    pub fn as_option(&self) -> Option<u8> {
        match self { Self::Base => None, Self::Indexed(i) => Some(*i) }
    }
}

/// Represents the index `n` for R[n], RC[n] commands.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ContinuousIndex(u8); // Value is guaranteed to be 0-9

impl ContinuousIndex {
    pub fn new(index: u8) -> Result<Self, CommandIndexError> {
        if index <= 9 { Ok(Self(index)) } else { Err(CommandIndexError::ContinuousOutOfRange) }
    }
    pub fn value(&self) -> u8 { self.0 }
}
impl TryFrom<u8> for ContinuousIndex {
    type Error = CommandIndexError;
    fn try_from(value: u8) -> Result<Self, Self::Error> { Self::new(value) }
}

/// Represents the index `n` for D[n], DB[n] commands.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DataIndex(u16); // Value is guaranteed to be 0-999

impl DataIndex {
    pub fn new(index: u16) -> Result<Self, CommandIndexError> {
        if index <= 999 { Ok(Self(index)) } else { Err(CommandIndexError::DataOutOfRange) }
    }
    pub fn value(&self) -> u16 { self.0 }
}
impl TryFrom<u16> for DataIndex {
    type Error = CommandIndexError;
    fn try_from(value: u16) -> Result<Self, Self::Error> { Self::new(value) }
}

/// Represents the parameter index `nnn` for Identify Measurement Parameter commands.
/// The spec implies 1-999 (Section 6.2 mentions "three-digit decimal number").
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct IdentifyParameterIndex(u16); // Value is guaranteed to be 1-999

impl IdentifyParameterIndex {
    pub fn new(index: u16) -> Result<Self, CommandIndexError> {
        if (1..=999).contains(&index) { Ok(Self(index)) } else { Err(CommandIndexError::IdentifyParamOutOfRange) }
    }
    pub fn value(&self) -> u16 { self.0 }
}
impl TryFrom<u16> for IdentifyParameterIndex {
    type Error = CommandIndexError;
    fn try_from(value: u16) -> Result<Self, Self::Error> { Self::new(value) }
}


// --- Main Command Enum ---

/// Represents any valid SDI-12 command to be sent by a recorder.
/// Construction often involves using validated index types or their constructors.
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
    StartMeasurement { address: Sdi12Addr, index: MeasurementIndex },
    /// Start Measurement with CRC Request (`aMC!`, `aMC1!`...`aMC9!`)
    StartMeasurementCRC { address: Sdi12Addr, index: MeasurementIndex },

    // --- Concurrent Measurement Commands ---
    /// Start Concurrent Measurement (`aC!`, `aC1!`...`aC9!`)
    StartConcurrentMeasurement { address: Sdi12Addr, index: MeasurementIndex },
    /// Start Concurrent Measurement with CRC Request (`aCC!`, `aCC1!`...`aCC9!`)
    StartConcurrentMeasurementCRC { address: Sdi12Addr, index: MeasurementIndex },

    // --- Data Retrieval Commands ---
    /// Send Data (`aD0!`...`aD999!`) - Used after M, C, V, HA commands
    SendData { address: Sdi12Addr, index: DataIndex },
    /// Send Binary Data (`aDB0!`...`aDB999!`) - Used after HB command
    SendBinaryData { address: Sdi12Addr, index: DataIndex },

    // --- Continuous Measurement Commands ---
    /// Continuous Measurement (`aR0!`...`aR9!`)
    ReadContinuous { address: Sdi12Addr, index: ContinuousIndex },
    /// Continuous Measurement with CRC Request (`aRC0!`...`aRC9!`)
    ReadContinuousCRC { address: Sdi12Addr, index: ContinuousIndex },

    // --- Verification Command ---
    /// Start Verification (`aV!`)
    StartVerification { address: Sdi12Addr },

    // --- High-Volume Commands ---
    /// Start High-Volume ASCII Measurement (`aHA!`)
    StartHighVolumeASCII { address: Sdi12Addr },
    /// Start High-Volume Binary Measurement (`aHB!`)
    StartHighVolumeBinary { address: Sdi12Addr },

    // --- Metadata Commands ---
    IdentifyMeasurement(IdentifyMeasurementCommand),
    IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand),

    // --- Extended Commands ---
    /// Represents a non-standard, manufacturer-specific extended command.
    /// The command body (after address) is stored raw. Requires 'alloc'.
    #[cfg(feature = "alloc")]
    ExtendedCommand { address: Sdi12Addr, command_body: String }, // Excludes '!' terminator
}


// --- Metadata Sub-Enums ---

/// Sub-enum for Identify Measurement commands (Sec 6.1)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifyMeasurementCommand {
    /// `aIM!` or `aIM1!`...`aIM9!`
    Measurement { address: Sdi12Addr, index: MeasurementIndex },
    /// `aIMC!` or `aIMC1!`...`aIMC9!`
    MeasurementCRC { address: Sdi12Addr, index: MeasurementIndex },
    /// `aIV!`
    Verification { address: Sdi12Addr },
    /// `aIC!` or `aIC1!`...`aIC9!`
    ConcurrentMeasurement { address: Sdi12Addr, index: MeasurementIndex },
    /// `aICC!` or `aICC1!`...`aICC9!`
    ConcurrentMeasurementCRC { address: Sdi12Addr, index: MeasurementIndex },
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
    Measurement { address: Sdi12Addr, m_index: MeasurementIndex, param_index: IdentifyParameterIndex },
    /// `aIMC_nnn!` or `aIMC1_nnn!`...`aIMC9_nnn!`
    MeasurementCRC { address: Sdi12Addr, m_index: MeasurementIndex, param_index: IdentifyParameterIndex },
    /// `aIV_nnn!`
    Verification { address: Sdi12Addr, param_index: IdentifyParameterIndex },
    /// `aIC_nnn!` or `aIC1_nnn!`...`aIC9_nnn!`
    ConcurrentMeasurement { address: Sdi12Addr, c_index: MeasurementIndex, param_index: IdentifyParameterIndex },
    /// `aICC_nnn!` or `aICC1_nnn!`...`aICC9_nnn!`
    ConcurrentMeasurementCRC { address: Sdi12Addr, c_index: MeasurementIndex, param_index: IdentifyParameterIndex },
    /// `aIR0_nnn!` ... `aIR9_nnn!`
    ReadContinuous { address: Sdi12Addr, r_index: ContinuousIndex, param_index: IdentifyParameterIndex },
    /// `aIRC0_nnn!` ... `aIRC9_nnn!`
    ReadContinuousCRC { address: Sdi12Addr, r_index: ContinuousIndex, param_index: IdentifyParameterIndex },
    /// `aIHA_nnn!`
    HighVolumeASCII { address: Sdi12Addr, param_index: IdentifyParameterIndex },
    /// `aIHB_nnn!`
    HighVolumeBinary { address: Sdi12Addr, param_index: IdentifyParameterIndex },
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::address::Sdi12Addr; // Use crate:: for test module

    // Helper to create addresses for tests
    fn addr(c: char) -> Sdi12Addr { Sdi12Addr::new(c).unwrap() }

    #[test]
    fn test_measurement_index_validation() {
        assert_eq!(MeasurementIndex::new(None), Ok(MeasurementIndex::Base));
        assert_eq!(MeasurementIndex::new(Some(1)), Ok(MeasurementIndex::Indexed(1)));
        assert_eq!(MeasurementIndex::new(Some(9)), Ok(MeasurementIndex::Indexed(9)));
        assert_eq!(MeasurementIndex::new(Some(0)), Err(CommandIndexError::MeasurementOutOfRange));
        assert_eq!(MeasurementIndex::new(Some(10)), Err(CommandIndexError::MeasurementOutOfRange));
    }

     #[test]
    fn test_continuous_index_validation() {
        assert_eq!(ContinuousIndex::new(0), Ok(ContinuousIndex(0)));
        assert_eq!(ContinuousIndex::new(9), Ok(ContinuousIndex(9)));
        assert_eq!(ContinuousIndex::try_from(5), Ok(ContinuousIndex(5)));
        assert_eq!(ContinuousIndex::new(10), Err(CommandIndexError::ContinuousOutOfRange));
        assert!(ContinuousIndex::try_from(10).is_err());
    }

    #[test]
    fn test_data_index_validation() {
        assert_eq!(DataIndex::new(0), Ok(DataIndex(0)));
        assert_eq!(DataIndex::new(999), Ok(DataIndex(999)));
        assert_eq!(DataIndex::try_from(123), Ok(DataIndex(123)));
        assert_eq!(DataIndex::new(1000), Err(CommandIndexError::DataOutOfRange));
        assert!(DataIndex::try_from(1000).is_err());
    }

     #[test]
    fn test_identify_param_index_validation() {
        assert_eq!(IdentifyParameterIndex::new(1), Ok(IdentifyParameterIndex(1)));
        assert_eq!(IdentifyParameterIndex::new(999), Ok(IdentifyParameterIndex(999)));
        assert_eq!(IdentifyParameterIndex::try_from(42), Ok(IdentifyParameterIndex(42)));
        assert_eq!(IdentifyParameterIndex::new(0), Err(CommandIndexError::IdentifyParamOutOfRange));
        assert_eq!(IdentifyParameterIndex::new(1000), Err(CommandIndexError::IdentifyParamOutOfRange));
        assert!(IdentifyParameterIndex::try_from(0).is_err());
        assert!(IdentifyParameterIndex::try_from(1000).is_err());
    }

    #[test]
    fn test_command_construction() {
        // Now construction uses the validated types or their constructors
        assert!(Command::AcknowledgeActive { address: addr('0') } == Command::AcknowledgeActive { address: addr('0') });
        assert!(Command::AddressQuery == Command::AddressQuery);
        assert!(Command::ChangeAddress { address: addr('0'), new_address: addr('1') } == Command::ChangeAddress { address: addr('0'), new_address: addr('1') });

        // Measurement - Use valid constructors
        let m_base = MeasurementIndex::new(None).unwrap();
        let m_idx3 = MeasurementIndex::new(Some(3)).unwrap();
        let m_idx9 = MeasurementIndex::new(Some(9)).unwrap();
        assert!(Command::StartMeasurement { address: addr('1'), index: m_base } == Command::StartMeasurement { address: addr('1'), index: MeasurementIndex::Base });
        assert!(Command::StartMeasurement { address: addr('1'), index: m_idx3 } == Command::StartMeasurement { address: addr('1'), index: MeasurementIndex::Indexed(3) });
        assert!(Command::StartMeasurementCRC { address: addr('1'), index: m_idx9 } == Command::StartMeasurementCRC { address: addr('1'), index: MeasurementIndex::Indexed(9) });

        // Concurrent (uses same MeasurementIndex)
        assert!(Command::StartConcurrentMeasurement { address: addr('0'), index: m_base } == Command::StartConcurrentMeasurement { address: addr('0'), index: m_base });
        let cc_idx1 = MeasurementIndex::new(Some(1)).unwrap();
        assert!(Command::StartConcurrentMeasurementCRC { address: addr('0'), index: cc_idx1 } == Command::StartConcurrentMeasurementCRC { address: addr('0'), index: cc_idx1 });

        // Data - Use valid constructors
        let d_idx0 = DataIndex::new(0).unwrap();
        let d_idx10 = DataIndex::new(10).unwrap();
        let d_idx999 = DataIndex::new(999).unwrap();
        let db_idx123 = DataIndex::new(123).unwrap();
        assert!(Command::SendData { address: addr('1'), index: d_idx0 } == Command::SendData { address: addr('1'), index: d_idx0 });
        assert!(Command::SendData { address: addr('1'), index: d_idx10 } == Command::SendData { address: addr('1'), index: d_idx10 });
        assert!(Command::SendData { address: addr('1'), index: d_idx999 } == Command::SendData { address: addr('1'), index: d_idx999 });
        assert!(Command::SendBinaryData { address: addr('0'), index: db_idx123 } == Command::SendBinaryData { address: addr('0'), index: db_idx123 });

        // Continuous - Use valid constructors
        let r_idx0 = ContinuousIndex::new(0).unwrap();
        let rc_idx9 = ContinuousIndex::new(9).unwrap();
        assert!(Command::ReadContinuous { address: addr('0'), index: r_idx0 } == Command::ReadContinuous { address: addr('0'), index: r_idx0 });
        assert!(Command::ReadContinuousCRC { address: addr('0'), index: rc_idx9 } == Command::ReadContinuousCRC { address: addr('0'), index: rc_idx9 });

        // Verification
        assert!(Command::StartVerification { address: addr('1') } == Command::StartVerification { address: addr('1') });

        // High Volume
        assert!(Command::StartHighVolumeASCII { address: addr('0') } == Command::StartHighVolumeASCII { address: addr('0') });
        assert!(Command::StartHighVolumeBinary { address: addr('1') } == Command::StartHighVolumeBinary { address: addr('1') });

        // Metadata Identify Measurement
        let m_idx2 = MeasurementIndex::new(Some(2)).unwrap();
        let ident_m = IdentifyMeasurementCommand::Measurement { address: addr('1'), index: m_idx2 };
        assert!(Command::IdentifyMeasurement(ident_m.clone()) == Command::IdentifyMeasurement(ident_m));
        let ident_ha = IdentifyMeasurementCommand::HighVolumeASCII{ address: addr('0') };
        assert!(Command::IdentifyMeasurement(ident_ha.clone()) == Command::IdentifyMeasurement(ident_ha));

        // Metadata Identify Parameter
        let p_idx1 = IdentifyParameterIndex::new(1).unwrap();
        let p_idx10 = IdentifyParameterIndex::new(10).unwrap();
        let m_base = MeasurementIndex::new(None).unwrap();
        let r_idx5 = ContinuousIndex::new(5).unwrap();

        let identp_mc = IdentifyMeasurementParameterCommand::MeasurementCRC { address: addr('1'), m_index: m_base, param_index: p_idx1 };
        assert!(Command::IdentifyMeasurementParameter(identp_mc.clone()) == Command::IdentifyMeasurementParameter(identp_mc));
        let identp_r = IdentifyMeasurementParameterCommand::ReadContinuous { address: addr('0'), r_index: r_idx5, param_index: p_idx10 };
        assert!(Command::IdentifyMeasurementParameter(identp_r.clone()) == Command::IdentifyMeasurementParameter(identp_r));
    }

     #[test]
    #[cfg(feature = "alloc")]
    fn test_extended_command() {
        let addr = addr('Z');
        // Note: command_body excludes address and '!'
        let cmd = Command::ExtendedCommand { address: addr, command_body: "XCAL".to_string() };
        assert_eq!(cmd, Command::ExtendedCommand { address: addr, command_body: "XCAL".to_string() });
    }
}