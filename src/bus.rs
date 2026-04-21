use crate::{addressable::Addressable, memory::Memory, tools::debug::MemoryWriteWatchpoint, vic::VIC};
use log::info;
use std::fs;

pub struct Bus {
    memory: Memory,
    vic: VIC,
    watchpoints: Vec<MemoryWriteWatchpoint>,
}

pub const SCREEN_RAM_START: u16 = 0x1E00;
pub const SCREEN_RAM_END: u16 = 0x2000;
pub const COLOUR_RAM_START: usize = 0x9400;
pub const COLOUR_RAM_END: usize = 0x97FF;
pub const CHARACTER_ROM_START: usize = 0x8000;
pub const CHARACTER_ROM_END: usize = 0x8FFF;
pub const BASIC_ROM_START: usize = 0xC000;
pub const BASIC_ROM_END: usize = 0xDFFF;
pub const KERNEL_ROM_START: usize = 0xE000;
pub const KERNEL_ROM_END: usize = 0xFFFF;
pub const VIC_REGISTERS_START: u16 = 0x9000;
pub const VIC_REGISTERS_END: u16 = 0x9010;

impl Default for Bus {
    fn default() -> Self {
        Self {
            memory: [0; 65536],
            vic: VIC::default(),
            watchpoints: vec![],
        }
    }
}

impl Addressable for Bus {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            VIC_REGISTERS_START..VIC_REGISTERS_END => self.vic.read_byte(address),
            _ => self.memory.read_byte(address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.watchpoints
            .iter()
            .for_each(|watchpoint| watchpoint.on_write(address, value));
        match address {
            VIC_REGISTERS_START..VIC_REGISTERS_END => self.vic.write_byte(address, value),
            _ => self.memory.write_byte(address, value),
        }
    }
}

impl Bus {
    pub fn add_watchpoint_at(&mut self, address: u16) {
        self.watchpoints.push(MemoryWriteWatchpoint::new(address));
    }

    pub fn step_devices(&mut self) {
        self.vic.step(&self.memory);
    }

    pub fn render_active_screen(&self) -> Vec<u32> {
        self.vic.render_active_screen(&self.memory)
    }

    pub fn border_rgba(&self) -> u32 {
        self.vic.border_rgba()
    }

    pub fn load_standard_roms_from_data_dir(&mut self) {
        let data_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/data");
        let basic_rom = fs::read(format!("{}/basic.901486-01.bin", data_dir)).expect("Missing basic_rom");
        let characters_rom =
            fs::read(format!("{}/characters.901460-03.bin", data_dir)).expect("Missing characters_rom");
        let kernal_rom = fs::read(format!("{}/kernal.901486-07.bin", data_dir)).expect("Missing kernal_rom");

        self.load_rom(&basic_rom, "BASIC", BASIC_ROM_START, BASIC_ROM_END);
        self.load_rom(&characters_rom, "CHARACTER", CHARACTER_ROM_START, CHARACTER_ROM_END);
        self.load_rom(&kernal_rom, "KERNEL", KERNEL_ROM_START, KERNEL_ROM_END);
    }

    fn load_rom(&mut self, data: &[u8], rom_name: &str, start_address: usize, end_address: usize) {
        info!("Loading {} ROM", rom_name);
        let expected_len = end_address - start_address + 1;
        assert!(
            data.len() == expected_len,
            "ROM data is not expected size: expected {} bytes, got {} bytes",
            expected_len,
            data.len()
        );
        self.memory[start_address..=end_address].copy_from_slice(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn bus() -> Bus {
        Bus::default()
    }

    #[rstest]
    fn test_write_byte_and_read_zero_page(mut bus: Bus) {
        bus.write_byte(0x42, 0xAB);
        assert_eq!(bus.read_zero_page_byte(0x42), 0xAB);
    }

    #[rstest]
    fn test_read_word_little_endian(mut bus: Bus) {
        bus.write_byte(0x0200, 0x34);
        bus.write_byte(0x0201, 0x12);
        assert_eq!(bus.read_word(0x0200), 0x1234);
    }

    #[rstest]
    fn test_set_word_little_endian(mut bus: Bus) {
        bus.write_word(0x0300, 0xABCD);
        assert_eq!(bus.read_word(0x0300), 0xABCD);
    }

    #[rstest]
    fn test_load_standard_roms_from_data_dir(mut bus: Bus) {
        bus.load_standard_roms_from_data_dir();
    }
}
