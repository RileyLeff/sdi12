// src/common/timing.rs

use core::time::Duration;

// Note: Tolerances are generally +/- 0.40 ms according to the spec (Sec 7.0),
// except for inter-character time. We define the nominal values here.
// Implementations using these should factor in tolerances where appropriate,
// especially when checking received timings.

// === Break Timing (Sec 7.0, 7.1) ===

/// Minimum duration for a valid break signal (recorder must send >= 12 ms).
pub const BREAK_DURATION_MIN: Duration = Duration::from_millis(12);
/// Sensor must recognize a break if spacing > 12 ms.
pub const BREAK_RECOGNITION_MAX: Duration = Duration::from_millis(12);
/// Sensor will *not* recognize a break if spacing < 6.5 ms.
pub const BREAK_IGNORE_MAX: Duration = Duration::from_micros(6500);
/// Marking time required after a break before sensor looks for an address.
pub const POST_BREAK_MARKING_MIN: Duration = Duration::from_micros(8330);

// === Command/Response Timing (Sec 7.0) ===

/// Maximum time from end of command stop bit for recorder to release line.
pub const RECORDER_RELEASE_TIME_MAX: Duration = Duration::from_micros(7500 + 400); // 7.5ms + 0.4ms tol
/// Nominal marking time sent by sensor before starting response.
pub const SENSOR_PRE_RESPONSE_MARKING: Duration = Duration::from_micros(8330);
/// Maximum time from end of command stop bit to start bit of first response byte.
pub const RESPONSE_START_TIME_MAX: Duration = Duration::from_millis(15) + Duration::from_micros(400); // 15ms + 0.4ms tol
/// Maximum time from end of response stop bit for sensor to release line.
pub const SENSOR_RELEASE_TIME_MAX: Duration = Duration::from_micros(7500 + 400); // 7.5ms + 0.4ms tol
/// Maximum marking time allowed between characters in a command or response.
/// (Spec says 1.66 ms with *no* tolerance).
pub const INTER_CHARACTER_MARKING_MAX: Duration = Duration::from_micros(1660);

// === Sensor Wake/Sleep Timing (Sec 7.0) ===

/// Maximum time for sensor to wake up after detecting a break and be ready for command start bit.
pub const SENSOR_WAKEUP_TIME_MAX: Duration = Duration::from_millis(100);
/// Marking time after which sensor returns to low-power standby (if not actively processing/responding).
pub const SENSOR_SLEEP_MARKING_TIME: Duration = Duration::from_millis(100);
/// Time threshold after which a break *must* precede the next command if line was marking.
/// (Spec says > 87 ms in Sec 7.1 implies break needed, aligned with retry logic in Sec 7.2).
pub const PRE_COMMAND_BREAK_MARKING_THRESHOLD: Duration = Duration::from_millis(87);

// === Retry Timing (Sec 7.2) ===

/// Minimum wait time after a command before recorder issues a retry (if no response).
pub const RETRY_WAIT_MIN: Duration = Duration::from_micros(16670); // 16.67 ms
/// Maximum wait time after a command before recorder issues a retry without a preceding break.
/// (This period also covers the RETRY_WAIT_MIN).
pub const RETRY_WAIT_MAX_NO_BREAK: Duration = Duration::from_millis(87);
/// Minimum delay after the *end of the break* before issuing at least one retry,
/// to ensure sensor has had SENSOR_WAKEUP_TIME_MAX to wake up.
pub const RETRY_POST_BREAK_DELAY_MIN: Duration = SENSOR_WAKEUP_TIME_MAX;

// === Other ===

/// Time between lines for multi-line text responses (Sec 4.4.13.1). Max 150ms.
pub const MULTILINE_INTER_LINE_DELAY_MAX: Duration = Duration::from_millis(150);

// === Byte Timing at 1200 Baud (7E1) ===
// 1 start bit + 7 data bits + 1 parity bit + 1 stop bit = 10 bits per byte
// Time per bit = 1 / 1200 seconds = 0.8333... ms
// Time per byte = 10 * (1 / 1200) seconds = 10 / 1200 s = 1 / 120 s = 8.333... ms

/// Nominal duration of a single bit at 1200 baud.
pub const BIT_DURATION: Duration = Duration::from_nanos(833_333); // Approx 0.833 ms
/// Nominal duration of a single byte (10 bits total) at 1200 baud (7E1 format).
pub const BYTE_DURATION: Duration = Duration::from_micros(8333); // Approx 8.33 ms