use crate::addressable::Addressable;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemoryExpansion {
    #[default]
    None,
    ThreeK,
    EightK,
    SixteenK,
    ThirtyTwoK,
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
        MemoryExpansion::ThirtyTwoK => pages[0x20..=0x7F].fill(true),
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

    #[test]
    fn test_ram_read_write() {
        let mut memory = Memory::default();
        let ram_address = 0x0001;

        memory.write_byte(ram_address, 0xAB);

        assert_eq!(memory.read_byte(ram_address), 0xAB);
    }

    #[test]
    fn test_rom_read_write() {
        let mut memory = Memory::default();
        let rom_address = 0x8000;

        memory.data[rom_address as usize] = 0xCD;
        memory.write_byte(rom_address, 0x12);

        assert_eq!(memory.read_byte(rom_address), 0xCD);
    }

    #[test]
    fn test_3k_expansion_ram_at_0400() {
        let mut memory = Memory::default();
        memory.set_expansion(MemoryExpansion::ThreeK);
        let addr = 0x0400;

        memory.write_byte(addr, 0x42);
        assert_eq!(memory.read_byte(addr), 0x42);
    }

    #[test]
    fn test_3k_expansion_range_end() {
        let mut memory = Memory::default();
        memory.set_expansion(MemoryExpansion::ThreeK);

        memory.write_byte(0x0FFF, 0xAB);
        assert_eq!(memory.read_byte(0x0FFF), 0xAB);
    }

    #[test]
    fn test_no_expansion_0400_is_read_only() {
        let mut memory = Memory::default();
        let addr = 0x0400;

        memory.data[addr as usize] = 0xFF;
        memory.write_byte(addr, 0x00);

        assert_eq!(memory.read_byte(addr), 0xFF);
    }

    #[test]
    fn test_8k_expansion_ram_at_2000() {
        let mut memory = Memory::default();
        memory.set_expansion(MemoryExpansion::EightK);
        let addr = 0x2000;

        memory.write_byte(addr, 0x42);
        assert_eq!(memory.read_byte(addr), 0x42);
    }

    #[test]
    fn test_8k_expansion_ram_at_3fff() {
        let mut memory = Memory::default();
        memory.set_expansion(MemoryExpansion::EightK);

        memory.write_byte(0x3FFF, 0xAB);
        assert_eq!(memory.read_byte(0x3FFF), 0xAB);
    }

    #[test]
    fn test_8k_expansion_0400_still_read_only() {
        let mut memory = Memory::default();
        memory.set_expansion(MemoryExpansion::EightK);
        let addr = 0x0400;

        memory.data[addr as usize] = 0xFF;
        memory.write_byte(addr, 0x00);

        assert_eq!(memory.read_byte(addr), 0xFF);
    }

    #[test]
    fn test_colour_ram_lower_half_writable() {
        let mut memory = Memory::default();
        let addr = 0x9400;

        memory.write_byte(addr, 0xAB);
        assert_eq!(memory.read_byte(addr), 0xAB);
    }

    #[test]
    fn test_colour_ram_upper_half_writable() {
        let mut memory = Memory::default();
        let addr = 0x9700;

        memory.write_byte(addr, 0xCD);
        assert_eq!(memory.read_byte(addr), 0xCD);
    }

    #[test]
    fn test_16k_expansion_ram_at_5fff() {
        let mut memory = Memory::default();
        memory.set_expansion(MemoryExpansion::SixteenK);

        memory.write_byte(0x5FFF, 0xAB);
        assert_eq!(memory.read_byte(0x5FFF), 0xAB);
    }

    #[test]
    fn test_32k_expansion_ram_at_7fff() {
        let mut memory = Memory::default();
        memory.set_expansion(MemoryExpansion::ThirtyTwoK);

        memory.write_byte(0x7FFF, 0xAB);
        assert_eq!(memory.read_byte(0x7FFF), 0xAB);
    }
}
