pub use crate::memory::MemoryExpansion;
use crate::peripherals::joystick::JoystickDirection;

pub mod brake;
pub mod display;

#[derive(Clone, Debug, Default)]
pub struct SharedPerformanceMetrics {
    pub cycles_per_second: f64,
    pub frames_per_second: f64,
    pub total_cycles: u64,
    pub total_frames: u64,
}

pub use crate::peripherals::brake::BrakeSpeed;

#[derive(Default)]
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
    pub memory_expansion: MemoryExpansion,
    pub memory_action_pending: Option<MemoryAction>,
    pub brake_speed: BrakeSpeed,
    pub brake_action_pending: Option<BrakeAction>,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlTab {
    #[default]
    Perf,
    Io,
    Joystick,
    Memory,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryAction {
    SetExpansion(MemoryExpansion),
    Reboot,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BrakeAction {
    SetSpeed(BrakeSpeed),
}
