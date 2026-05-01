use crate::virtual_clock::{Clock, SystemClock};
use std::time::{Duration, Instant};

pub struct ScheduledOperation<C: Clock = SystemClock>
where
    C: Clock,
{
    interval: Duration,
    clock: C,
    last_called: Instant,
}

impl<C> ScheduledOperation<C>
where
    C: Clock,
{
    pub fn new(interval: Duration, clock: C) -> Self {
        Self {
            interval,
            clock,
            last_called: Instant::now() - Duration::from_hours(10_000_000),
        }
    }

    pub fn call_if_ready<F: FnOnce()>(&mut self, f: F) {
        let now = self.clock.now();
        if now >= self.last_called + self.interval {
            self.last_called = now;
            f();
        }
    }
}
