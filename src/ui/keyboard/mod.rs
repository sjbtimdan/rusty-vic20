//! Keyboard layout, click/hold/flash state machine — no GUI dependencies.

pub mod display;

use crate::virtual_clock::{Clock, SystemClock};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub struct Keyboard {
    restore_triggered: Arc<AtomicBool>,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self {
            restore_triggered: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Keyboard {
    pub fn new() -> (Self, Arc<AtomicBool>) {
        let flag = Arc::new(AtomicBool::new(false));
        (
            Self {
                restore_triggered: Arc::clone(&flag),
            },
            flag,
        )
    }

    pub fn on_restore(&self) {
        self.restore_triggered.store(true, Ordering::SeqCst);
    }

    pub fn restore_trigger(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.restore_triggered)
    }
}
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

pub const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(350);
pub const FLASH_DURATION: Duration = Duration::from_millis(200);

#[derive(Clone, Debug, PartialEq)]
pub struct KeyRegion {
    pub label: &'static str,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClickKind {
    Single,
    Double,
}

/// Non-GUI keyboard interaction state.
pub struct KeyboardState<C = SystemClock> {
    pub clock: C,
    pub key_regions: Vec<KeyRegion>,
    pub last_click: Option<(String, Instant)>,
    pub held_key: Option<String>,
    pub flash_key: Option<(String, Instant)>,
    pub physical_keys: HashSet<&'static str>,
    pub status_message: String,
    pub keyboard: Keyboard,
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardState {
    pub fn new() -> Self {
        Self::with_clock(SystemClock)
    }
}

impl<C: Clock> KeyboardState<C> {
    pub fn with_clock(clock: C) -> Self {
        Self {
            clock,
            key_regions: build_key_regions(),
            last_click: None,
            held_key: None,
            flash_key: None,
            physical_keys: HashSet::new(),
            status_message: String::from("Click a key. Double-click toggles hold."),
            keyboard: Keyboard::default(),
        }
    }

    pub fn restore_trigger(&self) -> Arc<AtomicBool> {
        self.keyboard.restore_trigger()
    }

    /// Given image-space pixel coordinates, returns the label of the key there (if any).
    pub fn key_at_pixel(&self, image_x: f32, image_y: f32) -> Option<&'static str> {
        self.key_regions
            .iter()
            .find(|r| image_x >= r.x && image_x <= r.x + r.w && image_y >= r.y && image_y <= r.y + r.h)
            .map(|r| r.label)
    }

    /// Classify a click as [`ClickKind::Single`] or [`ClickKind::Double`].
    /// Updates `last_click` as a side-effect.
    pub fn classify_click(&mut self, key: &str) -> ClickKind {
        let now = self.clock.now();
        if let Some((last_key, last_time)) = &self.last_click
            && last_key == key
            && now.duration_since(*last_time) <= DOUBLE_CLICK_THRESHOLD
        {
            self.last_click = None;
            return ClickKind::Double;
        }
        self.last_click = Some((key.to_string(), now));
        ClickKind::Single
    }

    /// Process a mouse click on `key`, updating held/flash/status state.
    pub fn on_key_click(&mut self, key: &str) {
        match self.classify_click(key) {
            ClickKind::Single => {
                let now = self.clock.now();
                if let Some(held) = &self.held_key
                    && held != key
                {
                    self.status_message = format!("CHORD: {held} + {key} pressed together, then released");
                    self.held_key = None;
                    self.flash_key = Some((key.to_string(), now));
                    if key == "RESTORE" {
                        self.keyboard.on_restore();
                    }
                    return;
                }
                self.flash_key = Some((key.to_string(), now));
                self.status_message = format!("CLICK: {key}");
                if key == "RESTORE" {
                    self.keyboard.on_restore();
                }
            }
            ClickKind::Double => {
                if self.held_key.as_deref() == Some(key) {
                    self.held_key = None;
                    self.status_message = format!("RELEASE HOLD: {key}");
                } else {
                    self.held_key = Some(key.to_string());
                    self.status_message = format!("HOLD: {key}");
                }
                if key == "RESTORE" {
                    self.keyboard.on_restore();
                }
            }
        }
    }

    /// Record a physical key press. Returns `true` if the key was newly pressed.
    pub fn physical_key_pressed(&mut self, key: &'static str) -> bool {
        if self.physical_keys.insert(key) {
            self.status_message = format!("KEY: {key}");
            true
        } else {
            false
        }
    }

    /// Record a physical key release, clearing status when no keys remain held.
    pub fn physical_key_released(&mut self, key: &'static str) {
        self.physical_keys.remove(key);
        if self.physical_keys.is_empty() {
            self.status_message = String::from("Click a key. Double-click toggles hold.");
        }
    }

    /// Expire the active flash if its duration has elapsed.
    pub fn tick_flash(&mut self) {
        let now = self.clock.now();
        let expired = self
            .flash_key
            .as_ref()
            .is_some_and(|(_, t)| now.duration_since(*t) >= FLASH_DURATION);
        if expired {
            self.flash_key = None;
        }
    }

    /// Time remaining in the current flash, or `None` if no flash is active.
    pub fn flash_remaining(&self) -> Option<Duration> {
        let now = self.clock.now();
        self.flash_key
            .as_ref()
            .map(|(_, t)| FLASH_DURATION.saturating_sub(now.duration_since(*t)))
    }
}

pub fn build_key_regions() -> Vec<KeyRegion> {
    // All coordinates are measured directly from data/vic20-c64-layout.png (1006×290).
    #[rustfmt::skip]
    let data: &[(&'static str, i32, i32, i32, i32)] = &[
        // ── Number row (y=11..62) ──────────────────────────────────────────
        ("LEFT",     23,  11,  54, 52),
        ("1",        78,  11,  53, 52),
        ("2",       132,  11,  53, 52),
        ("3",       186,  11,  53, 52),
        ("4",       240,  11,  53, 52),
        ("5",       294,  11,  53, 52),
        ("6",       348,  11,  53, 52),
        ("7",       402,  11,  53, 52),
        ("8",       456,  11,  53, 52),
        ("9",       510,  11,  53, 52),
        ("0",       564,  11,  53, 52),
        ("+",       618,  11,  53, 52),
        ("-",       672,  11,  53, 52),
        ("POUND",   726,  11,  53, 52),
        ("CLR/HOME",780,  11,  53, 52),
        ("INS/DEL", 834,  11,  36, 52),

        // ── CTRL/Q row (y=65..116) ────────────────────────────────────────
        ("CTRL",     23,  65,  81, 52),
        ("Q",       105,  65,  53, 52),
        ("W",       159,  65,  53, 52),
        ("E",       213,  65,  53, 52),
        ("R",       267,  65,  53, 52),
        ("T",       321,  65,  53, 52),
        ("Y",       375,  65,  53, 52),
        ("U",       429,  65,  53, 52),
        ("I",       483,  65,  53, 52),
        ("O",       537,  65,  53, 52),
        ("P",       591,  65,  53, 52),
        ("@",       645,  65,  53, 52),
        ("*",       699,  65,  53, 52),
        ("UP",      753,  65,  53, 52),
        ("RESTORE", 807,  65,  63, 52),

        // ── RUN/STOP / A row (y=119..170) ────────────────────────────────
        ("RUN/STOP",   11, 119,  52, 52),
        ("SHIFT LOCK", 65, 119,  52, 52),
        ("A",         119, 119,  52, 52),
        ("S",         173, 119,  52, 52),
        ("D",         227, 119,  52, 52),
        ("F",         281, 119,  52, 52),
        ("G",         335, 119,  52, 52),
        ("H",         389, 119,  52, 52),
        ("J",         443, 119,  52, 52),
        ("K",         497, 119,  52, 52),
        ("L",         551, 119,  52, 52),
        ("[",         605, 119,  52, 52),
        ("]",         659, 119,  52, 52),
        ("=",         713, 119,  52, 52),
        ("RETURN",    767, 119, 103, 52),

        // ── CBM / Z row (y=173..224) ──────────────────────────────────────
        ("CBM",        11, 173,  52, 52),
        ("SHIFT",      65, 173,  79, 52),
        ("Z",         146, 173,  52, 52),
        ("X",         200, 173,  52, 52),
        ("C",         254, 173,  52, 52),
        ("V",         308, 173,  52, 52),
        ("B",         362, 173,  52, 52),
        ("N",         416, 173,  52, 52),
        ("M",         470, 173,  52, 52),
        (",",         524, 173,  52, 52),
        (".",         578, 173,  52, 52),
        ("/",         632, 173,  52, 52),
        ("SHIFT",     686, 173,  79, 52),
        ("CRSR UD",   767, 173,  52, 52),
        ("CRSR LR",   821, 173,  49, 52),

        // ── Space bar row (y=226..289) ────────────────────────────────────
        ("SPACE",     158, 226, 487, 64),

        // ── F-keys (right column, each spans a full key row) ──────────────
        ("F1/F2",  910,  11, 90, 52),
        ("F3/F4",  910,  65, 90, 52),
        ("F5/F6",  910, 119, 90, 52),
        ("F7/F8",  910, 173, 90, 52),
    ];

    data.iter()
        .map(|&(label, x, y, w, h)| KeyRegion {
            label,
            x: x as f32,
            y: y as f32,
            w: w as f32,
            h: h as f32,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::Cell,
        time::{Duration, Instant},
    };

    struct MockClock {
        time: Cell<Instant>,
    }

    impl MockClock {
        fn new() -> Self {
            Self {
                time: Cell::new(Instant::now()),
            }
        }

        fn advance(&self, d: Duration) {
            self.time.set(self.time.get() + d);
        }
    }

    impl Clock for MockClock {
        fn now(&self) -> Instant {
            self.time.get()
        }
    }

    fn mock_state() -> KeyboardState<MockClock> {
        KeyboardState::with_clock(MockClock::new())
    }

    fn just_clicked<C: Clock>(state: &KeyboardState<C>, label: &str) -> bool {
        state.flash_key.as_ref().is_some_and(|(k, _)| k == label)
    }

    // ── key_at_pixel ──────────────────────────────────────────────────────

    #[test]
    fn pixel_inside_space_bar() {
        let state = mock_state();
        assert_eq!(state.key_at_pixel(300.0, 250.0), Some("SPACE"));
    }

    #[test]
    fn pixel_inside_a_key() {
        let state = mock_state();
        assert_eq!(state.key_at_pixel(140.0, 130.0), Some("A"));
    }

    #[test]
    fn pixel_in_gap_returns_none() {
        let state = mock_state();
        // x=0, y=0 is outside every defined region
        assert_eq!(state.key_at_pixel(0.0, 0.0), None);
    }

    #[test]
    fn pixel_inside_fkey() {
        let state = mock_state();
        assert_eq!(state.key_at_pixel(940.0, 30.0), Some("F1/F2"));
        assert_eq!(state.key_at_pixel(940.0, 80.0), Some("F3/F4"));
        assert_eq!(state.key_at_pixel(940.0, 140.0), Some("F5/F6"));
        assert_eq!(state.key_at_pixel(940.0, 190.0), Some("F7/F8"));
    }

    // ── single click ──────────────────────────────────────────────────────

    #[test]
    fn single_click_sets_flash_and_status() {
        let mut state = mock_state();
        state.on_key_click("A");
        assert!(just_clicked(&state, "A"), "flash should be set to A");
        assert_eq!(state.status_message, "CLICK: A");
        assert!(state.held_key.is_none());
    }

    // ── double click ──────────────────────────────────────────────────────

    #[test]
    fn classify_click_returns_double_within_threshold() {
        let mut state = mock_state();
        let now = state.clock.now();
        state.last_click = Some(("A".to_string(), now));
        assert_eq!(state.classify_click("A"), ClickKind::Double);
    }

    #[test]
    fn classify_click_returns_single_after_threshold() {
        let mut state = mock_state();
        let past = state.clock.now();
        state.last_click = Some(("A".to_string(), past));
        state.clock.advance(DOUBLE_CLICK_THRESHOLD + Duration::from_millis(1));
        assert_eq!(state.classify_click("A"), ClickKind::Single);
    }

    #[test]
    fn double_click_holds_key() {
        let mut state = mock_state();
        let now = state.clock.now();
        state.last_click = Some(("A".to_string(), now));
        state.on_key_click("A"); // triggers Double
        assert_eq!(state.held_key.as_deref(), Some("A"));
        assert_eq!(state.status_message, "HOLD: A");
    }

    #[test]
    fn double_click_on_held_key_releases_it() {
        let mut state = mock_state();
        state.held_key = Some("A".to_string());
        let now = state.clock.now();
        state.last_click = Some(("A".to_string(), now));
        state.on_key_click("A"); // triggers Double
        assert!(state.held_key.is_none());
        assert_eq!(state.status_message, "RELEASE HOLD: A");
    }

    // ── chord ──────────────────────────────────────────────────────────────

    #[test]
    fn single_click_while_holding_produces_chord() {
        let mut state = mock_state();
        state.held_key = Some("SHIFT".to_string());
        // Ensure first click registers as single (no recent last_click)
        state.on_key_click("A");
        assert_eq!(state.status_message, "CHORD: SHIFT + A pressed together, then released");
        assert!(state.held_key.is_none());
        assert!(just_clicked(&state, "A"), "flash should be set to the chord key");
    }

    #[test]
    fn single_click_on_already_held_key_just_flashes() {
        let mut state = mock_state();
        state.held_key = Some("A".to_string());
        state.on_key_click("A"); // single click on the held key itself
        // Should be treated as normal click, not chord
        assert_eq!(state.status_message, "CLICK: A");
        assert!(just_clicked(&state, "A"));
    }

    // ── physical keys ─────────────────────────────────────────────────────

    #[test]
    fn physical_key_pressed_updates_status() {
        let mut state = mock_state();
        let inserted = state.physical_key_pressed("Q");
        assert!(inserted);
        assert!(state.physical_keys.contains("Q"));
        assert_eq!(state.status_message, "KEY: Q");
    }

    #[test]
    fn physical_key_pressed_again_returns_false() {
        let mut state = mock_state();
        state.physical_key_pressed("Q");
        let inserted = state.physical_key_pressed("Q");
        assert!(!inserted);
    }

    #[test]
    fn physical_key_released_clears_status_when_empty() {
        let mut state = mock_state();
        state.physical_key_pressed("Q");
        state.physical_key_released("Q");
        assert!(state.physical_keys.is_empty());
        assert_eq!(state.status_message, "Click a key. Double-click toggles hold.");
    }

    #[test]
    fn physical_key_released_keeps_status_while_other_keys_held() {
        let mut state = mock_state();
        state.physical_key_pressed("Q");
        state.physical_key_pressed("W");
        state.physical_key_released("Q");
        assert!(state.physical_keys.contains("W"));
        // status should not reset to default
        assert_ne!(state.status_message, "Click a key. Double-click toggles hold.");
    }

    // ── flash timer ───────────────────────────────────────────────────────

    #[test]
    fn tick_flash_does_nothing_when_no_flash() {
        let mut state = mock_state();
        state.tick_flash(); // must not panic
        assert!(state.flash_key.is_none());
    }

    #[test]
    fn tick_flash_clears_expired_flash() {
        let mut state = mock_state();
        let past = state.clock.now();
        state.flash_key = Some(("A".to_string(), past));
        state.clock.advance(FLASH_DURATION + Duration::from_millis(1));
        state.tick_flash();
        assert!(state.flash_key.is_none());
    }

    #[test]
    fn tick_flash_keeps_active_flash() {
        let mut state = mock_state();
        let now = state.clock.now();
        state.flash_key = Some(("A".to_string(), now));
        state.tick_flash();
        assert!(state.flash_key.is_some());
    }

    #[test]
    fn flash_remaining_none_when_no_flash() {
        let state = mock_state();
        assert_eq!(state.flash_remaining(), None);
    }

    #[test]
    fn flash_remaining_some_for_active_flash() {
        let mut state = mock_state();
        let now = state.clock.now();
        state.flash_key = Some(("A".to_string(), now));
        assert!(state.flash_remaining().is_some());
    }
}
