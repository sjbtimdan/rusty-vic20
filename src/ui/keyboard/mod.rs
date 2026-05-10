//! Keyboard layout, click/hold/flash state machine — no GUI dependencies.

pub mod display;
pub mod key;

use crate::{
    ui::keyboard::key::Key,
    virtual_clock::{Clock, SystemClock},
};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

pub struct Keyboard;

pub const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(350);
pub const FLASH_DURATION: Duration = Duration::from_millis(200);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyRegion {
    pub label: Key,
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
    pub last_click: Option<(Key, Instant)>,
    pub held_key: Option<Key>,
    pub flash_key: Option<(Key, Instant)>,
    pub physical_keys: HashSet<Key>,
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
            keyboard: Keyboard,
        }
    }

    /// Given image-space pixel coordinates, returns the label of the key there (if any).
    pub fn key_at_pixel(&self, image_x: f32, image_y: f32) -> Option<Key> {
        self.key_regions
            .iter()
            .find(|r| image_x >= r.x && image_x <= r.x + r.w && image_y >= r.y && image_y <= r.y + r.h)
            .map(|r| r.label)
    }

    /// Classify a click as [`ClickKind::Single`] or [`ClickKind::Double`].
    /// Updates `last_click` as a side-effect.
    pub fn classify_click(&mut self, key: Key) -> ClickKind {
        let now = self.clock.now();
        if let Some((last_key, last_time)) = &self.last_click
            && *last_key == key
            && now.duration_since(*last_time) <= DOUBLE_CLICK_THRESHOLD
        {
            self.last_click = None;
            return ClickKind::Double;
        }
        self.last_click = Some((key, now));
        ClickKind::Single
    }

    /// Process a mouse click on `key`, updating held/flash/status state.
    pub fn on_key_click(&mut self, key: Key) {
        match self.classify_click(key) {
            ClickKind::Single => {
                let now = self.clock.now();
                if let Some(held) = self.held_key
                    && held != key
                {
                    self.physical_keys.remove(&held);
                    self.status_message = format!("CHORD: {held} + {key} pressed together, then released");
                    self.held_key = None;
                    self.flash_key = Some((key, now));
                    self.physical_keys.insert(key);
                    return;
                }
                self.flash_key = Some((key, now));
                self.status_message = format!("CLICK: {key}");
                self.physical_keys.insert(key);
            }
            ClickKind::Double => {
                if self.held_key == Some(key) {
                    self.held_key = None;
                    self.physical_keys.remove(&key);
                    self.status_message = format!("RELEASE HOLD: {key}");
                } else {
                    self.held_key = Some(key);
                    self.physical_keys.insert(key);
                    self.status_message = format!("HOLD: {key}");
                }
            }
        }
    }

    /// Record a physical key press. Returns `true` if the key was newly pressed.
    pub fn physical_key_pressed(&mut self, key: Key) -> bool {
        if self.physical_keys.insert(key) {
            self.status_message = format!("KEY: {key}");
            true
        } else {
            false
        }
    }

    /// Record a physical key release, clearing status when no keys remain held.
    pub fn physical_key_released(&mut self, key: Key) {
        self.physical_keys.remove(&key);
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
        if expired && let Some((key, _)) = self.flash_key.take() {
            self.physical_keys.remove(&key);
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
    let data: &[(Key, i32, i32, i32, i32)] = &[
        // ── Number row (y=11..62) ──────────────────────────────────────────
        (Key::Left,     23,  11,  54, 52),
        (Key::Single('1'),        78,  11,  53, 52),
        (Key::Single('2'),       132,  11,  53, 52),
        (Key::Single('3'),       186,  11,  53, 52),
        (Key::Single('4'),       240,  11,  53, 52),
        (Key::Single('5'),       294,  11,  53, 52),
        (Key::Single('6'),       348,  11,  53, 52),
        (Key::Single('7'),       402,  11,  53, 52),
        (Key::Single('8'),       456,  11,  53, 52),
        (Key::Single('9'),       510,  11,  53, 52),
        (Key::Single('0'),       564,  11,  53, 52),
        (Key::Single('+'),       618,  11,  53, 52),
        (Key::Single('-'),       672,  11,  53, 52),
        (Key::Single('£'),       726,  11,  53, 52),
        (Key::ClrHome, 780,  11,  53, 52),
        (Key::InsDel,  834,  11,  36, 52),

        // ── CTRL/Q row (y=65..116) ────────────────────────────────────────
        (Key::Ctrl,     23,  65,  81, 52),
        (Key::Single('Q'),       105,  65,  53, 52),
        (Key::Single('W'),       159,  65,  53, 52),
        (Key::Single('E'),       213,  65,  53, 52),
        (Key::Single('R'),       267,  65,  53, 52),
        (Key::Single('T'),       321,  65,  53, 52),
        (Key::Single('Y'),       375,  65,  53, 52),
        (Key::Single('U'),       429,  65,  53, 52),
        (Key::Single('I'),       483,  65,  53, 52),
        (Key::Single('O'),       537,  65,  53, 52),
        (Key::Single('P'),       591,  65,  53, 52),
        (Key::Single('@'),       645,  65,  53, 52),
        (Key::Single('*'),       699,  65,  53, 52),
        (Key::Up,       753,  65,  53, 52),
        (Key::Restore,  807,  65,  63, 52),

        // ── RUN/STOP / A row (y=119..170) ────────────────────────────────
        (Key::RunStop,     11, 119,  52, 52),
        (Key::ShiftLock,   65, 119,  52, 52),
        (Key::Single('A'),         119, 119,  52, 52),
        (Key::Single('S'),         173, 119,  52, 52),
        (Key::Single('D'),         227, 119,  52, 52),
        (Key::Single('F'),         281, 119,  52, 52),
        (Key::Single('G'),         335, 119,  52, 52),
        (Key::Single('H'),         389, 119,  52, 52),
        (Key::Single('J'),         443, 119,  52, 52),
        (Key::Single('K'),         497, 119,  52, 52),
        (Key::Single('L'),         551, 119,  52, 52),
        (Key::Single('['),         605, 119,  52, 52),
        (Key::Single(']'),         659, 119,  52, 52),
        (Key::Single('='),         713, 119,  52, 52),
        (Key::Return,    767, 119, 103, 52),

        // ── CBM / Z row (y=173..224) ──────────────────────────────────────
        (Key::Cbm,        11, 173,  52, 52),
        (Key::Shift,      65, 173,  79, 52),
        (Key::Single('Z'),         146, 173,  52, 52),
        (Key::Single('X'),         200, 173,  52, 52),
        (Key::Single('C'),         254, 173,  52, 52),
        (Key::Single('V'),         308, 173,  52, 52),
        (Key::Single('B'),         362, 173,  52, 52),
        (Key::Single('N'),         416, 173,  52, 52),
        (Key::Single('M'),         470, 173,  52, 52),
        (Key::Single(','),         524, 173,  52, 52),
        (Key::Single('.'),         578, 173,  52, 52),
        (Key::Single('/'),         632, 173,  52, 52),
        (Key::Shift,      686, 173,  79, 52),
        (Key::CrsrUD,    767, 173,  52, 52),
        (Key::CrsrLR,    821, 173,  49, 52),

        // ── Space bar row (y=226..289) ────────────────────────────────────
        (Key::Single(' '),     158, 226, 487, 64),

        // ── F-keys (right column, each spans a full key row) ──────────────
        (Key::F1F2,  910,  11, 90, 52),
        (Key::F3F4,  910,  65, 90, 52),
        (Key::F5F6,  910, 119, 90, 52),
        (Key::F7F8,  910, 173, 90, 52),
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

    fn just_clicked<C: Clock>(state: &KeyboardState<C>, key: Key) -> bool {
        state.flash_key.as_ref().is_some_and(|(k, _)| *k == key)
    }

    // ── key_at_pixel ──────────────────────────────────────────────────────

    #[test]
    fn pixel_inside_space_bar() {
        let state = mock_state();
        assert_eq!(state.key_at_pixel(300.0, 250.0), Some(Key::Single(' ')));
    }

    #[test]
    fn pixel_inside_a_key() {
        let state = mock_state();
        assert_eq!(state.key_at_pixel(140.0, 130.0), Some(Key::Single('A')));
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
        assert_eq!(state.key_at_pixel(940.0, 30.0), Some(Key::F1F2));
        assert_eq!(state.key_at_pixel(940.0, 80.0), Some(Key::F3F4));
        assert_eq!(state.key_at_pixel(940.0, 140.0), Some(Key::F5F6));
        assert_eq!(state.key_at_pixel(940.0, 190.0), Some(Key::F7F8));
    }

    // ── single click ──────────────────────────────────────────────────────

    #[test]
    fn single_click_sets_flash_and_status() {
        let mut state = mock_state();
        state.on_key_click(Key::Single('A'));
        assert!(just_clicked(&state, Key::Single('A')), "flash should be set to A");
        assert_eq!(state.status_message, "CLICK: A");
        assert!(state.held_key.is_none());
        assert!(state.physical_keys.contains(&Key::Single('A')));
    }

    // ── double click ──────────────────────────────────────────────────────

    #[test]
    fn classify_click_returns_double_within_threshold() {
        let mut state = mock_state();
        let now = state.clock.now();
        state.last_click = Some((Key::Single('A'), now));
        assert_eq!(state.classify_click(Key::Single('A')), ClickKind::Double);
    }

    #[test]
    fn classify_click_returns_single_after_threshold() {
        let mut state = mock_state();
        let past = state.clock.now();
        state.last_click = Some((Key::Single('A'), past));
        state.clock.advance(DOUBLE_CLICK_THRESHOLD + Duration::from_millis(1));
        assert_eq!(state.classify_click(Key::Single('A')), ClickKind::Single);
    }

    #[test]
    fn double_click_holds_key() {
        let mut state = mock_state();
        let now = state.clock.now();
        state.last_click = Some((Key::Single('A'), now));
        state.on_key_click(Key::Single('A')); // triggers Double
        assert_eq!(state.held_key, Some(Key::Single('A')));
        assert_eq!(state.status_message, "HOLD: A");
        assert!(state.physical_keys.contains(&Key::Single('A')));
    }

    #[test]
    fn double_click_on_held_key_releases_it() {
        let mut state = mock_state();
        state.held_key = Some(Key::Single('A'));
        state.physical_keys.insert(Key::Single('A'));
        let now = state.clock.now();
        state.last_click = Some((Key::Single('A'), now));
        state.on_key_click(Key::Single('A')); // triggers Double
        assert!(state.held_key.is_none());
        assert_eq!(state.status_message, "RELEASE HOLD: A");
        assert!(!state.physical_keys.contains(&Key::Single('A')));
    }

    // ── chord ──────────────────────────────────────────────────────────────

    #[test]
    fn single_click_while_holding_produces_chord() {
        let mut state = mock_state();
        state.held_key = Some(Key::Shift);
        state.physical_keys.insert(Key::Shift);
        // Ensure first click registers as single (no recent last_click)
        state.on_key_click(Key::Single('A'));
        assert_eq!(state.status_message, "CHORD: SHIFT + A pressed together, then released");
        assert!(state.held_key.is_none());
        assert!(
            !state.physical_keys.contains(&Key::Shift),
            "held key should be released from physical_keys"
        );
        assert!(
            just_clicked(&state, Key::Single('A')),
            "flash should be set to the chord key"
        );
        assert!(state.physical_keys.contains(&Key::Single('A')));
    }

    #[test]
    fn single_click_on_already_held_key_just_flashes() {
        let mut state = mock_state();
        state.held_key = Some(Key::Single('A'));
        state.physical_keys.insert(Key::Single('A'));
        state.on_key_click(Key::Single('A')); // single click on the held key itself
        // Should be treated as normal click, not chord
        assert_eq!(state.status_message, "CLICK: A");
        assert!(just_clicked(&state, Key::Single('A')));
        assert!(state.physical_keys.contains(&Key::Single('A')));
    }

    // ── physical keys ─────────────────────────────────────────────────────

    #[test]
    fn physical_key_pressed_updates_status() {
        let mut state = mock_state();
        let inserted = state.physical_key_pressed(Key::Single('Q'));
        assert!(inserted);
        assert!(state.physical_keys.contains(&Key::Single('Q')));
        assert_eq!(state.status_message, "KEY: Q");
    }

    #[test]
    fn physical_key_pressed_again_returns_false() {
        let mut state = mock_state();
        state.physical_key_pressed(Key::Single('Q'));
        let inserted = state.physical_key_pressed(Key::Single('Q'));
        assert!(!inserted);
    }

    #[test]
    fn physical_key_released_clears_status_when_empty() {
        let mut state = mock_state();
        state.physical_key_pressed(Key::Single('Q'));
        state.physical_key_released(Key::Single('Q'));
        assert!(state.physical_keys.is_empty());
        assert_eq!(state.status_message, "Click a key. Double-click toggles hold.");
    }

    #[test]
    fn physical_key_released_keeps_status_while_other_keys_held() {
        let mut state = mock_state();
        state.physical_key_pressed(Key::Single('Q'));
        state.physical_key_pressed(Key::Single('W'));
        state.physical_key_released(Key::Single('Q'));
        assert!(state.physical_keys.contains(&Key::Single('W')));
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
        state.flash_key = Some((Key::Single('A'), past));
        state.physical_keys.insert(Key::Single('A'));
        state.clock.advance(FLASH_DURATION + Duration::from_millis(1));
        state.tick_flash();
        assert!(state.flash_key.is_none());
        assert!(!state.physical_keys.contains(&Key::Single('A')));
    }

    #[test]
    fn tick_flash_keeps_active_flash() {
        let mut state = mock_state();
        let now = state.clock.now();
        state.flash_key = Some((Key::Single('A'), now));
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
        state.flash_key = Some((Key::Single('A'), now));
        assert!(state.flash_remaining().is_some());
    }
}
