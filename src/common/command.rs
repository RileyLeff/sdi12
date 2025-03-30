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
}


impl Command {
    const MAX_FORMATTED_LEN: usize = 10; // aICC9_999!

    /// Formats the command into the standard byte sequence (e.g., "0M!", "1D10!") including the '!'.
    /// Writes into a fixed-size buffer (ArrayString) to avoid allocation.
    pub fn format_into(&self) -> Result<ArrayString<[u8; {Self::MAX_FORMATTED_LEN}]>, CommandFormatError> { // Used {{N}} as suggested by compiler error
        let mut buffer = ArrayString::<[u8; {Self::MAX_FORMATTED_LEN}]>::new(); // Used {{N}}

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
                 write!(buffer, "{}{}", address, command_body)?;
                 if buffer.len() >= buffer.capacity() { return Err(CommandFormatError::BufferOverflow); }
                 buffer.try_push('!').map_err(|_| CommandFormatError::BufferOverflow)?;
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
    use alloc::string::ToString;

    fn addr(c: char) -> Sdi12Addr { Sdi12Addr::new(c).unwrap() }

    #[test] fn test_measurement_index_validation() { /* unchanged */ }
    #[test] fn test_continuous_index_validation() { /* unchanged */ }
    #[test] fn test_data_index_validation() { /* unchanged */ }
    #[test] fn test_identify_param_index_validation() { /* unchanged */ }
    #[test] fn test_command_construction() { /* unchanged */ }
    #[test] fn test_command_formatting() { /* unchanged */ }
    #[test] #[cfg(feature = "alloc")] fn test_format_extended_command() { /* unchanged */ }
}