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

pub struct MockClock {
    pub now: Instant,
}

impl MockClock {
    pub fn new(now: Instant) -> Self {
        Self { now }
    }

    pub fn advance(&mut self, by: std::time::Duration) {
        self.now += by;
    }
}

impl Clock for MockClock {
    fn now(&self) -> Instant {
        self.now
    }
}
