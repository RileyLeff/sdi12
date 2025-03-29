// src/common/crc.rs

use super::error::Sdi12Error;
use crc::{Crc, Algorithm};

/// Custom CRC algorithm matching SDI-12 specification (CRC-16/ARC).
/// Polynomial: 0x8005 (normal representation of 0xA001 reversed)
/// Initial Value: 0x0000
/// Input Reflected: true
/// Output Reflected: true
/// Final XOR: 0x0000
/// Check Value: 0xBB3D (for "123456789") - standard for CRC-16/ARC
/// Residue: 0x0000
pub const SDI12_CRC: Algorithm<u16> = Algorithm {
    poly: 0x8005,
    init: 0x0000,
    refin: true,
    refout: true,
    xorout: 0x0000,
    check: 0xBB3D,
    width: 16,
    residue: 0x0000,
};

// Create a Crc instance for the SDI-12 algorithm for reuse.
const CRC_COMPUTER: Crc<u16> = Crc::<u16>::new(&SDI12_CRC);

/// Calculates the SDI-12 CRC-16 (CRC-16/ARC) for the given data buffer.
///
/// Uses the `crc` crate configured for the standard CRC-16/ARC algorithm,
/// which matches the SDI-12 specification. The calculation starts from the
/// first byte (typically the address) up to the byte *before* the CRC
/// itself or the trailing `<CR><LF>`.
///
/// # Arguments
///
/// * `data`: A slice of bytes for which to calculate the CRC.
///
/// # Returns
///
/// The calculated 16-bit CRC value.
#[inline]
pub fn calculate_crc16(data: &[u8]) -> u16 {
    CRC_COMPUTER.checksum(data)
}

/// Encodes a 16-bit CRC value into three ASCII characters according to SDI-12 standard.
///
/// Follows section 4.4.12.2 of the SDI-12 specification v1.4.
///
/// # Arguments
///
/// * `crc_value`: The 16-bit CRC to encode.
///
/// # Returns
///
/// An array of three `u8` bytes representing the ASCII-encoded CRC.
pub fn encode_crc_ascii(crc_value: u16) -> [u8; 3] {
    let char1 = 0x40 | ((crc_value >> 12) & 0x3F) as u8;
    let char2 = 0x40 | ((crc_value >> 6) & 0x3F) as u8;
    let char3 = 0x40 | (crc_value & 0x3F) as u8;
    [char1, char2, char3]
}

/// Decodes three SDI-12 ASCII-encoded CRC characters back into a 16-bit value.
///
/// # Arguments
///
/// * `crc_chars`: A slice or array of three `u8` bytes representing the ASCII-encoded CRC.
///
/// # Returns
///
/// The decoded 16-bit CRC value.
///
/// # Panics
///
/// Panics if `crc_chars` does not have a length of exactly 3.
pub fn decode_crc_ascii(crc_chars: &[u8]) -> u16 {
    assert_eq!(crc_chars.len(), 3, "ASCII CRC must be 3 bytes long");
    let byte1 = u16::from(crc_chars[0] & 0x3F);
    let byte2 = u16::from(crc_chars[1] & 0x3F);
    let byte3 = u16::from(crc_chars[2] & 0x3F);
    (byte1 << 12) | (byte2 << 6) | byte3
}

/// Verifies an SDI-12 response string that includes an ASCII CRC.
///
/// Assumes the buffer ends with the 3 CRC bytes and does *not* include `<CR><LF>`.
///
/// # Arguments
///
/// * `response_with_crc`: The response buffer including the 3-byte ASCII CRC.
///
/// # Returns
///
/// * `Ok(())` if the CRC is valid.
/// * `Err(Sdi12Error::InvalidFormat)` if the buffer is too short.
/// * `Err(Sdi12Error::CrcMismatch)` if the CRCs don't match.
pub fn verify_response_crc_ascii<E>(response_with_crc: &[u8]) -> Result<(), Sdi12Error<E>>
where
    E: core::fmt::Debug,
{
    if response_with_crc.len() < 3 {
        return Err(Sdi12Error::InvalidFormat);
    }
    let data_len = response_with_crc.len() - 3;
    let data_part = &response_with_crc[..data_len];
    let received_crc_bytes = &response_with_crc[data_len..];

    let calculated_crc = calculate_crc16(data_part);
    let received_crc = decode_crc_ascii(received_crc_bytes);

    if calculated_crc == received_crc {
        Ok(())
    } else {
        Err(Sdi12Error::CrcMismatch { expected: received_crc, calculated: calculated_crc, })
    }
}

/// Encodes a 16-bit CRC value into two bytes (LSB first) for binary responses.
///
/// # Arguments
///
/// * `crc_value`: The 16-bit CRC to encode.
///
/// # Returns
///
/// An array of two `u8` bytes `[LSB, MSB]`.
pub fn encode_crc_binary(crc_value: u16) -> [u8; 2] {
    crc_value.to_le_bytes()
}

/// Decodes two bytes (LSB first) from a binary response into a 16-bit CRC value.
///
/// # Arguments
///
/// * `crc_bytes`: A slice or array of two `u8` bytes `[LSB, MSB]`.
///
/// # Returns
///
/// The decoded 16-bit CRC value.
///
/// # Panics
///
/// Panics if `crc_bytes` does not have a length of exactly 2.
pub fn decode_crc_binary(crc_bytes: &[u8]) -> u16 {
    assert_eq!(crc_bytes.len(), 2, "Binary CRC must be 2 bytes long");
    u16::from_le_bytes([crc_bytes[0], crc_bytes[1]])
}

/// Verifies an SDI-12 high-volume binary response packet including its binary CRC.
///
/// Assumes the buffer ends with the 2 raw CRC bytes.
///
/// # Arguments
///
/// * `packet_with_crc`: The complete binary packet buffer including the 2-byte CRC.
///
/// # Returns
///
/// * `Ok(())` if the CRC is valid.
/// * `Err(Sdi12Error::InvalidFormat)` if the buffer is too short.
/// * `Err(Sdi12Error::CrcMismatch)` if the CRCs don't match.
pub fn verify_packet_crc_binary<E>(packet_with_crc: &[u8]) -> Result<(), Sdi12Error<E>>
where
    E: core::fmt::Debug,
{
    if packet_with_crc.len() < 2 {
        return Err(Sdi12Error::InvalidFormat);
    }
    let data_len = packet_with_crc.len() - 2;
    let data_part = &packet_with_crc[..data_len];
    let received_crc_bytes = &packet_with_crc[data_len..];

    let calculated_crc = calculate_crc16(data_part);
    let received_crc = decode_crc_binary(received_crc_bytes);

    if calculated_crc == received_crc {
        Ok(())
    } else {
        Err(Sdi12Error::CrcMismatch { expected: received_crc, calculated: calculated_crc, })
    }
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    // Mock error type for verify function generic parameter
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct MockIoError;
    impl core::fmt::Display for MockIoError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { write!(f, "Mock I/O Error") }
    }

    // --- ASCII CRC Tests Based Directly on Spec v1.4 Section 4.4.12.3 ---

    #[test]
    fn test_spec_example_a() {
        // "0D0!0+3.14OqZ<CR><LF>"
        let data = b"0+3.14";
        let expected_crc_str = b"OqZ";
        let expected_crc_val = decode_crc_ascii(expected_crc_str); // Derive value from spec string

        // 1. Test calculation
        let calculated_crc = calculate_crc16(data);
        assert_eq!(calculated_crc, expected_crc_val, "Example A: Calculation mismatch");

        // 2. Test encoding
        let encoded_crc = encode_crc_ascii(calculated_crc);
        assert_eq!(&encoded_crc, expected_crc_str, "Example A: Encoding mismatch");

        // 3. Test verification helper
        let mut response = data.to_vec();
        response.extend_from_slice(expected_crc_str);
        assert!(verify_response_crc_ascii::<MockIoError>(&response).is_ok(), "Example A: Verification failed");
    }

    #[test]
    fn test_spec_example_b() {
        // "0D0!0+3.14+2.718+1.414Ipz<CR><LF>"
        let data = b"0+3.14+2.718+1.414";
        let expected_crc_str = b"Ipz";
        let expected_crc_val = decode_crc_ascii(expected_crc_str);

        let calculated_crc = calculate_crc16(data);
        assert_eq!(calculated_crc, expected_crc_val, "Example B: Calculation mismatch");

        let encoded_crc = encode_crc_ascii(calculated_crc);
        assert_eq!(&encoded_crc, expected_crc_str, "Example B: Encoding mismatch");

        let mut response = data.to_vec();
        response.extend_from_slice(expected_crc_str);
        assert!(verify_response_crc_ascii::<MockIoError>(&response).is_ok(), "Example B: Verification failed");
    }

    #[test]
    fn test_spec_example_c_d0() {
        // "0D0!0+1.11+2.22+3.33+4.44+5.55+6.66I]q<CR><LF>"
        let data = b"0+1.11+2.22+3.33+4.44+5.55+6.66";
        let expected_crc_str = b"I]q";
        let expected_crc_val = decode_crc_ascii(expected_crc_str);

        let calculated_crc = calculate_crc16(data);
        assert_eq!(calculated_crc, expected_crc_val, "Example C D0: Calculation mismatch");

        let encoded_crc = encode_crc_ascii(calculated_crc);
        assert_eq!(&encoded_crc, expected_crc_str, "Example C D0: Encoding mismatch");

        let mut response = data.to_vec();
        response.extend_from_slice(expected_crc_str);
        assert!(verify_response_crc_ascii::<MockIoError>(&response).is_ok(), "Example C D0: Verification failed");
    }

    #[test]
    fn test_spec_example_c_d1() {
        // "0D1!0+7.77+8.88+9.99IvW<CR><LF>"
        let data = b"0+7.77+8.88+9.99";
        let expected_crc_str = b"IvW";
        let expected_crc_val = decode_crc_ascii(expected_crc_str);

        let calculated_crc = calculate_crc16(data);
        assert_eq!(calculated_crc, expected_crc_val, "Example C D1: Calculation mismatch");

        let encoded_crc = encode_crc_ascii(calculated_crc);
        assert_eq!(&encoded_crc, expected_crc_str, "Example C D1: Encoding mismatch");

        let mut response = data.to_vec();
        response.extend_from_slice(expected_crc_str);
        assert!(verify_response_crc_ascii::<MockIoError>(&response).is_ok(), "Example C D1: Verification failed");
    }

    #[test]
    fn test_spec_example_d() {
        // "0D0!0+3.14+2.718IWO<CR><LF>"
        let data = b"0+3.14+2.718";
        let expected_crc_str = b"IWO";
        let expected_crc_val = decode_crc_ascii(expected_crc_str);

        let calculated_crc = calculate_crc16(data);
        assert_eq!(calculated_crc, expected_crc_val, "Example D: Calculation mismatch");

        let encoded_crc = encode_crc_ascii(calculated_crc);
        assert_eq!(&encoded_crc, expected_crc_str, "Example D: Encoding mismatch");

        let mut response = data.to_vec();
        response.extend_from_slice(expected_crc_str);
        assert!(verify_response_crc_ascii::<MockIoError>(&response).is_ok(), "Example D: Verification failed");
    }

    #[test]
    fn test_spec_example_e_d0() {
        // "0D0!0+3.14OqZ<CR><LF>" - Same as Example A
        let data = b"0+3.14";
        let expected_crc_str = b"OqZ";
        let expected_crc_val = decode_crc_ascii(expected_crc_str);

        let calculated_crc = calculate_crc16(data);
        assert_eq!(calculated_crc, expected_crc_val, "Example E D0: Calculation mismatch");

        let encoded_crc = encode_crc_ascii(calculated_crc);
        assert_eq!(&encoded_crc, expected_crc_str, "Example E D0: Encoding mismatch");

        let mut response = data.to_vec();
        response.extend_from_slice(expected_crc_str);
        assert!(verify_response_crc_ascii::<MockIoError>(&response).is_ok(), "Example E D0: Verification failed");
    }

    #[test]
    fn test_spec_example_e_d1() {
        // "0D1!0+2.718Gbc<CR><LF>"
        let data = b"0+2.718";
        let expected_crc_str = b"Gbc";
        let expected_crc_val = decode_crc_ascii(expected_crc_str);

        let calculated_crc = calculate_crc16(data);
        assert_eq!(calculated_crc, expected_crc_val, "Example E D1: Calculation mismatch");

        let encoded_crc = encode_crc_ascii(calculated_crc);
        assert_eq!(&encoded_crc, expected_crc_str, "Example E D1: Encoding mismatch");

        let mut response = data.to_vec();
        response.extend_from_slice(expected_crc_str);
        assert!(verify_response_crc_ascii::<MockIoError>(&response).is_ok(), "Example E D1: Verification failed");
    }

    #[test]
    fn test_spec_example_e_d2() {
        // "0D2!0+1.414GtW<CR><LF>"
        let data = b"0+1.414";
        let expected_crc_str = b"GtW";
        let expected_crc_val = decode_crc_ascii(expected_crc_str);

        let calculated_crc = calculate_crc16(data);
        assert_eq!(calculated_crc, expected_crc_val, "Example E D2: Calculation mismatch");

        let encoded_crc = encode_crc_ascii(calculated_crc);
        assert_eq!(&encoded_crc, expected_crc_str, "Example E D2: Encoding mismatch");

        let mut response = data.to_vec();
        response.extend_from_slice(expected_crc_str);
        assert!(verify_response_crc_ascii::<MockIoError>(&response).is_ok(), "Example E D2: Verification failed");
    }

    #[test]
    fn test_spec_example_f_sensor1() {
        // "1D0!1+1.23+2.34+345+4.4678KoO<CR><LF>"
        let data = b"1+1.23+2.34+345+4.4678";
        let expected_crc_str = b"KoO";
        let expected_crc_val = decode_crc_ascii(expected_crc_str);

        let calculated_crc = calculate_crc16(data);
        assert_eq!(calculated_crc, expected_crc_val, "Example F S1: Calculation mismatch");

        let encoded_crc = encode_crc_ascii(calculated_crc);
        assert_eq!(&encoded_crc, expected_crc_str, "Example F S1: Encoding mismatch");

        let mut response = data.to_vec();
        response.extend_from_slice(expected_crc_str);
        assert!(verify_response_crc_ascii::<MockIoError>(&response).is_ok(), "Example F S1: Verification failed");
    }

     #[test]
    fn test_spec_example_f_sensor0() {
        // "0D0!0+1.234-4.56+12354-0.00045+2.223+145.5+7.7003+4328.8+9+10+11.433+12Ba]<CR><LF>"
        let data = b"0+1.234-4.56+12354-0.00045+2.223+145.5+7.7003+4328.8+9+10+11.433+12";
        let expected_crc_str = b"Ba]";
        let expected_crc_val = decode_crc_ascii(expected_crc_str);

        let calculated_crc = calculate_crc16(data);
        assert_eq!(calculated_crc, expected_crc_val, "Example F S0: Calculation mismatch");

        let encoded_crc = encode_crc_ascii(calculated_crc);
        assert_eq!(&encoded_crc, expected_crc_str, "Example F S0: Encoding mismatch");

        let mut response = data.to_vec();
        response.extend_from_slice(expected_crc_str);
        assert!(verify_response_crc_ascii::<MockIoError>(&response).is_ok(), "Example F S0: Verification failed");
    }


    // --- Binary CRC Tests Based Directly on Spec v1.4 Section 5.2.2 ---

    #[test]
    fn test_spec_binary_example_db0() {
        // Data: 0x31 0x04 0x00 0x03 0xFF 0xFF 0x01 0x00
        // CRC: 0xC2 0xAC -> 0xACC2
        let data = &[0x31, 0x04, 0x00, 0x03, 0xFF, 0xFF, 0x01, 0x00];
        let expected_crc_bytes = &[0xC2, 0xAC]; // LSB, MSB
        let expected_crc_val = decode_crc_binary(expected_crc_bytes);

        // 1. Test calculation
        let calculated_crc = calculate_crc16(data);
        assert_eq!(calculated_crc, expected_crc_val, "Binary Ex DB0: Calculation mismatch");

        // 2. Test encoding
        let encoded_crc = encode_crc_binary(calculated_crc);
        assert_eq!(&encoded_crc, expected_crc_bytes, "Binary Ex DB0: Encoding mismatch");

        // 3. Test verification helper
        let mut packet = data.to_vec();
        packet.extend_from_slice(expected_crc_bytes);
        assert!(verify_packet_crc_binary::<MockIoError>(&packet).is_ok(), "Binary Ex DB0: Verification failed");
    }

     #[test]
    fn test_spec_binary_example_db1() {
        // Data: 0x31 0x08 0x00 0x09 0xC3 0xF5 0x48 0x40 0x00 0x00 0x80 0x3F
        // CRC: 0x3B 0x6E -> 0x6E3B
        let data = &[0x31, 0x08, 0x00, 0x09, 0xC3, 0xF5, 0x48, 0x40, 0x00, 0x00, 0x80, 0x3F];
        let expected_crc_bytes = &[0x3B, 0x6E]; // LSB, MSB
        let expected_crc_val = decode_crc_binary(expected_crc_bytes);

        let calculated_crc = calculate_crc16(data);
        assert_eq!(calculated_crc, expected_crc_val, "Binary Ex DB1: Calculation mismatch");

        let encoded_crc = encode_crc_binary(calculated_crc);
        assert_eq!(&encoded_crc, expected_crc_bytes, "Binary Ex DB1: Encoding mismatch");

        let mut packet = data.to_vec();
        packet.extend_from_slice(expected_crc_bytes);
        assert!(verify_packet_crc_binary::<MockIoError>(&packet).is_ok(), "Binary Ex DB1: Verification failed");
    }

    #[test]
    fn test_spec_binary_example_db2_empty() {
        // Data: 0x31 0x00 0x00 0x00 (Empty packet indicator)
        // CRC: 0x0E 0xFC -> 0xFC0E
        let data = &[0x31, 0x00, 0x00, 0x00];
        let expected_crc_bytes = &[0x0E, 0xFC]; // LSB, MSB
        let expected_crc_val = decode_crc_binary(expected_crc_bytes);

        let calculated_crc = calculate_crc16(data);
        assert_eq!(calculated_crc, expected_crc_val, "Binary Ex DB2: Calculation mismatch");

        let encoded_crc = encode_crc_binary(calculated_crc);
        assert_eq!(&encoded_crc, expected_crc_bytes, "Binary Ex DB2: Encoding mismatch");

        let mut packet = data.to_vec();
        packet.extend_from_slice(expected_crc_bytes);
        assert!(verify_packet_crc_binary::<MockIoError>(&packet).is_ok(), "Binary Ex DB2: Verification failed");
    }

    // --- Optional: Keep basic roundtrip/error tests if desired ---
    #[test]
    fn test_crc_ascii_encoding_decoding_roundtrip_extra() {
        let test_cases = [0x0000, 0xFFFF, 0x1234, 0xABCD]; // Non-spec examples
        for crc_val in test_cases {
            let encoded = encode_crc_ascii(crc_val);
            let decoded = decode_crc_ascii(&encoded);
            assert_eq!(decoded, crc_val, "ASCII Encode/Decode roundtrip failed for {:#06x}", crc_val);
        }
    }

    #[test]
    fn test_binary_crc_encoding_decoding_roundtrip_extra() {
         let test_cases = [0x0000, 0xFFFF, 0x1234, 0xABCD]; // Non-spec examples
        for crc_val in test_cases {
            let encoded = encode_crc_binary(crc_val);
            let decoded = decode_crc_binary(&encoded);
            assert_eq!(decoded, crc_val, "Binary Encode/Decode roundtrip failed for {:#06x}", crc_val);
        }
    }

    #[test]
    fn test_verify_ascii_crc_invalid_cases() {
        // Wrong CRC characters
        let result1 = verify_response_crc_ascii::<MockIoError>(b"0+3.14OqX"); // Correct is OqZ
        assert!(matches!(result1, Err(Sdi12Error::CrcMismatch { .. })));

        // Corrupted data, correct CRC characters
        let result2 = verify_response_crc_ascii::<MockIoError>(b"0+3.15OqZ"); // Changed 4 to 5
        assert!(matches!(result2, Err(Sdi12Error::CrcMismatch { .. })));

        // Buffer too short for CRC
        assert!(matches!(verify_response_crc_ascii::<MockIoError>(b"0+"), Err(Sdi12Error::InvalidFormat)));
        assert!(matches!(verify_response_crc_ascii::<MockIoError>(b"Oq"), Err(Sdi12Error::InvalidFormat)));
        assert!(matches!(verify_response_crc_ascii::<MockIoError>(b""), Err(Sdi12Error::InvalidFormat)));
    }

     #[test]
    fn test_verify_binary_crc_invalid_cases() {
        // Correct data, wrong CRC bytes
        let data = &[0x31, 0x04, 0x00, 0x03, 0xFF, 0xFF, 0x01, 0x00];
        let mut packet_bad_crc = data.to_vec();
        packet_bad_crc.extend_from_slice(&[0xC3, 0xAC]); // Original CRC was C2 AC
        assert!(matches!(verify_packet_crc_binary::<MockIoError>(&packet_bad_crc), Err(Sdi12Error::CrcMismatch { .. })));

        // Corrupted data, original CRC bytes
        let data_bad = &[0x31, 0x04, 0x00, 0x03, 0xFE, 0xFF, 0x01, 0x00];
        let mut packet_bad_data = data_bad.to_vec();
        let correct_crc = calculate_crc16(data); // CRC for original data
        packet_bad_data.extend_from_slice(&encode_crc_binary(correct_crc));
        assert!(matches!(verify_packet_crc_binary::<MockIoError>(&packet_bad_data), Err(Sdi12Error::CrcMismatch { .. })));

        // Buffer has data but only 1 byte for CRC
        let packet_short_crc = &[0x31, 0x04, 0x00, 0x03, 0xFF, 0xFF, 0x01, 0x00, 0xC2];
        assert!(matches!(verify_packet_crc_binary::<MockIoError>(packet_short_crc), Err(Sdi12Error::CrcMismatch { .. })));

        // Buffer genuinely too short
        assert!(matches!(verify_packet_crc_binary::<MockIoError>(&[0x31]), Err(Sdi12Error::InvalidFormat)));
        assert!(matches!(verify_packet_crc_binary::<MockIoError>(b""), Err(Sdi12Error::InvalidFormat)));
    }

    // Panic tests for decode functions remain useful
    #[test]
    #[should_panic]
    fn test_decode_ascii_panic_short() { decode_crc_ascii(b"Oq"); }
    #[test]
    #[should_panic]
    fn test_decode_ascii_panic_long() { decode_crc_ascii(b"OqZZ"); }
    #[test]
    #[should_panic]
    fn test_decode_binary_panic_short() { decode_crc_binary(&[0xC2]); }
    #[test]
    #[should_panic]
    fn test_decode_binary_panic_long() { decode_crc_binary(&[0xC2, 0xAC, 0x00]); }
}