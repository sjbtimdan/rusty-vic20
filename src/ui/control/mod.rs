use crate::peripherals::joystick::JoystickDirection;
use std::sync::{Arc, Mutex};

pub mod display;

#[derive(Clone, Debug)]
pub struct SharedPerformanceMetrics {
    pub cycles_per_second: f64,
    pub frames_per_second: f64,
    pub total_cycles: u64,
    pub total_frames: u64,
}

impl Default for SharedPerformanceMetrics {
    fn default() -> Self {
        Self {
            cycles_per_second: 0.0,
            frames_per_second: 0.0,
            total_cycles: 0,
            total_frames: 0,
        }
    }
}

pub type SharedPerfState = Arc<Mutex<SharedPerformanceMetrics>>;

pub struct ControlState {
    pub current_tab: ControlTab,
    pub cassette_playing: bool,
    pub cassette_file: Option<String>,
    pub io_action_pending: Option<IoAction>,
    pub joystick_direction: Option<JoystickDirection>,
    pub joystick_fire: bool,
    pub use_arrow_keys: bool,
    pub joystick_action_pending: Option<JoystickAction>,
    pub arrow_keys_mask: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ControlTab {
    Perf,
    Io,
    Joystick,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IoAction {
    OpenFile,
    TogglePlay,
    DirectLoad,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum JoystickAction {
    StateChanged,
}

impl Default for ControlState {
    fn default() -> Self {
        Self::new()
    }
}

impl ControlState {
    pub fn new() -> Self {
        Self {
            current_tab: ControlTab::Perf,
            cassette_playing: false,
            cassette_file: None,
            io_action_pending: None,
            joystick_direction: None,
            joystick_fire: false,
            use_arrow_keys: false,
            joystick_action_pending: None,
            arrow_keys_mask: 0,
        }
    }
}
