// src/common/response/metadata.rs

use crate::common::address::Sdi12Addr;

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

/// Metadata information returned by Identify Measurement Parameter commands. (Sec 6.2)
/// Requires the `alloc` feature.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataInfo {
    /// The address of the responding sensor.
    pub address: Sdi12Addr,
    /// The parsed fields (comma-separated values). Field 0=address(redundant), 1=param ID, 2=units...
    pub fields: Vec<String>,
    /// CRC value included in the response, if one was requested and present.
    pub crc: Option<u16>,
}