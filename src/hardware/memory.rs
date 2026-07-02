use crate::hardware::addressable::Addressable;
use log::info;
use std::fs;

pub const BASIC_ROM_START: usize = 0xC000;
pub const BASIC_ROM_END: usize = 0xDFFF;
pub const CHARACTER_ROM_START: usize = 0x8000;
pub const CHARACTER_ROM_END: usize = 0x8FFF;
pub const KERNEL_ROM_START: usize = 0xE000;
pub const KERNEL_ROM_END: usize = 0xFFFF;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemoryExpansion {
    #[default]
    None,
    ThreeK,
    EightK,
    SixteenK,
    TwentyFourK,
    ThirtyTwoK,
    FortyK,
}

fn build_ram_pages(expansion: MemoryExpansion) -> [bool; 256] {
    let mut pages = [false; 256];
    pages[0x00..=0x03].fill(true);
    pages[0x10..=0x1F].fill(true);
    pages[0x94..=0x97].fill(true);
    match expansion {
        MemoryExpansion::None => {}
        MemoryExpansion::ThreeK => pages[0x04..=0x0F].fill(true),
        MemoryExpansion::EightK => pages[0x20..=0x3F].fill(true),
        MemoryExpansion::SixteenK => pages[0x20..=0x5F].fill(true),
        MemoryExpansion::TwentyFourK => pages[0x20..=0x7F].fill(true),
        MemoryExpansion::ThirtyTwoK => {
            pages[0x04..=0x0F].fill(true);
            pages[0x20..=0x7F].fill(true);
        }
        MemoryExpansion::FortyK => {
            pages[0x04..=0x0F].fill(true);
            pages[0x20..=0x7F].fill(true);
            pages[0xA0..=0xBF].fill(true);
        }
    }
    pages
}

pub struct Memory {
    pub data: [u8; 65536],
    ram_pages: [bool; 256],
}

impl Default for Memory {
    fn default() -> Self {
        let expansion = MemoryExpansion::default();
        Self {
            data: [0; 65536],
            ram_pages: build_ram_pages(expansion),
        }
    }
}

impl Memory {
    pub fn copy_from_slice(&mut self, start: usize, end: usize, data: &[u8]) {
        self.data[start..end].copy_from_slice(data);
    }

    pub fn as_bytes(&self) -> &[u8; 65536] {
        &self.data
    }

    pub fn set_expansion(&mut self, expansion: MemoryExpansion) {
        self.ram_pages = build_ram_pages(expansion);
    }

    fn is_ram(&self, address: u16) -> bool {
        self.ram_pages[(address >> 8) as usize]
    }

    pub fn load_rom(&mut self, data: &[u8], rom_name: &str, start_address: usize, end_address: usize) {
        info!("Loading {} ROM", rom_name);
        let expected_len = end_address - start_address + 1;
        assert!(
            data.len() == expected_len,
            "ROM data is not expected size: expected {} bytes, got {} bytes",
            expected_len,
            data.len()
        );
        self.copy_from_slice(start_address, end_address + 1, data);
    }
}

pub fn new_memory_with_roms(expansion: MemoryExpansion) -> Memory {
    let data_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/data");
    let basic_rom = fs::read(format!("{}/basic.901486-01.bin", data_dir)).expect("Missing basic_rom");
    let characters_rom = fs::read(format!("{}/characters.901460-03.bin", data_dir)).expect("Missing characters_rom");
    let kernal_rom = fs::read(format!("{}/kernal.901486-07.bin", data_dir)).expect("Missing kernal_rom");

    let mut memory = Memory::default();
    memory.set_expansion(expansion);
    memory.load_rom(&basic_rom, "BASIC", BASIC_ROM_START, BASIC_ROM_END);
    memory.load_rom(&characters_rom, "CHARACTER", CHARACTER_ROM_START, CHARACTER_ROM_END);
    memory.load_rom(&kernal_rom, "KERNEL", KERNEL_ROM_START, KERNEL_ROM_END);
    memory
}

impl Addressable for Memory {
    fn read_byte(&self, address: u16) -> u8 {
        self.data[address as usize]
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        if self.is_ram(address) {
            self.data[address as usize] = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn memory() -> Memory {
        Memory::default()
    }

    #[rstest]
    fn test_ram_read_write(mut memory: Memory) {
        let ram_address = 0x0001;

        memory.write_byte(ram_address, 0xAB);

        assert_eq!(memory.read_byte(ram_address), 0xAB);
    }

    #[rstest]
    fn test_rom_read_write(mut memory: Memory) {
        let rom_address = 0x8000;

        memory.data[rom_address as usize] = 0xCD;
        memory.write_byte(rom_address, 0x12);

        assert_eq!(memory.read_byte(rom_address), 0xCD);
    }

    #[rstest]
    fn test_3k_expansion_ram_at_0400(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::ThreeK);
        let addr = 0x0400;

        memory.write_byte(addr, 0x42);
        assert_eq!(memory.read_byte(addr), 0x42);
    }

    #[rstest]
    fn test_3k_expansion_range_end(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::ThreeK);

        memory.write_byte(0x0FFF, 0xAB);
        assert_eq!(memory.read_byte(0x0FFF), 0xAB);
    }

    #[rstest]
    fn test_no_expansion_0400_is_read_only(mut memory: Memory) {
        let addr = 0x0400;

        memory.data[addr as usize] = 0xFF;
        memory.write_byte(addr, 0x00);

        assert_eq!(memory.read_byte(addr), 0xFF);
    }

    #[rstest]
    fn test_8k_expansion_ram_at_2000(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::EightK);
        let addr = 0x2000;

        memory.write_byte(addr, 0x42);
        assert_eq!(memory.read_byte(addr), 0x42);
    }

    #[rstest]
    fn test_8k_expansion_ram_at_3fff(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::EightK);

        memory.write_byte(0x3FFF, 0xAB);
        assert_eq!(memory.read_byte(0x3FFF), 0xAB);
    }

    #[rstest]
    fn test_8k_expansion_0400_still_read_only(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::EightK);
        let addr = 0x0400;

        memory.data[addr as usize] = 0xFF;
        memory.write_byte(addr, 0x00);

        assert_eq!(memory.read_byte(addr), 0xFF);
    }

    #[rstest]
    fn test_colour_ram_lower_half_writable(mut memory: Memory) {
        let addr = 0x9400;

        memory.write_byte(addr, 0xAB);
        assert_eq!(memory.read_byte(addr), 0xAB);
    }

    #[rstest]
    fn test_colour_ram_upper_half_writable(mut memory: Memory) {
        let addr = 0x9700;

        memory.write_byte(addr, 0xCD);
        assert_eq!(memory.read_byte(addr), 0xCD);
    }

    #[rstest]
    fn test_16k_expansion_ram_at_5fff(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::SixteenK);

        memory.write_byte(0x5FFF, 0xAB);
        assert_eq!(memory.read_byte(0x5FFF), 0xAB);
    }

    #[rstest]
    fn test_24k_expansion_ram_at_2000(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::TwentyFourK);

        memory.write_byte(0x2000, 0x42);
        assert_eq!(memory.read_byte(0x2000), 0x42);
    }

    #[rstest]
    fn test_24k_expansion_range_end(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::TwentyFourK);

        memory.write_byte(0x7FFF, 0xAB);
        assert_eq!(memory.read_byte(0x7FFF), 0xAB);
    }

    #[rstest]
    fn test_24k_expansion_no_3k_zone(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::TwentyFourK);

        memory.data[0x0400] = 0xFF;
        memory.write_byte(0x0400, 0x00);
        assert_eq!(memory.read_byte(0x0400), 0xFF);
    }

    #[rstest]
    fn test_32k_expansion_ram_at_7fff(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::ThirtyTwoK);

        memory.write_byte(0x7FFF, 0xAB);
        assert_eq!(memory.read_byte(0x7FFF), 0xAB);
    }

    #[rstest]
    fn test_32k_expansion_includes_3k_zone(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::ThirtyTwoK);

        memory.write_byte(0x0400, 0x42);
        assert_eq!(memory.read_byte(0x0400), 0x42);
    }

    #[rstest]
    fn test_40k_expansion_block_5_ram(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::FortyK);

        memory.write_byte(0xA000, 0x42);
        assert_eq!(memory.read_byte(0xA000), 0x42);
    }

    #[rstest]
    fn test_40k_expansion_block_5_range_end(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::FortyK);

        memory.write_byte(0xBFFF, 0xAB);
        assert_eq!(memory.read_byte(0xBFFF), 0xAB);
    }

    #[rstest]
    fn test_40k_expansion_includes_3k_zone(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::FortyK);

        memory.write_byte(0x0400, 0x42);
        assert_eq!(memory.read_byte(0x0400), 0x42);
    }

    #[rstest]
    fn test_40k_expansion_includes_blocks_1_to_3(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::FortyK);

        memory.write_byte(0x7000, 0x99);
        assert_eq!(memory.read_byte(0x7000), 0x99);
    }

    #[rstest]
    fn test_40k_expansion_no_ram_below_block_5(mut memory: Memory) {
        memory.set_expansion(MemoryExpansion::FortyK);

        memory.data[0x9000] = 0xFF;
        memory.write_byte(0x9000, 0x00);
        assert_eq!(memory.read_byte(0x9000), 0xFF);
    }
}
