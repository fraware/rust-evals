//! Injectable clock abstraction.
//!
//! Every timestamp that lands in a trace event or an evidence bundle index
//! flows through a [`Clock`]. Production code uses [`SystemClock`], which
//! returns `Utc::now()`. Deterministic tests (and the Milestone C rerun
//! acceptance proof) use [`FixedClock`], which returns a monotonically
//! advancing sequence derived from a fixed start.
//!
//! This indirection is the single most load-bearing change in the runner:
//! without it, two reruns of the same candidate produce different trace
//! bytes (because `Utc::now()` differs) and therefore different evidence
//! bundle hashes. The acceptance criterion for Milestone C is that bundle
//! hashes are stable across reruns; that is only possible when time is
//! injected.

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};

/// Returns the current UTC timestamp.
///
/// Implementations must be `Send + Sync` so that a `&dyn Clock` can be
/// shared across the pipeline.
pub trait Clock: Send + Sync + std::fmt::Debug {
    /// Wall-clock time.
    fn now(&self) -> DateTime<Utc>;
}

/// Real wall-clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Clock that returns a monotonically-advancing sequence of timestamps
/// starting at a fixed origin and incrementing by a fixed step on every
/// call to [`Clock::now`].
///
/// The step is taken in milliseconds and defaults to 1 ms. A zero step
/// returns the same instant for every call, which is useful for tests
/// that explicitly do not want any clock-derived variation.
///
/// Thread-safe: the counter is an [`AtomicI64`] so a `FixedClock` can be
/// shared by reference across threads.
#[derive(Debug)]
pub struct FixedClock {
    origin_millis: i64,
    step_millis: i64,
    counter: AtomicI64,
}

impl FixedClock {
    /// Construct a `FixedClock` starting at `origin` and stepping by
    /// `step` between successive calls.
    #[must_use]
    pub fn new(origin: DateTime<Utc>, step: Duration) -> Self {
        let step_millis = i64::try_from(step.as_millis()).unwrap_or(i64::MAX);
        Self {
            origin_millis: origin.timestamp_millis(),
            step_millis,
            counter: AtomicI64::new(0),
        }
    }

    /// Shorthand for a clock anchored at `2025-01-01T00:00:00Z` with a 1 ms step.
    ///
    /// Used by the deterministic rerun test; the exact value is unimportant
    /// but must be stable.
    #[must_use]
    pub fn deterministic() -> Self {
        Self::new(
            Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
            Duration::from_millis(1),
        )
    }

    /// Reset the counter. Lets a single `FixedClock` be reused across two
    /// pipeline runs and still produce identical timestamps.
    pub fn reset(&self) {
        self.counter.store(0, Ordering::SeqCst);
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        let step = self.counter.fetch_add(1, Ordering::SeqCst);
        let offset = step.saturating_mul(self.step_millis);
        let millis = self.origin_millis.saturating_add(offset);
        Utc.timestamp_millis_opt(millis)
            .single()
            .unwrap_or_else(Utc::now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_roughly_returns_now() {
        let before = Utc::now();
        let c = SystemClock;
        let t = c.now();
        let after = Utc::now();
        assert!(t >= before && t <= after);
    }

    #[test]
    fn fixed_clock_advances_monotonically() {
        let c = FixedClock::deterministic();
        let t0 = c.now();
        let t1 = c.now();
        let t2 = c.now();
        assert!(t1 > t0 && t2 > t1);
        assert_eq!((t1 - t0).num_milliseconds(), 1);
    }

    #[test]
    fn fixed_clock_reset_is_reusable() {
        let c = FixedClock::deterministic();
        let a0 = c.now();
        let _a1 = c.now();
        c.reset();
        let b0 = c.now();
        assert_eq!(a0, b0, "reset must return the sequence to the origin");
    }
}
