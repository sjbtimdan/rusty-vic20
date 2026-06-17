#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Rising,
    Falling,
}

#[derive(Debug)]
pub struct EdgeLatch {
    level: bool,
    was_level: bool,
    latched: bool,
    edge: Edge,
}

impl EdgeLatch {
    pub fn new_rising() -> Self {
        Self::new(Edge::Rising)
    }

    pub fn new_falling() -> Self {
        Self::new(Edge::Falling)
    }

    const fn new(edge: Edge) -> Self {
        Self {
            level: false,
            was_level: false,
            latched: false,
            edge,
        }
    }

    pub fn set_level(&mut self, level: bool) {
        self.was_level = self.level;
        self.level = level;
        let edge_detected = match self.edge {
            Edge::Rising => !self.was_level && self.level,
            Edge::Falling => self.was_level && !self.level,
        };
        if edge_detected {
            self.latched = true;
        }
    }

    #[must_use = "latch state is lost if not checked"]
    pub fn take(&mut self) -> bool {
        if self.latched {
            self.latched = false;
            true
        } else {
            false
        }
    }

    pub fn set_edge(&mut self, edge: Edge) {
        self.edge = edge;
    }

    #[must_use]
    pub fn is_latched(&self) -> bool {
        self.latched
    }

    pub fn reset(&mut self) {
        self.level = false;
        self.was_level = false;
        self.latched = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn latch() -> EdgeLatch {
        EdgeLatch::new_rising()
    }

    #[rstest]
    fn new_rising_is_default_state() {
        let mut latch = EdgeLatch::new_rising();
        assert!(!latch.is_latched());
        assert!(!latch.take());
    }

    #[rstest]
    fn rising_edge_detects_false_to_true(mut latch: EdgeLatch) {
        latch.set_level(false);
        assert!(!latch.is_latched());

        latch.set_level(true);
        assert!(latch.is_latched());
    }

    #[rstest]
    fn rising_edge_ignores_true_to_true(mut latch: EdgeLatch) {
        latch.set_level(true);
        assert!(latch.is_latched());

        let _ = latch.take();
        latch.set_level(true);
        assert!(!latch.is_latched());
    }

    #[rstest]
    fn rising_edge_ignores_true_to_false(mut latch: EdgeLatch) {
        latch.set_level(false);
        assert!(!latch.is_latched());

        latch.set_level(true);
        assert!(latch.is_latched());

        let _ = latch.take();
        latch.set_level(false);
        assert!(!latch.is_latched());
    }

    #[rstest]
    fn take_consumes_latch(mut latch: EdgeLatch) {
        latch.set_level(true);
        assert!(latch.is_latched());
        assert!(latch.take());
        assert!(!latch.is_latched());
        assert!(!latch.take());
    }

    #[rstest]
    fn falling_edge_detects_true_to_false() {
        let mut latch = EdgeLatch::new_falling();
        latch.set_level(true);
        assert!(!latch.is_latched());

        latch.set_level(false);
        assert!(latch.is_latched());
    }

    #[rstest]
    fn falling_edge_ignores_false_to_false() {
        let mut latch = EdgeLatch::new_falling();
        latch.set_level(true);
        let _ = latch.take();
        latch.set_level(false);
        let _ = latch.take();

        latch.set_level(false);
        assert!(!latch.is_latched());
    }

    #[rstest]
    fn reset_clears_all_state(mut latch: EdgeLatch) {
        latch.set_level(true);
        assert!(latch.is_latched());

        latch.reset();
        assert!(!latch.is_latched());

        latch.set_level(true);
        assert!(latch.is_latched());
    }
}
