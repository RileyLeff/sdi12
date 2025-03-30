# SDI-12 Rust Library (`sdi12-rs`) - Design & Status Report

**Date:** 2025-03-30 (Updated)

## 1. Overview

`sdi12-rs` aims to be a comprehensive, robust, and developer-friendly Rust library for interacting with the SDI-12 (Serial-Digital Interface at 1200 baud) protocol, commonly used for environmental sensors and dataloggers.

The library targets embedded systems (`no_std` by default) but is designed to be usable in `std` environments as well. It provides first-class support for both implementing SDI-12 recorder (e.g. datalogger, host) functionality and implementing SDI-12 sensor firmware. The goal is to abstract the complexities of the SDI-12 protocol (timing, framing, commands, responses, CRC, state management) behind an idiomatic and safe Rust API.

This document outlines the library's goals, design philosophy, architecture, key decisions, current status, and future directions.

## 2. Goals & Requirements

*   **Standard Compliance:** Implement the **SDI-12 Standard Version 1.4** (Feb 20, 2023), including all basic, concurrent, high-volume (ASCII & Binary), and metadata commands/responses.
*   **Target Audience:** Support developers building both **Recorders** and **Sensors**.
*   **Environment:** Be `#[no_std]` compatible by default. Provide optional, feature-gated support for `std` and `alloc`.
*   **Concurrency Models:** Offer first-class support for both **synchronous** and **asynchronous** operation patterns.
*   **Hardware Abstraction:** Integrate cleanly with the embedded Rust ecosystem, primarily via `embedded-hal` (v1.0+) traits or custom traits, remaining hardware-agnostic at its core.
*   **Framework Compatibility:** Be usable with async frameworks like Embassy, but without requiring Embassy as a direct dependency.
*   **Error Handling:** Utilize `thiserror` for robust, specific, and ergonomic error reporting.
*   **Modularity:** Organize code logically into modules for recorder logic, sensor logic, and shared common components.

## 3. Core Design Philosophy

*   **Robustness & Safety:** Leverage Rust's type system to make invalid states unrepresentable where possible (e.g., validated command indices). Provide strong error handling (`Sdi12Error`). Ensure correct protocol implementation according to the standard.
*   **Ergonomics (DX):** Offer intuitive, high-level APIs for both recorder and sensor implementors, abstracting away byte-level protocol details and state machine complexity where appropriate. Provide lower-level access (like `send_command`) for flexibility.
*   **Flexibility:** Support the four key use-case quadrants: sync `no_std`, sync `std`, async `no_std`, async `std`. Allow users to choose the hardware implementation strategy that best fits their constraints (native HAL features, generic HAL, bit-banging).
*   **Portability & Agnosticism:** Decouple the core protocol logic from specific hardware implementations via library-defined traits (`Sdi12Serial`, `Sdi12Timer`). The library provides the framework; the user provides the hardware-specific implementation or uses optional adapters.
*   **Maintainability:** Structure the code logically. Use external crates (like `crc`) for well-solved problems. Limit the burden of maintaining HAL-specific code *within* the core library.
*   **Standard Compliance:** Adhere strictly to the timings, formats, and procedures outlined in SDI-12 v1.4.

## 4. Architecture & Modules

The library uses the standard Rust crate structure (`src/lib.rs`) with the following primary modules:

*   **`common/`**: Contains foundational types, traits, and logic shared between recorder and sensor implementations. (Largely complete and stable).
    *   `address.rs`: `Sdi12Addr` struct for validated addresses.
    *   `command.rs`: `Command` enum (covering v1.4), validated index types (`MeasurementIndex`, etc.), formatting via `format_into` using `arrayvec`.
    *   `crc.rs`: CRC-16/ARC calculation (using `crc` crate) and SDI-12 specific ASCII/binary encoding/decoding/verification helpers.
    *   `error.rs`: `Sdi12Error<E>` generic protocol error enum using `thiserror`, wrapping specific command errors.
    *   `frame.rs`: `FrameFormat` enum (`Sdi12_7e1`, `Binary8N1`).
    *   `hal_traits.rs`: Defines the core hardware abstraction traits:
        *   `Sdi12Timer`: Now includes `type Instant: Sdi12Instant;` and `fn now()`.
        *   `Sdi12Instant`: Marker trait for time instants.
        *   `Sdi12Serial` (sync/nb): Defines serial operations including `send_break` and `set_config`.
        *   `Sdi12SerialAsync` (async).
        *   `NativeSdi12Uart`/`Async` (for optimized HAL integration, gated).
    *   `response.rs`: Defines `ResponseParseError`, `MeasurementTiming`, and `PayloadSlice`. Reflects the "Middle Ground" parsing approach.
    *   `timing.rs`: `const Duration` values for specified protocol timings.
    *   `types.rs`: `Sdi12Value` parsing/representation, `BinaryDataType` enum, `Sdi12ParsingError`.
*   **`recorder/`**: Contains logic for the Recorder (Datalogger) role.
    *   `mod.rs`: Declares `sync_recorder` submodule and re-exports `SyncRecorder`. Placeholder for `AsyncRecorder`.
    *   **`sync_recorder/`**: Implementation for synchronous recorder.
        *   `mod.rs`: Defines `SyncRecorder` struct, `new()` constructor, and public API methods (`acknowledge`, `send_command`).
        *   `io_helpers.rs`: Contains `execute_blocking_io_with_timeout`, `check_and_send_break`, `send_command_bytes`, `read_response_line`.
        *   `protocol_helpers.rs`: Contains `process_response_payload` (checks address, CRC, returns indices).
        *   `transaction.rs`: Contains the core `execute_transaction` logic (handles break, send, read, process, basic retries, returns indices).
*   **`sensor/`**: Contains logic and traits for the Sensor role. (Not yet implemented).
*   **`implementations/` (Directory)**: (Not yet implemented) Intended for optional, feature-gated HAL adapters.

## 5. Key Design Decisions & Rationale

*   **`no_std` First:** Default `#[no_std]` compilation. Optional `alloc` and `std` features.
*   **Sync/Async Separation:** Distinct structs (`SyncRecorder`, `AsyncRecorder`) for clarity.
*   **Hardware Abstraction Strategy:**
    *   Core logic relies on library-defined `Sdi12Serial` and `Sdi12Timer` traits.
    *   `Sdi12Timer` now includes `now()` and an associated `Instant` type constrained by `Sdi12Instant` for robust timeout handling without direct `embedded-hal` dependency in the core logic.
    *   Tiered implementation options (Native HAL, Generic HAL, Bitbang) via adapters remain the goal.
*   **Response Parsing ("Middle Ground"):**
    *   The core recorder transaction logic (`execute_transaction`) validates framing (address, CRC, CRLF) and returns the start/end **indices** of the valid payload within the user-provided buffer.
    *   Public API methods (like the planned `send_identification`) use these indices to extract the `PayloadSlice` and then perform specific payload parsing. Simpler methods like `acknowledge` might just check if the indices indicate an empty payload.
    *   **Rationale:** Keeps core library lean, `no_alloc` friendly by default, handles non-standard formats, delegates complex parsing. Optional helpers for common payloads can be added later.
*   **Command Input:**
    *   The primary way to send commands is via the type-safe `Command` enum, constructed using validated index types.
    *   The `SyncRecorder::send_command` method allows sending any pre-constructed `Command`.
    *   **Deferred:** Parsing command strings (e.g., `TryFrom<&str>` for `Command`) is deferred.
*   **Error Handling:** Using `thiserror` for `Sdi12Error<E>`; specific command/parsing errors are wrapped.
*   **CRC Handling:** Leverages the `crc` crate. Helpers for ASCII/binary CRC provided. Verification integrated into `process_response_payload`.

## 6. Current Implementation Status (2025-03-30)

*   **Crate Structure:** `common`, `recorder`, `sensor` modules established. `recorder::sync_recorder` submodule created and populated.
*   **`common` Module:** Largely complete and tested. Foundational types, traits (including enhanced `Sdi12Timer`), CRC, errors, commands are defined. `Command::format_into` implemented.
*   **`recorder::sync_recorder` Module:**
    *   `SyncRecorder` struct defined with `new` constructor.
    *   Core transaction logic (`execute_transaction`) implemented, including break checks, command sending, response reading (with timeout via `Sdi12Timer`), payload validation (address, CRC, framing), and basic retry logic for timeouts/read errors. Returns payload indices.
    *   Internal helpers (`check_and_send_break`, `send_command_bytes`, `read_response_line`, `process_response_payload`) implemented.
    *   Public API methods `acknowledge` and the flexible `send_command` are implemented.
*   **Features:** `alloc`, `std`, `async` features defined. Builds and tests pass for default (`no_std`) and `--features alloc`.
*   **Dependencies:** Updated based on needs (`arrayvec`, `crc`, `nb`, `thiserror`). `embedded-hal` is optional for adapters.

## 7. Remaining Core Implementation ("Minimum Viable Product")

*   **`recorder::SyncRecorder` Public Methods:** Implement remaining high-level methods (`identify`, `change_address`, `start_measurement`, `send_data`, etc.) using `execute_transaction` and adding necessary payload parsing logic (potentially gated by `alloc`/`heapless` features for returning structured data).
*   **Refine Retry Logic:** Review and potentially enhance the retry logic in `execute_transaction`, particularly regarding break conditions on retries as per Spec 7.2.
*   **Sensor Implementation:**
    *   Define `sensor::SensorHandler` trait.
    *   Implement `sensor::SyncSensor` runner (command parsing, handler dispatch, response formatting, state management).
    *   Requires implementing command *parsing* logic (likely `Command::try_from_bytes` or similar).
*   **Adapters (Minimal):** Implement at least the `NativeAdapter` (`implementations/native.rs`) using `NativeSdi12Uart` and a basic `GenericHalAdapter` to bridge `embedded-hal` traits to `Sdi12Serial`/`Sdi12Timer`.
*   **Basic Examples:** Create simple examples demonstrating recorder usage (e.g., `acknowledge`, `send_identification` on real hardware like Pico). A loopback test example would also be valuable.

## 8. Tech Debt / Future Improvements

*   **Command String Parsing:** Implement `TryFrom<&[u8]>` or similar for `Command` for user convenience and sensor-side implementation.
*   **`alloc`/`heapless` Features:** Fully integrate these features. Provide `heapless`-based alternatives for `Command::ExtendedCommand` and response parsing helpers. Offer `alloc`-based parsing helpers that return `Vec<Sdi12Value>`, `IdentificationInfo`, etc.
*   **Refine Retry Logic:** Implement the more complex break-on-retry timing specified in Sec 7.2 if needed for robustness.
*   **Payload Parsing Helpers:** Implement optional (`alloc`/`heapless` gated) functions to parse common payload types (`IdentificationInfo`, `MeasurementTiming`, `Sdi12Value` vectors) from the indices/buffer provided by `send_command`.
*   **Async Implementation:** Fully implement `AsyncRecorder`, `AsyncSensor`, and async adapters (`Sdi12SerialAsync`, async `Sdi12Timer`).
*   **Sensor Implementation:** Complete the sensor-side logic. Consider a macro helper (`#[sdi12_handler]`) for defining `SensorHandler` implementations.
*   **More Adapters:** Provide feature-gated adapters for popular `embedded-hal` families (STM32, RP2040, ESP-HAL, etc.) and `std` (using `serialport`).
*   **Documentation & Examples:** Add comprehensive `rustdoc`, usage examples (especially for Pico target), and potentially a small book/tutorial. Enforce documentation with `#![forbid(missing_docs)]`.
*   **Timeout Configuration:** Allow users to configure default timeout durations for transactions.
*   **Testing:** Add integration tests (simulated loopback, hardware loopback), hardware-in-the-loop tests with real sensors/recorders. Test edge cases and error conditions more thoroughly. Test async race conditions. Clean up test mock inconsistencies (e.g., `Clone` derive).
*   **Buffer Sizes:** Review fixed buffer sizes (like `MAX_FORMATTED_LEN`) and consider making them configurable or using alternatives where appropriate.

## 9. Conclusion

The `sdi12-rs` library has made significant progress. The `common` module is robust, and the core synchronous recorder transaction logic (`execute_transaction` with its helpers) is now implemented, including timeout handling via the enhanced `Sdi12Timer` trait and basic retries. The flexible `send_command` public API method provides immediate utility for testing and interacting with sensors. The next steps involve building out the remaining specific public recorder methods, implementing the sensor side, providing HAL adapters, and creating examples for real-world usage.