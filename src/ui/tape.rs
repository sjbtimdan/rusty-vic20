use log::info;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::{bus::Bus, cpu::cpu6502::CPU6502};

pub struct PrgLoadRequest {
    pub path: String,
    pub data: Vec<u8>,
}

pub type LoadQueue = Arc<Mutex<VecDeque<PrgLoadRequest>>>;

pub fn new_load_queue() -> LoadQueue {
    Arc::new(Mutex::new(VecDeque::new()))
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

pub fn process_load_queue(bus: &mut Bus, cpu: &mut CPU6502, queue: &LoadQueue) {
    if let Ok(mut q) = queue.try_lock() {
        while let Some(request) = q.pop_front() {
            apply_prg(bus, cpu, &request);
        }
    }
}

fn apply_prg(bus: &mut Bus, _cpu: &mut CPU6502, request: &PrgLoadRequest) {
    if request.data.len() < 2 {
        log::warn!("Skipping invalid .prg (too small): {}", request.path);
        return;
    }
    let load_address = u16::from_le_bytes([request.data[0], request.data[1]]);
    info!("Loading program into memory starting at {}", load_address);
    let program = &request.data[2..];

    let max_len = 65536usize.saturating_sub(load_address as usize);
    if program.len() > max_len {
        log::warn!(
            "Truncating .prg '{}': load ${:04X} + {} bytes exceeds 64KB",
            request.path,
            load_address,
            program.len()
        );
    }
    let len = program.len().min(max_len);
    bus.load_data(load_address as usize, program);

    log::info!(
        "Loaded '{}' at ${:04X} ({} bytes), resetting PC to ${:04X}",
        request.path,
        load_address,
        len,
        load_address
    );
    // cpu.reset(load_address);
}
