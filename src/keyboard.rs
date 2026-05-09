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

    pub fn step(&mut self) {
        let keys = self.read_keys();
        if !keys.is_empty() {
            log::info!("Keyboard keys pressed: {:?}", keys);
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
