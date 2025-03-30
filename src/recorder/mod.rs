// src/recorder/mod.rs

use crate::common::{
    address::Sdi12Addr,
    command::Command,
    error::Sdi12Error,
    hal_traits::{Sdi12Serial, Sdi12Timer},
    response::{PayloadSlice, ResponseParseError}, // ResponseParseError might be used later
    // timing, // timing constants might be used later, but not directly needed for timeout implementation itself
    FrameFormat, // May be needed later for config changes
};
use core::fmt::Debug; // Needed for IF::Error bound
use core::time::Duration; // Needed for timeout duration parameter

// Import Clock and Instant from embedded-hal
// Make sure these are gated by a feature that includes embedded-hal,
// but since recorder logic likely depends on HAL traits anyway, maybe not strictly necessary here.
// However, let's assume embedded-hal is available when using the recorder.
use embedded_hal::timer::Clock;
use embedded_hal::timer::Instant as HalInstant; // Alias to avoid potential conflicts

// Use nb::Result for non-blocking operations from Sdi12Serial
use nb::Result as NbResult;


/// Represents an SDI-12 Recorder (Datalogger) instance for SYNCHRONOUS operations.
///
/// This struct owns the SDI-12 interface (serial and timer abstraction) and a clock
/// for handling timeouts and protocol timing. It provides methods to interact with
/// sensors on the bus using a blocking approach.
#[derive(Debug)]
pub struct SyncRecorder<IF, C> // Added Clock type parameter C
where
    IF: Sdi12Serial + Sdi12Timer,
    IF::Error: Debug,
    C: Clock,
    C::Instant: HalInstant + Debug + core::ops::Add<Duration, Output = C::Instant> + PartialOrd + Copy, // Add required Instant traits
{
    interface: IF,
    clock: C, // Store the clock instance
    last_activity_time: Option<C::Instant>, // For break timing state
    // TODO: Add other state like requires_break?
}

// --- Constructor ---

impl<IF, C> SyncRecorder<IF, C>
where
    IF: Sdi12Serial + Sdi12Timer,
    IF::Error: Debug,
    C: Clock,
    C::Instant: HalInstant + Debug + core::ops::Add<Duration, Output = C::Instant> + PartialOrd + Copy, // Add required Instant traits
{
    /// Creates a new SyncRecorder instance using the provided SDI-12 interface and clock.
    ///
    /// The interface must implement both `Sdi12Serial` for communication
    /// and `Sdi12Timer` for handling delays.
    /// The clock must implement `embedded_hal::timer::Clock` for managing timeouts
    /// and internal protocol timing state.
    ///
    /// # Arguments
    ///
    /// * `interface`: An object implementing `Sdi12Serial` and `Sdi12Timer`.
    /// * `clock`: An object implementing `embedded_hal::timer::Clock`.
    pub fn new(interface: IF, clock: C) -> Self {
        SyncRecorder {
            interface,
            clock,
            last_activity_time: None, // Initialize timing state
        }
    }

    // --- Public Blocking Methods ---

    /// Sends the Acknowledge Active command (`a!`) and waits for a valid acknowledgement.
    ///
    /// Returns `Ok(())` if the sensor acknowledges correctly (empty payload received).
    /// Returns `Err(Sdi12Error::...)` on timeout, CRC error (if applicable later),
    /// incorrect response payload, or communication errors.
    pub fn acknowledge(&mut self, address: Sdi12Addr) -> Result<(), Sdi12Error<IF::Error>> {
        let cmd = Command::AcknowledgeActive { address };
        // TODO: Determine appropriate buffer size or pass slice from caller
        let mut read_buffer = [0u8; 8]; // Small buffer for simple ack/error

        // Placeholder: Define a default timeout. This should likely be configurable later.
        let timeout = Duration::from_millis(100); // Example: 100ms timeout for simple ack

        // Use execute_transaction (which will internally use the timeout helper)
        let payload = self.execute_transaction(&cmd, &mut read_buffer, timeout)?;

        // For acknowledge, the payload should be empty after stripping address/CRC/CRLF
        if payload.as_bytes().is_empty() {
            Ok(())
        } else {
            // Received unexpected data after address
            Err(Sdi12Error::InvalidFormat) // Or maybe UnexpectedResponse?
        }
    }

    // --- Core Transaction Logic (Private Helper) ---

    /// Executes a full command-response transaction with retries and timeout.
    /// Handles break signal (if needed), command formatting/sending, response reading/validation.
    fn execute_transaction<'buf>(
        &mut self,
        command: &Command,
        read_buffer: &'buf mut [u8], // Buffer provided by caller
        timeout: Duration,           // Pass timeout for the overall transaction
    ) -> Result<PayloadSlice<'buf>, Sdi12Error<IF::Error>>
    {
        // TODO: Implement full sequence using new helper and timeout:
        // 1. Check timing state & call check_and_send_break()
        // 2. Format command -> command_bytes
        // 3. Retry loop (up to 3 times per spec)
        //    a. Calculate deadline for *this attempt* (now + timeout)
        //    b. send_command_bytes(&command_bytes, attempt_timeout)?
        //    c. read_response_line(read_buffer, attempt_timeout)?
        //    d. process_response_payload(line)? -> Returns PayloadSlice on success
        //    e. If successful, break loop and return PayloadSlice
        //    f. If timeout/error, handle retry wait logic (Sec 7.2) - might need break on some retries.
        // 4. If retries exhausted, return last error (e.g., Timeout)
        // 5. Update timing state after successful communication

        // Placeholder implementation still returns error, but acknowledges timeout parameter
        let _ = command;
        let _ = read_buffer;
        let _ = timeout;
        Err(Sdi12Error::Timeout) // Placeholder
    }


    // --- Low-Level I/O Helpers (Private) ---

    // TODO: Implement check_and_send_break using self.clock and self.last_activity_time
    // fn check_and_send_break(&mut self) -> Result<(), Sdi12Error<IF::Error>> { ... }

    // TODO: Implement send_command_bytes using execute_blocking_io_with_timeout
    // fn send_command_bytes(&mut self, cmd_bytes_with_term: &[u8], timeout: Duration) -> Result<(), Sdi12Error<IF::Error>> { ... }

    // TODO: Implement read_response_line using execute_blocking_io_with_timeout
    // fn read_response_line<'buf>(&mut self, buffer: &'buf mut [u8], timeout: Duration) -> Result<&'buf [u8], Sdi12Error<IF::Error>> { ... }

    // TODO: Implement process_response_payload (needs CRC check, address check)
    // fn process_response_payload<'buf>(&mut self, response_line: &'buf [u8], expected_addr: Sdi12Addr) -> Result<PayloadSlice<'buf>, Sdi12Error<IF::Error>> { ... }

    /// Executes a non-blocking I/O operation (`f`) repeatedly until it
    /// stops returning `WouldBlock`, returning the final result or timing out.
    fn execute_blocking_io_with_timeout<F, T>(
        &mut self,
        timeout: Duration,
        mut f: F,
    ) -> Result<T, Sdi12Error<IF::Error>>
    where
        F: FnMut(&mut IF) -> NbResult<T, IF::Error>,
    {
        let start_time = self.clock.now();
        // Calculate deadline: start_time + timeout
        // We added Add<Duration> bound to C::Instant
        let deadline = start_time + timeout;

        loop {
            match f(&mut self.interface) {
                Ok(result) => {
                    // Update last activity time on successful I/O
                    self.last_activity_time = Some(self.clock.now());
                    return Ok(result);
                }
                Err(nb::Error::WouldBlock) => {
                    // Check for timeout BEFORE continuing
                    // We added PartialOrd bound to C::Instant
                    if self.clock.now() >= deadline {
                        return Err(Sdi12Error::Timeout);
                    }
                    // Optional short delay/yield to prevent pegging CPU in tight loop.
                    // Using the Sdi12Timer trait's delay_us.
                    // A very small delay like 50-100us is often sufficient.
                    // This is a simple busy-wait strategy. More advanced schedulers
                    // might yield here instead.
                    self.interface.delay_us(100); // Example: 100 microseconds
                }
                Err(nb::Error::Other(e)) => {
                    // Even on error, update activity time as the bus was used
                    self.last_activity_time = Some(self.clock.now());
                    return Err(Sdi12Error::Io(e)); // Map HAL error
                }
            }
        }
    }

} // end impl SyncRecorder


// --- Async Recorder Definition (Placeholder) ---
#[cfg(feature = "async")]
mod async_recorder { // Wrap in a module to avoid name clashes if types are similar
    use super::*; // Bring in types from parent scope
    use crate::common::hal_traits::Sdi12SerialAsync; // Use async trait
    // Needs async timer/clock - placeholder for now
    // use embedded_hal_async::delay::DelayNs as AsyncDelayNs;
    // use embedded_hal_async::timer::Clock as AsyncClock;

    pub struct AsyncRecorder<IF /*, AC */> // Async Clock AC? Async Timer AT?
    where
        IF: Sdi12SerialAsync + Sdi12Timer, // Async serial, maybe sync timer is ok? Or need async delay?
        IF::Error: Debug,
       // AC: AsyncClock, ... bounds
    {
        interface: IF,
        // clock: AC,
        // last_activity_time: Option<AC::Instant>,
        // ... state ...
    }

    impl<IF /*, AC */> AsyncRecorder<IF /*, AC */>
    where
        IF: Sdi12SerialAsync + Sdi12Timer, // Adjust bounds as needed
        IF::Error: Debug,
       // AC: AsyncClock, ... bounds
    {
         pub fn new(interface: IF /*, clock: AC */) -> Self {
             // ... constructor ...
             unimplemented!("AsyncRecorder constructor not implemented")
         }

         pub async fn acknowledge(&mut self, _address: Sdi12Addr) -> Result<(), Sdi12Error<IF::Error>> {
             // ... async implementation using .await and async timeout pattern ...
             unimplemented!("AsyncRecorder acknowledge not implemented")
         }

         // ... other async methods and helpers ...
    }
}
#[cfg(feature = "async")]
pub use async_recorder::AsyncRecorder;


// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{
        address::Sdi12Addr,
        hal_traits::{Sdi12Serial, Sdi12Timer},
        // FrameFormat, // Not used directly in these tests yet
        Sdi12Error, Command,
    };
    use core::cell::RefCell; // To allow modification in mock clock
    use embedded_hal::timer::{Clock, Instant as HalInstant};
    use nb;

    // --- Mock Interface (Unchanged) ---
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    struct MockCommError;

    struct MockInterface {
        break_sent: bool,
        config: crate::common::FrameFormat, // Use crate path
        bytes_to_read: RefCell<alloc::collections::VecDeque<u8>>, // Use RefCell for interior mutability
        write_calls: RefCell<alloc::vec::Vec<u8>>,
        delay_calls: RefCell<alloc::vec::Vec<u32>>, // Track delays (us)
    }
    impl MockInterface {
        fn new() -> Self {
            MockInterface {
                break_sent: false,
                config: crate::common::FrameFormat::Sdi12_7e1,
                bytes_to_read: RefCell::new(alloc::collections::VecDeque::new()),
                write_calls: RefCell::new(alloc::vec::Vec::new()),
                delay_calls: RefCell::new(alloc::vec::Vec::new()),
            }
        }
        // Helper to queue bytes for reading
        fn queue_read_bytes(&self, bytes: &[u8]) {
            self.bytes_to_read.borrow_mut().extend(bytes);
        }
    }
    impl Sdi12Timer for MockInterface {
        fn delay_us(&mut self, us: u32) { self.delay_calls.borrow_mut().push(us); }
        fn delay_ms(&mut self, ms: u32) { self.delay_calls.borrow_mut().push(ms * 1000); }
     }
    impl Sdi12Serial for MockInterface {
        type Error = MockCommError;
        fn read_byte(&mut self) -> NbResult<u8, Self::Error> {
            self.bytes_to_read.borrow_mut().pop_front().ok_or(nb::Error::WouldBlock)
        }
        fn write_byte(&mut self, byte: u8) -> NbResult<(), Self::Error> {
            self.write_calls.borrow_mut().push(byte);
            Ok(())
        }
        fn flush(&mut self) -> NbResult<(), Self::Error> { Ok(()) }
        fn send_break(&mut self) -> NbResult<(), Self::Error> { self.break_sent = true; Ok(()) }
        fn set_config(&mut self, config: crate::common::FrameFormat) -> Result<(), Self::Error> {
            self.config = config; Ok(())
        }
    }

    // --- Mock Clock ---
    #[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq)]
    struct MockInstant(u64); // Simple microsecond counter

    impl core::ops::Add<Duration> for MockInstant {
        type Output = Self;
        fn add(self, rhs: Duration) -> Self {
            MockInstant(self.0 + rhs.as_micros() as u64)
        }
    }
    impl HalInstant for MockInstant {}

    struct MockClock {
        current_time: RefCell<u64>, // Microseconds
    }
    impl MockClock {
        fn new() -> Self { MockClock { current_time: RefCell::new(0) } }
        fn advance(&self, micros: u64) { *self.current_time.borrow_mut() += micros; }
    }
    impl Clock for MockClock {
        type Instant = MockInstant;
        const SCALING_FACTOR: embedded_hal::timer::Fraction = embedded_hal::timer::Fraction { numerator: 1, denominator: 1_000_000 }; // Ticks are microseconds
        fn now(&self) -> Self::Instant { MockInstant(*self.current_time.borrow()) }
    }


    #[test]
    fn test_recorder_construction() {
        let mock_interface = MockInterface::new();
        let mock_clock = MockClock::new();
        let _recorder = SyncRecorder::new(mock_interface, mock_clock);
        // Check initial state if relevant
        // assert!(_recorder.last_activity_time.is_none());
    }

     #[test]
    fn test_acknowledge_placeholder_call() {
         let mock_interface = MockInterface::new();
         let mock_clock = MockClock::new();
         let mut recorder = SyncRecorder::new(mock_interface, mock_clock);
         let addr = Sdi12Addr::new('0').unwrap();
         // Still expecting placeholder error from execute_transaction
         assert!(matches!(recorder.acknowledge(addr), Err(Sdi12Error::Timeout)));
     }

    #[test]
    fn test_execute_blocking_io_with_timeout_ok() {
        let mock_interface = MockInterface::new();
        let mock_clock = MockClock::new();
        let mut recorder = SyncRecorder::new(mock_interface, mock_clock);

        let mut call_count = 0;
        let timeout = Duration::from_micros(500);
        // Test the Ok path, T is i32
        let result: Result<i32, Sdi12Error<MockCommError>> = recorder.execute_blocking_io_with_timeout(timeout, |_iface| {
             call_count += 1;
             mock_clock.advance(100); // Simulate time passing
             if call_count < 3 { Err(nb::Error::WouldBlock) } else { Ok(123) }
        });
        assert_eq!(call_count, 3);
        assert_eq!(result, Ok(123));
        // Clock advanced 3 * 100us = 300us, plus 2 delays of 100us = 500us total
        assert_eq!(recorder.clock.now().0, 300); // Time at return is 300us
        assert_eq!(recorder.last_activity_time, Some(MockInstant(300))); // Activity time updated
        assert!(recorder.interface.delay_calls.borrow().len() >= 2); // Check delays were called
        assert!(recorder.interface.delay_calls.borrow().iter().all(|&d| d == 100)); // Check delay duration
    }

    #[test]
    fn test_execute_blocking_io_with_timeout_err() {
        let mock_interface = MockInterface::new();
        let mock_clock = MockClock::new();
        let mut recorder = SyncRecorder::new(mock_interface, mock_clock);

        let mut call_count = 0;
        let timeout = Duration::from_micros(500);
        // Test the Err path. Specify the Ok type T as () for the Result.
        let result_err: Result<(), Sdi12Error<MockCommError>> = recorder.execute_blocking_io_with_timeout(timeout, |_iface| {
             call_count += 1;
             mock_clock.advance(100); // Simulate time passing
              if call_count < 2 {
                  Err(nb::Error::WouldBlock)
              } else {
                  // Return the underlying I/O error
                  Err(nb::Error::Other(MockCommError))
              }
         });
        assert_eq!(call_count, 2);
        assert_eq!(result_err, Err(Sdi12Error::Io(MockCommError)));
        assert_eq!(recorder.clock.now().0, 200); // Clock advanced 2 * 100us
        assert_eq!(recorder.last_activity_time, Some(MockInstant(200))); // Activity time updated even on IO error
        assert!(recorder.interface.delay_calls.borrow().len() >= 1); // Check delay was called
    }

     #[test]
    fn test_execute_blocking_io_with_timeout_timeout() {
        let mock_interface = MockInterface::new();
        let mock_clock = MockClock::new();
        let mut recorder = SyncRecorder::new(mock_interface, mock_clock);

        let mut call_count = 0;
        let timeout = Duration::from_micros(500);
        // Test the Timeout path. Specify the Ok type T as () for the Result.
        let result_timeout: Result<(), Sdi12Error<MockCommError>> = recorder.execute_blocking_io_with_timeout(timeout, |_iface| {
             call_count += 1;
             // Advance time significantly in each loop
             mock_clock.advance(300); // 300us pass per call
             // Always return WouldBlock to force timeout
             Err(nb::Error::WouldBlock)
         });

        // Loop 1: time = 300, < 500, delay(100), continue
        // Loop 2: time = 300 + 100(delay) + 300 = 700, >= 500 -> Timeout
        assert_eq!(call_count, 2); // Should exit on the second check
        assert_eq!(result_timeout, Err(Sdi12Error::Timeout));
        assert_eq!(recorder.clock.now().0, 700); // Clock time when timeout detected
        assert_eq!(recorder.last_activity_time, None); // Timeout occurred, no successful I/O or error to update time
        assert_eq!(recorder.interface.delay_calls.borrow().len(), 1); // Only one delay before timeout
    }

}