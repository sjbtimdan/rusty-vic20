use std::{
    cell::Cell,
    time::{Duration, Instant},
};

pub trait Clock {
    fn now(&self) -> Instant;
    fn sleep(&self, duration: Duration);
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

pub struct MockClock {
    now: Cell<Instant>,
}

impl MockClock {
    pub fn new(now: Instant) -> Self {
        Self { now: Cell::new(now) }
    }
}

impl Clock for MockClock {
    fn now(&self) -> Instant {
        self.now.get()
    }

    fn sleep(&self, duration: Duration) {
        self.now.set(self.now.get() + duration);
    }
}
