// src/lib.rs

#![no_std] // Specify no_std at the crate root

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod common;
pub mod recorder;
pub mod sensor;

// Re-export key types for convenience
pub use common::Sdi12Addr;
pub use common::Sdi12Error;