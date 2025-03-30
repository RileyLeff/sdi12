// src/common/response/data.rs

use crate::common::address::Sdi12Addr;
use crate::common::types::{BinaryDataType, Sdi12Value};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Data values returned by Send Data (`aDn!`) or Read Continuous (`aRn!`) commands.
/// Requires the `alloc` feature.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
pub struct DataInfo {
    /// The address of the responding sensor.
    pub address: Sdi12Addr,
    /// The parsed data values.
    pub values: Vec<Sdi12Value>,
    /// CRC value included in the response, if one was requested and present.
    pub crc: Option<u16>,
}

/// Binary data packet returned by Send Binary Data (`aDBn!`) command. (Sec 5.2)
/// Requires the `alloc` feature.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryDataInfo {
    /// The address of the responding sensor.
    pub address: Sdi12Addr,
    /// The total size in bytes of the `payload` (from packet header).
    pub packet_size: u16,
    /// The type of data contained in the `payload`.
    pub data_type: BinaryDataType,
    /// The raw binary payload. Interpretation depends on `data_type`. Max 1000 bytes.
    pub payload: Vec<u8>,
    /// The 16-bit binary CRC value received at the end of the packet.
    pub crc: u16,
}