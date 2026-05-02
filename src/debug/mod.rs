use std::sync::{Arc, Mutex};

pub mod display;

pub const DEBUG_WINDOW_BYTES: usize = 256;

pub type SharedMemory = Arc<Mutex<[u8; 65536]>>;
pub type PendingWrites = Arc<Mutex<Vec<(u16, u8)>>>;

pub struct DebugState {
    pub start_address: u16,
    pub selected_offset: Option<usize>,
    pub address_input: String,
    pub edit_byte_input: String,
    pub mode: DebugMode,
    pub blink_phase: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DebugMode {
    Browse,
    EditingAddress,
    EditingByte,
}

impl Default for DebugState {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugState {
    pub fn new() -> Self {
        Self {
            start_address: 0x1000,
            selected_offset: None,
            address_input: String::new(),
            edit_byte_input: String::new(),
            mode: DebugMode::Browse,
            blink_phase: false,
        }
    }

    pub fn commit_address(&mut self) {
        if let Ok(addr) = u16::from_str_radix(&self.address_input, 16) {
            self.start_address = addr;
            self.selected_offset = Some(0);
            self.mode = DebugMode::Browse;
        }
        self.address_input.clear();
    }

    pub fn cancel_input(&mut self) {
        self.address_input.clear();
        self.edit_byte_input.clear();
        self.mode = DebugMode::Browse;
    }

    pub fn commit_byte_edit(&mut self) -> Option<(u16, u8)> {
        if let Ok(value) = u8::from_str_radix(&self.edit_byte_input, 16) {
            let offset = self.selected_offset?;
            let address = self.start_address.wrapping_add(offset as u16);
            self.edit_byte_input.clear();
            self.selected_offset = None;
            self.mode = DebugMode::Browse;
            return Some((address, value));
        }
        self.edit_byte_input.clear();
        self.mode = DebugMode::Browse;
        None
    }

    pub fn navigate_address(&mut self, delta: i16) {
        self.start_address = self.start_address.wrapping_add_signed(delta);
    }
}
