//! Bounded observation polling. A predicate, not a sleep, decides completion.
use crate::{Error, Result};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct WaitOptions {
    pub timeout: Duration,
    pub interval: Duration,
}
impl Default for WaitOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            interval: Duration::from_millis(50),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitReason {
    Timeout,
    Cancelled,
    InvalidOptions,
    Capture(Error),
}
#[derive(Debug)]
pub struct Observation<T> {
    pub value: T,
    pub attempts: usize,
    pub elapsed: Duration,
}
#[derive(Debug)]
pub struct WaitFailure<T> {
    pub reason: WaitReason,
    pub last: Option<T>,
    pub attempts: usize,
    pub elapsed: Duration,
}
trait Clock {
    fn elapsed(&self) -> Duration;
    fn sleep(&mut self, duration: Duration);
}
struct RealClock(Instant);
impl Clock for RealClock {
    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }
    fn sleep(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }
}
/// Poll synchronously until the caller's predicate accepts an observation.
/// Cancellation/deadlines are checked before and after capture and predicate.
/// A blocking callback cannot be preempted: callbacks must themselves be bounded.
/// Use a stateful predicate when a new action must invalidate an old matching result.
pub fn wait_until<T>(
    options: WaitOptions,
    capture: impl FnMut() -> Result<T>,
    accept: impl FnMut(&T) -> bool,
    cancelled: impl FnMut() -> bool,
) -> std::result::Result<Observation<T>, WaitFailure<T>> {
    poll(
        options,
        capture,
        accept,
        cancelled,
        &mut RealClock(Instant::now()),
    )
}
fn poll<T>(
    options: WaitOptions,
    mut capture: impl FnMut() -> Result<T>,
    mut accept: impl FnMut(&T) -> bool,
    mut cancelled: impl FnMut() -> bool,
    clock: &mut impl Clock,
) -> std::result::Result<Observation<T>, WaitFailure<T>> {
    let mut attempts = 0;
    let mut last = None;
    let reason = if options.timeout.is_zero() || options.interval.is_zero() {
        WaitReason::InvalidOptions
    } else {
        loop {
            if cancelled() {
                break WaitReason::Cancelled;
            }
            if clock.elapsed() >= options.timeout {
                break WaitReason::Timeout;
            }
            attempts += 1;
            let value = match capture() {
                Ok(v) => v,
                Err(e) => break WaitReason::Capture(e),
            };
            last = Some(value);
            if cancelled() {
                break WaitReason::Cancelled;
            }
            if clock.elapsed() >= options.timeout {
                break WaitReason::Timeout;
            }
            let accepted = accept(last.as_ref().unwrap());
            if cancelled() {
                break WaitReason::Cancelled;
            }
            if clock.elapsed() >= options.timeout {
                break WaitReason::Timeout;
            }
            if accepted {
                return Ok(Observation {
                    value: last.unwrap(),
                    attempts,
                    elapsed: clock.elapsed(),
                });
            }
            clock.sleep(
                options
                    .interval
                    .min(options.timeout.saturating_sub(clock.elapsed())),
            );
        }
    };
    Err(WaitFailure {
        reason,
        last,
        attempts,
        elapsed: clock.elapsed(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct FakeClock(Duration);
    impl Clock for FakeClock {
        fn elapsed(&self) -> Duration {
            self.0
        }
        fn sleep(&mut self, d: Duration) {
            self.0 += d;
        }
    }
    fn opts() -> WaitOptions {
        WaitOptions {
            timeout: Duration::from_millis(10),
            interval: Duration::from_millis(3),
        }
    }
    #[test]
    fn delayed_success_and_timeout_retain_evidence() {
        let mut n = 0;
        let result = poll(
            opts(),
            || {
                n += 1;
                Ok(n)
            },
            |v| *v == 3,
            || false,
            &mut FakeClock::default(),
        )
        .unwrap();
        assert_eq!(
            (result.value, result.attempts, result.elapsed),
            (3, 3, Duration::from_millis(6))
        );
        let result = poll(
            opts(),
            || Ok(7),
            |_| false,
            || false,
            &mut FakeClock::default(),
        )
        .unwrap_err();
        assert_eq!(result.reason, WaitReason::Timeout);
        assert_eq!(
            (result.last, result.attempts, result.elapsed),
            (Some(7), 4, Duration::from_millis(10))
        );
    }
    #[test]
    fn cancellation_failure_and_invalid_options_do_not_hide_prior_frame() {
        let result = poll(
            opts(),
            || panic!("capture after cancellation"),
            |_: &()| true,
            || true,
            &mut FakeClock::default(),
        )
        .unwrap_err();
        assert_eq!(result.reason, WaitReason::Cancelled);
        assert_eq!(result.attempts, 0);
        let mut n = 0;
        let result = poll(
            opts(),
            || {
                n += 1;
                if n == 1 {
                    Ok(42)
                } else {
                    Err(Error("denied".into()))
                }
            },
            |_| false,
            || false,
            &mut FakeClock::default(),
        )
        .unwrap_err();
        assert_eq!(result.last, Some(42));
        assert_eq!(result.reason, WaitReason::Capture(Error("denied".into())));
        let result = poll(
            WaitOptions {
                interval: Duration::ZERO,
                ..opts()
            },
            || panic!("invalid"),
            |_: &()| false,
            || false,
            &mut FakeClock::default(),
        )
        .unwrap_err();
        assert_eq!(result.reason, WaitReason::InvalidOptions);
    }
    #[test]
    fn predicate_can_require_change_before_accepting_old_success() {
        let mut frames = [true, false, true].into_iter();
        let mut changed = false;
        let result = poll(
            opts(),
            || Ok(frames.next().unwrap()),
            |done| {
                changed |= !*done;
                changed && *done
            },
            || false,
            &mut FakeClock::default(),
        )
        .unwrap();
        assert_eq!(result.attempts, 3);
    }
    #[test]
    fn late_success_and_cancellation_after_callbacks_are_rejected() {
        use std::cell::Cell;
        struct SharedClock<'a>(&'a Cell<Duration>);
        impl Clock for SharedClock<'_> {
            fn elapsed(&self) -> Duration {
                self.0.get()
            }
            fn sleep(&mut self, duration: Duration) {
                self.0.set(self.0.get() + duration);
            }
        }
        for during_capture in [true, false] {
            let elapsed = Cell::new(Duration::ZERO);
            let result = poll(
                opts(),
                || {
                    if during_capture {
                        elapsed.set(opts().timeout);
                    }
                    Ok(42)
                },
                |_| {
                    elapsed.set(opts().timeout);
                    true
                },
                || false,
                &mut SharedClock(&elapsed),
            )
            .unwrap_err();
            assert_eq!(result.reason, WaitReason::Timeout);
            assert_eq!((result.last, result.attempts), (Some(42), 1));

            let cancelled = Cell::new(false);
            let result = poll(
                opts(),
                || {
                    if during_capture {
                        cancelled.set(true);
                    }
                    Ok(42)
                },
                |_| {
                    cancelled.set(true);
                    true
                },
                || cancelled.get(),
                &mut FakeClock::default(),
            )
            .unwrap_err();
            assert_eq!(result.reason, WaitReason::Cancelled);
            assert_eq!((result.last, result.attempts), (Some(42), 1));
        }
    }
}
