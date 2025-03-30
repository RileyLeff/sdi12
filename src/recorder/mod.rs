// src/recorder/mod.rs

// Declare the new sub-module
pub mod sync_recorder;

// Re-export the public SyncRecorder struct
pub use sync_recorder::SyncRecorder;

// Keep async placeholders if needed
#[cfg(feature = "async")]
use crate::common::{address::Sdi12Addr, error::Sdi12Error, hal_traits::Sdi12Timer};
#[cfg(feature = "async")]
use core::fmt::Debug;

#[cfg(feature = "async")]
pub struct AsyncRecorder<IF> {
    interface: IF,
    // ... state ...
     last_activity_time: Option<<IF as Sdi12Timer>::Instant>, // Use associated type
}

#[cfg(feature = "async")]
impl<IF> AsyncRecorder<IF>
where
    IF: crate::common::hal_traits::Sdi12SerialAsync + Sdi12Timer,
    IF::Error: Debug,
    // Add Sdi12Instant bound here too if needed for async state tracking
    <IF as Sdi12Timer>::Instant: crate::common::hal_traits::Sdi12Instant,
{
     pub fn new(interface: IF) -> Self {
         AsyncRecorder {
            interface,
            last_activity_time: None,
         }
     }

     pub async fn acknowledge(&mut self, _address: Sdi12Addr) -> Result<(), Sdi12Error<IF::Error>> {
         unimplemented!("Async acknowledge not implemented")
     }
     // ... other async methods and helpers ...
}

// No tests needed here anymore, they moved to sync_recorder/mod.rs