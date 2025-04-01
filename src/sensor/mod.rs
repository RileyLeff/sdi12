// src/sensor/mod.rs

// Declare the modules within the sensor directory.
// These modules contain the different logical parts of the sensor implementation.

// Shared logic (used by both sync and async sensor runners)
pub mod handler;      // Defines the SensorHandler trait (user implements this)
mod response;     // Defines the internal SensorResponse enum and related structs
mod formatter;    // Logic to format SensorResponse -> byte stream
mod parser;       // Logic to parse byte stream -> Command

// Specific runner implementations
pub mod sync_sensor; // Synchronous sensor runner

// Asynchronous sensor runner (feature-gated)
#[cfg(feature = "async")]
pub mod async_sensor;

// --- Public Re-exports ---
// Re-export the essential types that users of the library will interact with
// when implementing a sensor.

// The core trait the user needs to implement.
// pub use handler::SensorHandler;

// The synchronous runner struct the user will instantiate and run.
// pub use sync_sensor::SyncSensor;

// Conditionally re-export the asynchronous runner struct.
#[cfg(feature = "async")]
pub use async_sensor::AsyncSensor;

// Potential re-exports for response types if they are directly used
// in the SensorHandler trait signatures (might need adjustment later).
// pub use response::{ SensorResponse, IdentificationInfo, /* ... */ };