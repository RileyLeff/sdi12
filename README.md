# SDI-12 Rust Library (`sdi12-rs`) - Design & Status Report

**Date:** 2025-03-30

## 1. Overview

`sdi12-rs` aims to be a comprehensive, robust, and developer-friendly Rust library for interacting with the SDI-12 (Serial-Digital Interface at 1200 baud) protocol, commonly used for environmental sensors and dataloggers.

The library targets embedded systems (`no_std` by default) but is designed to be usable in `std` environments as well. It provides first-class support for both implementing SDI-12 recorder (datalogger/master) functionality and implementing SDI-12 sensor (slave) firmware. The goal is to abstract the complexities of the SDI-12 protocol (timing, framing, commands, responses, CRC, state management) behind an idiomatic and safe Rust API.

This document outlines the library's goals, design philosophy, architecture, key decisions made during initial development, current status, and future directions.

## 2. Goals & Requirements (Based on Initial Request)

*   **Standard Compliance:** Implement the **SDI-12 Standard Version 1.4** (Feb 20, 2023), including all basic, concurrent, high-volume (ASCII & Binary), and metadata commands/responses.
*   **Target Audience:** Support developers building both **Recorders (Masters)** and **Sensors (Slaves)**.
*   **Environment:** Be `#[no_std]` compatible by default. Provide optional, feature-gated support for `std` and `alloc`.
*   **Concurrency Models:** Offer first-class support for both **synchronous** and **asynchronous** operation patterns.
*   **Hardware Abstraction:** Integrate cleanly with the embedded Rust ecosystem, primarily via `embedded-hal` (v1.0+) traits, but remain hardware-agnostic at its core.
*   **Framework Compatibility:** Be usable with async frameworks like Embassy, but without requiring Embassy as a direct dependency.
*   **Error Handling:** Utilize `thiserror` for robust, specific, and ergonomic error reporting.
*   **Modularity:** Organize code logically into modules for recorder logic, sensor logic, and shared common components.

## 3. Core Design Philosophy

*   **Robustness & Safety:** Leverage Rust's type system to make invalid states unrepresentable where possible (e.g., command indices). Provide strong error handling. Ensure correct protocol implementation according to the standard.
*   **Ergonomics (DX):** Offer intuitive, high-level APIs for both recorder and sensor implementors, abstracting away byte-level protocol details and state machine complexity where appropriate.
*   **Flexibility:** Support the four key use-case quadrants: sync `no_std`, sync `std`, async `no_std`, async `std`. Allow users to choose the implementation strategy that best fits their hardware and constraints (native HAL features, generic HAL, bit-banging).
*   **Portability & Agnosticism:** Decouple the core protocol logic from specific hardware implementations via traits (`Sdi12Serial`, `Sdi12Timer`). The library provides the framework; the user provides the hardware-specific implementation or uses optional adapters.
*   **Maintainability:** Structure the code logically. Use external crates (like `crc`) for well-solved problems where appropriate. Limit the burden of maintaining HAL-specific code *within* the core library by favouring user-provided implementations or optional adapter crates.
*   **Standard Compliance:** Adhere strictly to the timings, formats, and procedures outlined in SDI-12 v1.4.

## 4. Architecture & Modules

The library uses the standard Rust crate structure (`src/lib.rs`) with the following primary modules:

*   **`common/`**: Contains foundational types, traits, and logic shared between recorder and sensor implementations. This is the most developed part so far.
    *   `address.rs`: `Sdi12Addr` struct for validated addresses.
    *   `command.rs`: `Command` enum and validated index types (`MeasurementIndex`, etc.) representing all SDI-12 commands.
    *   `crc.rs`: CRC-16/ARC calculation (using the `crc` crate) and SDI-12 specific ASCII/binary encoding/decoding/verification helpers.
    *   `error.rs`: `Sdi12Error<E>` generic protocol error enum using `thiserror`.
    *   `frame.rs`: `FrameFormat` enum (`Sdi12_7e1`, `Binary8n1`).
    *   `hal_traits.rs`: Defines the core hardware abstraction traits: `Sdi12Timer`, `Sdi12Serial` (sync/nb), `Sdi12SerialAsync` (async), and the `NativeSdi12Uart`/`Async` traits for optimized HAL integration.
    *   `response.rs`: Defines `ResponseParseError`, the `MeasurementTiming` struct, and the `PayloadSlice` wrapper for returning validated raw response payloads. (Note: Parsing detailed response types was deferred).
    *   `timing.rs`: `const Duration` values for all specified protocol timings.
    *   `types.rs`: `Sdi12Value` parsing/representation, `BinaryDataType` enum, `Sdi12ParsingError`.
*   **`recorder/`**: Contains logic for the Recorder (Master/Datalogger) role.
    *   `mod.rs`: Defines `SyncRecorder` and placeholder `AsyncRecorder` structs. Holds implementation logic (currently contains constructor and basic helpers).
*   **`sensor/`**: Contains logic and traits for the Sensor (Slave) role. (Not yet implemented).
    *   Will define the `SensorHandler` trait for user logic.
    *   Will define `SyncSensor` / `AsyncSensor` runner structs.
*   **`implementations/` (Directory)**: (Not yet implemented) Intended to hold optional, feature-gated adapter implementations bridging `embedded-hal` (and potentially other ecosystems like `std` serial) to the library's `hal_traits`.
    *   `native.rs`: `NativeAdapter` using the `NativeSdi12Uart` trait.
    *   `generic_hal.rs`: `GenericHalAdapter` using standard `embedded-hal` traits + pin manipulation.
    *   `bitbang.rs`: `BitbangAdapter`.

## 5. Key Design Decisions & Rationale

*   **`no_std` First with `alloc` Feature:** The library compiles as `#[no_std]` by default. An `alloc` feature flag enables dynamic allocation (`Vec`, `String`), primarily used for convenience in parsing variable-length responses (on the recorder side) or handling extended commands. Users of `no_std` + `alloc` must provide a `#[global_allocator]`. This provides flexibility for various targets.
*   **Sync/Async Separation:** Decided to use distinct structs (`SyncRecorder`, `AsyncRecorder` and similarly `SyncSensor`, `AsyncSensor`) rather than `cfg`-gating methods on a single struct. This provides a cleaner separation of concerns, simplifies trait bounds, and makes the user's choice explicit based on their runtime environment.
*   **Hardware Abstraction Strategy:**
    *   Core logic relies on library-defined traits (`Sdi12Serial`, `Sdi12Timer`).
    *   Users choose their implementation strategy:
        1.  **Native:** Implement the `NativeSdi12Uart` trait for their HAL's UART type if it supports native break/config changes. Use `NativeAdapter`. (Fastest, least abstraction cost).
        2.  **Generic HAL:** Use a library-provided `GenericHalAdapter` (feature-gated) which uses standard `embedded-hal` traits (`Read`, `Write`, `OutputPin`, etc.) and implements break/config via generic pin manipulation. (Good compatibility, relies on pin control traits being available).
        3.  **Bitbang:** Use a library-provided `BitbangAdapter` (feature-gated) relying only on GPIO/Delay traits. (Most compatible, highest CPU cost).
    *   This tiered approach minimizes the need for the library to maintain specific adapters for every HAL, placing the responsibility on the user or optional adapter crates, while providing standard fallbacks.
    *   The tricky parts (`send_break`, `set_config`) are explicitly required by the traits, forcing implementors to address them.
*   **Response Parsing ("Middle Ground"):**
    *   **Initial Decision:** Have the library fully parse responses into structured enums/structs (e.g., `Response::Identification`, `Response::Data`).
    *   **Revised Decision:** Simplify the core library's responsibility. The recorder's transaction methods will handle framing (address, CRC, CRLF) and return the validated inner **payload as a `PayloadSlice(&[u8])`**. The `ResponseParseError` enum covers errors related to this framing/CRC layer.
    *   **Rationale:** Significantly reduces core library complexity, improves `no_alloc` compatibility by default, handles non-standard sensor formats gracefully (user parses the payload), and enables sensor-specific parsing crates.
    *   **Future:** Optional parsing helpers (gated by `alloc`/`heapless` features) can be added later to parse `PayloadSlice` into structured types for user convenience. The `MeasurementTiming` struct *is* parsed by the library as it's a common, simple, fixed format not considered general "payload".
*   **Error Handling:** Using `thiserror` for the generic `Sdi12Error<E>` provides structured protocol/IO errors. `ResponseParseError` handles framing/CRC errors specifically. Command construction uses `Result` via validated index types.
*   **CRC Handling:** Leverages the external, well-tested `crc` crate configured for CRC-16/ARC, ensuring correctness and reducing implementation burden. SDI-12 specific ASCII/binary encoding/decoding helpers are provided.
*   **Sensor Handler API:** Planning a trait-based approach (`SensorHandler`) where users implement methods corresponding to specific SDI-12 actions (e.g., `start_measurement`, `get_identification`). The library's `Sensor` runner handles command parsing, dispatch, state management (e.g., for M->D sequences), and response formatting. Considering a macro helper (`handler_macro!`) as a future DX improvement for defining handlers.
*   **Command Representation:** Uses a main `Command` enum with sub-enums for Metadata commands. Incorporates validated index types (`MeasurementIndex`, etc.) to make invalid command indices unrepresentable after construction.

## 6. Current Implementation Status (2025-03-30)

*   **Crate Structure:** Basic structure (`src/{common, recorder, sensor}`) established.
*   **`common` Module:**
    *   `address.rs`: Implemented and tested.
    *   `command.rs`: Implemented (including validated index types) and tested.
    *   `crc.rs`: Implemented (using `crc` crate) and tested against all spec examples.
    *   `error.rs`: `Sdi12Error` defined using `thiserror`.
    *   `frame.rs`: `FrameFormat` enum defined.
    *   `hal_traits.rs`: `Sdi12Timer`, `Sdi12Serial`, `Sdi12SerialAsync`, `NativeSdi12Uart`, `NativeSdi12UartAsync` traits defined.
    *   `response.rs`: Refactored to "middle ground" approach. Defines `ResponseParseError`, `MeasurementTiming`, `PayloadSlice`. Removed complex `Response` enum and parsing logic from core. Tested basic struct definitions.
    *   `timing.rs`: Implemented and reviewed.
    *   `types.rs`: `Sdi12Value` (with basic parsing), `BinaryDataType`, `Sdi12ParsingError` implemented and tested.
*   **`recorder` Module:**
    *   `mod.rs`: `SyncRecorder` struct defined with `new` constructor. Basic `execute_blocking_io` helper implemented (without timeout). Placeholder `acknowledge` method added. Placeholder for `AsyncRecorder`. Basic tests for structure and helper pass.
*   **Features:** `alloc`, `async`, `impl-native` features defined in `Cargo.toml`. Conditional compilation attributes (`#[cfg(...)]`) used. Builds and tests pass for both default (`no_std`, no features) and `--features alloc`.
*   **Dependencies:** `thiserror`, `crc`, `nb`, `embedded-hal` (optional), `embedded-hal-async` (optional) added.

## 7. Remaining Core Implementation ("Minimum Viable Product")

*   **`recorder::SyncRecorder` Helpers:**
    *   Implement command formatting (e.g., `fn format_command(cmd: &Command) -> Result<ArrayVec<u8, N>, _>`). Needs a fixed-size buffer (e.g., from `arrayvec` or `heapless`) or stack allocation.
    *   Implement `check_and_send_break` (requires adding timing state).
    *   Implement `send_command_bytes`.
    *   Implement `read_response_line` (crucially needs timeout logic integrated with `execute_blocking_io` or similar).
    *   Implement `process_response_payload` (address checking, CRC verification using `crc.rs`).
    *   Implement `execute_transaction` using the above helpers, including retry logic (Sec 7.2).
*   **`recorder::SyncRecorder` Public Methods:** Implement the main action methods (`identify`, `change_address`, `start_measurement`, `send_data`, etc.) using `execute_transaction`. Define appropriate return types (e.g., `Result<MeasurementTiming, _>`, `Result<PayloadSlice, _>`, `Result<Sdi12Addr, _>`).
*   **`sensor::SensorHandler` Trait:** Finalize the trait definition based on the chosen philosophy (action-methods, specific return types like `MeasurementStartResult`).
*   **`sensor::SyncSensor`:** Implement the sensor runner struct and its `listen_and_respond` logic (parsing commands, calling handler methods, formatting responses based on handler results, managing protocol state).
* **Adapters (Minimal):** Implement at least the `NativeAdapter` (`implementations/native.rs`) to allow usage with HALs where the user implements `NativeSdi12Uart`. Consider a basic `GenericHalAdapter` foundation.
* **Basic Examples:** Create simple examples demonstrating recorder and sensor usage (e.g., a loopback test).

## 8. Future Directions & Improvements

* **Async Implementation:** Fully implement `AsyncRecorder`, `AsyncSensor`, and async adapters (`Sdi12SerialAsync`, etc.).
* **`heapless` Support:** Add a `heapless` feature flag. Provide alternative `no_alloc` structs using `heapless::String`/`Vec`. Provide `heapless`-based parsing helpers.
* **Parsing Helpers:** Implement the optional `alloc`/`heapless` helper functions for parsing common payload types (`IdentificationInfo`, `DataInfo`, `MetadataInfo`, etc.) from `PayloadSlice`.
* **Sensor Handler Macro:** Implement the `handler_macro!` to simplify defining `SensorHandler` implementations.
* **More Adapters:** Provide more feature-gated adapter implementations for popular `embedded-hal` families (STM32, RP2040, ESP-HAL, etc.), potentially including native break/config optimizations where possible. Implement `std` adapters (using `serialport`, `tokio-serial`).
* **Documentation:** Add comprehensive documentation (`#![forbid(missing_docs)]`), including usage examples for different platforms and configurations.
* **Examples:** Create more extensive examples for real-world scenarios (Embassy, RTIC, `std`).
* **Timeout Configuration:** Allow users to configure timeout durations.
* **Performance Optimization:** Profile and optimize critical code paths.
* **Testing:** Add integration tests, tests on real hardware. Test race conditions in async code.

## 9. Conclusion

The `sdi12-rs` library has a well-defined set of goals and a flexible, hardware-agnostic architecture based on traits and feature-gated implementations. The `common` module, providing foundational types and utilities, is largely complete and tested. Key design decisions regarding sync/async support, hardware abstraction, and response parsing have been made to balance usability, flexibility, and maintainability. The immediate next steps involve implementing the core transaction logic within the `SyncRecorder` and subsequently the `Sensor` runner, building upon the established common infrastructure. Future work will focus on async support, optional parsing helpers, and broader adapter/example coverage.
