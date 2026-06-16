use crate::virtual_clock::{Clock, SystemClock};
use std::{
    sync::mpsc::{Receiver, SyncSender},
    time::{Duration, Instant},
};

const TARGET_CPU_FREQ_HZ: u64 = 1_108_404;
const CYCLES_PER_TIMING_SYNC: u64 = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BrakeSpeed {
    #[default]
    Normal,
    Quarter,
    Half,
    TwoX,
    Max,
}

pub struct Brake<C: Clock = SystemClock> {
    cycle_duration: Duration,
    last_timing_sync: Instant,
    cycles_since_sync: u64,
    cycles_per_timing_sync: u64,
    speed_receiver: Receiver<BrakeSpeed>,
    clock: C,
}

pub fn make_brake_channel() -> (SyncSender<BrakeSpeed>, Receiver<BrakeSpeed>) {
    std::sync::mpsc::sync_channel(2)
}

impl Brake {
    pub fn new_default(speed_receiver: Receiver<BrakeSpeed>) -> Self {
        let clock = SystemClock;
        Brake {
            cycle_duration: Duration::from_nanos(1_000_000_000 / TARGET_CPU_FREQ_HZ),
            last_timing_sync: clock.now(),
            cycles_since_sync: 0,
            cycles_per_timing_sync: CYCLES_PER_TIMING_SYNC,
            speed_receiver,
            clock,
        }
    }
}

impl<C: Clock> Brake<C> {
    pub fn new(speed_receiver: Receiver<BrakeSpeed>, clock: C) -> Self {
        let now = clock.now();
        Brake {
            cycle_duration: Duration::from_nanos(1_000_000_000 / TARGET_CPU_FREQ_HZ),
            last_timing_sync: now,
            cycles_since_sync: 0,
            cycles_per_timing_sync: CYCLES_PER_TIMING_SYNC,
            speed_receiver,
            clock,
        }
    }

    pub fn step(&mut self) {
        if let Ok(speed) = self.speed_receiver.try_recv() {
            self.update_speed(speed);
        }

        self.cycles_since_sync += 1;
        if self.cycles_since_sync > self.cycles_per_timing_sync {
            let expected = self.cycle_duration.saturating_mul(self.cycles_since_sync as u32);
            let elapsed = self
                .clock
                .now()
                .checked_duration_since(self.last_timing_sync)
                .unwrap_or_default();
            if let Some(delay) = expected.checked_sub(elapsed) {
                self.clock.sleep(delay);
            }
            self.last_timing_sync = self.clock.now();
            self.cycles_since_sync = 0;
        }
    }

    fn update_speed(&mut self, speed: BrakeSpeed) {
        let multiplier = match speed {
            BrakeSpeed::Quarter => 0.25,
            BrakeSpeed::Half => 0.5,
            BrakeSpeed::Normal => 1.0,
            BrakeSpeed::TwoX => 2.0,
            BrakeSpeed::Max => 0.0,
        };
        let effective_hz = TARGET_CPU_FREQ_HZ as f64 * multiplier;
        self.cycle_duration = if effective_hz > 0.0 {
            Duration::from_nanos((1_000_000_000.0 / effective_hz) as u64)
        } else {
            Duration::ZERO
        };
        self.last_timing_sync = self.clock.now();
        self.cycles_since_sync = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::virtual_clock::MockClock;
    use rstest::{fixture, rstest};
    use std::sync::OnceLock;

    fn test_start() -> Instant {
        static START: OnceLock<Instant> = OnceLock::new();
        *START.get_or_init(Instant::now)
    }

    fn normal_cycle_duration() -> Duration {
        Duration::from_nanos(1_000_000_000 / TARGET_CPU_FREQ_HZ)
    }

    #[fixture]
    fn brake() -> Brake<MockClock> {
        let (_tx, rx) = make_brake_channel();
        let clock = MockClock::new(test_start());
        Brake::new(rx, clock)
    }

    #[fixture]
    fn brake_with_sender() -> (SyncSender<BrakeSpeed>, Brake<MockClock>) {
        let (tx, rx) = make_brake_channel();
        let clock = MockClock::new(test_start());
        (tx, Brake::new(rx, clock))
    }

    #[rstest]
    fn normal_speed_sleeps_correct_amount(mut brake: Brake<MockClock>) {
        let start = brake.clock.now();

        for _ in 0..(CYCLES_PER_TIMING_SYNC as usize + 2) {
            brake.step();
        }

        let elapsed = brake.clock.now().duration_since(start);
        let cycle_ns = normal_cycle_duration().as_nanos() as u64;
        let expected = Duration::from_nanos((CYCLES_PER_TIMING_SYNC + 1) * cycle_ns);
        assert_eq!(elapsed, expected);
    }

    #[rstest]
    fn max_speed_never_sleeps(brake_with_sender: (SyncSender<BrakeSpeed>, Brake<MockClock>)) {
        let (tx, mut brake) = brake_with_sender;
        let start = brake.clock.now();
        tx.send(BrakeSpeed::Max).unwrap();

        for _ in 0..(CYCLES_PER_TIMING_SYNC as usize + 2) {
            brake.step();
        }

        assert_eq!(brake.clock.now(), start);
    }

    #[rstest]
    fn quarter_speed_sleeps_longer(brake_with_sender: (SyncSender<BrakeSpeed>, Brake<MockClock>)) {
        let (tx, mut brake) = brake_with_sender;
        let start = brake.clock.now();
        tx.send(BrakeSpeed::Quarter).unwrap();

        for _ in 0..(CYCLES_PER_TIMING_SYNC as usize + 2) {
            brake.step();
        }

        let elapsed = brake.clock.now().duration_since(start);
        let cycle_ns = normal_cycle_duration().as_nanos() as u64 * 4;
        let expected = Duration::from_nanos((CYCLES_PER_TIMING_SYNC + 1) * cycle_ns);
        assert_eq!(elapsed, expected);
    }

    #[rstest]
    fn speed_update_resets_timing(brake_with_sender: (SyncSender<BrakeSpeed>, Brake<MockClock>)) {
        let (tx, mut brake) = brake_with_sender;
        let start = brake.clock.now();

        for _ in 0..(CYCLES_PER_TIMING_SYNC as usize / 2) {
            brake.step();
        }
        assert_eq!(brake.clock.now(), start);

        tx.send(BrakeSpeed::Half).unwrap();
        brake.step();

        for _ in 0..CYCLES_PER_TIMING_SYNC as usize {
            brake.step();
        }

        let elapsed = brake.clock.now().duration_since(start);
        let half_cycle_ns = normal_cycle_duration().as_nanos() as u64 * 2;
        let expected = Duration::from_nanos((CYCLES_PER_TIMING_SYNC + 1) * half_cycle_ns);
        assert_eq!(elapsed, expected);
    }
}
