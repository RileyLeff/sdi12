// src/sensor/parser.rs

use crate::common::{
    address::Sdi12Addr,
    command::{
        Command, CommandIndexError, ContinuousIndex, DataIndex, IdentifyMeasurementCommand,
        IdentifyMeasurementParameterCommand, IdentifyParameterIndex, MeasurementIndex,
    },
    error::Sdi12Error,
};

use core::str;

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::string::ToString; // Needed for to_string()


/// Parses a raw SDI-12 command byte sequence into a structured Command enum.
///
/// Expects input bytes starting with the address and ending with the '!' terminator.
/// Does not handle the initial break or serial timing/framing.
///
/// # Arguments
///
/// * `bytes`: A byte slice containing the raw command (e.g., `b"1M!"`, `b"0D0!"`, `b"aRC5_123!"`).
///
/// # Returns
///
/// * `Ok(Command)`: If the byte sequence represents a valid SDI-12 command.
/// * `Err(Sdi12Error<()>)`: If parsing fails due to invalid format, address, index, etc.
pub fn parse_command(bytes: &[u8]) -> Result<Command, Sdi12Error<()>> {
    // --- Basic Validation ---
    if bytes.len() < 2 {
        return Err(Sdi12Error::InvalidFormat); // Need at least 'a!' or '?!'
    }
    if bytes[bytes.len() - 1] != b'!' {
        return Err(Sdi12Error::InvalidFormat); // Must end with '!'
    }

    let address_char = bytes[0] as char;
    let address = Sdi12Addr::new(address_char)?; // Returns InvalidAddress error if needed

    // Command body excludes address and '!'
    let body = &bytes[1..bytes.len() - 1];

    // Handle Address Query (?!) separately
    if address.is_query() {
        if body.is_empty() {
            return Ok(Command::AddressQuery);
        } else {
            return Err(Sdi12Error::InvalidFormat); // "?..." is invalid, only "?!"
        }
    }

    // --- Parse Command Body ---
    // Convert body to str for easier matching (assuming printable ASCII as per spec)
    let body_str = str::from_utf8(body).map_err(|_| Sdi12Error::InvalidFormat)?;

    match body_str {
        // --- Basic Commands ---
        "" => Ok(Command::AcknowledgeActive { address }),
        "I" => Ok(Command::SendIdentification { address }),
        "V" => Ok(Command::StartVerification { address }),
        "HA" => Ok(Command::StartHighVolumeASCII { address }),
        "HB" => Ok(Command::StartHighVolumeBinary { address }),

        // Change Address: aAb!
        body if body.starts_with('A') && body.len() == 2 => {
            let new_addr_char = body.chars().nth(1).unwrap(); // Safe due to len check
            let new_address = Sdi12Addr::new(new_addr_char)?;
            Ok(Command::ChangeAddress { address, new_address })
        }

        // Measurement: aM[n]! / aMC[n]! / aC[n]! / aCC[n]!
        body if body.starts_with('M') || body.starts_with('C') => {
            parse_measurement_command(address, body_str)
        }

        // Send Data: aD[n]! / aDB[n]! (n = 0-999)
        body if body.starts_with('D') => parse_data_command(address, body_str),

        // Read Continuous: aR[n]! / aRC[n]! (n = 0-9)
        body if body.starts_with('R') => parse_continuous_command(address, body_str),

        // --- Metadata Commands ---
        // Identify Measurement: aIM[n]! / aIV! / aIC[n]! / aIHA! / etc.
        body if body.starts_with('I') => parse_identify_command(address, body_str),

        // Extended Command (Fallback, requires 'alloc')
        #[cfg(feature = "alloc")]
        _ => {
            // Check for valid extended command characters if needed (spec doesn't strictly limit)
            // For now, accept any non-empty body not matched above as extended
            if body_str.is_empty() {
                 Err(Sdi12Error::InvalidFormat) // Should have been caught by "" case
            } else {
                 Ok(Command::ExtendedCommand { address, command_body: body_str.to_string() })
            }
        }
        #[cfg(not(feature = "alloc"))]
        _ => Err(Sdi12Error::InvalidFormat), // Or a specific "ExtendedCommandNotSupported" error?
    }
}

// --- Helper: Parse M/MC/C/CC commands ---
fn parse_measurement_command(
    address: Sdi12Addr,
    body: &str,
) -> Result<Command, Sdi12Error<()>> {
    let (cmd_code, index_str) = match body.len() {
        1 => (body, None), // M, C
        2 => {
             // MC, CC, M1-9, C1-9
            let code_part = &body[..body.len() - 1]; // M, C, MC, CC
            let index_part = &body[body.len() - 1..];
            if index_part.chars().all(|c| c.is_ascii_digit()) {
                (code_part, Some(index_part))
            } else {
                 // Must be MC or CC
                 (body, None)
            }
        }
        3 => {
            // MC1-9, CC1-9
             let code_part = &body[..body.len() - 1]; // MC, CC
             let index_part = &body[body.len() - 1..];
            if index_part.chars().all(|c| c.is_ascii_digit()) {
                (code_part, Some(index_part))
            } else {
                 return Err(Sdi12Error::InvalidFormat); // e.g., MCX
            }
        }
        _ => return Err(Sdi12Error::InvalidFormat),
    };

    let index_val = index_str
        .map(|s| s.parse::<u8>().map_err(|_| Sdi12Error::InvalidFormat)) // Invalid number format
        .transpose()?; // Convert Option<Result<u8, _>> to Result<Option<u8>, _>

    let index = MeasurementIndex::new(index_val)?; // Returns InvalidCommandIndex error

    match cmd_code {
        "M" => Ok(Command::StartMeasurement { address, index }),
        "MC" => Ok(Command::StartMeasurementCRC { address, index }),
        "C" => Ok(Command::StartConcurrentMeasurement { address, index }),
        "CC" => Ok(Command::StartConcurrentMeasurementCRC { address, index }),
        _ => Err(Sdi12Error::InvalidFormat),
    }
}

// --- Helper: Parse D/DB commands ---
fn parse_data_command(address: Sdi12Addr, body: &str) -> Result<Command, Sdi12Error<()>> {
    let (is_binary, index_str) = if body.starts_with("DB") {
        (true, &body[2..])
    } else if body.starts_with('D') {
        (false, &body[1..])
    } else {
        return Err(Sdi12Error::InvalidFormat); // Should not happen if called correctly
    };

    if index_str.is_empty() || !index_str.chars().all(|c| c.is_ascii_digit()) {
        return Err(Sdi12Error::InvalidFormat); // Need index digits
    }

    let index_val = index_str.parse::<u16>().map_err(|_| Sdi12Error::InvalidFormat)?;
    let index = DataIndex::new(index_val)?; // Returns InvalidCommandIndex

    if is_binary {
        Ok(Command::SendBinaryData { address, index })
    } else {
        Ok(Command::SendData { address, index })
    }
}

// --- Helper: Parse R/RC commands ---
fn parse_continuous_command(
    address: Sdi12Addr,
    body: &str,
) -> Result<Command, Sdi12Error<()>> {
    let (is_crc, index_str) = if body.starts_with("RC") {
        (true, &body[2..])
    } else if body.starts_with('R') {
        (false, &body[1..])
    } else {
        return Err(Sdi12Error::InvalidFormat);
    };

    if index_str.len() != 1 || !index_str.chars().all(|c| c.is_ascii_digit()) {
        return Err(Sdi12Error::InvalidFormat); // Needs exactly one index digit
    }

    let index_val = index_str.parse::<u8>().map_err(|_| Sdi12Error::InvalidFormat)?;
    let index = ContinuousIndex::new(index_val)?; // Returns InvalidCommandIndex

    if is_crc {
        Ok(Command::ReadContinuousCRC { address, index })
    } else {
        Ok(Command::ReadContinuous { address, index })
    }
}

// --- Helper: Parse Identify Measurement / Parameter commands ---
// Example formats: aIM!, aIMC1!, aIV!, aIC5!, aICC!, aIHA!, aIHB!
// Parameter: aIM_001!, aIMC1_010!, aIV_123!, aIC5_999!, aICC_001!, aIHA_050!, aIHB_001!
// Parameter Continuous: aIR0_001!, aIRC9_100!
fn parse_identify_command(
    address: Sdi12Addr,
    body: &str,
) -> Result<Command, Sdi12Error<()>> {
    // Separate main command part from optional parameter part (_nnn)
    let parts: Vec<&str> = body.splitn(2, '_').collect();
    let main_cmd_part = parts[0];
    let param_index_opt: Option<Result<IdentifyParameterIndex, Sdi12Error<()>>> =
        parts.get(1).map(|param_str| {
            if param_str.len() == 3 && param_str.chars().all(|c| c.is_ascii_digit()) {
                param_str.parse::<u16>()
                    .map_err(|_| Sdi12Error::InvalidFormat) // Should not happen with checks
                    .and_then(IdentifyParameterIndex::new) // Map CommandIndexError
            } else {
                 Err(Sdi12Error::InvalidFormat) // Parameter index format incorrect
            }
        });

    // Extract base command code (e.g., IM, IMC, IV, IC, ICC, IR, IRC, IHA, IHB) and measurement index if present
    let base_code;
    let index_opt_str;

    if main_cmd_part.starts_with("IM")
        || main_cmd_part.starts_with("IC")
        || main_cmd_part.starts_with("IR") // Handle IR/IRC here too
    {
        let potential_code_len = if main_cmd_part.starts_with("IRC") {
            3
        } else if main_cmd_part.starts_with("IMC") || main_cmd_part.starts_with("ICC") {
             3
        } else if main_cmd_part.starts_with("IM") || main_cmd_part.starts_with("IC") || main_cmd_part.starts_with("IR") {
             2
        } else {
             return Err(Sdi12Error::InvalidFormat); // Should start with I<Cmd>
        };

        if main_cmd_part.len() == potential_code_len {
            base_code = &main_cmd_part[..potential_code_len];
            index_opt_str = None;
        } else if main_cmd_part.len() == potential_code_len + 1 {
            base_code = &main_cmd_part[..potential_code_len];
            index_opt_str = Some(&main_cmd_part[potential_code_len..]);
             if !index_opt_str.unwrap().chars().all(|c| c.is_ascii_digit()) {
                 return Err(Sdi12Error::InvalidFormat); // Index must be digit
             }
        } else {
             return Err(Sdi12Error::InvalidFormat); // Invalid length
        }
    } else if main_cmd_part == "IV" || main_cmd_part == "IHA" || main_cmd_part == "IHB" {
        base_code = main_cmd_part;
        index_opt_str = None;
    } else {
        return Err(Sdi12Error::InvalidFormat); // Unrecognized Identify command start
    }

    // --- Build Specific Command Enum ---

    match param_index_opt {
        // --- Parameter Commands ---
        Some(Ok(param_index)) => {
            match base_code {
                 "IM" | "IMC" | "IC" | "ICC" => {
                    // Measurement/Concurrent Parameter
                    let m_index_val = index_opt_str
                        .map(|s| s.parse::<u8>().map_err(|_| Sdi12Error::InvalidFormat))
                        .transpose()?;
                    let m_index = MeasurementIndex::new(m_index_val)?;
                    match base_code {
                        "IM" => Ok(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::Measurement { address, m_index, param_index })),
                        "IMC" => Ok(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::MeasurementCRC { address, m_index, param_index })),
                        "IC" => Ok(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ConcurrentMeasurement { address, c_index: m_index, param_index })),
                        "ICC" => Ok(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ConcurrentMeasurementCRC { address, c_index: m_index, param_index })),
                        _ => unreachable!(),
                    }
                }
                 "IV" => {
                    if index_opt_str.is_some() { return Err(Sdi12Error::InvalidFormat); } // IV_nnn! doesn't have M index
                    Ok(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::Verification { address, param_index }))
                }
                "IR" | "IRC" => {
                    // Continuous Parameter
                    let r_index_val = index_opt_str
                         .ok_or(Sdi12Error::InvalidFormat)? // IR/IRC needs R index
                         .parse::<u8>().map_err(|_| Sdi12Error::InvalidFormat)?;
                    let r_index = ContinuousIndex::new(r_index_val)?;
                    match base_code {
                        "IR" => Ok(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ReadContinuous { address, r_index, param_index })),
                        "IRC" => Ok(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ReadContinuousCRC { address, r_index, param_index })),
                         _ => unreachable!(),
                    }
                }
                 "IHA" => {
                    if index_opt_str.is_some() { return Err(Sdi12Error::InvalidFormat); }
                    Ok(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::HighVolumeASCII { address, param_index }))
                }
                 "IHB" => {
                    if index_opt_str.is_some() { return Err(Sdi12Error::InvalidFormat); }
                    Ok(Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::HighVolumeBinary { address, param_index }))
                }
                _ => Err(Sdi12Error::InvalidFormat), // Unrecognized base code for parameter command
            }
        }
        Some(Err(e)) => Err(e), // Parameter parsing failed

        // --- Measurement Commands (No Parameter Index) ---
        None => {
            match base_code {
                "IM" | "IMC" | "IC" | "ICC" => {
                     // Measurement/Concurrent Identify
                    let index_val = index_opt_str
                        .map(|s| s.parse::<u8>().map_err(|_| Sdi12Error::InvalidFormat))
                        .transpose()?;
                    let index = MeasurementIndex::new(index_val)?;
                     match base_code {
                        "IM" => Ok(Command::IdentifyMeasurement(IdentifyMeasurementCommand::Measurement { address, index })),
                        "IMC" => Ok(Command::IdentifyMeasurement(IdentifyMeasurementCommand::MeasurementCRC { address, index })),
                        "IC" => Ok(Command::IdentifyMeasurement(IdentifyMeasurementCommand::ConcurrentMeasurement { address, index })),
                        "ICC" => Ok(Command::IdentifyMeasurement(IdentifyMeasurementCommand::ConcurrentMeasurementCRC { address, index })),
                        _ => unreachable!(),
                    }
                }
                 "IV" => {
                    if index_opt_str.is_some() { return Err(Sdi12Error::InvalidFormat); }
                    Ok(Command::IdentifyMeasurement(IdentifyMeasurementCommand::Verification { address }))
                }
                 "IHA" => {
                    if index_opt_str.is_some() { return Err(Sdi12Error::InvalidFormat); }
                    Ok(Command::IdentifyMeasurement(IdentifyMeasurementCommand::HighVolumeASCII { address }))
                }
                 "IHB" => {
                    if index_opt_str.is_some() { return Err(Sdi12Error::InvalidFormat); }
                    Ok(Command::IdentifyMeasurement(IdentifyMeasurementCommand::HighVolumeBinary { address }))
                }
                // IR/IRC without parameter index is invalid
                 "IR" | "IRC" => Err(Sdi12Error::InvalidFormat),
                 _ => Err(Sdi12Error::InvalidFormat), // Unrecognized base code for measurement command
            }
        }
    }
}


// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::address::Sdi12Addr; // Need this for addr helper
    use crate::common::command::CommandFormatError; // Need for mapping test

    fn addr(c: char) -> Sdi12Addr { Sdi12Addr::new(c).unwrap() }

    #[test]
    fn test_parse_basic_commands() {
        assert_eq!(parse_command(b"0!").unwrap(), Command::AcknowledgeActive { address: addr('0') });
        assert_eq!(parse_command(b"1I!").unwrap(), Command::SendIdentification { address: addr('1') });
        assert_eq!(parse_command(b"?!").unwrap(), Command::AddressQuery);
        assert_eq!(parse_command(b"2A3!").unwrap(), Command::ChangeAddress { address: addr('2'), new_address: addr('3') });
        assert_eq!(parse_command(b"4V!").unwrap(), Command::StartVerification { address: addr('4') });
        assert_eq!(parse_command(b"5HA!").unwrap(), Command::StartHighVolumeASCII { address: addr('5') });
        assert_eq!(parse_command(b"6HB!").unwrap(), Command::StartHighVolumeBinary { address: addr('6') });
    }

    #[test]
    fn test_parse_measurement_commands() {
        // M
        assert_eq!(parse_command(b"0M!").unwrap(), Command::StartMeasurement { address: addr('0'), index: MeasurementIndex::Base });
        assert_eq!(parse_command(b"1M1!").unwrap(), Command::StartMeasurement { address: addr('1'), index: MeasurementIndex::Indexed(1) });
        assert_eq!(parse_command(b"2M9!").unwrap(), Command::StartMeasurement { address: addr('2'), index: MeasurementIndex::Indexed(9) });
        // MC
        assert_eq!(parse_command(b"3MC!").unwrap(), Command::StartMeasurementCRC { address: addr('3'), index: MeasurementIndex::Base });
        assert_eq!(parse_command(b"4MC1!").unwrap(), Command::StartMeasurementCRC { address: addr('4'), index: MeasurementIndex::Indexed(1) });
        assert_eq!(parse_command(b"5MC9!").unwrap(), Command::StartMeasurementCRC { address: addr('5'), index: MeasurementIndex::Indexed(9) });
        // C
        assert_eq!(parse_command(b"6C!").unwrap(), Command::StartConcurrentMeasurement { address: addr('6'), index: MeasurementIndex::Base });
        assert_eq!(parse_command(b"7C1!").unwrap(), Command::StartConcurrentMeasurement { address: addr('7'), index: MeasurementIndex::Indexed(1) });
        assert_eq!(parse_command(b"8C9!").unwrap(), Command::StartConcurrentMeasurement { address: addr('8'), index: MeasurementIndex::Indexed(9) });
        // CC
        assert_eq!(parse_command(b"9CC!").unwrap(), Command::StartConcurrentMeasurementCRC { address: addr('9'), index: MeasurementIndex::Base });
        assert_eq!(parse_command(b"aCC1!").unwrap(), Command::StartConcurrentMeasurementCRC { address: addr('a'), index: MeasurementIndex::Indexed(1) });
        assert_eq!(parse_command(b"bCC9!").unwrap(), Command::StartConcurrentMeasurementCRC { address: addr('b'), index: MeasurementIndex::Indexed(9) });
    }

     #[test]
    fn test_parse_data_commands() {
        // D
        assert_eq!(parse_command(b"0D0!").unwrap(), Command::SendData { address: addr('0'), index: DataIndex::new(0).unwrap() });
        assert_eq!(parse_command(b"1D9!").unwrap(), Command::SendData { address: addr('1'), index: DataIndex::new(9).unwrap() });
        assert_eq!(parse_command(b"2D10!").unwrap(), Command::SendData { address: addr('2'), index: DataIndex::new(10).unwrap() });
        assert_eq!(parse_command(b"3D999!").unwrap(), Command::SendData { address: addr('3'), index: DataIndex::new(999).unwrap() });
        // DB
        assert_eq!(parse_command(b"4DB0!").unwrap(), Command::SendBinaryData { address: addr('4'), index: DataIndex::new(0).unwrap() });
        assert_eq!(parse_command(b"5DB123!").unwrap(), Command::SendBinaryData { address: addr('5'), index: DataIndex::new(123).unwrap() });
        assert_eq!(parse_command(b"6DB999!").unwrap(), Command::SendBinaryData { address: addr('6'), index: DataIndex::new(999).unwrap() });
    }

    #[test]
    fn test_parse_continuous_commands() {
         // R
        assert_eq!(parse_command(b"0R0!").unwrap(), Command::ReadContinuous { address: addr('0'), index: ContinuousIndex::new(0).unwrap() });
        assert_eq!(parse_command(b"1R9!").unwrap(), Command::ReadContinuous { address: addr('1'), index: ContinuousIndex::new(9).unwrap() });
        // RC
        assert_eq!(parse_command(b"2RC0!").unwrap(), Command::ReadContinuousCRC { address: addr('2'), index: ContinuousIndex::new(0).unwrap() });
        assert_eq!(parse_command(b"3RC9!").unwrap(), Command::ReadContinuousCRC { address: addr('3'), index: ContinuousIndex::new(9).unwrap() });
    }

    #[test]
    fn test_parse_identify_measurement_commands() {
        assert_eq!(parse_command(b"0IM!").unwrap(), Command::IdentifyMeasurement(IdentifyMeasurementCommand::Measurement { address: addr('0'), index: MeasurementIndex::Base }));
        assert_eq!(parse_command(b"1IM1!").unwrap(), Command::IdentifyMeasurement(IdentifyMeasurementCommand::Measurement { address: addr('1'), index: MeasurementIndex::Indexed(1) }));
        assert_eq!(parse_command(b"2IMC!").unwrap(), Command::IdentifyMeasurement(IdentifyMeasurementCommand::MeasurementCRC { address: addr('2'), index: MeasurementIndex::Base }));
        assert_eq!(parse_command(b"3IMC9!").unwrap(), Command::IdentifyMeasurement(IdentifyMeasurementCommand::MeasurementCRC { address: addr('3'), index: MeasurementIndex::Indexed(9) }));
        assert_eq!(parse_command(b"4IV!").unwrap(), Command::IdentifyMeasurement(IdentifyMeasurementCommand::Verification { address: addr('4') }));
        assert_eq!(parse_command(b"5IC!").unwrap(), Command::IdentifyMeasurement(IdentifyMeasurementCommand::ConcurrentMeasurement { address: addr('5'), index: MeasurementIndex::Base }));
        assert_eq!(parse_command(b"6IC2!").unwrap(), Command::IdentifyMeasurement(IdentifyMeasurementCommand::ConcurrentMeasurement { address: addr('6'), index: MeasurementIndex::Indexed(2) }));
        assert_eq!(parse_command(b"7ICC!").unwrap(), Command::IdentifyMeasurement(IdentifyMeasurementCommand::ConcurrentMeasurementCRC { address: addr('7'), index: MeasurementIndex::Base }));
        assert_eq!(parse_command(b"8ICC8!").unwrap(), Command::IdentifyMeasurement(IdentifyMeasurementCommand::ConcurrentMeasurementCRC { address: addr('8'), index: MeasurementIndex::Indexed(8) }));
        assert_eq!(parse_command(b"9IHA!").unwrap(), Command::IdentifyMeasurement(IdentifyMeasurementCommand::HighVolumeASCII { address: addr('9') }));
        assert_eq!(parse_command(b"aIHB!").unwrap(), Command::IdentifyMeasurement(IdentifyMeasurementCommand::HighVolumeBinary { address: addr('a') }));
    }

     #[test]
    fn test_parse_identify_parameter_commands() {
        assert_eq!(parse_command(b"0IM_001!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::Measurement { address: addr('0'), m_index: MeasurementIndex::Base, param_index: IdentifyParameterIndex::new(1).unwrap() }));
        assert_eq!(parse_command(b"1IM1_010!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::Measurement { address: addr('1'), m_index: MeasurementIndex::Indexed(1), param_index: IdentifyParameterIndex::new(10).unwrap() }));
        assert_eq!(parse_command(b"2IMC_999!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::MeasurementCRC { address: addr('2'), m_index: MeasurementIndex::Base, param_index: IdentifyParameterIndex::new(999).unwrap() }));
        assert_eq!(parse_command(b"3IMC9_001!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::MeasurementCRC { address: addr('3'), m_index: MeasurementIndex::Indexed(9), param_index: IdentifyParameterIndex::new(1).unwrap() }));
        assert_eq!(parse_command(b"4IV_123!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::Verification { address: addr('4'), param_index: IdentifyParameterIndex::new(123).unwrap() }));
        assert_eq!(parse_command(b"5IC_050!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ConcurrentMeasurement { address: addr('5'), c_index: MeasurementIndex::Base, param_index: IdentifyParameterIndex::new(50).unwrap() }));
        assert_eq!(parse_command(b"6IC2_002!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ConcurrentMeasurement { address: addr('6'), c_index: MeasurementIndex::Indexed(2), param_index: IdentifyParameterIndex::new(2).unwrap() }));
        assert_eq!(parse_command(b"7ICC_001!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ConcurrentMeasurementCRC { address: addr('7'), c_index: MeasurementIndex::Base, param_index: IdentifyParameterIndex::new(1).unwrap() }));
        assert_eq!(parse_command(b"8ICC8_100!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ConcurrentMeasurementCRC { address: addr('8'), c_index: MeasurementIndex::Indexed(8), param_index: IdentifyParameterIndex::new(100).unwrap() }));
        assert_eq!(parse_command(b"9IR0_001!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ReadContinuous { address: addr('9'), r_index: ContinuousIndex::new(0).unwrap(), param_index: IdentifyParameterIndex::new(1).unwrap() }));
        assert_eq!(parse_command(b"aIR9_999!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ReadContinuous { address: addr('a'), r_index: ContinuousIndex::new(9).unwrap(), param_index: IdentifyParameterIndex::new(999).unwrap() }));
        assert_eq!(parse_command(b"bIRC0_002!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ReadContinuousCRC { address: addr('b'), r_index: ContinuousIndex::new(0).unwrap(), param_index: IdentifyParameterIndex::new(2).unwrap() }));
        assert_eq!(parse_command(b"cIRC9_010!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::ReadContinuousCRC { address: addr('c'), r_index: ContinuousIndex::new(9).unwrap(), param_index: IdentifyParameterIndex::new(10).unwrap() }));
        assert_eq!(parse_command(b"dIHA_001!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::HighVolumeASCII { address: addr('d'), param_index: IdentifyParameterIndex::new(1).unwrap() }));
        assert_eq!(parse_command(b"eIHB_999!").unwrap(), Command::IdentifyMeasurementParameter(IdentifyMeasurementParameterCommand::HighVolumeBinary { address: addr('e'), param_index: IdentifyParameterIndex::new(999).unwrap() }));
    }


    #[test]
    #[cfg(feature = "alloc")]
    fn test_parse_extended_commands() {
        assert_eq!(parse_command(b"0XABC!").unwrap(), Command::ExtendedCommand { address: addr('0'), command_body: "XABC".to_string() });
        assert_eq!(parse_command(b"1SOME_CMD_123!").unwrap(), Command::ExtendedCommand { address: addr('1'), command_body: "SOME_CMD_123".to_string() });
    }

    #[test]
    fn test_parse_invalid_formats() {
        // Basic structure
        assert!(matches!(parse_command(b""), Err(Sdi12Error::InvalidFormat)));
        assert!(matches!(parse_command(b"0"), Err(Sdi12Error::InvalidFormat))); // Missing !
        assert!(matches!(parse_command(b"!"), Err(Sdi12Error::InvalidAddress('!')))); // Invalid address
        assert!(matches!(parse_command(b"0M"), Err(Sdi12Error::InvalidFormat))); // Missing !
        assert!(matches!(parse_command(b"?A!"), Err(Sdi12Error::InvalidFormat))); // Query cannot have body

        // Address
        assert!(matches!(parse_command(b"$!"), Err(Sdi12Error::InvalidAddress('$'))));
        assert!(matches!(parse_command(b"_M!"), Err(Sdi12Error::InvalidAddress('_'))));

        // Command Codes
        assert!(matches!(parse_command(b"0Q!"), Err(Sdi12Error::InvalidFormat))); // Unknown command Q
        assert!(matches!(parse_command(b"1MA!"), Err(Sdi12Error::InvalidFormat))); // Invalid char after M
        assert!(matches!(parse_command(b"2MCC!"), Err(Sdi12Error::InvalidFormat))); // Double C
        assert!(matches!(parse_command(b"3DA!"), Err(Sdi12Error::InvalidFormat))); // D needs digits
        assert!(matches!(parse_command(b"4D!"), Err(Sdi12Error::InvalidFormat))); // D needs digits
        assert!(matches!(parse_command(b"5R!"), Err(Sdi12Error::InvalidFormat))); // R needs digit
        assert!(matches!(parse_command(b"6RA!"), Err(Sdi12Error::InvalidFormat))); // R needs digit
        assert!(matches!(parse_command(b"7RC!"), Err(Sdi12Error::InvalidFormat))); // RC needs digit

        // Indices
        assert!(matches!(parse_command(b"0M0!"), Err(Sdi12Error::InvalidCommandIndex(CommandIndexError::MeasurementOutOfRange))));
        assert!(matches!(parse_command(b"1M10!"), Err(Sdi12Error::InvalidCommandIndex(CommandIndexError::MeasurementOutOfRange))));
        assert!(matches!(parse_command(b"2D1000!"), Err(Sdi12Error::InvalidCommandIndex(CommandIndexError::DataOutOfRange))));
        assert!(matches!(parse_command(b"3R10!"), Err(Sdi12Error::InvalidFormat))); // R has only 1 digit index
        assert!(matches!(parse_command(b"4RC10!"), Err(Sdi12Error::InvalidFormat))); // RC has only 1 digit index
        assert!(matches!(parse_command(b"5IM0!"), Err(Sdi12Error::InvalidCommandIndex(CommandIndexError::MeasurementOutOfRange))));
        assert!(matches!(parse_command(b"6IM_000!"), Err(Sdi12Error::InvalidCommandIndex(CommandIndexError::IdentifyParamOutOfRange))));
        assert!(matches!(parse_command(b"7IM_1000!"), Err(Sdi12Error::InvalidCommandIndex(CommandIndexError::IdentifyParamOutOfRange))));
        assert!(matches!(parse_command(b"8IM_12!"), Err(Sdi12Error::InvalidFormat))); // Parameter index must be 3 digits
        assert!(matches!(parse_command(b"9IM_ABC!"), Err(Sdi12Error::InvalidFormat))); // Parameter index must be digits

        // UTF8 error (though spec requires printable ASCII)
        assert!(matches!(parse_command(&[b'0', 0xE2, 0x82, 0xAC, b'!']), Err(Sdi12Error::InvalidFormat))); // Euro sign €

        // Extended command without alloc
        #[cfg(not(feature = "alloc"))]
        assert!(matches!(parse_command(b"0XABC!"), Err(Sdi12Error::InvalidFormat)));
    }

    // Test that CommandIndexError maps correctly (via From trait in error.rs)
    #[test]
    fn test_index_error_mapping() {
         // Simulate a failure during index parsing/validation
         let result = IdentifyParameterIndex::new(1000); // This returns CommandIndexError
         assert!(result.is_err());
         let index_err = result.err().unwrap();

         // Now use this in a context where it would be mapped to Sdi12Error
         let sdi12_err: Sdi12Error<()> = index_err.into();

         assert_eq!(sdi12_err, Sdi12Error::InvalidCommandIndex(CommandIndexError::IdentifyParamOutOfRange));
    }

     // Test that CommandFormatError maps correctly (via From trait in error.rs)
     #[test]
     fn test_format_error_mapping() {
        // Simulate a formatting error (e.g., buffer overflow)
         let format_err = CommandFormatError::BufferOverflow;

         // Map it
         let sdi12_err: Sdi12Error<()> = format_err.into();

         assert_eq!(sdi12_err, Sdi12Error::CommandFormatFailed(CommandFormatError::BufferOverflow));
     }
}