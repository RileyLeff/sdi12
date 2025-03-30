// src/common/command.rs

use super::address::Sdi12Addr;
use core::convert::TryFrom;
use core::fmt::{self, Write}; // Need core::fmt::Write
use arrayvec::ArrayString; // Use ArrayString for formatting

// --- Conditionally import String ---
#[cfg(feature = "alloc")]
use alloc::string::String;

// --- Error Type for Index Validation ---

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CommandIndexError {
    MeasurementOutOfRange,    // For M/MC/C/CC (1-9)
    ContinuousOutOfRange,     // For R/RC (0-9)
    DataOutOfRange,           // For D/DB (0-999)
    IdentifyParamOutOfRange, // For _nnn (1-999)
}

impl fmt::Display for CommandIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandIndexError::MeasurementOutOfRange => write!(f, "Measurement index must be 1-9"),
            CommandIndexError::ContinuousOutOfRange => write!(f, "Continuous index must be 0-9"),
            CommandIndexError::DataOutOfRange => write!(f, "Data index must be 0-999"),
            CommandIndexError::IdentifyParamOutOfRange => write!(f, "Identify Parameter index must be 1-999"),
        }
    }
}

// --- Error Type for Formatting ---
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CommandFormatError {
    /// The provided buffer was too small.
    BufferOverflow,
    /// A formatting error occurred (e.g., writing number failed).
    FmtError,
}
impl From<core::fmt::Error> for CommandFormatError {
    fn from(_: core::fmt::Error) -> Self { CommandFormatError::FmtError }
}

// Add Display impl to satisfy thiserror constraint (Error E0599)
impl fmt::Display for CommandFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandFormatError::BufferOverflow => write!(f, "Buffer overflow during formatting"),
            CommandFormatError::FmtError => write!(f, "Internal formatting error"),
        }
    }
}


// --- Validated Index Types ---

/// Represents the index `n` for M[n], MC[n], C[n], CC[n] commands.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MeasurementIndex {
    Base,
    Indexed(u8), // 1-9
}

impl MeasurementIndex {
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
pub struct ContinuousIndex(u8); // 0-9

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
pub struct DataIndex(u16); // 0-999

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
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct IdentifyParameterIndex(u16); // 1-999

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    AcknowledgeActive { address: Sdi12Addr },
    SendIdentification { address: Sdi12Addr },
    AddressQuery,
    ChangeAddress { address: Sdi12Addr, new_address: Sdi12Addr },
    StartMeasurement { address: Sdi12Addr, index: MeasurementIndex },
    StartMeasurementCRC { address: Sdi12Addr, index: MeasurementIndex },
    StartConcurrentMeasurement { address: Sdi12Addr, index: MeasurementIndex },
    StartConcurrentMeasurementCRC { address: Sdi12Addr, index: MeasurementIndex },
    SendData { address: Sdi12Addr, index: DataIndex },
    SendBinaryData { address: Sdi12Addr, index: DataIndex },
    ReadContinuous { address: Sdi12Addr, index: ContinuousIndex },
    ReadContinuousCRC { address: Sdi12Addr, index: ContinuousIndex },
    StartVerification { address: Sdi12Addr },
    StartHighVolumeASCII { address: Sdi12Addr },
    StartHighVolumeBinary { address: Sdi12Addr },
    IdentifyMeasurement(IdentifyMeasurementCommand),
    IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand),
    #[cfg(feature = "alloc")]
    ExtendedCommand { address: Sdi12Addr, command_body: String },
    // TODO: Consider adding a non-alloc ExtendedCommand variant using a fixed buffer
    // #[cfg(not(feature = "alloc"))]
    // ExtendedCommandFixed { address: Sdi12Addr, command_body: ArrayString<{MAX_EXT_LEN?}> }, // Fixed type here too
}


impl Command {
    /// Maximum length of the *formatted* standard command string (e.g., "aICC9_999!").
    /// Calculated as: address(1) + ICC(3) + index(1) + underscore(1) + param(3) + !(1) = 10
    const MAX_FORMATTED_LEN: usize = 10;

    /// Formats the command into the standard byte sequence (e.g., "0M!", "1D10!") including the '!'.
    /// Writes into a fixed-size buffer (ArrayString) to avoid allocation.
    /// Extended commands require the 'alloc' feature.
    pub fn format_into(&self) -> Result<ArrayString<{Self::MAX_FORMATTED_LEN}>, CommandFormatError> { // Re-added braces
        let mut buffer = ArrayString::<{Self::MAX_FORMATTED_LEN}>::new(); // Re-added braces

        match self {
            Command::AcknowledgeActive { address } => write!(buffer, "{}!", address)?,
            Command::SendIdentification { address } => write!(buffer, "{}I!", address)?,
            Command::AddressQuery => write!(buffer, "?!")?,
            Command::ChangeAddress { address, new_address } => write!(buffer, "{}A{}!", address, new_address)?,

            Command::StartMeasurement { address, index } => {
                write!(buffer, "{}M", address)?;
                if let MeasurementIndex::Indexed(i) = index { write!(buffer, "{}", i)?; }
                write!(buffer, "!")?;
            }
            Command::StartMeasurementCRC { address, index } => {
                write!(buffer, "{}MC", address)?;
                if let MeasurementIndex::Indexed(i) = index { write!(buffer, "{}", i)?; }
                write!(buffer, "!")?;
            }
            Command::StartConcurrentMeasurement { address, index } => {
                 write!(buffer, "{}C", address)?;
                if let MeasurementIndex::Indexed(i) = index { write!(buffer, "{}", i)?; }
                write!(buffer, "!")?;
            }
            Command::StartConcurrentMeasurementCRC { address, index } => {
                 write!(buffer, "{}CC", address)?;
                if let MeasurementIndex::Indexed(i) = index { write!(buffer, "{}", i)?; }
                write!(buffer, "!")?;
            }
            Command::SendData { address, index } => write!(buffer, "{}D{}!", address, index.value())?,
            Command::SendBinaryData { address, index } => write!(buffer, "{}DB{}!", address, index.value())?,
            Command::ReadContinuous { address, index } => write!(buffer, "{}R{}!", address, index.value())?,
            Command::ReadContinuousCRC { address, index } => write!(buffer, "{}RC{}!", address, index.value())?,
            Command::StartVerification { address } => write!(buffer, "{}V!", address)?,
            Command::StartHighVolumeASCII { address } => write!(buffer, "{}HA!", address)?,
            Command::StartHighVolumeBinary { address } => write!(buffer, "{}HB!", address)?,

            Command::IdentifyMeasurement(cmd) => {
                match cmd {
                    IdentifyMeasurementCommand::Measurement { address, index } => { write!(buffer, "{}IM", address)?; if let MeasurementIndex::Indexed(i) = index { write!(buffer, "{}", i)?; } }
                    IdentifyMeasurementCommand::MeasurementCRC { address, index } => { write!(buffer, "{}IMC", address)?; if let MeasurementIndex::Indexed(i) = index { write!(buffer, "{}", i)?; } }
                    IdentifyMeasurementCommand::Verification { address } => write!(buffer, "{}IV", address)?,
                    IdentifyMeasurementCommand::ConcurrentMeasurement { address, index } => { write!(buffer, "{}IC", address)?; if let MeasurementIndex::Indexed(i) = index { write!(buffer, "{}", i)?; } }
                    IdentifyMeasurementCommand::ConcurrentMeasurementCRC { address, index } => { write!(buffer, "{}ICC", address)?; if let MeasurementIndex::Indexed(i) = index { write!(buffer, "{}", i)?; } }
                    IdentifyMeasurementCommand::HighVolumeASCII { address } => write!(buffer, "{}IHA", address)?,
                    IdentifyMeasurementCommand::HighVolumeBinary { address } => write!(buffer, "{}IHB", address)?,
                }
                write!(buffer, "!")?;
            }
            Command::IdentifyMeasurementParameter(cmd) => {
                match cmd {
                     IdentifyMeasurementParameterCommand::Measurement { address, m_index, param_index } => { write!(buffer, "{}IM", address)?; if let MeasurementIndex::Indexed(i) = m_index { write!(buffer, "{}", i)?; } write!(buffer, "_{:03}", param_index.value())?; }
                     IdentifyMeasurementParameterCommand::MeasurementCRC { address, m_index, param_index } => { write!(buffer, "{}IMC", address)?; if let MeasurementIndex::Indexed(i) = m_index { write!(buffer, "{}", i)?; } write!(buffer, "_{:03}", param_index.value())?; }
                     IdentifyMeasurementParameterCommand::Verification { address, param_index } => { write!(buffer, "{}IV_{:03}", address, param_index.value())?; }
                     IdentifyMeasurementParameterCommand::ConcurrentMeasurement { address, c_index, param_index } => { write!(buffer, "{}IC", address)?; if let MeasurementIndex::Indexed(i) = c_index { write!(buffer, "{}", i)?; } write!(buffer, "_{:03}", param_index.value())?; }
                     IdentifyMeasurementParameterCommand::ConcurrentMeasurementCRC { address, c_index, param_index } => { write!(buffer, "{}ICC", address)?; if let MeasurementIndex::Indexed(i) = c_index { write!(buffer, "{}", i)?; } write!(buffer, "_{:03}", param_index.value())?; }
                     IdentifyMeasurementParameterCommand::ReadContinuous { address, r_index, param_index } => { write!(buffer, "{}IR{}_{:03}", address, r_index.value(), param_index.value())?; }
                     IdentifyMeasurementParameterCommand::ReadContinuousCRC { address, r_index, param_index } => { write!(buffer, "{}IRC{}_{:03}", address, r_index.value(), param_index.value())?; }
                     IdentifyMeasurementParameterCommand::HighVolumeASCII { address, param_index } => { write!(buffer, "{}IHA_{:03}", address, param_index.value())?; }
                     IdentifyMeasurementParameterCommand::HighVolumeBinary { address, param_index } => { write!(buffer, "{}IHB_{:03}", address, param_index.value())?; }
                }
                 write!(buffer, "!")?;
            }

            #[cfg(feature = "alloc")]
            Command::ExtendedCommand { address, command_body } => {
                // Write the address first
                write!(buffer, "{}", address)?;

                // Check if there's enough space for the command body AND the trailing '!'
                // Use +1 for the '!' character.
                if buffer.remaining_capacity() < command_body.len() + 1 {
                    return Err(CommandFormatError::BufferOverflow);
                }

                // Write the command body (now safe capacity-wise)
                // Use try_push_str as it returns Result and works with ArrayString's capacity checks
                buffer.try_push_str(command_body)
                      .map_err(|_| CommandFormatError::BufferOverflow)?; // Should not fail if capacity check is right

                // Write the terminator (now safe capacity-wise)
                buffer.try_push('!')
                      .map_err(|_| CommandFormatError::BufferOverflow)?; // Should not fail
             }

        }
        Ok(buffer)
    }
}


// --- Metadata Sub-Enums ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifyMeasurementCommand {
    Measurement { address: Sdi12Addr, index: MeasurementIndex },
    MeasurementCRC { address: Sdi12Addr, index: MeasurementIndex },
    Verification { address: Sdi12Addr },
    ConcurrentMeasurement { address: Sdi12Addr, index: MeasurementIndex },
    ConcurrentMeasurementCRC { address: Sdi12Addr, index: MeasurementIndex },
    HighVolumeASCII { address: Sdi12Addr },
    HighVolumeBinary { address: Sdi12Addr },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifyMeasurementParameterCommand {
    Measurement { address: Sdi12Addr, m_index: MeasurementIndex, param_index: IdentifyParameterIndex },
    MeasurementCRC { address: Sdi12Addr, m_index: MeasurementIndex, param_index: IdentifyParameterIndex },
    Verification { address: Sdi12Addr, param_index: IdentifyParameterIndex },
    ConcurrentMeasurement { address: Sdi12Addr, c_index: MeasurementIndex, param_index: IdentifyParameterIndex },
    ConcurrentMeasurementCRC { address: Sdi12Addr, c_index: MeasurementIndex, param_index: IdentifyParameterIndex },
    ReadContinuous { address: Sdi12Addr, r_index: ContinuousIndex, param_index: IdentifyParameterIndex },
    ReadContinuousCRC { address: Sdi12Addr, r_index: ContinuousIndex, param_index: IdentifyParameterIndex },
    HighVolumeASCII { address: Sdi12Addr, param_index: IdentifyParameterIndex },
    HighVolumeBinary { address: Sdi12Addr, param_index: IdentifyParameterIndex },
}


// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::address::Sdi12Addr;

    #[cfg(feature = "alloc")]
    use alloc::string::ToString; // Needed for .to_string() on Command

    fn addr(c: char) -> Sdi12Addr { Sdi12Addr::new(c).unwrap() }

    #[test]
    fn test_measurement_index_validation() {
        assert!(MeasurementIndex::new(None).is_ok());
        assert!(MeasurementIndex::new(Some(1)).is_ok());
        assert!(MeasurementIndex::new(Some(9)).is_ok());
        assert!(matches!(MeasurementIndex::new(Some(0)), Err(CommandIndexError::MeasurementOutOfRange)));
        assert!(matches!(MeasurementIndex::new(Some(10)), Err(CommandIndexError::MeasurementOutOfRange)));
    }

    #[test]
    fn test_continuous_index_validation() {
        assert!(ContinuousIndex::new(0).is_ok());
        assert!(ContinuousIndex::new(9).is_ok());
        assert!(matches!(ContinuousIndex::new(10), Err(CommandIndexError::ContinuousOutOfRange)));
        assert!(ContinuousIndex::try_from(5).is_ok());
        assert!(ContinuousIndex::try_from(15).is_err());
    }

    #[test]
    fn test_data_index_validation() {
        assert!(DataIndex::new(0).is_ok());
        assert!(DataIndex::new(999).is_ok());
        assert!(matches!(DataIndex::new(1000), Err(CommandIndexError::DataOutOfRange)));
        assert!(DataIndex::try_from(123).is_ok());
        assert!(DataIndex::try_from(1000).is_err());
    }

    #[test]
    fn test_identify_param_index_validation() {
        assert!(IdentifyParameterIndex::new(1).is_ok());
        assert!(IdentifyParameterIndex::new(999).is_ok());
        assert!(matches!(IdentifyParameterIndex::new(0), Err(CommandIndexError::IdentifyParamOutOfRange)));
        assert!(matches!(IdentifyParameterIndex::new(1000), Err(CommandIndexError::IdentifyParamOutOfRange)));
        assert!(IdentifyParameterIndex::try_from(456).is_ok());
        assert!(IdentifyParameterIndex::try_from(1000).is_err());
    }

    #[test]
    fn test_command_construction() {
        let cmd = Command::StartConcurrentMeasurementCRC {
            address: addr('2'),
            index: MeasurementIndex::Indexed(3)
        };
        assert!(matches!(cmd, Command::StartConcurrentMeasurementCRC { .. }));
    }

    #[test]
    fn test_command_formatting_standard() {
        // Basic
        assert_eq!(Command::AcknowledgeActive { address: addr('0') }.format_into().unwrap().as_str(), "0!");
        assert_eq!(Command::SendIdentification { address: addr('1') }.format_into().unwrap().as_str(), "1I!");
        assert_eq!(Command::AddressQuery.format_into().unwrap().as_str(), "?!");
        assert_eq!(Command::ChangeAddress { address: addr('2'), new_address: addr('3') }.format_into().unwrap().as_str(), "2A3!");
        // Measurement
        assert_eq!(Command::StartMeasurement { address: addr('4'), index: MeasurementIndex::Base }.format_into().unwrap().as_str(), "4M!");
        assert_eq!(Command::StartMeasurement { address: addr('5'), index: MeasurementIndex::Indexed(1) }.format_into().unwrap().as_str(), "5M1!");
        assert_eq!(Command::StartMeasurementCRC { address: addr('6'), index: MeasurementIndex::Base }.format_into().unwrap().as_str(), "6MC!");
        assert_eq!(Command::StartMeasurementCRC { address: addr('7'), index: MeasurementIndex::Indexed(9) }.format_into().unwrap().as_str(), "7MC9!");
        // Concurrent
        assert_eq!(Command::StartConcurrentMeasurement { address: addr('8'), index: MeasurementIndex::Base }.format_into().unwrap().as_str(), "8C!");
        assert_eq!(Command::StartConcurrentMeasurement { address: addr('9'), index: MeasurementIndex::Indexed(2) }.format_into().unwrap().as_str(), "9C2!");
        assert_eq!(Command::StartConcurrentMeasurementCRC { address: addr('a'), index: MeasurementIndex::Base }.format_into().unwrap().as_str(), "aCC!");
        assert_eq!(Command::StartConcurrentMeasurementCRC { address: addr('b'), index: MeasurementIndex::Indexed(8) }.format_into().unwrap().as_str(), "bCC8!");
        // Data / Continuous
        assert_eq!(Command::SendData { address: addr('c'), index: DataIndex::new(0).unwrap() }.format_into().unwrap().as_str(), "cD0!");
        assert_eq!(Command::SendData { address: addr('d'), index: DataIndex::new(9).unwrap() }.format_into().unwrap().as_str(), "dD9!");
        assert_eq!(Command::SendData { address: addr('e'), index: DataIndex::new(10).unwrap() }.format_into().unwrap().as_str(), "eD10!");
        assert_eq!(Command::SendData { address: addr('f'), index: DataIndex::new(999).unwrap() }.format_into().unwrap().as_str(), "fD999!");
        assert_eq!(Command::SendBinaryData { address: addr('A'), index: DataIndex::new(123).unwrap() }.format_into().unwrap().as_str(), "ADB123!");
        assert_eq!(Command::ReadContinuous { address: addr('B'), index: ContinuousIndex::new(0).unwrap() }.format_into().unwrap().as_str(), "BR0!");
        assert_eq!(Command::ReadContinuous { address: addr('C'), index: ContinuousIndex::new(9).unwrap() }.format_into().unwrap().as_str(), "CR9!");
        assert_eq!(Command::ReadContinuousCRC { address: addr('D'), index: ContinuousIndex::new(5).unwrap() }.format_into().unwrap().as_str(), "DRC5!");
        // Other Basic
        assert_eq!(Command::StartVerification { address: addr('E') }.format_into().unwrap().as_str(), "EV!");
        // High Volume
        assert_eq!(Command::StartHighVolumeASCII { address: addr('F') }.format_into().unwrap().as_str(), "FHA!");
        assert_eq!(Command::StartHighVolumeBinary { address: addr('G') }.format_into().unwrap().as_str(), "GHB!");
        // Metadata - Identify Measurement
        assert_eq!(Command::IdentifyMeasurement(IdentifyMeasurementCommand::Measurement { address: addr('H'), index: MeasurementIndex::Base }).format_into().unwrap().as_str(), "HIM!");
        assert_eq!(Command::IdentifyMeasurement(IdentifyMeasurementCommand::MeasurementCRC { address: addr('I'), index: MeasurementIndex::Indexed(3) }).format_into().unwrap().as_str(), "IIMC3!");
        assert_eq!(Command::IdentifyMeasurement(IdentifyMeasurementCommand::Verification { address: addr('J') }).format_into().unwrap().as_str(), "JIV!");
        assert_eq!(Command::IdentifyMeasurement(IdentifyMeasurementCommand::ConcurrentMeasurement { address: addr('K'), index: MeasurementIndex::Indexed(5) }).format_into().unwrap().as_str(), "KIC5!");
        assert_eq!(Command::IdentifyMeasurement(IdentifyMeasurementCommand::ConcurrentMeasurementCRC { address: addr('L'), index: MeasurementIndex::Base }).format_into().unwrap().as_str(), "LICC!");
        assert_eq!(Command::IdentifyMeasurement(IdentifyMeasurementCommand::HighVolumeASCII { address: addr('M') }).format_into().unwrap().as_str(), "MIHA!");
        assert_eq!(Command::IdentifyMeasurement(IdentifyMeasurementCommand::HighVolumeBinary { address: addr('N') }).format_into().unwrap().as_str(), "NIHB!");
        // Metadata - Identify Parameter
        assert_eq!(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::Measurement { address: addr('O'), m_index: MeasurementIndex::Base, param_index: IdentifyParameterIndex::new(1).unwrap() }).format_into().unwrap().as_str(), "OIM_001!");
        assert_eq!(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::MeasurementCRC { address: addr('P'), m_index: MeasurementIndex::Indexed(7), param_index: IdentifyParameterIndex::new(12).unwrap() }).format_into().unwrap().as_str(), "PIMC7_012!");
        assert_eq!(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::Verification { address: addr('Q'), param_index: IdentifyParameterIndex::new(345).unwrap() }).format_into().unwrap().as_str(), "QIV_345!");
        assert_eq!(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ConcurrentMeasurement { address: addr('R'), c_index: MeasurementIndex::Indexed(9), param_index: IdentifyParameterIndex::new(999).unwrap() }).format_into().unwrap().as_str(), "RIC9_999!");
        assert_eq!(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ConcurrentMeasurementCRC { address: addr('S'), c_index: MeasurementIndex::Base, param_index: IdentifyParameterIndex::new(50).unwrap() }).format_into().unwrap().as_str(), "SICC_050!");
        assert_eq!(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ReadContinuous { address: addr('T'), r_index: ContinuousIndex::new(0).unwrap(), param_index: IdentifyParameterIndex::new(1).unwrap() }).format_into().unwrap().as_str(), "TIR0_001!");
        assert_eq!(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ReadContinuousCRC { address: addr('U'), r_index: ContinuousIndex::new(8).unwrap(), param_index: IdentifyParameterIndex::new(2).unwrap() }).format_into().unwrap().as_str(), "UIRC8_002!");
        assert_eq!(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::HighVolumeASCII { address: addr('V'), param_index: IdentifyParameterIndex::new(100).unwrap() }).format_into().unwrap().as_str(), "VIHA_100!");
        assert_eq!(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::HighVolumeBinary { address: addr('W'), param_index: IdentifyParameterIndex::new(10).unwrap() }).format_into().unwrap().as_str(), "WIHB_010!");
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_format_extended_command() {
        // Test successful formatting within capacity
        let cmd_short = Command::ExtendedCommand { address: addr('X'), command_body: "YZ".to_string() }; // Note: body does NOT include '!' here
        let expected_short = "XYZ!";
        let formatted_short = cmd_short.format_into().unwrap();
        assert_eq!(formatted_short.as_str(), expected_short);

        // Test exact capacity fill
        let cmd_exact = Command::ExtendedCommand { address: addr('A'), command_body: "BCDEFGHI".to_string() }; // 1 + 8 + 1 = 10 chars
        let formatted_exact = cmd_exact.format_into().unwrap();
        assert_eq!(formatted_exact.as_str(), "ABCDEFGHI!");

        // Test overflow
        let cmd_long = Command::ExtendedCommand { address: addr('A'), command_body: "BCDEFGHIJ".to_string() }; // 1 + 9 + 1 = 11 chars
        let formatted_long_result = cmd_long.format_into();
        assert!(matches!(formatted_long_result, Err(CommandFormatError::BufferOverflow)));
    }

    #[test]
    fn test_format_error_from_fmt() {
        // This test is a bit artificial, as ArrayString formatting itself rarely fails with FmtError
        // unless the Display impl of a component fails, but we can check the From impl.
        let fmt_err = core::fmt::Error;
        let cmd_fmt_err: CommandFormatError = fmt_err.into();
        assert_eq!(cmd_fmt_err, CommandFormatError::FmtError);
    }
}