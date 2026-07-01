mod api;
pub use api::*;
pub mod paste;
mod runner;
pub use runner::EmulatorRunner;

use crate::{
    hardware::{
        bus::Bus,
        memory::{MemoryExpansion, new_memory_with_roms},
    },
    peripherals::{
        brake::{Brake, BrakeSpeed},
        cassette_player::CassettePlayer,
        direct_loader::DirectLoad,
        joystick::Joystick,
        keyboard::Keyboard,
        serial_port::SerialPort,
    },
    ui::audio,
};
use nmos6502::CPU6502;
use std::{
    collections::HashSet,
    sync::mpsc::Receiver,
    thread::{self, JoinHandle},
    time::Duration,
};

pub const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / 50);

pub fn spawn_emulator(
    memory_expansion: MemoryExpansion,
    audio_producer: audio::AudioProducer,
    keyboard_receiver: Receiver<HashSet<crate::ui::keyboard::key::Key>>,
    brake_receiver: Receiver<BrakeSpeed>,
    receivers: ThreadReceivers,
) -> JoinHandle<()> {
    let keyboard = Keyboard::new(keyboard_receiver);
    let brake = Brake::new_default(brake_receiver);
    thread::Builder::new()
        .name("vic20-core-loop".to_string())
        .spawn(move || {
            let mut bus = Bus::default();
            bus.memory = new_memory_with_roms(memory_expansion);
            let mut cpu = CPU6502::default();
            cpu.reset(&mut bus);
            EmulatorRunner::new(
                receivers,
                bus,
                cpu,
                keyboard,
                brake,
                CassettePlayer::default(),
                Joystick::default(),
                SerialPort,
                DirectLoad::default(),
                audio_producer,
            )
            .run_loop()
        })
        .expect("failed to spawn VIC-20 core thread")
}

pub fn read_prg_file(path: &str) -> Result<PrgLoadRequest, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read '{}': {}", path, e))?;
    if data.len() < 2 {
        return Err(format!("'{}' is too small to be a valid .prg file", path));
    }
    Ok(PrgLoadRequest {
        path: path.to_string(),
        data,
    })
}
