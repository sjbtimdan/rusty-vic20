use crate::bus::Bus;
use std::sync::mpsc::{self, Receiver, SyncSender};

pub fn make_direct_loader_channel() -> (SyncSender<Vec<u8>>, Receiver<Vec<u8>>) {
    mpsc::sync_channel(2)
}

#[derive(Default)]
pub struct DirectLoad {
    pending_data: Option<Vec<u8>>,
}

impl DirectLoad {
    pub fn set_state(&mut self, data: Vec<u8>) {
        self.pending_data = Some(data);
    }

    pub fn step(&mut self, bus: &mut Bus) {
        if let Some(data) = self.pending_data.take() {
            if data.len() < 2 {
                log::warn!("DirectLoad: data too small ({} bytes)", data.len());
                return;
            }
            let load_address = u16::from_le_bytes([data[0], data[1]]);
            let program = &data[2..];
            let max_len = 65536usize.saturating_sub(load_address as usize);
            let len = program.len().min(max_len);
            bus.load_data(load_address as usize, program);
            log::info!("DirectLoad: loaded at ${:04X} ({} bytes)", load_address, len,);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::addressable::Addressable;
    use rstest::{fixture, rstest};

    #[fixture]
    fn bus() -> Bus {
        Bus::default()
    }

    #[fixture]
    fn direct_loader() -> DirectLoad {
        DirectLoad::default()
    }

    #[rstest]
    fn step_without_data_does_nothing(mut direct_loader: DirectLoad, mut bus: Bus) {
        direct_loader.step(&mut bus);

        assert_eq!(bus.read_byte(0x1001), 0x00);
    }

    #[rstest]
    fn step_with_data_loads_into_memory(mut direct_loader: DirectLoad, mut bus: Bus) {
        let prg = vec![0x01, 0x10, 0xAA, 0xBB];
        direct_loader.set_state(prg);

        direct_loader.step(&mut bus);

        assert_eq!(bus.read_byte(0x1001), 0xAA);
        assert_eq!(bus.read_byte(0x1002), 0xBB);
    }
}
