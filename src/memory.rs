use std::fs;

pub struct Memory {
    pub bytes: [u8; 65536],
}

impl Default for Memory {
    fn default() -> Self {
        Self { bytes: [0; 65536] }
    }
}

const CHARACTER_ROM_START: usize = 0x8000;
const BASIC_ROM_START: usize = 0xC000;
const KERNEL_ROM_START: usize = 0xE000;

impl Memory {
    fn load_rom(&mut self, data: &[u8], start_address: usize) {
        let end_address = start_address + data.len();
        self.bytes[start_address..end_address].copy_from_slice(data);
    }

    pub fn load_standard_roms_from_data_dir(&mut self) {
        let basic_rom = fs::read("data/basic.901486-01.bin").expect("Missing basic_rom");
        let characters_rom = fs::read("data/characters.901460-03.bin").expect("Missing characters_rom");
        let kernal_rom = fs::read("data/kernal.901486-07.bin").expect("Missing kernal_rom");

        self.load_rom(&basic_rom, BASIC_ROM_START);
        self.load_rom(&characters_rom, CHARACTER_ROM_START);
        self.load_rom(&kernal_rom, KERNEL_ROM_START);
    }
}