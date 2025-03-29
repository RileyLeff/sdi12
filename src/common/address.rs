// src/common/address.rs

use super::error::Sdi12Error;
use core::convert::TryFrom;
use core::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Sdi12Addr(char);

impl Sdi12Addr {
    pub const DEFAULT_ADDRESS: Sdi12Addr = Sdi12Addr('0');
    pub const QUERY_ADDRESS: Sdi12Addr = Sdi12Addr('?');

    /// Creates a new `Sdi12Addr` if the given character is a valid address.
    /// Returns `Result<Self, Sdi12Error<()>>` because validation itself
    /// cannot cause an I/O error.
    pub fn new(address_char: char) -> Result<Self, Sdi12Error<()>> {
        if Self::is_valid_address_char(address_char) || address_char == '?' {
            Ok(Sdi12Addr(address_char))
        } else {
            // Directly create the specific error variant with E = ()
            Err(Sdi12Error::InvalidAddress(address_char))
        }
    }

    // Unsafe constructor remains the same
    pub const unsafe fn new_unchecked(address_char: char) -> Self {
        Sdi12Addr(address_char)
    }

    #[inline]
    pub const fn as_char(&self) -> char {
        self.0
    }

    #[inline]
    pub const fn is_query(&self) -> bool {
        self.0 == '?'
    }

    #[inline]
    pub const fn is_standard(&self) -> bool {
        // This one was okay because '0'..='9' is a single range pattern
        matches!(self.0, '0'..='9')
    }

    #[inline]
    pub const fn is_extended(&self) -> bool {
        // CORRECTED: Use '|' directly as a pattern separator
        matches!(self.0, 'a'..='z' | 'A'..='Z')
    }

    #[inline]
    pub const fn is_valid_address_char(c: char) -> bool {
        // CORRECTED: Use '|' directly as a pattern separator
        matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z')
    }
}

impl Default for Sdi12Addr {
    fn default() -> Self {
        Self::DEFAULT_ADDRESS
    }
}

// CORRECTED: Implement TryFrom<char> without the generic E
impl TryFrom<char> for Sdi12Addr {
    // The error type here is specific: Sdi12Error with no I/O error possibility
    type Error = Sdi12Error<()>;

    /// Attempts to convert a character into an `Sdi12Addr`.
    fn try_from(value: char) -> Result<Self, Self::Error> {
        // Reuse the validation logic from Self::new()
        // Since Self::new() now returns Result<_, Sdi12Error<()>>, this works directly.
        Self::new(value)
        // Or, implement directly:
        // if Sdi12Addr::is_valid_address_char(value) || value == '?' {
        //     Ok(Sdi12Addr(value))
        // } else {
        //     Err(Sdi12Error::InvalidAddress(value))
        // }
    }
}


impl From<Sdi12Addr> for char {
    fn from(value: Sdi12Addr) -> Self {
        value.0
    }
}

impl fmt::Display for Sdi12Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    // Mock error type is no longer strictly needed inside the tests for `new` or `try_from`
    // because they now return `Sdi12Error<()>` which doesn't involve a generic `E`.
    // We might still need it later for testing functions that *do* take an E.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct MockIoError;
    impl fmt::Display for MockIoError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "Mock I/O Error") }
    }

    #[test]
    fn test_valid_addresses() {
        assert!(Sdi12Addr::new('0').is_ok());
        assert!(Sdi12Addr::new('5').is_ok());
        assert!(Sdi12Addr::new('9').is_ok());
        assert!(Sdi12Addr::new('a').is_ok());
        assert!(Sdi12Addr::new('z').is_ok());
        assert!(Sdi12Addr::new('A').is_ok());
        assert!(Sdi12Addr::new('Z').is_ok());
        assert!(Sdi12Addr::new('?').is_ok());
    }

    #[test]
    fn test_invalid_addresses() {
        assert!(matches!(Sdi12Addr::new(' '), Err(Sdi12Error::InvalidAddress(' '))));
        assert!(matches!(Sdi12Addr::new('$'), Err(Sdi12Error::InvalidAddress('$'))));
        assert!(matches!(Sdi12Addr::new('\n'), Err(Sdi12Error::InvalidAddress('\n'))));
        assert!(matches!(Sdi12Addr::new('é'), Err(Sdi12Error::InvalidAddress('é'))));
    }

    // test_default_address, test_query_address, test_address_types remain the same

    #[test]
    fn test_try_from_char() {
        assert_eq!(Sdi12Addr::try_from('1').unwrap(), Sdi12Addr('1'));
        assert_eq!(Sdi12Addr::try_from('b').unwrap(), Sdi12Addr('b'));
        assert_eq!(Sdi12Addr::try_from('C').unwrap(), Sdi12Addr('C'));
        assert_eq!(Sdi12Addr::try_from('?').unwrap(), Sdi12Addr('?'));
        assert!(matches!(Sdi12Addr::try_from('*'), Err(Sdi12Error::InvalidAddress('*'))));
    }

    // test_into_char, test_display, test_as_char, test_is_valid_address_char, test_new_unchecked remain the same
}