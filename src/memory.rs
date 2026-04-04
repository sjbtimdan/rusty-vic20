use log::info;
use std::fs;

pub struct Memory {
    pub bytes: [u8; 65536],
}

impl Default for Memory {
    fn default() -> Self {
        Self { bytes: [0; 65536] }
    }
}

pub const SCREEN_RAM_START: usize = 0x1E00;
pub const SCREEN_RAM_END: usize = 0x1FFF;
pub const COLOUR_RAM_START: usize = 0x9400;
pub const COLOUR_RAM_END: usize = 0x97FF;
pub const CHARACTER_ROM_START: usize = 0x8000;
pub const CHARACTER_ROM_END: usize = 0x8FFF;
pub const BASIC_ROM_START: usize = 0xC000;
pub const BASIC_ROM_END: usize = 0xDFFF;
pub const KERNEL_ROM_START: usize = 0xE000;
pub const KERNEL_ROM_END: usize = 0xFFFF;

impl Memory {
    pub fn read_zero_page(&self, address: u8) -> u8 {
        self.bytes[address as usize]
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
        self.bytes[start_address..=end_address].copy_from_slice(data);
    }

    pub fn load_standard_roms_from_data_dir(&mut self) {
        let basic_rom = fs::read("data/basic.901486-01.bin").expect("Missing basic_rom");
        let characters_rom = fs::read("data/characters.901460-03.bin").expect("Missing characters_rom");
        let kernal_rom = fs::read("data/kernal.901486-07.bin").expect("Missing kernal_rom");

        self.load_rom(&basic_rom, "BASIC", BASIC_ROM_START, BASIC_ROM_END);
        self.load_rom(&characters_rom, "CHARACTER", CHARACTER_ROM_START, CHARACTER_ROM_END);
        self.load_rom(&kernal_rom, "KERNEL", KERNEL_ROM_START, KERNEL_ROM_END);
    }
}
