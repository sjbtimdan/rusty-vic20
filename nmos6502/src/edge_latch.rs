/// Edge detector for NMI line (active-low, falling-edge triggered).
///
/// The NMOS 6502 NMI input is edge-sensitive: an interrupt fires when the
/// NMI line transitions from HIGH to LOW. Holding NMI low does not cause
/// repeated interrupts — the line must return HIGH before another falling
/// edge is recognized.
#[derive(Debug, Clone)]
pub struct EdgeLatch {
    previous_level: bool,
    latched: bool,
}

impl EdgeLatch {
    pub fn new(initial_level: bool) -> Self {
        Self {
            previous_level: initial_level,
            latched: false,
        }
    }

    /// Update the input level. If a falling edge (HIGH → LOW) is detected,
    /// the latch is set. Once latched, further level changes do not clear
    /// the latch — only `take()` clears it.
    pub fn set_level(&mut self, level: bool) {
        if self.previous_level && !level {
            self.latched = true;
        }
        self.previous_level = level;
    }

    /// Returns true if a falling edge was detected since the last `take()`.
    /// Clears the latch.
    pub fn take(&mut self) -> bool {
        let was_latched = self.latched;
        self.latched = false;
        was_latched
    }

    /// Returns true if a falling edge is currently latched (without clearing).
    pub fn is_latched(&self) -> bool {
        self.latched
    }

    /// Reset the latch (e.g., on CPU reset).
    pub fn reset(&mut self) {
        self.latched = false;
    }
}

impl Default for EdgeLatch {
    fn default() -> Self {
        Self::new(true) // NMI line idles HIGH
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_falling_edge_detected() {
        let mut latch = EdgeLatch::new(true);
        assert!(!latch.take());

        latch.set_level(false); // HIGH → LOW
        assert!(latch.take());

        latch.set_level(false); // stays LOW
        assert!(!latch.take()); // No new edge
    }

    #[test]
    fn test_rising_then_falling() {
        let mut latch = EdgeLatch::new(true);

        latch.set_level(false);
        assert!(latch.take());

        latch.set_level(true); // LOW → HIGH
        assert!(!latch.take());

        latch.set_level(false); // HIGH → LOW (new edge)
        assert!(latch.take());
    }

    #[test]
    fn test_reset_clears_latch() {
        let mut latch = EdgeLatch::new(true);
        latch.set_level(false);
        assert!(latch.is_latched());

        latch.reset();
        assert!(!latch.is_latched());
    }
}
