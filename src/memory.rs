use crate::addressable::Addressable;

pub type Memory = [u8; 65536];

pub fn default() -> Memory {
    [0; 65536]
}

impl Addressable for Memory {
    fn read_byte(&self, address: u16) -> u8 {
        self[address as usize]
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        if is_ram(address) {
            self[address as usize] = value;
        }
    }
}

fn is_ram(address: u16) -> bool {
    matches!(address, 0x0000..=0x03FF | 0x1000..=0x1FFF | 0x9600..=0x97FF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ram_read_write() {
        let mut memory = default();
        let ram_address = 0x0001;

        memory.write_byte(ram_address, 0xAB);

        assert_eq!(memory.read_byte(ram_address), 0xAB);
    }

    #[test]
    fn test_rom_read_write() {
        let mut memory = default();
        let rom_address = 0x8000;

        memory[rom_address as usize] = 0xCD;
        memory.write_byte(rom_address, 0x12);

        assert_eq!(memory.read_byte(rom_address), 0xCD);
    }
}
