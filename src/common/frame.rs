// src/common/frame.rs

/// Represents the serial frame formats used in SDI-12.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FrameFormat {
    /// Standard SDI-12 format: 1200 baud, 7 data bits, Even parity, 1 stop bit.
    Sdi12_7e1,
    /// Format for High-Volume Binary data: 1200 baud, 8 data bits, No parity, 1 stop bit.
    Binary8N1,
}