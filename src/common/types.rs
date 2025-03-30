// src/common/types.rs

use core::fmt;
use core::str::FromStr; // For parsing strings to numbers

// --- SDI-12 Standard Data Value (`<values>`) ---

/// Represents a single data value as returned in the `<values>` part of D or R commands.
/// Format: `p[d.d]` where p is '+' or '-', d are digits, '.' is optional. Max 7 digits. Max 9 chars total.
///
/// We store it internally potentially as a scaled integer or a float, depending on needs.
/// Using f32 might be simplest for representation, but parsing needs care.
/// Alternatively, parse into integer + scale factor. Let's try f32 for now.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Sdi12Value(f32); // Store as f32 for simplicity

impl Sdi12Value {
    /// Creates a new Sdi12Value.
    pub fn new(value: f32) -> Self {
        // TODO: Potentially add checks/clamping based on SDI-12 format limits?
        // The format itself limits precision/range implicitly.
        Self(value)
    }

    /// Returns the value as f32.
    pub fn as_f32(&self) -> f32 {
        self.0
    }

    /// Parses a single value string (like "+1.23", "-10", "+1234567") into an Sdi12Value.
    /// Does not handle multiple values in one string.
    pub fn parse_single(s: &str) -> Result<Self, Sdi12ParsingError> {
        // Validate basic structure and length (max 9 chars: sign + 7 digits + opt decimal)
        if s.is_empty() || s.len() > 9 {
            return Err(Sdi12ParsingError::InvalidFormat);
        }
        let mut chars = s.chars();
        let sign_char = chars.next().ok_or(Sdi12ParsingError::InvalidFormat)?;
        let sign = match sign_char {
            '+' => 1.0,
            '-' => -1.0,
            _ => return Err(Sdi12ParsingError::InvalidSign),
        };

        let rest = chars.as_str();
        // Validate remaining chars are digits or a single '.'
        let mut decimal_found = false;
        let mut digit_count = 0;
        for c in rest.chars() {
            match c {
                '0'..='9' => digit_count += 1,
                '.' => {
                    if decimal_found { return Err(Sdi12ParsingError::MultipleDecimals); }
                    decimal_found = true;
                }
                _ => return Err(Sdi12ParsingError::InvalidCharacter),
            }
        }
        if digit_count == 0 || digit_count > 7 {
            return Err(Sdi12ParsingError::InvalidDigitCount);
        }

        // Attempt to parse the numeric part (without sign)
        let num_part = f32::from_str(rest).map_err(|_| Sdi12ParsingError::ParseFloatError)?;

        Ok(Self(sign * num_part))
    }

    // TODO: Implement formatting logic later if needed (e.g., for sensor implementation)
    // pub fn format(&self, buffer: &mut [u8]) -> Result<usize, Sdi12FormattingError> { ... }
}

/// Error during parsing of SDI-12 <values>.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Sdi12ParsingError {
    InvalidFormat,
    InvalidSign,
    MultipleDecimals,
    InvalidCharacter,
    InvalidDigitCount,
    ParseFloatError, // Error converting string part to float
}

impl fmt::Display for Sdi12ParsingError {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Sdi12ParsingError::*;
        match self {
            InvalidFormat => write!(f, "Invalid SDI-12 value format"),
            InvalidSign => write!(f, "Invalid or missing sign character"),
            MultipleDecimals => write!(f, "Multiple decimal points found"),
            InvalidCharacter => write!(f, "Invalid character in numeric part"),
            InvalidDigitCount => write!(f, "Invalid number of digits (must be 1-7)"),
            ParseFloatError => write!(f, "Failed to parse numeric part as float"),
        }
    }
}


// --- High Volume Binary Data Types (Sec 5.2.1, Table 16) ---

/// Data types used in High-Volume Binary command responses.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum BinaryDataType {
    InvalidRequest = 0, // Indicates an invalid DBn request index
    SignedI8 = 1,
    UnsignedU8 = 2,
    SignedI16 = 3,
    UnsignedU16 = 4,
    SignedI32 = 5,
    UnsignedU32 = 6,
    SignedI64 = 7,
    UnsignedU64 = 8,
    Float32 = 9, // IEEE 754 Single Precision
    Float64 = 10, // IEEE 754 Double Precision
}

impl BinaryDataType {
    /// Tries to convert a u8 into a BinaryDataType.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(BinaryDataType::InvalidRequest),
            1 => Some(BinaryDataType::SignedI8),
            2 => Some(BinaryDataType::UnsignedU8),
            3 => Some(BinaryDataType::SignedI16),
            4 => Some(BinaryDataType::UnsignedU16),
            5 => Some(BinaryDataType::SignedI32),
            6 => Some(BinaryDataType::UnsignedU32),
            7 => Some(BinaryDataType::SignedI64),
            8 => Some(BinaryDataType::UnsignedU64),
            9 => Some(BinaryDataType::Float32),
            10 => Some(BinaryDataType::Float64),
            _ => None,
        }
    }

    /// Returns the size in bytes of a single value of this data type.
    /// Returns 0 for InvalidRequest.
    pub fn size_in_bytes(&self) -> usize {
        match self {
            BinaryDataType::InvalidRequest => 0,
            BinaryDataType::SignedI8 => 1,
            BinaryDataType::UnsignedU8 => 1,
            BinaryDataType::SignedI16 => 2,
            BinaryDataType::UnsignedU16 => 2,
            BinaryDataType::SignedI32 => 4,
            BinaryDataType::UnsignedU32 => 4,
            BinaryDataType::SignedI64 => 8,
            BinaryDataType::UnsignedU64 => 8,
            BinaryDataType::Float32 => 4,
            BinaryDataType::Float64 => 8,
        }
    }
}


// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdi12value_parsing_valid() {
        assert_eq!(Sdi12Value::parse_single("+1.23").unwrap(), Sdi12Value(1.23));
        assert_eq!(Sdi12Value::parse_single("-0.456").unwrap(), Sdi12Value(-0.456));
        assert_eq!(Sdi12Value::parse_single("+100").unwrap(), Sdi12Value(100.0));
        assert_eq!(Sdi12Value::parse_single("-5").unwrap(), Sdi12Value(-5.0));
        assert_eq!(Sdi12Value::parse_single("+1234567").unwrap(), Sdi12Value(1234567.0));
        assert_eq!(Sdi12Value::parse_single("-9999999").unwrap(), Sdi12Value(-9999999.0));
        assert_eq!(Sdi12Value::parse_single("+.1").unwrap(), Sdi12Value(0.1));
        assert_eq!(Sdi12Value::parse_single("-0.").unwrap(), Sdi12Value(-0.0)); // Note: -0.0 comparison
        assert_eq!(Sdi12Value::parse_single("+0").unwrap(), Sdi12Value(0.0));
    }

    #[test]
    fn test_sdi12value_parsing_invalid() {
        assert_eq!(Sdi12Value::parse_single(""), Err(Sdi12ParsingError::InvalidFormat));
        assert_eq!(Sdi12Value::parse_single("+"), Err(Sdi12ParsingError::InvalidDigitCount));
        assert_eq!(Sdi12Value::parse_single("-"), Err(Sdi12ParsingError::InvalidDigitCount));
        assert_eq!(Sdi12Value::parse_single("1.23"), Err(Sdi12ParsingError::InvalidSign));
        assert_eq!(Sdi12Value::parse_single(" +1.23"), Err(Sdi12ParsingError::InvalidSign));
        assert_eq!(Sdi12Value::parse_single("+1.2.3"), Err(Sdi12ParsingError::MultipleDecimals));
        assert_eq!(Sdi12Value::parse_single("+1a2"), Err(Sdi12ParsingError::InvalidCharacter));
        assert_eq!(Sdi12Value::parse_single("+."), Err(Sdi12ParsingError::InvalidDigitCount));
        assert_eq!(Sdi12Value::parse_single("+12345678"), Err(Sdi12ParsingError::InvalidDigitCount)); // 8 digits, len 9 -> OK length, bad digit count

        // Input "+123.45678" (Length 10) - Should fail length check first.
        assert_eq!(
            Sdi12Value::parse_single("+123.45678"), // This was line 186
            Err(Sdi12ParsingError::InvalidFormat) // CORRECTED: Expect InvalidFormat due to length > 9
        );

        // Input "+1234567." (Length 9) - Should parse OK if trailing '.' is allowed
        assert!(Sdi12Value::parse_single("+1234567.").is_ok());

        // Input "+1234567.0" (Length 10) - Should fail length check first.
        assert_eq!(
            Sdi12Value::parse_single("+1234567.0"),
            Err(Sdi12ParsingError::InvalidFormat) // Expect InvalidFormat due to length > 9
        );
        // Input "+12345.678" (Length 10) - This also fails length check first.
         assert_eq!(
            Sdi12Value::parse_single("+12345.678"), // This was previously expecting InvalidDigitCount incorrectly
            Err(Sdi12ParsingError::InvalidFormat) // CORRECTED: Expect InvalidFormat due to length > 9
        );
        assert_eq!(Sdi12Value::parse_single("+123456789"), Err(Sdi12ParsingError::InvalidFormat)); // Too long (len 10)
    }

    #[test]
    fn test_binary_data_type_from_u8() {
        assert_eq!(BinaryDataType::from_u8(0), Some(BinaryDataType::InvalidRequest));
        assert_eq!(BinaryDataType::from_u8(1), Some(BinaryDataType::SignedI8));
        assert_eq!(BinaryDataType::from_u8(2), Some(BinaryDataType::UnsignedU8));
        assert_eq!(BinaryDataType::from_u8(3), Some(BinaryDataType::SignedI16));
        assert_eq!(BinaryDataType::from_u8(4), Some(BinaryDataType::UnsignedU16));
        assert_eq!(BinaryDataType::from_u8(5), Some(BinaryDataType::SignedI32));
        assert_eq!(BinaryDataType::from_u8(6), Some(BinaryDataType::UnsignedU32));
        assert_eq!(BinaryDataType::from_u8(7), Some(BinaryDataType::SignedI64));
        assert_eq!(BinaryDataType::from_u8(8), Some(BinaryDataType::UnsignedU64));
        assert_eq!(BinaryDataType::from_u8(9), Some(BinaryDataType::Float32));
        assert_eq!(BinaryDataType::from_u8(10), Some(BinaryDataType::Float64));
        assert_eq!(BinaryDataType::from_u8(11), None);
        assert_eq!(BinaryDataType::from_u8(255), None);
    }

     #[test]
    fn test_binary_data_type_size() {
        assert_eq!(BinaryDataType::InvalidRequest.size_in_bytes(), 0);
        assert_eq!(BinaryDataType::SignedI8.size_in_bytes(), 1);
        assert_eq!(BinaryDataType::UnsignedU8.size_in_bytes(), 1);
        assert_eq!(BinaryDataType::SignedI16.size_in_bytes(), 2);
        assert_eq!(BinaryDataType::UnsignedU16.size_in_bytes(), 2);
        assert_eq!(BinaryDataType::SignedI32.size_in_bytes(), 4);
        assert_eq!(BinaryDataType::UnsignedU32.size_in_bytes(), 4);
        assert_eq!(BinaryDataType::SignedI64.size_in_bytes(), 8);
        assert_eq!(BinaryDataType::UnsignedU64.size_in_bytes(), 8);
        assert_eq!(BinaryDataType::Float32.size_in_bytes(), 4);
        assert_eq!(BinaryDataType::Float64.size_in_bytes(), 8);
    }
}