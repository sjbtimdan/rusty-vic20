mod api;
pub use api::*;
pub mod paste;
pub use paste::PasteQueue;
mod runner;
pub use runner::EmulatorRunner;

use crate::{hardware::memory::MemoryExpansion, ui::audio};
use std::{
    thread::{self, JoinHandle},
    time::Duration,
};

pub const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / 50);

pub fn spawn_emulator(
    memory_expansion: MemoryExpansion,
    audio_producer: audio::AudioProducer,
    receivers: ThreadReceivers,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("vic20-core-loop".to_string())
        .spawn(move || {
            let runner = EmulatorRunner::from_receiver(
                receivers.keyboard_receiver,
                receivers.paste_queue,
                memory_expansion,
                receivers.brake_receiver,
                audio_producer,
            );
            runner.run_loop(
                receivers.video,
                receivers.perf,
                receivers.load_queue,
                receivers.cassette_receiver,
                receivers.joystick_receiver,
                receivers.direct_loader_receiver,
                receivers.shutdown_receiver,
            )
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
