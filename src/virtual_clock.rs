use std::time::Instant;

pub trait Clock {
    fn now(&self) -> Instant;
}

#[derive(Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
