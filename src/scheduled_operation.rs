use crate::virtual_clock::{Clock, SystemClock};
use std::time::{Duration, Instant};

pub struct ScheduledOperation<F, C = SystemClock>
where
    C: Clock,
    F: FnMut(),
{
    interval: Duration,
    operation: F,
    clock: C,
    last_called: Instant,
}

impl<F, C> ScheduledOperation<F, C>
where
    C: Clock,
    F: FnMut(),
{
    pub fn new(interval: Duration, operation: F, clock: C) -> Self {
        Self {
            interval,
            operation,
            clock,
            last_called: Instant::now() - Duration::from_hours(10_000_000),
        }
    }

    pub fn run(&mut self) {
        let now = self.clock.now();
        if now >= self.last_called + self.interval {
            (self.operation)();
            self.last_called = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::virtual_clock::MockClock;
    use rstest::{fixture, rstest};
    use std::cell::Cell;
    use std::rc::Rc;

    #[fixture]
    fn mock_clock() -> MockClock {
        MockClock::new(Instant::now())
    }

    #[fixture]
    fn call_count() -> Rc<Cell<usize>> {
        Rc::new(Cell::new(0))
    }

    #[rstest]
    fn first_run_executes_operation(mock_clock: MockClock, call_count: Rc<Cell<usize>>) {
        let call_count_clone = Rc::clone(&call_count);
        let mut scheduled_operation = ScheduledOperation::new(
            Duration::from_millis(100),
            move || call_count_clone.set(call_count_clone.get() + 1),
            mock_clock,
        );

        scheduled_operation.run();

        assert_eq!(call_count.get(), 1);
    }

    #[rstest]
    fn run_within_interval_does_not_execute_operation(mock_clock: MockClock, call_count: Rc<Cell<usize>>) {
        let call_count_clone = Rc::clone(&call_count);
        let mut scheduled_operation = ScheduledOperation::new(
            Duration::from_millis(100),
            move || call_count_clone.set(call_count_clone.get() + 1),
            mock_clock,
        );

        scheduled_operation.run();
        scheduled_operation.clock.advance(Duration::from_millis(50));
        scheduled_operation.run();

        assert_eq!(call_count.get(), 1);
    }

    #[rstest]
    fn run_at_interval_executes_operation(mock_clock: MockClock, call_count: Rc<Cell<usize>>) {
        let call_count_clone = Rc::clone(&call_count);
        let mut scheduled_operation = ScheduledOperation::new(
            Duration::from_millis(100),
            move || call_count_clone.set(call_count_clone.get() + 1),
            mock_clock,
        );

        scheduled_operation.run();
        scheduled_operation.clock.advance(Duration::from_millis(100));
        scheduled_operation.run();

        assert_eq!(call_count.get(), 2);
    }
}
