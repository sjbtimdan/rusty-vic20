use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

pub type SharedKeyboardState = Arc<Mutex<HashSet<String>>>;

pub struct Keyboard {
    shared_state: SharedKeyboardState,
}

impl Keyboard {
    pub fn new(shared_state: SharedKeyboardState) -> Self {
        Self { shared_state }
    }

    #[must_use]
    pub fn step(&self) -> Option<u8> {
        let keys = self.read_keys();
        if !keys.is_empty() {
            Some(0xFE) // Indicate keys are pressed
        } else {
            None
        }
    }

    fn read_keys(&self) -> HashSet<String> {
        if let Ok(keys) = self.shared_state.lock() {
            keys.clone()
        } else {
            HashSet::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use std::collections::HashSet;

    #[fixture]
    fn keyboard_with_state() -> (Keyboard, SharedKeyboardState) {
        let shared: SharedKeyboardState = Arc::new(Mutex::new(HashSet::new()));
        let keyboard = Keyboard::new(Arc::clone(&shared));
        (keyboard, shared)
    }

    #[rstest]
    fn step_returns_none_when_no_key_pressed(keyboard_with_state: (Keyboard, SharedKeyboardState)) {
        let (keyboard, _) = keyboard_with_state;
        assert_eq!(keyboard.step(), None);
    }

    #[rstest]
    fn step_returns_some_when_key_1_pressed(keyboard_with_state: (Keyboard, SharedKeyboardState)) {
        let (keyboard, shared_state) = keyboard_with_state;
        shared_state.lock().unwrap().insert("1".to_string());
        assert_eq!(keyboard.step(), Some(0xFE));
    }
}
