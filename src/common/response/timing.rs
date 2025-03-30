// src/common/response/timing.rs

use crate::common::address::Sdi12Addr;

/// Timing and count information returned by Measurement/Concurrent/Identify commands. (Sec 4.4.5 etc.)
/// This struct does *not* require `alloc`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MeasurementTiming {
    /// The address of the responding sensor.
    pub address: Sdi12Addr,
    /// Time estimate in seconds until data is ready (ttt). 0-999.
    pub time_seconds: u16,
    /// Number of measurement values that will be returned (n, nn, or nnn). 0-999.
    pub values_count: u16,
}