//! Time and clock abstractions.
//!
//! `Time` is a type alias for `chrono::DateTime<chrono::Utc>`. The
//! [`Clock`] trait lets tests inject deterministic time without touching
//! the system clock.

use chrono::{DateTime, Utc};

/// Re-export of `chrono::DateTime<chrono::Utc>` for consistent typing
/// across crates.
pub type Time = DateTime<Utc>;

/// A clock that can return the current time.
///
/// Production code uses [`SystemClock`]. Tests use a stub
/// implementation that returns a fixed time, so tests are deterministic.
pub trait Clock: Send + Sync {
    /// Return the current UTC time.
    fn now(&self) -> Time;
}

/// The system clock — reads `chrono::Utc::now()`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Time {
        Utc::now()
    }
}

/// Generate the current UTC time using the system clock.
///
/// Equivalent to `SystemClock.now()`. Provided for convenience in
/// contexts where the clock is not injected.
#[must_use]
pub fn now() -> Time {
    SystemClock.now()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// A stub clock that always returns the same fixed time.
    #[derive(Debug, Clone, Copy)]
    struct FixedClock(Time);

    impl Clock for FixedClock {
        fn now(&self) -> Time {
            self.0
        }
    }

    #[test]
    fn system_clock_advances() {
        let c = SystemClock;
        let t1 = c.now();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let t2 = c.now();
        assert!(t2 > t1);
    }

    #[test]
    fn fixed_clock_is_stable() {
        let fixed = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        let c = FixedClock(fixed);
        assert_eq!(c.now(), fixed);
        assert_eq!(c.now(), fixed);
    }

    #[test]
    fn now_returns_utc() {
        let t = now();
        assert_eq!(t.timezone(), chrono::Utc);
    }
}
